//! Per-zone activity / Routine / Insight / Boss / Tokens RPCs.

use shared::DelegateRequest as AppRequest;
use yew::UseStateSetter;

use crate::{now_ms, CoreCell, PendingCell};

use super::dispatch::delegate_op_once;

pub fn set_activity_once(
    core: CoreCell,
    pending: PendingCell,
    bump: UseStateSetter<u64>,
    activity_id: u8,
) {
    let now_ms = now_ms();
    delegate_op_once(
        core,
        pending,
        bump,
        AppRequest::SetActivity { activity_id, now_ms },
        "set activity",
    );
}

pub fn set_routine_estate_target_once(
    core: CoreCell,
    pending: PendingCell,
    bump: UseStateSetter<u64>,
    tier_id: u8,
    target: u64,
) {
    let now_ms = now_ms();
    delegate_op_once(
        core,
        pending,
        bump,
        AppRequest::SetRoutineEstateTarget { tier_id, target, now_ms },
        "set routine target",
    );
}

pub fn buy_insight_node_once(
    core: CoreCell,
    pending: PendingCell,
    bump: UseStateSetter<u64>,
    node_id: u8,
) {
    let now_ms = now_ms();
    delegate_op_once(
        core,
        pending,
        bump,
        AppRequest::BuyInsightNode { node_id, now_ms },
        "buy insight node",
    );
}

pub fn boss_attack_once(core: CoreCell, pending: PendingCell, bump: UseStateSetter<u64>) {
    let now_ms = now_ms();
    delegate_op_once(
        core,
        pending,
        bump,
        AppRequest::BossAttack { now_ms },
        "boss attack",
    );
}

pub fn buy_token_perk_once(
    core: CoreCell,
    pending: PendingCell,
    bump: UseStateSetter<u64>,
    perk_id: u8,
) {
    let now_ms = now_ms();
    delegate_op_once(
        core,
        pending,
        bump,
        AppRequest::BuyTokenPerk { perk_id, now_ms },
        "buy token perk",
    );
}

/// Claim era-advance stars + tokens after the frontend
/// observes an era change in the presence-contract state.
pub fn claim_boss_kill_once(
    core: CoreCell,
    pending: PendingCell,
    bump: UseStateSetter<u64>,
    era: u64,
    era_max_hp: u64,
    rank: u8,
) {
    let now_ms = now_ms();
    delegate_op_once(
        core,
        pending,
        bump,
        AppRequest::ClaimBossKill { era, era_max_hp, rank, now_ms },
        "claim boss kill",
    );
}
