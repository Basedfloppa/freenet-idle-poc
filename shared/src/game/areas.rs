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
    /// Clears required in **any one** of `predecessors` before this
    /// area unlocks. `0` (the starter case) keeps the area open even
    /// when no predecessor is set.
    pub clears_required: u64,
    /// Upstream area ids — the player needs `clears_required` clears
    /// in at least one of these to unlock the current area (OR
    /// semantics). Empty for starter areas (no gate). Multiple
    /// entries encode the graph from backlog item C3: branches can
    /// share parents, parents can fan out.
    pub predecessors: &'static [u8],
}

pub const AREAS: &[AreaDef] = &[
    // Linear spine — Village → Forest → Mountain → Boss's Lair.
    // Branches off Forest (Deep Forest, Eastern Plains) and a
    // Snowfields branch off Mountain Pass let Wolf-form players
    // detour through essence-rich nodes while Bear-form players
    // can rush down the gold-heavy Mountain track. The Boss's Lair
    // still requires clearing one of the two pre-boss areas, but
    // either route works — backlog C3a's design intent.
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
        predecessors: &[],
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
        predecessors: &[0],
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
        predecessors: &[1],
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
        predecessors: &[2, 5],
    },
    // C3a branch off Forest Road — essence-skewed alternate that
    // shares Forest's level tier. Both nodes (4 and 1) unlock the
    // Mountain Pass, so a Wolf-form player can specialise without
    // backtracking through the main line.
    AreaDef {
        id: 4,
        name: "Deep Forest",
        blurb: "thicket runs — better essence drops, harder enemies",
        min_level: 4,
        gold_mult: 2,
        essence_mult: 5,
        damage_mult: 0,
        enemy_hp: 55,
        enemy_atk: 12,
        enemy_def: 4,
        clears_required: 15,
        predecessors: &[1],
    },
    // C3a snow branch off Mountain Pass — pre-boss alternate route
    // with higher gold yield and a stricter enemy stat block.
    AreaDef {
        id: 5,
        name: "Snowfields",
        blurb: "wind-cut plateaus — gold-rich, attrition-heavy",
        min_level: 8,
        gold_mult: 6,
        essence_mult: 2,
        damage_mult: 0,
        enemy_hp: 120,
        enemy_atk: 26,
        enemy_def: 12,
        clears_required: 25,
        predecessors: &[2],
    },
];

pub fn area_of(id: u8) -> &'static AreaDef {
    AREAS.iter().find(|a| a.id == id).unwrap_or(&AREAS[0])
}

/// Resolve an area by id with Wilds fallback. `id < WILDS_AREA_BASE`
/// hits the hardcoded `AREAS` table; `id >= WILDS_AREA_BASE`
/// generates the procedural Wilds set from `plot_seed` and
/// searches it. Returns an owned `AreaDef` because Wilds nodes
/// are dynamic — combat and `set_area` use this when they need
/// to support both worlds.
pub fn resolve_area(id: u8, plot_seed: u32) -> Option<AreaDef> {
    if id < super::wilds::WILDS_AREA_BASE {
        AREAS.iter().find(|a| a.id == id).copied()
    } else {
        super::wilds::wilds_areas(plot_seed)
            .into_iter()
            .find(|a| a.id == id)
    }
}

/// Convenience over `resolve_area` that takes `&Inventory` so
/// callers don't have to thread `current_area` + `plot_seed`
/// separately. Falls back to the starter area on a lookup miss
/// so combat / render paths never have to handle `None`.
pub fn current_area_def(inv: &super::inventory::Inventory) -> AreaDef {
    resolve_area(inv.current_area, inv.plot_seed).unwrap_or(AREAS[0])
}

/// All predecessor area ids whose clear-count can satisfy the gate
/// on `area_id`. Empty slice = starter area / no predecessor.
/// OR-semantic: the player only needs `clears_required` clears in
/// **one** of these to unlock the area (backlog C3).
pub fn area_predecessors(area_id: u8) -> &'static [u8] {
    area_of(area_id).predecessors
}

/// Helper for the "best predecessor progress" UI badge — returns
/// `(have_in_best_predecessor, clears_required)`. Picks the
/// predecessor with the highest current clear count so the badge
/// always reflects the closest-to-unlocked path. `None` means no
/// predecessor (starter area), so the gate is trivially satisfied.
pub fn area_predecessor_progress(
    area: &AreaDef,
    clear_count_of: impl Fn(u8) -> u64,
) -> Option<(u64, u64)> {
    if area.predecessors.is_empty() {
        return None;
    }
    let best = area
        .predecessors
        .iter()
        .map(|p| clear_count_of(*p))
        .max()
        .unwrap_or(0);
    Some((best, area.clears_required))
}

/// Scale a base stat by an area's `min_level`. Linear ramp:
/// `base * (10 + min_level - 1) / 10` so area level 1 stays
/// neutral (×1.0), level 10 lands at ×1.9, level 20 at ×2.9.
/// Used for enemy HP/atk/def/XP in `start_battle` and
/// `end_encounter_win` so deeper areas hit harder and pay more.
pub fn scale_by_area_level(base: u64, min_level: u64) -> u64 {
    let lvl = min_level.max(1);
    let factor = 10u64.saturating_add(lvl.saturating_sub(1));
    base.saturating_mul(factor) / 10
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
