//! Persistent player state — `Inventory` and its versioned wire
//! wrapper. `InventoryV9` is the frozen historical shape; `InventoryV10`
//! is the current canonical shape. Migration lives in `InventoryWire`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::*;

/// Every counter, gear piece, skill, ending, and timestamp the
/// delegate persists about the player. Mutated only through the
/// `crate::rpc` RPC surface, so a tampered webapp cannot mint
/// loot, equip what it doesn't own, or claim ending-state changes
/// it didn't earn.
///
/// **Naming**: this struct's *real* name is `InventoryV9` so future
/// schema bumps can introduce sibling structs (`InventoryV10`, …)
/// without renaming the existing fields. `pub type Inventory = …`
/// below points to the current version — every consumer keeps using
/// `Inventory` as a name; only the persistence layer
/// (`InventoryWire`) is aware that there are multiple versions.
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
    /// Was the player's "auto: on" toggle the last thing they
    /// touched? Persisted so closing the tab does NOT pause the
    /// adventure — the delegate catches up the time-elapsed when the
    /// player returns. Mutated only through `SetAutoRun`.
    pub auto_run_enabled: bool,
    /// Wall clock at which the delegate last simulated an auto-tick
    /// for this player. Used as the lower bound of the catch-up
    /// window. Reset to `now_ms` when `auto_run_enabled` flips on,
    /// cleared to `0` when it flips off.
    pub auto_last_tick_ms: u64,
    /// Catch-up summary from the most recent offline window — how
    /// many ticks were simulated and the resulting gold/XP/boss
    /// deltas. Surfaced as a one-shot status banner by the frontend;
    /// cleared on the next manual action.
    pub last_catchup: Option<CatchupSummary>,
}

/// Compact ledger of what happened during an offline-auto window.
/// All deltas are computed against the inventory at the start of
/// `catch_up_auto_missions` so the UI can show "you ran 23 missions
/// while away" without diffing two `Inventory` snapshots.
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
    /// Active interactive battle (none → idle). The delegate advances
    /// this in `tick_battle` and the frontend reads it for the
    /// real-time combat panel. `RunMission` / auto-mode still work:
    /// they internally start a battle, then resolve it tick-by-tick.
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
    /// Field-by-field copy from V9 → V10. `current_battle` defaults
    /// to `None` — players returning from a pre-battle-system save
    /// haven't started any interactive fights yet, so idle is the
    /// right resting state.
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

/// Public name for "the current inventory shape". Every consumer
/// imports `Inventory`; only the persistence layer in the delegate
/// is aware that this is a versioned type.
pub type Inventory = InventoryV10;

/// Disk-format wrapper around the inventory. Future schema versions
/// add new variants here (`V11(InventoryV11)`, …) and provide a
/// migration to the latest in `into_latest`. The wrapper makes
/// schema evolution non-destructive: bumping a field no longer
/// requires bumping `INVENTORY_SECRET_ID` and losing every save.
///
/// New variants must be appended at the end so existing-on-disk
/// encodings keep the same bincode discriminator. Deleting or
/// reordering variants is a breaking change.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InventoryWire {
    V9(InventoryV9),
    V10(InventoryV10),
}

impl InventoryWire {
    /// Migrate any historical variant up to the current `Inventory`
    /// shape. The match arm for the latest variant is a no-op;
    /// older variants thread through their field-by-field upgrade
    /// path. Returning `Inventory` (= latest) means every caller
    /// downstream of the secret store only ever sees one shape.
    pub fn into_latest(self) -> Inventory {
        match self {
            Self::V9(v9) => InventoryV10::from(v9),
            Self::V10(v10) => v10,
        }
    }
}

impl From<Inventory> for InventoryWire {
    fn from(inv: Inventory) -> Self {
        Self::V10(inv)
    }
}
