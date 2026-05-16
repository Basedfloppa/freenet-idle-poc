//! Shop RPCs — consumables and passive skills.

use shared::DelegateRequest as AppRequest;
use yew::UseStateSetter;

use crate::{now_ms, CoreCell, PendingCell};

use super::dispatch::delegate_op_once;

pub fn use_consumable_once(
    core: CoreCell,
    pending: PendingCell,
    bump: UseStateSetter<u64>,
    kind: u8,
) {
    let now_ms = now_ms();
    delegate_op_once(
        core,
        pending,
        bump,
        AppRequest::UseConsumable { kind, now_ms },
        "use",
    );
}

pub fn buy_item_once(
    core: CoreCell,
    pending: PendingCell,
    bump: UseStateSetter<u64>,
    kind: u8,
) {
    let now_ms = now_ms();
    delegate_op_once(
        core,
        pending,
        bump,
        AppRequest::BuyItem { kind, now_ms },
        "buy",
    );
}

pub fn buy_skill_once(
    core: CoreCell,
    pending: PendingCell,
    bump: UseStateSetter<u64>,
    skill_id: u8,
) {
    let now_ms = now_ms();
    delegate_op_once(
        core,
        pending,
        bump,
        AppRequest::BuySkill { skill_id, now_ms },
        "buy skill",
    );
}

/// Bulk-buy `count` consumables. `count == 0` ≡ "buy
/// max-affordable", capped at 1000 in the delegate.
pub fn bulk_buy_item_once(
    core: CoreCell,
    pending: PendingCell,
    bump: UseStateSetter<u64>,
    kind: u8,
    count: u32,
) {
    let now_ms = now_ms();
    delegate_op_once(
        core, pending, bump,
        AppRequest::BulkBuyItem { kind, count, now_ms },
        "bulk buy item",
    );
}

/// Sell `amount` copies of a consumable. `amount == 0` is the
/// "sell whole stack" shortcut.
pub fn sell_consumable_once(
    core: CoreCell,
    pending: PendingCell,
    bump: UseStateSetter<u64>,
    kind: u8,
    amount: u32,
) {
    let now_ms = now_ms();
    delegate_op_once(
        core,
        pending,
        bump,
        AppRequest::SellConsumable { kind, amount, now_ms },
        "sell consumable",
    );
}

pub fn buy_form_once(
    core: CoreCell,
    pending: PendingCell,
    bump: UseStateSetter<u64>,
    form: u8,
) {
    let now_ms = now_ms();
    delegate_op_once(
        core,
        pending,
        bump,
        AppRequest::BuyForm { form, now_ms },
        "buy form",
    );
}
