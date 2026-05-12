//! Area selection — eagerly updates `status` with the new area name
//! pulled from the returned inventory.

use shared::{DelegateRequest as AppRequest, DelegateResponse as AppResponse};
use wasm_bindgen_futures::spawn_local;
use yew::UseStateSetter;

use crate::delegate_client;
use crate::{area_of_name, now_ms, CoreCell, PendingCell};

pub fn set_area_once(
    core: CoreCell,
    pending: PendingCell,
    bump: UseStateSetter<u64>,
    area_id: u8,
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
    let now = now_ms();
    spawn_local(async move {
        let result = delegate_client::call(
            ws.clone(),
            pending.clone(),
            &delegate_key,
            AppRequest::SetArea { area_id, now_ms: now },
        )
        .await;
        if let Some(c) = core.borrow_mut().as_mut() {
            match result {
                Ok(AppResponse::Inventory(inv)) => {
                    let area_name = area_of_name(inv.current_area);
                    crate::ingest_inventory(c, inv);
                    c.status = format!("area changed to '{area_name}'");
                }
                Ok(AppResponse::Error(e)) => {
                    c.status = format!("area change rejected: {e}");
                }
                Ok(other) => {
                    web_sys::console::warn_1(
                        &format!("SetArea unexpected: {other:?}").into(),
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
