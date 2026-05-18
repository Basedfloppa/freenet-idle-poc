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
    /// Locked to `EstateStateV1` to keep V12 bytes byte-identical to
    /// what production writes; future extensions ship as
    /// `EstateStateV2` inside a new `InventoryV(N+1)` (same pattern
    /// as `RoutineStateV1`).
    pub estate: super::estate::EstateStateV1,
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
            estate: super::estate::EstateStateV1::default(),
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
            estate: super::estate::EstateStateV1::default(),
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
    /// Locked to `LegacyStateV1` for wire compatibility — see
    /// the wire-format rule on `LegacyStateV1` and the canonical
    /// `RoutineStateV1`/`V2` example in `routine.rs`.
    pub legacy: super::legacy::LegacyStateV1,
}

impl Default for InventoryV13 {
    fn default() -> Self {
        Self {
            base: InventoryV12::default(),
            legacy: super::legacy::LegacyStateV1::default(),
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
            legacy: super::legacy::LegacyStateV1::default(),
        }
    }
}

/// V14 — adds per-zone activities (A1), Routine auto-hire (B1),
/// Insight currency (B5), and Tokens (C2). Additive composition
/// over V13, same wire-format rule as V11/V12/V13.
///
/// `routine` is locked to `RoutineStateV1` (frozen 1-field shape)
/// to keep V14 bytes byte-identical to what production writes.
/// V17 introduces the V2 routine shape via field-shadowing; see
/// `InventoryV17` below.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InventoryV14 {
    pub base: InventoryV13,
    /// Selected per-zone activity id; `ACTIVITY_NONE = 0` means
    /// none chosen. Only meaningful when `idle_action ==
    /// IDLE_ACTION_ACTIVITY`.
    pub active_activity: u8,
    /// Wall-clock anchor for activity accrual — analogue of
    /// `estate.last_tick_ms`.
    pub activity_last_tick_ms: u64,
    pub routine: super::routine::RoutineStateV1,
    /// Locked to `InsightStateV1` — see wire-format rule on that
    /// type and the canonical `RoutineStateV1`/`V2` example.
    pub insight: super::insight::InsightStateV1,
    /// Locked to `TokenStateV1` for the same reason as `insight`.
    pub tokens: super::tokens::TokenStateV1,
}

impl Default for InventoryV14 {
    fn default() -> Self {
        Self {
            base: InventoryV13::default(),
            active_activity: super::activities::ACTIVITY_NONE,
            activity_last_tick_ms: 0,
            routine: super::routine::RoutineStateV1::default(),
            insight: super::insight::InsightStateV1::default(),
            tokens: super::tokens::TokenStateV1::default(),
        }
    }
}

impl std::ops::Deref for InventoryV14 {
    type Target = InventoryV13;
    fn deref(&self) -> &InventoryV13 {
        &self.base
    }
}

impl std::ops::DerefMut for InventoryV14 {
    fn deref_mut(&mut self) -> &mut InventoryV13 {
        &mut self.base
    }
}

impl From<InventoryV13> for InventoryV14 {
    fn from(v13: InventoryV13) -> Self {
        Self {
            base: v13,
            active_activity: super::activities::ACTIVITY_NONE,
            activity_last_tick_ms: 0,
            routine: super::routine::RoutineStateV1::default(),
            insight: super::insight::InsightStateV1::default(),
            tokens: super::tokens::TokenStateV1::default(),
        }
    }
}

/// V15 — adds era-watermark tracking for the C1 contract-side
/// boss-kill flow (`boss_era_witnessed`,
/// `boss_damage_at_era_start`) and the C2 ranked-token claim
/// log (`tokens_claimed_eras`). Additive composition over V14,
/// same wire-format rule as earlier bumps.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InventoryV15 {
    pub base: InventoryV14,
    /// Highest world-boss era for which the player has already
    /// claimed stars / tokens. Frontend bumps it via the
    /// `ClaimBossKill` RPC when it observes an era advance.
    /// `u64` to match `era_of_total`'s return type (eras are
    /// uncapped in principle; the contract clamps at 50 today
    /// but `u64` keeps room for future).
    pub boss_era_witnessed: u64,
    /// Snapshot of `boss_damage` at the start of the current
    /// era. Used to compute `dmg_share = boss_damage -
    /// boss_damage_at_era_start` for the star-award curve.
    pub boss_damage_at_era_start: u64,
}

impl Default for InventoryV15 {
    fn default() -> Self {
        Self {
            base: InventoryV14::default(),
            boss_era_witnessed: 0,
            boss_damage_at_era_start: 0,
        }
    }
}

impl std::ops::Deref for InventoryV15 {
    type Target = InventoryV14;
    fn deref(&self) -> &InventoryV14 {
        &self.base
    }
}

impl std::ops::DerefMut for InventoryV15 {
    fn deref_mut(&mut self) -> &mut InventoryV14 {
        &mut self.base
    }
}

impl From<InventoryV14> for InventoryV15 {
    fn from(v14: InventoryV14) -> Self {
        Self {
            base: v14,
            boss_era_witnessed: 0,
            boss_damage_at_era_start: 0,
        }
    }
}

/// V16 — adds `landmark_claims` (Wilds first-clear watermark per
/// area_id). Additive composition over V15, same wire-format
/// rule as earlier bumps.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InventoryV16 {
    pub base: InventoryV15,
    /// Per-Wilds-area watermark — first-clear timestamp_ms. Set
    /// once when the player wins an encounter in a Wilds area
    /// that has an associated landmark; subsequent clears in the
    /// same area do not re-award the bundle.
    pub landmark_claims: BTreeMap<u8, u64>,
}

impl Default for InventoryV16 {
    fn default() -> Self {
        Self {
            base: InventoryV15::default(),
            landmark_claims: BTreeMap::new(),
        }
    }
}

impl std::ops::Deref for InventoryV16 {
    type Target = InventoryV15;
    fn deref(&self) -> &InventoryV15 {
        &self.base
    }
}

impl std::ops::DerefMut for InventoryV16 {
    fn deref_mut(&mut self) -> &mut InventoryV15 {
        &mut self.base
    }
}

impl From<InventoryV15> for InventoryV16 {
    fn from(v15: InventoryV15) -> Self {
        Self {
            base: v15,
            landmark_claims: BTreeMap::new(),
        }
    }
}

/// V17 — first version with the V2 routine shape (six fields).
/// Embeds `InventoryV16` whole so the V14 frozen `routine: V1`
/// stays in-memory but is shadowed by `Self::routine: V2` at the
/// field-access level: `inv.routine` resolves to V17's field,
/// while Deref-reachable V16 / V15 / V14 fields keep working.
///
/// `From<V16> for V17` lifts `v16.routine` (V1) into `V2` so old
/// blobs migrate transparently. New writes always go through V17
/// — see `InventoryWire::from(Inventory)` below.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InventoryV17 {
    pub base: InventoryV16,
    pub routine: super::routine::RoutineStateV2,
}

impl Default for InventoryV17 {
    fn default() -> Self {
        Self {
            base: InventoryV16::default(),
            routine: super::routine::RoutineStateV2::default(),
        }
    }
}

impl std::ops::Deref for InventoryV17 {
    type Target = InventoryV16;
    fn deref(&self) -> &InventoryV16 {
        &self.base
    }
}

impl std::ops::DerefMut for InventoryV17 {
    fn deref_mut(&mut self) -> &mut InventoryV16 {
        &mut self.base
    }
}

impl From<InventoryV16> for InventoryV17 {
    fn from(v16: InventoryV16) -> Self {
        // Walk to V14 to grab the frozen V1 routine, then lift it
        // into V2 with defaults for the five new fields.
        let v1 = v16.base.base.routine.clone();
        Self {
            base: v16,
            routine: super::routine::RoutineStateV2::from(v1),
        }
    }
}

/// V18 — adds the `auto_equip_best_on_drop` routine toggle by
/// embedding the V17 base and shadowing `routine` with V3. Same
/// field-shadowing trick as V17 (V17.routine: V2 shadowed V14's
/// frozen V1). `From<V17> for V18` lifts V17's V2 routine into V3
/// with the new field defaulted false.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InventoryV18 {
    pub base: InventoryV17,
    pub routine: super::routine::RoutineStateV3,
}

impl Default for InventoryV18 {
    fn default() -> Self {
        Self {
            base: InventoryV17::default(),
            routine: super::routine::RoutineStateV3::default(),
        }
    }
}

impl std::ops::Deref for InventoryV18 {
    type Target = InventoryV17;
    fn deref(&self) -> &InventoryV17 {
        &self.base
    }
}

impl std::ops::DerefMut for InventoryV18 {
    fn deref_mut(&mut self) -> &mut InventoryV17 {
        &mut self.base
    }
}

impl From<InventoryV17> for InventoryV18 {
    fn from(v17: InventoryV17) -> Self {
        // V17.routine is V2; lift to V3 with the new toggle
        // defaulted false. Clone is cheap (BTreeMaps share
        // ownership semantics via Drop).
        let v2 = v17.routine.clone();
        Self {
            base: v17,
            routine: super::routine::RoutineStateV3::from(v2),
        }
    }
}

/// V19 — adds `routine: V4` (offline_cap_hours + mission cycle).
/// Same shadow-field trick as V17 / V18. From<V18> lifts V18's V3
/// routine into V4 with defaults preserving legacy behavior
/// (offline_cap_hours = 0 → server falls back to 1-hour cap;
/// mission_cycle_mode = 0 → static area).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InventoryV19 {
    pub base: InventoryV18,
    pub routine: super::routine::RoutineStateV4,
}

impl Default for InventoryV19 {
    fn default() -> Self {
        Self {
            base: InventoryV18::default(),
            routine: super::routine::RoutineStateV4::default(),
        }
    }
}

impl std::ops::Deref for InventoryV19 {
    type Target = InventoryV18;
    fn deref(&self) -> &InventoryV18 {
        &self.base
    }
}

impl std::ops::DerefMut for InventoryV19 {
    fn deref_mut(&mut self) -> &mut InventoryV18 {
        &mut self.base
    }
}

impl From<InventoryV18> for InventoryV19 {
    fn from(v18: InventoryV18) -> Self {
        let v3 = v18.routine.clone();
        Self {
            base: v18,
            routine: super::routine::RoutineStateV4::from(v3),
        }
    }
}

/// V20 — adds `routine: V5` (public cosmetics motto/accent/frame).
/// Field-shadowing same as V17/V18/V19. From<V19> lifts V19's V4
/// routine into V5 with empty cosmetics (no published motto / no
/// accent / no frame).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InventoryV20 {
    pub base: InventoryV19,
    pub routine: super::routine::RoutineStateV5,
}

impl Default for InventoryV20 {
    fn default() -> Self {
        Self {
            base: InventoryV19::default(),
            routine: super::routine::RoutineStateV5::default(),
        }
    }
}

impl std::ops::Deref for InventoryV20 {
    type Target = InventoryV19;
    fn deref(&self) -> &InventoryV19 {
        &self.base
    }
}

impl std::ops::DerefMut for InventoryV20 {
    fn deref_mut(&mut self) -> &mut InventoryV19 {
        &mut self.base
    }
}

impl From<InventoryV19> for InventoryV20 {
    fn from(v19: InventoryV19) -> Self {
        let v4 = v19.routine.clone();
        Self {
            base: v19,
            routine: super::routine::RoutineStateV5::from(v4),
        }
    }
}

/// Public name for "the current inventory shape". Every consumer
/// imports `Inventory`; only the persistence layer in the delegate
/// is aware that this is a versioned type.
pub type Inventory = InventoryV20;

/// On-disk wrapper. Append new variants at the end — deleting or
/// reordering breaks the bincode discriminant for existing blobs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InventoryWire {
    V9(InventoryV9),
    V10(InventoryV10),
    V11(InventoryV11),
    V12(InventoryV12),
    V13(InventoryV13),
    V14(InventoryV14),
    V15(InventoryV15),
    V16(InventoryV16),
    V17(InventoryV17),
    V18(InventoryV18),
    V19(InventoryV19),
    V20(InventoryV20),
}

impl InventoryWire {
    /// Migrate any historical variant to the current `Inventory`.
    pub fn into_latest(self) -> Inventory {
        match self {
            Self::V9(v9) => InventoryV20::from(InventoryV19::from(InventoryV18::from(InventoryV17::from(InventoryV16::from(
                InventoryV15::from(InventoryV14::from(InventoryV13::from(InventoryV12::from(
                    InventoryV11::from(InventoryV10::from(v9)),
                )))),
            ))))),
            Self::V10(v10) => InventoryV20::from(InventoryV19::from(InventoryV18::from(InventoryV17::from(InventoryV16::from(
                InventoryV15::from(InventoryV14::from(InventoryV13::from(InventoryV12::from(
                    InventoryV11::from(v10),
                )))),
            ))))),
            Self::V11(v11) => InventoryV20::from(InventoryV19::from(InventoryV18::from(InventoryV17::from(InventoryV16::from(
                InventoryV15::from(InventoryV14::from(InventoryV13::from(InventoryV12::from(
                    v11,
                )))),
            ))))),
            Self::V12(v12) => InventoryV20::from(InventoryV19::from(InventoryV18::from(InventoryV17::from(InventoryV16::from(
                InventoryV15::from(InventoryV14::from(InventoryV13::from(v12))),
            ))))),
            Self::V13(v13) => InventoryV20::from(InventoryV19::from(InventoryV18::from(InventoryV17::from(InventoryV16::from(
                InventoryV15::from(InventoryV14::from(v13)),
            ))))),
            Self::V14(v14) => InventoryV20::from(InventoryV19::from(InventoryV18::from(InventoryV17::from(InventoryV16::from(
                InventoryV15::from(v14),
            ))))),
            Self::V15(v15) => InventoryV20::from(InventoryV19::from(InventoryV18::from(InventoryV17::from(InventoryV16::from(v15))))),
            Self::V16(v16) => InventoryV20::from(InventoryV19::from(InventoryV18::from(InventoryV17::from(v16)))),
            Self::V17(v17) => InventoryV20::from(InventoryV19::from(InventoryV18::from(v17))),
            Self::V18(v18) => InventoryV20::from(InventoryV19::from(v18)),
            Self::V19(v19) => InventoryV20::from(v19),
            Self::V20(v20) => v20,
        }
    }
}

impl From<Inventory> for InventoryWire {
    fn from(inv: Inventory) -> Self {
        Self::V20(inv)
    }
}
