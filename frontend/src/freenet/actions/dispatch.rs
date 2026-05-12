//! Shared "fire one delegate RPC and update local state" plumbing
//! plus the mission orchestrator (which also pokes a reactive
//! presence publish on success).

use shared::{DelegateRequest as AppRequest, DelegateResponse as AppResponse};
use wasm_bindgen_futures::spawn_local;
use yew::UseStateSetter;

use crate::delegate_client;
use crate::freenet::heartbeat::heartbeat_once;
use crate::{now_ms, CoreCell, PendingCell, REACTIVE_PUBLISH_MIN_MS};

/// Generic "fire-and-update" delegate call: send `req`, expect an
/// `Inventory` (or `Error`) back, copy it into local state, set a
/// human-readable status line tagged with `label`.
pub fn delegate_op_once(
    core: CoreCell,
    pending: PendingCell,
    bump: UseStateSetter<u64>,
    req: AppRequest,
    label: &'static str,
) {
    let (ws, delegate_key) = {
        let g = core.borrow();
        let Some(c) = g.as_ref() else { return };
        let Some(ws) = c.ws.clone() else { return };
        if c.pubkey.is_none() {
            return;
        }
        (ws, c.delegate_key.clone())
    };
    spawn_local(async move {
        let result = delegate_client::call(ws, pending, &delegate_key, req).await;
        if let Some(c) = core.borrow_mut().as_mut() {
            match result {
                Ok(AppResponse::Inventory(inv)) => {
                    crate::ingest_inventory(c, inv);
                    c.status = format!("{label} ok");
                }
                Ok(AppResponse::Error(e)) => {
                    c.status = format!("{label} rejected: {e}");
                }
                Ok(other) => {
                    web_sys::console::warn_1(
                        &format!("{label} unexpected: {other:?}").into(),
                    );
                }
                Err(e) => {
                    c.status = format!("delegate transport error: {e}");
                }
            }
        }
        bump.set(now_ms());
    });
}

/// Run a single mission via the delegate. The delegate is the
/// authoritative producer of loot — the result it returns IS the
/// new inventory, and we just copy it into local state for
/// display. After a success, optionally trigger an immediate
/// publish so the leaderboard updates without waiting up to 10s
/// for the next heartbeat.
pub fn run_mission_once(core: CoreCell, pending: PendingCell, bump: UseStateSetter<u64>) {
    let (ws, delegate_key) = {
        let g = core.borrow();
        let Some(c) = g.as_ref() else { return };
        if c.mission_in_flight {
            return;
        }
        let Some(ws) = c.ws.clone() else { return };
        if c.pubkey.is_none() {
            return;
        }
        (ws, c.delegate_key.clone())
    };
    if let Some(c) = core.borrow_mut().as_mut() {
        c.mission_in_flight = true;
    }
    let now = now_ms();
    spawn_local(async move {
        let result = delegate_client::call(
            ws.clone(),
            pending.clone(),
            &delegate_key,
            AppRequest::RunMission { now_ms: now },
        )
        .await;
        let mut should_publish = false;
        if let Some(c) = core.borrow_mut().as_mut() {
            c.mission_in_flight = false;
            match result {
                Ok(AppResponse::Inventory(inv)) => {
                    crate::ingest_inventory(c, inv);
                    c.status = "subscribed".into();
                    let last = c.last_published_ms.unwrap_or(0);
                    // Skip the reactive bump entirely when the player
                    // has disabled it in Settings — the periodic
                    // heartbeat still catches up at the usual cadence.
                    if c.prefs.reactive_publish
                        && now_ms().saturating_sub(last) >= REACTIVE_PUBLISH_MIN_MS
                    {
                        should_publish = true;
                    }
                }
                Ok(AppResponse::Error(e)) => {
                    c.status = format!("mission error: {e}");
                }
                Ok(other) => {
                    web_sys::console::warn_1(
                        &format!("RunMission unexpected: {other:?}").into(),
                    );
                }
                Err(e) => {
                    c.status = format!("delegate transport error: {e}");
                }
            }
        }
        bump.set(now_ms());
        if should_publish {
            heartbeat_once(core.clone(), pending.clone(), bump.clone());
        }
    });
}
