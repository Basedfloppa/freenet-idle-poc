//! Inbound presence-contract traffic: WS response router + state
//! merges. Outbound publish path is in `crate::freenet::heartbeat`.

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
            // ClientError frames have no per-tx routing info — fail
            // every awaiting future so they don't hang forever.
            pending.borrow_mut().fail_all(msg);
            return;
        }
    };
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
            let route = if is_mailbox(&core, &key) {
                "mailbox"
            } else if is_guilds(&core, &key) {
                "guilds"
            } else {
                "presence"
            };
            let merged = match route {
                "mailbox" => merge_mailbox_update(&core, update),
                "guilds" => merge_guilds_update(&core, update),
                _ => merge_update(&core, update),
            };
            web_sys::console::log_1(
                &format!(
                    "[update] UpdateNotification route={route} key={} merged={merged}",
                    key.id()
                )
                .into(),
            );
            merged
        }
        HostResponse::ContractResponse(ContractResponse::GetResponse {
            key,
            state,
            ..
        }) => {
            let route = if is_mailbox(&core, &key) {
                "mailbox"
            } else if is_guilds(&core, &key) {
                "guilds"
            } else {
                "presence"
            };
            let bytes_len = state.as_ref().len();
            let merged = match route {
                "mailbox" => merge_mailbox_full_state(&core, state.as_ref()),
                "guilds" => merge_guilds_full_state(&core, state.as_ref()),
                _ => merge_full_state(&core, state.as_ref()),
            };
            web_sys::console::log_1(
                &format!(
                    "[update] GetResponse route={route} key={} bytes={bytes_len} merged={merged}",
                    key.id()
                )
                .into(),
            );
            merged
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

/// Replace `c.guilds` with the contract snapshot. Bad deser leaves
/// the previous mirror intact.
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

/// Guilds `update_state` always emits full state; `Delta` would
/// carry ops not state, so we ignore it and wait for the next
/// full-state notification.
pub fn merge_guilds_update(core: &CoreCell, update: UpdateData<'static>) -> usize {
    match update {
        UpdateData::State(s) => merge_guilds_full_state(core, s.as_ref()),
        UpdateData::StateAndDelta { state, delta: _ } => {
            merge_guilds_full_state(core, state.as_ref())
        }
        UpdateData::Delta(_) => 0,
        _ => 0,
    }
}

/// Replace local `others` + `cumulative_damage` with the contract's
/// authoritative snapshot. Set-membership semantics: keys absent
/// from the new state are dropped locally.
pub fn merge_full_state(core: &CoreCell, bytes: &[u8]) -> usize {
    let state = match bincode::deserialize::<ContractState>(bytes) {
        Ok(s) => s,
        Err(e) => {
            web_sys::console::warn_1(
                &format!(
                    "[presence] merge_full_state: deserialize failed bytes={} err={e}",
                    bytes.len()
                )
                .into(),
            );
            return 0;
        }
    };
    let total_entries = state.entries.len();
    let now = now_ms();
    let mut count = 0;
    let mut skipped_me = 0usize;
    let mut bad_sig = 0usize;
    let mut my_prefix = String::from("none");
    let mut entry_prefixes: Vec<String> = Vec::new();
    if let Some(c) = core.borrow_mut().as_mut() {
        let my = c.pubkey;
        if let Some(pk) = my {
            my_prefix = hex::encode(&pk[..4]);
        }
        let mut fresh: BTreeMap<PubKey, (shared::PresencePayload, u64)> = BTreeMap::new();
        for entry in state.entries.into_values() {
            let Ok(payload) = entry.verify() else {
                bad_sig += 1;
                continue;
            };
            entry_prefixes.push(hex::encode(&payload.public_key[..4]));
            if Some(payload.public_key) == my {
                skipped_me += 1;
                continue;
            }
            fresh.insert(payload.public_key, (payload, now));
            count += 1;
        }
        c.others = fresh;
        c.cumulative_damage = state.cumulative_damage;
    }
    web_sys::console::log_1(
        &format!(
            "[presence] merge_full_state entries={total_entries} others={count} skipped_me={skipped_me} bad_sig={bad_sig} my={my_prefix} entries_pk=[{}]",
            entry_prefixes.join(",")
        )
        .into(),
    );
    count
}

pub fn merge_update(core: &CoreCell, update: UpdateData<'static>) -> usize {
    match update {
        UpdateData::State(s) => merge_full_state(core, s.as_ref()),
        UpdateData::StateAndDelta { state, delta: _ } => {
            merge_full_state(core, state.as_ref())
        }
        UpdateData::Delta(d) => merge_delta_only(core, d.as_ref()),
        _ => 0,
    }
}

/// Additive LWW merge of a bare `ContractDelta`. Keys missing from
/// the delta stay in `others` — only a full state can prune.
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

/// Replace `c.mailbox` with verified messages addressed to us from
/// the new state. Signature check happens here so callers see only
/// trusted `MessagePayload`s.
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
        fresh.sort_by(|a, b| b.timestamp_ms.cmp(&a.timestamp_ms));
        c.mailbox = fresh;
        return 1;
    }
    0
}

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
                    // De-dupe by (from, ts) — same key the contract uses.
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

