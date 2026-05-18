//! Wheat sell-back RPC. The legacy `work_farm_once` click was
//! removed when the Farm tab became the home tab — wheat is now
//! produced only by passive Estate Farmhand yield.

use shared::DelegateRequest as AppRequest;
use yew::UseStateSetter;

use crate::{now_ms, CoreCell, PendingCell};

use super::dispatch::delegate_op_once;

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

/// Convert `amount` essence into gold at the merchant
/// (`ESSENCE_TO_GOLD_RATE` gold/essence). `amount = 0` converts the
/// whole essence pile.
pub fn convert_essence_to_gold_once(
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
        AppRequest::ConvertEssenceToGold { amount, now_ms },
        "convert essence",
    );
}
