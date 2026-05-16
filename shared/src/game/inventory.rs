//! Persistent player state. `Inventory` is the type alias to the
//! current latest version; older `InventoryVN` structs are kept so
//! `InventoryWire` can deserialize blobs from past releases.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::*;

/// Frozen V9 shape — kept so historical blobs deserialize. Mutated
/// only through `crate::rpc` so a tampered webapp cannot mint loot.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InventoryV9 {
    pub gold: u64,
    pub essence: u64,
    pub mission_count: u64,
    pub boss_damage: u64,
    pub current_area: u8,
    pub unequipped: Vec<u16>,
    pub equipped: [Option<u16>; SLOT_COUNT],
    pub potions: u32,
    pub fireballs: u32,
    pub current_hp: u64,
    pub last_hp_tick_ms: u64,
    pub last_action_ms: u64,
    pub achievement_unlocks: BTreeMap<u8, u64>,
    pub last_combat: Option<CombatLog>,
    pub current_form: u8,
    pub combat_history: Vec<EncounterLog>,
    pub wheat: u64,
    pub plot_seed: u32,
    pub shop_purchase_count: u64,
    pub experience: u64,
    pub skills_unlocked: BTreeMap<u8, u64>,
    pub forms_visited: BTreeMap<u8, u64>,
    pub ending_unlocks: BTreeMap<u8, u64>,
    pub wheat_sold_total: u64,
    /// Auto-run toggle. Persisted so closing the tab doesn't pause
    /// the adventure — delegate catches up on next load.
    pub auto_run_enabled: bool,
    /// Wall clock of the last auto-tick — lower bound of the
    /// catch-up window.
    pub auto_last_tick_ms: u64,
    /// One-shot summary of the most recent offline catch-up.
    pub last_catchup: Option<CatchupSummary>,
}

/// Compact deltas from one offline catch-up window — surfaces in
/// the welcome-back UI without diffing two `Inventory` snapshots.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CatchupSummary {
    pub started_ms: u64,
    pub ended_ms: u64,
    pub ticks_simulated: u32,
    pub missions_won: u32,
    pub missions_lost: u32,
    pub gold_gained: u64,
    pub essence_gained: u64,
    pub xp_gained: u64,
    pub boss_damage_gained: u64,
}

impl Default for InventoryV9 {
    fn default() -> Self {
        Self {
            gold: 0,
            essence: 0,
            mission_count: 0,
            boss_damage: 0,
            current_area: 0,
            unequipped: Vec::new(),
            equipped: [None; SLOT_COUNT],
            potions: 0,
            fireballs: 0,
            current_hp: STARTING_HP,
            last_hp_tick_ms: 0,
            last_action_ms: 0,
            achievement_unlocks: BTreeMap::new(),
            last_combat: None,
            current_form: FORM_HUMAN,
            combat_history: Vec::new(),
            wheat: 0,
            plot_seed: 0,
            shop_purchase_count: 0,
            experience: 0,
            skills_unlocked: BTreeMap::new(),
            forms_visited: BTreeMap::new(),
            ending_unlocks: BTreeMap::new(),
            wheat_sold_total: 0,
            auto_run_enabled: false,
            auto_last_tick_ms: 0,
            last_catchup: None,
        }
    }
}

/// V10 — adds `current_battle` for interactive tick-based combat.
/// Every other field is identical to V9; the migration in
/// `InventoryWire::into_latest` copies them across and seeds
/// `current_battle = None`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InventoryV10 {
    pub gold: u64,
    pub essence: u64,
    pub mission_count: u64,
    pub boss_damage: u64,
    pub current_area: u8,
    pub unequipped: Vec<u16>,
    pub equipped: [Option<u16>; SLOT_COUNT],
    pub potions: u32,
    pub fireballs: u32,
    pub current_hp: u64,
    pub last_hp_tick_ms: u64,
    pub last_action_ms: u64,
    pub achievement_unlocks: BTreeMap<u8, u64>,
    pub last_combat: Option<CombatLog>,
    pub current_form: u8,
    pub combat_history: Vec<EncounterLog>,
    pub wheat: u64,
    pub plot_seed: u32,
    pub shop_purchase_count: u64,
    pub experience: u64,
    pub skills_unlocked: BTreeMap<u8, u64>,
    pub forms_visited: BTreeMap<u8, u64>,
    pub ending_unlocks: BTreeMap<u8, u64>,
    pub wheat_sold_total: u64,
    pub auto_run_enabled: bool,
    pub auto_last_tick_ms: u64,
    pub last_catchup: Option<CatchupSummary>,
    /// Active interactive battle (`None` = idle). Advanced by
    /// `tick_battle`; the real-time combat panel reads it.
    pub current_battle: Option<BattleState>,
}

impl Default for InventoryV10 {
    fn default() -> Self {
        Self {
            gold: 0,
            essence: 0,
            mission_count: 0,
            boss_damage: 0,
            current_area: 0,
            unequipped: Vec::new(),
            equipped: [None; SLOT_COUNT],
            potions: 0,
            fireballs: 0,
            current_hp: STARTING_HP,
            last_hp_tick_ms: 0,
            last_action_ms: 0,
            achievement_unlocks: BTreeMap::new(),
            last_combat: None,
            current_form: FORM_HUMAN,
            combat_history: Vec::new(),
            wheat: 0,
            plot_seed: 0,
            shop_purchase_count: 0,
            experience: 0,
            skills_unlocked: BTreeMap::new(),
            forms_visited: BTreeMap::new(),
            ending_unlocks: BTreeMap::new(),
            wheat_sold_total: 0,
            auto_run_enabled: false,
            auto_last_tick_ms: 0,
            last_catchup: None,
            current_battle: None,
        }
    }
}

impl From<InventoryV9> for InventoryV10 {
    fn from(v9: InventoryV9) -> Self {
        Self {
            gold: v9.gold,
            essence: v9.essence,
            mission_count: v9.mission_count,
            boss_damage: v9.boss_damage,
            current_area: v9.current_area,
            unequipped: v9.unequipped,
            equipped: v9.equipped,
            potions: v9.potions,
            fireballs: v9.fireballs,
            current_hp: v9.current_hp,
            last_hp_tick_ms: v9.last_hp_tick_ms,
            last_action_ms: v9.last_action_ms,
            achievement_unlocks: v9.achievement_unlocks,
            last_combat: v9.last_combat,
            current_form: v9.current_form,
            combat_history: v9.combat_history,
            wheat: v9.wheat,
            plot_seed: v9.plot_seed,
            shop_purchase_count: v9.shop_purchase_count,
            experience: v9.experience,
            skills_unlocked: v9.skills_unlocked,
            forms_visited: v9.forms_visited,
            ending_unlocks: v9.ending_unlocks,
            wheat_sold_total: v9.wheat_sold_total,
            auto_run_enabled: v9.auto_run_enabled,
            auto_last_tick_ms: v9.auto_last_tick_ms,
            last_catchup: v9.last_catchup,
            current_battle: None,
        }
    }
}

/// V11 — adds `area_clears` (per-area unlock gate) and `revealed`
/// (phased-reveal bitmask, see `reveal.rs`).
///
/// Additive composition over `InventoryV10`. Bincode serializes
/// structs as concatenated fields, so `{ base, area_clears,
/// revealed }` is byte-identical to a flat 30-field layout —
/// already-persisted V11 blobs keep decoding. Use this pattern
/// only when the bump is purely additive; for remove/rename,
/// re-declare flat like `InventoryV10`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InventoryV11 {
    pub base: InventoryV10,
    /// Per-area cleared-encounter counter. Gates the next area
    /// alongside `min_level` (see `set_area`).
    pub area_clears: BTreeMap<u8, u64>,
    /// Latching bitmask of revealed UI sections. One bit per
    /// `RevealKey`; flipped by `recompute_reveals` and never
    /// cleared, so transient state doesn't hide a once-revealed
    /// panel.
    pub revealed: u64,
}

impl Default for InventoryV11 {
    fn default() -> Self {
        Self {
            base: InventoryV10::default(),
            area_clears: BTreeMap::new(),
            revealed: 0,
        }
    }
}

impl std::ops::Deref for InventoryV11 {
    type Target = InventoryV10;
    fn deref(&self) -> &InventoryV10 {
        &self.base
    }
}

impl std::ops::DerefMut for InventoryV11 {
    fn deref_mut(&mut self) -> &mut InventoryV10 {
        &mut self.base
    }
}

impl From<InventoryV10> for InventoryV11 {
    /// `area_clears` starts empty (V10's flat `mission_count` can't
    /// be split per area). `revealed` is derived from the V10 state
    /// so returning players don't lose already-accessible sections.
    fn from(v10: InventoryV10) -> Self {
        let revealed = super::reveal::derive_initial_reveals_v10(&v10);
        Self {
            base: v10,
            area_clears: BTreeMap::new(),
            revealed,
        }
    }
}

impl InventoryV11 {
    /// Clears accumulated in `area_id`. Zero if the player has
    /// never won an encounter there.
    pub fn area_clears_of(&self, area_id: u8) -> u64 {
        self.area_clears.get(&area_id).copied().unwrap_or(0)
    }

    /// Increment the clear counter for `area_id` by 1, saturating.
    pub fn area_clears_inc(&mut self, area_id: u8) {
        let entry = self.area_clears.entry(area_id).or_insert(0);
        *entry = entry.saturating_add(1);
    }
}

/// V12 — adds Estate worker economy (backlog B2) + `idle_action`
/// selector (§5.6: idle actions are mutually exclusive).
///
/// Additive composition over `InventoryV11`. Same wire-format rule
/// as V11: bincode concatenates struct fields, so persisted V12 blobs
/// are byte-identical to a flat layout. Older blobs continue to
/// decode via the `From<InventoryV11>` migration below.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InventoryV12 {
    pub base: InventoryV11,
    /// Estate state — worker counts + last yield tick.
    pub estate: super::estate::EstateState,
    /// Single active idle action (`IDLE_ACTION_*` constants). The
    /// delegate ticks exactly one of `auto_run_enabled` or the
    /// Estate yield loop depending on the value here; the legacy
    /// `auto_run_enabled` field still drives auto-mission and is
    /// kept in sync by `SetAutoRun` / `SetIdleAction`.
    pub idle_action: u8,
}

impl Default for InventoryV12 {
    fn default() -> Self {
        Self {
            base: InventoryV11::default(),
            estate: super::estate::EstateState::default(),
            idle_action: super::estate::IDLE_ACTION_NONE,
        }
    }
}

impl std::ops::Deref for InventoryV12 {
    type Target = InventoryV11;
    fn deref(&self) -> &InventoryV11 {
        &self.base
    }
}

impl std::ops::DerefMut for InventoryV12 {
    fn deref_mut(&mut self) -> &mut InventoryV11 {
        &mut self.base
    }
}

impl From<InventoryV11> for InventoryV12 {
    /// `estate` starts empty; `idle_action` derives from the legacy
    /// `auto_run_enabled` flag so returning players who had auto-mission
    /// on stay in that mode after the schema bump.
    fn from(v11: InventoryV11) -> Self {
        let idle_action = if v11.base.auto_run_enabled {
            super::estate::IDLE_ACTION_AUTO_MISSION
        } else {
            super::estate::IDLE_ACTION_NONE
        };
        Self {
            base: v11,
            estate: super::estate::EstateState::default(),
            idle_action,
        }
    }
}

/// V13 — adds Legacy / Epoch state (backlog C1). Stars are awarded
/// on every `STARS_PER_N_LEVELS` levels milestone and spent on
/// permanent multiplier nodes; Ascend soft-resets the run while
/// keeping stars + nodes.
///
/// Additive composition over V12, same wire-format rule as V11/V12.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InventoryV13 {
    pub base: InventoryV12,
    pub legacy: super::legacy::LegacyState,
}

impl Default for InventoryV13 {
    fn default() -> Self {
        Self {
            base: InventoryV12::default(),
            legacy: super::legacy::LegacyState::default(),
        }
    }
}

impl std::ops::Deref for InventoryV13 {
    type Target = InventoryV12;
    fn deref(&self) -> &InventoryV12 {
        &self.base
    }
}

impl std::ops::DerefMut for InventoryV13 {
    fn deref_mut(&mut self) -> &mut InventoryV12 {
        &mut self.base
    }
}

impl From<InventoryV12> for InventoryV13 {
    fn from(v12: InventoryV12) -> Self {
        Self {
            base: v12,
            legacy: super::legacy::LegacyState::default(),
        }
    }
}

/// Public name for "the current inventory shape". Every consumer
/// imports `Inventory`; only the persistence layer in the delegate
/// is aware that this is a versioned type.
pub type Inventory = InventoryV13;

/// On-disk wrapper. Append new variants at the end — deleting or
/// reordering breaks the bincode discriminant for existing blobs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InventoryWire {
    V9(InventoryV9),
    V10(InventoryV10),
    V11(InventoryV11),
    V12(InventoryV12),
    V13(InventoryV13),
}

impl InventoryWire {
    /// Migrate any historical variant to the current `Inventory`.
    pub fn into_latest(self) -> Inventory {
        match self {
            Self::V9(v9) => {
                InventoryV13::from(InventoryV12::from(InventoryV11::from(InventoryV10::from(v9))))
            }
            Self::V10(v10) => InventoryV13::from(InventoryV12::from(InventoryV11::from(v10))),
            Self::V11(v11) => InventoryV13::from(InventoryV12::from(v11)),
            Self::V12(v12) => InventoryV13::from(v12),
            Self::V13(v13) => v13,
        }
    }
}

impl From<Inventory> for InventoryWire {
    fn from(inv: Inventory) -> Self {
        Self::V13(inv)
    }
}
