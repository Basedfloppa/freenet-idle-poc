//! Interactive tick-based battle — `BattleState` persisted in
//! `Inventory.current_battle`, advanced by the delegate's
//! `tick_battle` on every inventory touch.

use serde::{Deserialize, Serialize};

/// Wall-clock between consecutive combat turns. One full turn
/// = (queued player action) → (player swing) → (enemy swing).
/// 1 s feels mid-paced; fast enough to grind, slow enough to
/// react with a potion mid-fight.
pub const TURN_COOLDOWN_MS: u64 = 1_000;

/// Tail-cap on `BattleState.recent_turns`. Only used for client
/// display — full history is rebuilt from `combat_history` (which
/// the delegate updates on encounter end).
pub const BATTLE_TURN_HISTORY_CAP: usize = 10;

pub const BATTLE_ACTION_NONE: u8 = 0;
pub const BATTLE_ACTION_POTION: u8 = 1;
pub const BATTLE_ACTION_FIREBALL: u8 = 2;

/// One resolved turn of an active battle. Stored in the
/// `BattleState.recent_turns` ring so the frontend can render
/// a live combat feed without polling for every individual swing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BattleTurn {
    pub ts_ms: u64,
    /// What the player did this turn before swinging: 0 none,
    /// 1 potion (filled HP), 2 fireball (extra damage to enemy).
    pub action: u8,
    pub player_dmg: u32,
    pub enemy_dmg: u32,
    pub enemy_hp_after: u64,
    pub player_hp_after: u64,
}

/// Active battle state — persisted in `Inventory.current_battle`
/// so closing the tab doesn't void an in-progress fight. The
/// delegate advances it via `tick_battle` on every inventory touch
/// (pull/heartbeat/mission RPCs); the frontend queues actions by
/// setting `queued_action` through the dedicated RPC.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BattleState {
    /// Which encounter in the current mission chain (0..
    /// `ENCOUNTERS_PER_MISSION`). Hitting the cap ends the mission.
    pub encounter_idx: u8,
    pub enemy_id: u16,
    pub enemy_hp: u64,
    pub enemy_max_hp: u64,
    /// Wall-clock of the most recently resolved turn. Next turn
    /// fires when `now_ms - last_turn_ms >= TURN_COOLDOWN_MS`.
    pub last_turn_ms: u64,
    /// Player-queued action to apply at the start of the next turn.
    /// One slot — clicking a second action before the turn fires
    /// overwrites the first.
    pub queued_action: u8,
    pub started_ms: u64,
    /// Picks the next enemy from the area roster — derived from
    /// `mission_count` at battle start so consecutive missions
    /// rotate predictably (matches the legacy `run_mission_chain`).
    pub chain_seed: u64,
    pub recent_turns: Vec<BattleTurn>,
}
