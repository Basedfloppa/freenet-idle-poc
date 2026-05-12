//! Interactive battle RPCs — queue action, tick, persist auto-run.

use shared::DelegateRequest as AppRequest;
use yew::UseStateSetter;

use crate::{now_ms, CoreCell, PendingCell};

use super::dispatch::delegate_op_once;

pub fn queue_battle_action_once(
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
        AppRequest::QueueBattleAction { action, now_ms },
        "action queued",
    );
}

pub fn tick_battle_once(core: CoreCell, pending: PendingCell, bump: UseStateSetter<u64>) {
    let now_ms = now_ms();
    delegate_op_once(
        core,
        pending,
        bump,
        AppRequest::TickBattle { now_ms },
        // Empty label avoids the per-tick "tick ok" status spam.
        // The status line is for connection lifecycle, not gameplay.
        "",
    );
}

/// Persist the auto-mission toggle on the delegate side. The
/// delegate's response carries the catch-up summary (if any) — it
/// rides on the standard `Inventory` payload via `last_catchup`,
/// which the UI reads from `c.inventory`. No extra wiring here.
pub fn set_auto_run_once(
    core: CoreCell,
    pending: PendingCell,
    bump: UseStateSetter<u64>,
    enabled: bool,
) {
    let now_ms = now_ms();
    delegate_op_once(
        core,
        pending,
        bump,
        AppRequest::SetAutoRun { enabled, now_ms },
        if enabled { "auto on" } else { "auto off" },
    );
}
