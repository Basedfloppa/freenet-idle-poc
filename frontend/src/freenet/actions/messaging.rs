//! Mailbox + guild op publishing — two-phase: delegate signs, then
//! we publish the resulting `MailboxEntry` / `GuildOp` to the
//! corresponding contract via `ContractOp::Update`.

use shared::{DelegateRequest as AppRequest, DelegateResponse as AppResponse};
use wasm_bindgen_futures::spawn_local;
use yew::UseStateSetter;

use crate::delegate_client;
use crate::{now_ms, CoreCell, PendingCell};

/// Run a guild op end-to-end: delegate signs → webapp publishes
/// the resulting `GuildOp` to the guilds contract as a `Delta`.
/// `name_or_id` is the trimmed guild name for CREATE, or the
/// 32-byte id hex-encoded (64 chars) for JOIN / LEAVE.
pub fn guild_op_once(
    core: CoreCell,
    pending: PendingCell,
    bump: UseStateSetter<u64>,
    op_kind: u8,
    name_or_id: String,
) {
    use freenet_stdlib::client_api::{ClientRequest, ContractRequest};
    use freenet_stdlib::prelude::{StateDelta, UpdateData};
    let (ws, delegate_key, guilds_key) = {
        let g = core.borrow();
        let Some(c) = g.as_ref() else { return };
        let Some(ws) = c.ws.clone() else { return };
        let Some(gk) = c.guilds_key.clone() else {
            web_sys::console::warn_1(&"guilds not configured".into());
            return;
        };
        if c.pubkey.is_none() {
            return;
        }
        (ws, c.delegate_key.clone(), gk)
    };
    let now = now_ms();
    spawn_local(async move {
        let signed = match delegate_client::call(
            ws.clone(),
            pending.clone(),
            &delegate_key,
            AppRequest::SignGuildOp { op_kind, name_or_id, now_ms: now },
        )
        .await
        {
            Ok(AppResponse::SignedGuildOp { payload, signature }) => {
                shared::GuildOp { payload, signature }
            }
            Ok(AppResponse::Error(e)) => {
                if let Some(c) = core.borrow_mut().as_mut() {
                    c.status = format!("guild op error: {e}");
                }
                bump.set(now_ms());
                return;
            }
            Ok(other) => {
                web_sys::console::warn_1(&format!("guild unexpected: {other:?}").into());
                return;
            }
            Err(e) => {
                if let Some(c) = core.borrow_mut().as_mut() {
                    c.status = format!("guild transport: {e}");
                }
                bump.set(now_ms());
                return;
            }
        };
        let delta = shared::GuildsDelta { ops: vec![signed] };
        let delta_bytes = match bincode::serialize(&delta) {
            Ok(b) => b,
            Err(_) => return,
        };
        let req = ClientRequest::ContractOp(ContractRequest::Update {
            key: guilds_key,
            data: UpdateData::Delta(StateDelta::from(delta_bytes)),
        });
        if let Err(e) = ws.borrow_mut().send(req).await {
            web_sys::console::warn_1(&format!("guild publish: {e:?}").into());
        }
        bump.set(now_ms());
    });
}

/// Send a mailbox message: ask the delegate to sign a
/// `MessagePayload` from us → `to`, then publish the resulting
/// `MailboxEntry` to the mailbox contract via the standard
/// `ContractOp::Update` path. Mailbox key must already be subscribed
/// (`Core.mailbox_key.is_some()`).
pub fn send_message_once(
    core: CoreCell,
    pending: PendingCell,
    bump: UseStateSetter<u64>,
    to: [u8; 32],
    kind: u8,
    body: Vec<u8>,
) {
    use freenet_stdlib::client_api::{ClientRequest, ContractRequest};
    use freenet_stdlib::prelude::{StateDelta, UpdateData};
    let (ws, delegate_key, mailbox_key) = {
        let g = core.borrow();
        let Some(c) = g.as_ref() else { return };
        let Some(ws) = c.ws.clone() else { return };
        let Some(mb) = c.mailbox_key.clone() else {
            web_sys::console::warn_1(&"mailbox not configured".into());
            return;
        };
        if c.pubkey.is_none() {
            return;
        }
        (ws, c.delegate_key.clone(), mb)
    };
    let now = now_ms();
    spawn_local(async move {
        // Phase 1 — delegate signs.
        let signed = match delegate_client::call(
            ws.clone(),
            pending.clone(),
            &delegate_key,
            AppRequest::SendMessage { to, kind, body, now_ms: now },
        )
        .await
        {
            Ok(AppResponse::SignedMessage { payload, signature }) => {
                shared::MailboxEntry { payload, signature }
            }
            Ok(AppResponse::Error(e)) => {
                web_sys::console::warn_1(&format!("send sign: {e}").into());
                return;
            }
            Ok(other) => {
                web_sys::console::warn_1(
                    &format!("send unexpected: {other:?}").into(),
                );
                return;
            }
            Err(e) => {
                web_sys::console::warn_1(&format!("send transport: {e}").into());
                return;
            }
        };
        // Phase 2 — publish as a Delta on the mailbox contract.
        let delta = shared::MailboxDelta { entries: vec![signed] };
        let delta_bytes = match bincode::serialize(&delta) {
            Ok(b) => b,
            Err(_) => return,
        };
        let req = ClientRequest::ContractOp(ContractRequest::Update {
            key: mailbox_key,
            data: UpdateData::Delta(StateDelta::from(delta_bytes)),
        });
        if let Err(e) = ws.borrow_mut().send(req).await {
            web_sys::console::warn_1(&format!("mailbox publish: {e:?}").into());
        }
        bump.set(now_ms());
    });
}
