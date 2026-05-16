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
