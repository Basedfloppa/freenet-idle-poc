//! Gear management RPC wrappers — thin shims around `delegate_op_once`.

use shared::DelegateRequest as AppRequest;
use yew::UseStateSetter;

use crate::{now_ms, CoreCell, PendingCell};

use super::dispatch::delegate_op_once;

pub fn equip_gear_once(
    core: CoreCell,
    pending: PendingCell,
    bump: UseStateSetter<u64>,
    catalog_id: u16,
) {
    let now_ms = now_ms();
    delegate_op_once(
        core,
        pending,
        bump,
        AppRequest::EquipGear { catalog_id, now_ms },
        "equip",
    );
}

pub fn unequip_slot_once(
    core: CoreCell,
    pending: PendingCell,
    bump: UseStateSetter<u64>,
    slot: u8,
) {
    let now_ms = now_ms();
    delegate_op_once(
        core,
        pending,
        bump,
        AppRequest::UnequipSlot { slot, now_ms },
        "unequip",
    );
}

pub fn sell_gear_once(
    core: CoreCell,
    pending: PendingCell,
    bump: UseStateSetter<u64>,
    catalog_id: u16,
) {
    let now_ms = now_ms();
    delegate_op_once(
        core,
        pending,
        bump,
        AppRequest::SellGear { catalog_id, now_ms },
        "sell",
    );
}

/// Bulk-buy `count` rolls of (slot, tier). `count == 0` ≡
/// "buy max-affordable", capped at 100 in the delegate.
pub fn bulk_buy_gear_roll_once(
    core: CoreCell,
    pending: PendingCell,
    bump: UseStateSetter<u64>,
    slot: u8,
    tier: u8,
    count: u32,
) {
    let now_ms = now_ms();
    delegate_op_once(
        core, pending, bump,
        AppRequest::BulkBuyGearRoll { slot, tier, count, now_ms },
        "bulk buy gear",
    );
}

/// Bulk-sell every copy of `catalog_id` in the stash.
pub fn sell_gear_all_once(
    core: CoreCell,
    pending: PendingCell,
    bump: UseStateSetter<u64>,
    catalog_id: u16,
) {
    let now_ms = now_ms();
    delegate_op_once(
        core,
        pending,
        bump,
        AppRequest::SellGearAll { catalog_id, now_ms },
        "sell all",
    );
}

pub fn forge_upgrade_once(
    core: CoreCell,
    pending: PendingCell,
    bump: UseStateSetter<u64>,
    catalog_id: u16,
) {
    let now_ms = now_ms();
    delegate_op_once(
        core,
        pending,
        bump,
        AppRequest::ForgeUpgrade { catalog_id, now_ms },
        "forge",
    );
}

pub fn buy_gear_once(
    core: CoreCell,
    pending: PendingCell,
    bump: UseStateSetter<u64>,
    slot: u8,
    tier: u8,
) {
    let now_ms = now_ms();
    delegate_op_once(
        core,
        pending,
        bump,
        AppRequest::BuyGearRoll { slot, tier, now_ms },
        "buy gear",
    );
}

pub fn auto_equip_once(core: CoreCell, pending: PendingCell, bump: UseStateSetter<u64>) {
    let now_ms = now_ms();
    delegate_op_once(
        core,
        pending,
        bump,
        AppRequest::AutoEquipBest { now_ms },
        "auto-equip",
    );
}
