//! Two periodic background tasks that talk to the local node:
//!   * [`heartbeat_once`] — sign + publish the player's current
//!     inventory to the presence contract so other clients can see
//!     us on the leaderboard / World Boss aggregator.
//!   * [`pull_inventory_once`] — call `LoadInventory` to refresh
//!     the local view from the delegate (HP regen, achievements
//!     unlocked while the tab was idle, cross-tab state changes).

use freenet_stdlib::client_api::{ClientRequest, ContractRequest};
use freenet_stdlib::prelude::{State, UpdateData};
use shared::{
    ContractState, DelegateRequest as AppRequest, DelegateResponse as AppResponse, SignedEntry,
};
use std::collections::BTreeMap;
use wasm_bindgen_futures::spawn_local;
use yew::UseStateSetter;

use crate::delegate_client;
use crate::{now_ms, CoreCell, PendingCell};

pub fn heartbeat_once(core: CoreCell, pending: PendingCell, bump: UseStateSetter<u64>) {
    // No contract key = single-player mode, nothing to publish.
    let (ws, name, delegate_key, contract_key) = {
        let g = core.borrow();
        let Some(c) = g.as_ref() else { return };
        let Some(ws) = c.ws.clone() else { return };
        if c.pubkey.is_none() {
            return;
        }
        // Gate first publish on settings-load so we ship the saved
        // display name, not the `DEFAULT_NAME` placeholder.
        if !c.prefs_loaded {
            return;
        }
        let Some(contract_key) = c.contract_key.clone() else {
            return;
        };
        (ws, c.name.clone(), c.delegate_key.clone(), contract_key)
    };

    spawn_local(async move {
        let ts = now_ms();
        let (payload_bytes, signature) = match delegate_client::call(
            ws.clone(),
            pending.clone(),
            &delegate_key,
            AppRequest::PublishPresence {
                name,
                area: "lobby".into(),
                now_ms: ts,
            },
        )
        .await
        {
            Ok(AppResponse::SignedPresence { payload, signature }) => (payload, signature),
            Ok(AppResponse::Error(e)) => {
                if let Some(c) = core.borrow_mut().as_mut() {
                    c.status = format!("delegate publish error: {e}");
                }
                bump.set(now_ms());
                return;
            }
            Ok(other) => {
                web_sys::console::warn_1(
                    &format!("unexpected delegate response: {other:?}").into(),
                );
                return;
            }
            Err(e) => {
                if let Some(c) = core.borrow_mut().as_mut() {
                    c.status = format!("delegate transport error: {e}");
                }
                bump.set(now_ms());
                return;
            }
        };

        // Single-entry `ContractState`; contract's `update_state`
        // merges additively, so a partial state acts as a delta.
        let signed = SignedEntry {
            payload: payload_bytes,
            signature,
        };
        let pubkey = signed
            .decode()
            .map(|p| p.public_key)
            .unwrap_or_default();
        let mut entries: BTreeMap<_, _> = BTreeMap::new();
        entries.insert(pubkey, signed);
        let state = ContractState {
            version: shared::CONTRACT_STATE_VERSION,
            entries,
            cumulative_damage: BTreeMap::new(),
        };
        let state_bytes = match bincode::serialize(&state) {
            Ok(b) => b,
            Err(_) => return,
        };
        let delta_len = state_bytes.len();
        let req = ClientRequest::ContractOp(ContractRequest::Update {
            key: contract_key,
            data: UpdateData::State(State::from(state_bytes)),
        });
        let result = ws.borrow_mut().send(req).await;
        if let Some(c) = core.borrow_mut().as_mut() {
            match result {
                Ok(()) => {
                    c.last_published_ms = Some(now_ms());
                    c.last_published = c.inventory.clone();
                    c.status = "subscribed".into();
                    let my_prefix = c
                        .pubkey
                        .map(|pk| hex::encode(&pk[..4]))
                        .unwrap_or_else(|| "none".to_string());
                    web_sys::console::log_1(
                        &format!(
                            "[presence] published heartbeat ts={ts} delta_bytes={delta_len} my={my_prefix}"
                        )
                        .into(),
                    );
                }
                Err(e) => {
                    web_sys::console::warn_1(
                        &format!("[presence] heartbeat publish error: {e:?}").into(),
                    );
                    c.status = format!("publish error: {e:?}");
                }
            }
        }
        bump.set(now_ms());
    });
}

/// Pull-refresh: ask the delegate for the latest inventory and
/// quietly merge it into local state. Doesn't update the status
/// line so we don't flicker the connection indicator every 10 s.
pub fn pull_inventory_once(core: CoreCell, pending: PendingCell, bump: UseStateSetter<u64>) {
    let (ws, delegate_key) = {
        let g = core.borrow();
        let Some(c) = g.as_ref() else { return };
        let Some(ws) = c.ws.clone() else { return };
        if c.pubkey.is_none() {
            return;
        }
        (ws, c.delegate_key.clone())
    };
    let now = now_ms();
    spawn_local(async move {
        let result = delegate_client::call(
            ws,
            pending,
            &delegate_key,
            AppRequest::LoadInventory { now_ms: now },
        )
        .await;
        if let Some(c) = core.borrow_mut().as_mut() {
            if let Ok(AppResponse::Inventory(inv)) = result {
                crate::ingest_inventory(c, inv);
            }
        }
        bump.set(now_ms());
    });
}
