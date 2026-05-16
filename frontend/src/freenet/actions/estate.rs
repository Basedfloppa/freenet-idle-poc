//! Estate RPCs — hire workers + idle-action selector.

use shared::DelegateRequest as AppRequest;
use yew::UseStateSetter;

use crate::{now_ms, CoreCell, PendingCell};

use super::dispatch::delegate_op_once;

pub fn buy_estate_worker_once(
    core: CoreCell,
    pending: PendingCell,
    bump: UseStateSetter<u64>,
    tier_id: u8,
) {
    let now_ms = now_ms();
    delegate_op_once(
        core,
        pending,
        bump,
        AppRequest::BuyEstateWorker { tier_id, now_ms },
        "buy estate worker",
    );
}

pub fn set_idle_action_once(
    core: CoreCell,
    pending: PendingCell,
    bump: UseStateSetter<u64>,
    action: u8,
) {
    let now_ms = now_ms();
    delegate_op_once(
        core,
        pending,
        bump,
        AppRequest::SetIdleAction { action, now_ms },
        "set idle action",
    );
}
