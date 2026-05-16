//! World map — `AreaDef` table with per-area enemy stats and
//! reward multipliers, plus the World Boss era ramp.

use super::WORLD_BOSS_MAX_HP;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AreaDef {
    pub id: u8,
    pub name: &'static str,
    pub blurb: &'static str,
    pub min_level: u64,
    pub gold_mult: u64,
    pub essence_mult: u64,
    pub damage_mult: u64,
    pub enemy_hp: u64,
    pub enemy_atk: u64,
    pub enemy_def: u64,
    /// Clears required in the **predecessor** area before this one
    /// unlocks. `area_id - 1` is the predecessor today (the area
    /// chain is linear). When the map turns into a graph (backlog
    /// item C3), this generalises to "in any one of the
    /// predecessor nodes along the chosen edge."
    ///
    /// `0` for `Village Fields` (the starter — no predecessor).
    /// Tuned to feel like "a casual session in the previous zone."
    pub clears_required: u64,
}

pub const AREAS: &[AreaDef] = &[
    // Only `Boss's Lair` credits the shared World Boss (`damage_mult > 0`).
    // The earlier zones are pure XP / gold / essence farms — narratively
    // you're not even within earshot of the boss there.
    AreaDef {
        id: 0,
        name: "Village Fields",
        blurb: "easy work — balanced rewards (no boss contact)",
        min_level: 1,
        gold_mult: 1,
        essence_mult: 1,
        damage_mult: 0,
        enemy_hp: 10,
        enemy_atk: 3,
        enemy_def: 1,
        clears_required: 0,
    },
    AreaDef {
        id: 1,
        name: "Forest Road",
        blurb: "essence-rich, low danger (no boss contact)",
        min_level: 3,
        gold_mult: 2,
        essence_mult: 3,
        damage_mult: 0,
        enemy_hp: 35,
        enemy_atk: 8,
        enemy_def: 3,
        clears_required: 10,
    },
    AreaDef {
        id: 2,
        name: "Mountain Pass",
        blurb: "merchants pay well; less essence (no boss contact)",
        min_level: 6,
        gold_mult: 4,
        essence_mult: 2,
        damage_mult: 0,
        enemy_hp: 80,
        enemy_atk: 18,
        enemy_def: 8,
        clears_required: 20,
    },
    AreaDef {
        id: 3,
        name: "Boss's Lair",
        blurb: "damage-heavy; the only area that chips the World Boss",
        min_level: 10,
        gold_mult: 3,
        essence_mult: 3,
        damage_mult: 5,
        enemy_hp: 160,
        enemy_atk: 40,
        enemy_def: 18,
        clears_required: 30,
    },
];

pub fn area_of(id: u8) -> &'static AreaDef {
    AREAS.iter().find(|a| a.id == id).unwrap_or(&AREAS[0])
}

/// The predecessor area whose clear-count gates this area, if any.
/// Returns `None` for the starter (`Village Fields`). Today the chain
/// is strictly linear (`id - 1`); when the map becomes a graph
/// (backlog C3), this becomes a per-edge lookup.
pub fn area_predecessor(area_id: u8) -> Option<u8> {
    if area_id == 0 {
        None
    } else {
        Some(area_id.saturating_sub(1))
    }
}

pub fn era_max_hp(era: u64) -> u64 {
    WORLD_BOSS_MAX_HP.saturating_mul((era + 1).saturating_mul(era + 1))
}

pub fn era_threshold(era: u64) -> u64 {
    let mut total: u64 = 0;
    let mut e: u64 = 0;
    while e < era {
        total = total.saturating_add(era_max_hp(e));
        e += 1;
    }
    total
}

pub fn era_of_total(total: u64) -> u64 {
    let mut era: u64 = 0;
    let mut consumed: u64 = 0;
    while era < 50 {
        let need = era_max_hp(era);
        if consumed.saturating_add(need) > total {
            return era;
        }
        consumed = consumed.saturating_add(need);
        era += 1;
    }
    50
}
