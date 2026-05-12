//! Combat ledger — `CombatLog` (one fight) and `EncounterLog`
//! (one round in a mission chain), plus HP/regen constants.

use serde::{Deserialize, Serialize};

pub const COMBAT_OUTCOME_WIN: u8 = 0;
pub const COMBAT_OUTCOME_LOSS: u8 = 1;
pub const COMBAT_HISTORY_CAP: usize = 30;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CombatLog {
    pub area_id: u8,
    pub player_hp_start: u64,
    pub player_hp_end: u64,
    pub enemy_hp_start: u64,
    pub turns: u32,
    pub dmg_dealt: u64,
    pub dmg_taken: u64,
    pub outcome: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EncounterLog {
    pub area_id: u8,
    pub enemy_id: u16,
    pub player_hp_start: u64,
    pub player_hp_end: u64,
    pub turns: u32,
    pub dmg_dealt: u64,
    pub dmg_taken: u64,
    pub gold_gained: u64,
    pub outcome: u8,
    pub form_after: u8,
    pub timestamp_ms: u64,
}

pub const STARTING_HP: u64 = 20;
pub const HP_FULL_REGEN_MS: u64 = 60_000;
