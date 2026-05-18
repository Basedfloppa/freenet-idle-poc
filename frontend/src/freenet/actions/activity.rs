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

pub fn set_routine_gear_target_once(
    core: CoreCell,
    pending: PendingCell,
    bump: UseStateSetter<u64>,
    slot_idx: u8,
    tier: u8,
) {
    let now_ms = now_ms();
    delegate_op_once(
        core, pending, bump,
        AppRequest::SetRoutineGearTarget { slot_idx, tier, now_ms },
        "set routine gear target",
    );
}

/// One-click preset: set every form-allowed slot's routine target
/// to whatever's currently equipped there. After this, gear drops
/// of equal-or-higher tier auto-equip via `pump_gear_targets`.
pub fn lock_routine_gear_to_equipped_once(
    core: CoreCell,
    pending: PendingCell,
    bump: UseStateSetter<u64>,
) {
    let now_ms = now_ms();
    delegate_op_once(
        core, pending, bump,
        AppRequest::LockRoutineGearTargetsToEquipped { now_ms },
        "lock routine gear targets",
    );
}

pub fn set_routine_consumable_target_once(
    core: CoreCell,
    pending: PendingCell,
    bump: UseStateSetter<u64>,
    kind: u8,
    target: u32,
) {
    let now_ms = now_ms();
    delegate_op_once(
        core, pending, bump,
        AppRequest::SetRoutineConsumableTarget { kind, target, now_ms },
        "set routine consumable target",
    );
}

pub fn set_routine_auto_skill_once(
    core: CoreCell,
    pending: PendingCell,
    bump: UseStateSetter<u64>,
    enabled: bool,
) {
    let now_ms = now_ms();
    delegate_op_once(
        core, pending, bump,
        AppRequest::SetRoutineAutoSkill { enabled, now_ms },
        "set routine auto-skill",
    );
}

pub fn set_routine_activity_for_zone_once(
    core: CoreCell,
    pending: PendingCell,
    bump: UseStateSetter<u64>,
    area_id: u8,
    activity_id: u8,
) {
    let now_ms = now_ms();
    delegate_op_once(
        core, pending, bump,
        AppRequest::SetRoutineActivityForZone { area_id, activity_id, now_ms },
        "set routine activity for zone",
    );
}

pub fn set_routine_auto_equip_best_once(
    core: CoreCell,
    pending: PendingCell,
    bump: UseStateSetter<u64>,
    enabled: bool,
) {
    let now_ms = now_ms();
    delegate_op_once(
        core, pending, bump,
        AppRequest::SetRoutineAutoEquipBest { enabled, now_ms },
        "set routine auto-equip best",
    );
}

pub fn set_routine_offline_cap_hours_once(
    core: CoreCell,
    pending: PendingCell,
    bump: UseStateSetter<u64>,
    hours: u8,
) {
    let now_ms = now_ms();
    delegate_op_once(
        core, pending, bump,
        AppRequest::SetRoutineOfflineCapHours { hours, now_ms },
        "set routine offline cap",
    );
}

pub fn set_routine_mission_cycle_once(
    core: CoreCell,
    pending: PendingCell,
    bump: UseStateSetter<u64>,
    mode: u8,
    areas: Vec<u8>,
) {
    let now_ms = now_ms();
    delegate_op_once(
        core, pending, bump,
        AppRequest::SetRoutineMissionCycle { mode, areas, now_ms },
        "set routine mission cycle",
    );
}

pub fn set_routine_combat_speed_once(
    core: CoreCell,
    pending: PendingCell,
    bump: UseStateSetter<u64>,
    mult_bp: u32,
) {
    let now_ms = now_ms();
    delegate_op_once(
        core, pending, bump,
        AppRequest::SetRoutineCombatSpeed { mult_bp, now_ms },
        "set routine combat speed",
    );
}

pub fn set_public_cosmetics_once(
    core: CoreCell,
    pending: PendingCell,
    bump: UseStateSetter<u64>,
    motto: String,
    accent: u8,
    frame: u8,
) {
    let now_ms = now_ms();
    delegate_op_once(
        core, pending, bump,
        AppRequest::SetPublicCosmetics { motto, accent, frame, now_ms },
        "set public cosmetics",
    );
}

pub fn claim_daily_checkin_once(
    core: CoreCell,
    pending: PendingCell,
    bump: UseStateSetter<u64>,
) {
    let now_ms = now_ms();
    delegate_op_once(
        core, pending, bump,
        AppRequest::ClaimDailyCheckin { now_ms },
        "claim daily checkin",
    );
}

pub fn set_routine_battle_policy_once(
    core: CoreCell,
    pending: PendingCell,
    bump: UseStateSetter<u64>,
    policy: shared::BattleActionPolicy,
) {
    let now_ms = now_ms();
    delegate_op_once(
        core, pending, bump,
        AppRequest::SetRoutineBattlePolicy { policy, now_ms },
        "set routine battle policy",
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

pub fn buy_insight_node_bulk_once(
    core: CoreCell,
    pending: PendingCell,
    bump: UseStateSetter<u64>,
    node_id: u8,
    count: u32,
) {
    let now_ms = now_ms();
    delegate_op_once(
        core, pending, bump,
        AppRequest::BuyInsightNodeBulk { node_id, count, now_ms },
        "buy insight node bulk",
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
