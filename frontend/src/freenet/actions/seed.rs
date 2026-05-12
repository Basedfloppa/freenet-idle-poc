//! Identity-management RPCs — reset inventory and export the seed.
//! Seed bytes are sensitive; they flow through a callback rather
//! than being stashed in `Core`.

use shared::{DelegateRequest as AppRequest, DelegateResponse as AppResponse};
use yew::UseStateSetter;

use crate::{now_ms, CoreCell, PendingCell};

use super::dispatch::delegate_op_once;

pub fn reset_inventory_once(core: CoreCell, pending: PendingCell, bump: UseStateSetter<u64>) {
    let now_ms = now_ms();
    delegate_op_once(
        core,
        pending,
        bump,
        AppRequest::ResetInventory { now_ms },
        "progress reset",
    );
}

/// Request the Ed25519 seed bytes from the delegate. Returns the
/// raw 32 bytes on success — the caller is responsible for hex /
/// base58 / passphrase encoding before showing them to the user.
/// Result is delivered through the callback once the WS round-trip
/// completes; we don't store the seed in `Core` (sensitive).
pub fn export_seed_once<F>(
    core: CoreCell,
    pending: PendingCell,
    on_result: F,
) where
    F: 'static + FnOnce(Result<[u8; 32], String>),
{
    let (ws, delegate_key) = {
        let g = core.borrow();
        let Some(c) = g.as_ref() else {
            on_result(Err("core not ready".into()));
            return;
        };
        let Some(ws) = c.ws.clone() else {
            on_result(Err("no WS connection".into()));
            return;
        };
        (ws, c.delegate_key.clone())
    };
    wasm_bindgen_futures::spawn_local(async move {
        let result = crate::delegate_client::call(
            ws,
            pending,
            &delegate_key,
            AppRequest::ExportSeed,
        )
        .await;
        let outcome = match result {
            Ok(AppResponse::Seed { seed }) => Ok(seed),
            Ok(AppResponse::Error(e)) => Err(e),
            Ok(other) => Err(format!("unexpected: {other:?}")),
            Err(e) => Err(e),
        };
        on_result(outcome);
    });
}
