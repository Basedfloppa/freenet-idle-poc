//! Two periodic background tasks that talk to the local node:
//!   * [`heartbeat_once`] — sign + publish the player's current
//!     inventory to the presence contract so other clients can see
//!     us on the leaderboard / World Boss aggregator.
//!   * [`pull_inventory_once`] — call `LoadInventory` to refresh
//!     the local view from the delegate (HP regen, achievements
//!     unlocked while the tab was idle, cross-tab state changes).

use freenet_stdlib::client_api::{ClientRequest, ContractRequest};
use freenet_stdlib::prelude::{StateDelta, UpdateData};
use shared::{
    ContractDelta, DelegateRequest as AppRequest, DelegateResponse as AppResponse, SignedEntry,
};
use wasm_bindgen_futures::spawn_local;
use yew::UseStateSetter;

use crate::delegate_client;
use crate::{now_ms, CoreCell, PendingCell};

pub fn heartbeat_once(core: CoreCell, pending: PendingCell, bump: UseStateSetter<u64>) {
    // We no longer pre-build the payload on the webapp side — the
    // delegate is authoritative. Pull only the fields the player
    // owns (`name`) plus the WS handles and contract key.
    let (ws, name, delegate_key, contract_key) = {
        let g = core.borrow();
        let Some(c) = g.as_ref() else { return };
        let Some(ws) = c.ws.clone() else { return };
        if c.pubkey.is_none() {
            return;
        }
        (
            ws,
            c.name.clone(),
            c.delegate_key.clone(),
            c.contract_key.clone(),
        )
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

        let signed = SignedEntry { payload: payload_bytes, signature };
        let delta = ContractDelta { entries: vec![signed] };
        let delta_bytes = match bincode::serialize(&delta) {
            Ok(b) => b,
            Err(_) => return,
        };
        let req = ClientRequest::ContractOp(ContractRequest::Update {
            key: contract_key,
            data: UpdateData::Delta(StateDelta::from(delta_bytes)),
        });
        let result = ws.borrow_mut().send(req).await;
        if let Some(c) = core.borrow_mut().as_mut() {
            match result {
                Ok(()) => {
                    c.last_published_ms = Some(now_ms());
                    c.last_published = c.inventory.clone();
                    c.status = "subscribed".into();
                }
                Err(e) => c.status = format!("publish error: {e:?}"),
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
