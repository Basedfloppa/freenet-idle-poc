//! Farm RPCs — wheat work / sell.

use shared::DelegateRequest as AppRequest;
use yew::UseStateSetter;

use crate::{now_ms, CoreCell, PendingCell};

use super::dispatch::delegate_op_once;

pub fn work_farm_once(core: CoreCell, pending: PendingCell, bump: UseStateSetter<u64>) {
    let now_ms = now_ms();
    delegate_op_once(core, pending, bump, AppRequest::WorkFarm { now_ms }, "farm");
}

pub fn sell_wheat_once(
    core: CoreCell,
    pending: PendingCell,
    bump: UseStateSetter<u64>,
    amount: u64,
) {
    let now_ms = now_ms();
    delegate_op_once(
        core,
        pending,
        bump,
        AppRequest::SellWheat { amount, now_ms },
        "sell wheat",
    );
}
