//! Inbound presence-contract traffic: WebSocket response router,
//! state merge for fresh `Get` responses and incremental
//! `UpdateNotification` frames, and the contract-key parser.
//!
//! Everything else in this module is "respond to bytes coming
//! from the local node" — the outbound publish path lives in
//! `crate::freenet::heartbeat`.

use freenet_stdlib::client_api::{ContractResponse, HostResponse};
use freenet_stdlib::prelude::{
    CodeHash, ContractInstanceId, ContractKey, OutboundDelegateMsg, UpdateData,
};

use shared::{ContractDelta, ContractState, PubKey};
use std::collections::BTreeMap;

use crate::{now_ms, CoreCell, PendingCell};
use yew::UseStateSetter;

pub fn parse_contract_key(
    contract_id_b58: &str,
    code_hash_b58: &str,
) -> Result<ContractKey, String> {
    let instance_id = ContractInstanceId::try_from(contract_id_b58.to_string())
        .map_err(|e| format!("bad contract id: {e}"))?;
    let code_hash_bytes = bs58::decode(code_hash_b58)
        .with_alphabet(bs58::Alphabet::BITCOIN)
        .into_vec()
        .map_err(|e| format!("bad code hash base58: {e}"))?;
    let arr: [u8; 32] = code_hash_bytes
        .as_slice()
        .try_into()
        .map_err(|_| "code hash must be 32 bytes".to_string())?;
    Ok(ContractKey::from_id_and_code(instance_id, CodeHash::from(&arr)))
}

pub fn handle_response(
    resp: Result<HostResponse, freenet_stdlib::client_api::ClientError>,
    core: CoreCell,
    pending: PendingCell,
    bump: UseStateSetter<u64>,
) {
    let resp = match resp {
        Ok(r) => r,
        Err(e) => {
            let msg = format!("ws response error: {e}");
            web_sys::console::warn_1(&msg.clone().into());
            // ClientError frames have no per-tx routing info, so any
            // in-flight delegate `OneshotRx` would otherwise hang
            // forever waiting on a response that will never come (the
            // user-visible symptom was "asking delegate for
            // identity…" stuck on boot). Fail every awaiting future
            // with the same reason so the call site bubbles it up to
            // the status line. False positives are acceptable —
            // worst case is a spurious retry on the next reconnect.
            pending.borrow_mut().fail_all(msg);
            return;
        }
    };
    // Routing: contract responses can come from either the presence
    // contract or the (optional) mailbox contract. They carry the
    // originating `key` so we dispatch by instance_id; everything
    // else stays in the presence path.
    let merged = match resp {
        HostResponse::DelegateResponse { values, .. } => {
            for v in values {
                if let OutboundDelegateMsg::ApplicationMessage(app) = v {
                    pending.borrow_mut().deliver(&app.payload);
                }
            }
            0
        }
        HostResponse::ContractResponse(ContractResponse::SubscribeResponse { .. }) => 0,
        HostResponse::ContractResponse(ContractResponse::UpdateNotification {
            key,
            update,
        }) => {
            if is_mailbox(&core, &key) {
                merge_mailbox_update(&core, update)
            } else if is_guilds(&core, &key) {
                merge_guilds_update(&core, update)
            } else {
                merge_update(&core, update)
            }
        }
        HostResponse::ContractResponse(ContractResponse::GetResponse {
            key,
            state,
            ..
        }) => {
            if is_mailbox(&core, &key) {
                merge_mailbox_full_state(&core, state.as_ref())
            } else if is_guilds(&core, &key) {
                merge_guilds_full_state(&core, state.as_ref())
            } else {
                merge_full_state(&core, state.as_ref())
            }
        }
        HostResponse::ContractResponse(ContractResponse::UpdateResponse { .. }) => {
            mark_publish_success(&core)
        }
        _ => 0,
    };
    if merged > 0 {
        bump.set(now_ms());
    }
}

fn is_mailbox(core: &CoreCell, key: &ContractKey) -> bool {
    let g = core.borrow();
    let Some(c) = g.as_ref() else { return false };
    c.mailbox_key
        .as_ref()
        .map(|mb| mb.id() == key.id())
        .unwrap_or(false)
}

fn is_guilds(core: &CoreCell, key: &ContractKey) -> bool {
    let g = core.borrow();
    let Some(c) = g.as_ref() else { return false };
    c.guilds_key
        .as_ref()
        .map(|gl| gl.id() == key.id())
        .unwrap_or(false)
}

/// Replace `c.guilds` with the contract snapshot. Guilds state is a
/// materialized view (not a delta log), so we just deserialize and
/// store. Bad deser leaves the previous mirror intact.
pub fn merge_guilds_full_state(core: &CoreCell, bytes: &[u8]) -> usize {
    let Ok(state) = bincode::deserialize::<shared::GuildsState>(bytes) else {
        return 0;
    };
    if let Some(c) = core.borrow_mut().as_mut() {
        c.guilds = state;
        return 1;
    }
    0
}

/// Guilds contract `update_state` always replies with a fresh full
/// state (per the contract's `Delta` branch comment). Treat any
/// update notification — `State`, `StateAndDelta`, or even `Delta`
/// — as a signal to refetch by accepting the most useful side.
pub fn merge_guilds_update(core: &CoreCell, update: UpdateData<'static>) -> usize {
    match update {
        UpdateData::State(s) => merge_guilds_full_state(core, s.as_ref()),
        UpdateData::StateAndDelta { state, delta: _ } => {
            merge_guilds_full_state(core, state.as_ref())
        }
        // A bare `Delta` from this contract carries ops, not state.
        // Replaying it locally would diverge from the delegate's
        // canonical apply — easier to wait for the next full-state
        // notification (the contract emits one per update).
        UpdateData::Delta(_) => 0,
        _ => 0,
    }
}

/// Replace local `others` + `cumulative_damage` with the contract's
/// authoritative snapshot. Set-membership semantics: any key absent
/// from the new state is dropped locally, so a pruned-from-contract
/// entry no longer lingers in viewers' tabs (root cause of the
/// "long-session shows ghosts" divergence).
pub fn merge_full_state(core: &CoreCell, bytes: &[u8]) -> usize {
    let Ok(state) = bincode::deserialize::<ContractState>(bytes) else {
        return 0;
    };
    let now = now_ms();
    let mut count = 0;
    if let Some(c) = core.borrow_mut().as_mut() {
        let my = c.pubkey;
        let mut fresh: BTreeMap<PubKey, (shared::PresencePayload, u64)> = BTreeMap::new();
        for entry in state.entries.into_values() {
            let Ok(payload) = entry.verify() else { continue };
            if Some(payload.public_key) == my {
                continue;
            }
            fresh.insert(payload.public_key, (payload, now));
            count += 1;
        }
        c.others = fresh;
        c.cumulative_damage = state.cumulative_damage;
    }
    count
}

pub fn merge_update(core: &CoreCell, update: UpdateData<'static>) -> usize {
    // The presence contract's `update_state` always returns a full
    // `State`, so in practice we land in the `State` /
    // `StateAndDelta` branches. The `Delta`-only branch is kept for
    // robustness against future contract changes — it merges
    // additively without touching `cumulative_damage` (the next full
    // state reconciles it).
    match update {
        UpdateData::State(s) => merge_full_state(core, s.as_ref()),
        UpdateData::StateAndDelta { state, delta: _ } => {
            // The state half is already the post-delta snapshot; the
            // delta half would just re-apply the same entries, so we
            // ignore it and treat the state as authoritative.
            merge_full_state(core, state.as_ref())
        }
        UpdateData::Delta(d) => merge_delta_only(core, d.as_ref()),
        _ => 0,
    }
}

/// Fallback path: merge a bare `ContractDelta` (no membership info).
/// Inserts/updates entries via LWW but does NOT remove keys missing
/// from the delta, since a delta is purely additive.
fn merge_delta_only(core: &CoreCell, bytes: &[u8]) -> usize {
    let Ok(delta) = bincode::deserialize::<ContractDelta>(bytes) else {
        return 0;
    };
    let now = now_ms();
    let mut count = 0;
    if let Some(c) = core.borrow_mut().as_mut() {
        let my = c.pubkey;
        for entry in delta.entries {
            let Ok(payload) = entry.verify() else { continue };
            if Some(payload.public_key) == my {
                continue;
            }
            let newer = c
                .others
                .get(&payload.public_key)
                .map_or(true, |(p, _)| payload.timestamp_ms > p.timestamp_ms);
            if newer {
                // Mirror the contract's cumulative-damage maintenance
                // so the local view stays close to the contract's
                // ledger even on delta-only updates.
                let slot = c
                    .cumulative_damage
                    .entry(payload.public_key)
                    .or_insert(0);
                if payload.boss_damage > *slot {
                    *slot = payload.boss_damage;
                }
                c.others.insert(payload.public_key, (payload, now));
                count += 1;
            }
        }
    }
    count
}

pub fn mark_publish_success(core: &CoreCell) -> usize {
    if let Some(c) = core.borrow_mut().as_mut() {
        c.last_published_ms = Some(now_ms());
        c.last_published = c.inventory.clone();
    }
    1
}

/// Mailbox full-state merge: replace `c.mailbox` with every message
/// addressed to us in the new state. Verification + filtering
/// happens here; the rest of the app only sees decoded
/// `MessagePayload` values it can trust.
pub fn merge_mailbox_full_state(core: &CoreCell, bytes: &[u8]) -> usize {
    let Ok(state) = bincode::deserialize::<shared::MailboxState>(bytes) else {
        return 0;
    };
    if let Some(c) = core.borrow_mut().as_mut() {
        let my = c.pubkey;
        let mut fresh: Vec<shared::MessagePayload> = Vec::new();
        for entry in state.entries.into_iter() {
            let Ok(payload) = entry.verify() else { continue };
            if Some(payload.to) == my {
                fresh.push(payload);
            }
        }
        // Newest first — most recent inbox at the top.
        fresh.sort_by(|a, b| b.timestamp_ms.cmp(&a.timestamp_ms));
        c.mailbox = fresh;
        return 1;
    }
    0
}

/// Mailbox delta merge: contract returns `UpdateData::State(...)`
/// (full state) from `update_state`, so the common path lands here
/// via `merge_mailbox_full_state`. The `Delta`-only branch handles
/// raw `MailboxDelta` frames (additive — append new verified
/// recipient-matched messages, no removal).
pub fn merge_mailbox_update(core: &CoreCell, update: UpdateData<'static>) -> usize {
    match update {
        UpdateData::State(s) => merge_mailbox_full_state(core, s.as_ref()),
        UpdateData::StateAndDelta { state, delta: _ } => {
            merge_mailbox_full_state(core, state.as_ref())
        }
        UpdateData::Delta(d) => {
            let Ok(delta) = bincode::deserialize::<shared::MailboxDelta>(d.as_ref()) else {
                return 0;
            };
            let mut count = 0;
            if let Some(c) = core.borrow_mut().as_mut() {
                let my = c.pubkey;
                for entry in delta.entries.into_iter() {
                    let Ok(payload) = entry.verify() else { continue };
                    if Some(payload.to) != my {
                        continue;
                    }
                    // De-dupe by (from, ts) — same rule as the
                    // contract uses.
                    let dup = c
                        .mailbox
                        .iter()
                        .any(|m| m.from == payload.from && m.timestamp_ms == payload.timestamp_ms);
                    if dup {
                        continue;
                    }
                    c.mailbox.push(payload);
                    count += 1;
                }
                c.mailbox
                    .sort_by(|a, b| b.timestamp_ms.cmp(&a.timestamp_ms));
            }
            count
        }
        _ => 0,
    }
}

