//! Periodic `ContractRequest::Get` on the presence contract — a
//! workaround for the freenet-core bug where `UpdateNotification` is
//! never delivered to subscribers of a locally-hosted contract
//! (observed 2026-05-15: 165+ publishes / 0 notifications on orange
//! + baka, with the `9a3JA8E6...` presence contract flagged
//! `is_locally_hosted=true`).
//!
//! Our local node still applies and caches the full state on every
//! Update — it just doesn't surface it. A bare `Get` (subscribe=false)
//! returns from the local cache fast (`Returning locally cached
//! contract state` in node logs), the response flows through the
//! existing `merge_full_state` path, and `c.others` refreshes. Cost
//! is one extra GET per `pull_ms` (default 10 s) — negligible
//! compared to the broken-leaderboard alternative.
//!
//! Remove this once
//! `commit_state_update → BroadcastStateChange → UpdateNotification`
//! propagation is fixed upstream (filed as TODO).

use freenet_stdlib::client_api::{ClientRequest, ContractRequest};
use wasm_bindgen_futures::spawn_local;
use yew::UseStateSetter;

use crate::{now_ms, CoreCell, PendingCell};

/// Send a non-subscribing `Get` on the presence contract. Response
/// is routed via the normal WS handler to `merge_full_state`. No
/// awaiting — bump is fired by the merge path itself on success.
///
/// Silently no-ops if WS isn't connected, identity isn't loaded yet,
/// or no presence contract is configured (single-player mode).
pub fn pull_presence_state_once(
    core: CoreCell,
    _pending: PendingCell,
    _bump: UseStateSetter<u64>,
) {
    let (ws, contract_key) = {
        let g = core.borrow();
        let Some(c) = g.as_ref() else { return };
        let Some(ws) = c.ws.clone() else { return };
        if c.pubkey.is_none() {
            return;
        }
        let Some(contract_key) = c.contract_key.clone() else {
            return;
        };
        (ws, contract_key)
    };
    spawn_local(async move {
        let req = ClientRequest::ContractOp(ContractRequest::Get {
            key: *contract_key.id(),
            return_contract_code: false,
            subscribe: false,
            blocking_subscribe: false,
        });
        if let Err(e) = ws.borrow_mut().send(req).await {
            web_sys::console::warn_1(
                &format!("[pull_presence] Get transport: {e:?}").into(),
            );
        }
        // Response — including the merge into `c.others` and the bump
        // that drives a re-render — happens asynchronously when the
        // WS frame arrives at `freenet::contract::handle_response`.
        let _ = now_ms;
    });
}
