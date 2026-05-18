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
/// Wall-clock ms for HP to refill from 0 to max while not in a
/// mission. Tuned 2026-05-18: halved from 60s → 30s so the death
/// penalty doesn't dominate the AFK rhythm. The HP regen banner
/// still surfaces the wait, and a 30s window is short enough that a
/// player checking back in rarely loses meaningful AFK time. If a
/// future playtest finds the death penalty trivialised, bump back
/// toward 45–60s.
pub const HP_FULL_REGEN_MS: u64 = 30_000;
