//! Routine (backlog B1). Set-and-forget automation: declare your
//! desired Estate headcount / gear tier / consumable stockpile /
//! auto-activity / battle policy, and the delegate's pump-driver
//! pushes the inventory toward those targets on every mutation.
//!
//! Wire-format note: bincode 1 does NOT apply `#[serde(default)]`
//! to truncated input — extending an embedded struct's fields
//! breaks every old blob that doesn't already have them. So
//! `RoutineState` is split into frozen `RoutineStateV1` (the
//! original 1-field shape that production currently writes) and
//! `RoutineStateV2` (the current 6-field shape). The outer
//! `Inventory` version chain decides which one to deserialize:
//! `InventoryV14..V16` embed `V1` (matching production bytes),
//! `InventoryV17+` embed `V2`. See §6.2 in
//! `docs/planned-work-2026-05-17.md`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Auto-action policy applied during AFK battles. `Manual` mirrors
/// the legacy zero-input behavior (player clicks Use Potion /
/// Use Fireball to queue); `Auto { … }` lets the routine pump
/// queue potion / fireball based on simple thresholds.
///
/// Default (2026-05-18): fresh installs land on
/// `Auto { potion_below_hp_pct: 40, fireball_per_n_turns: 5 }`. This
/// only affects new players — existing blobs deserialize whatever
/// variant they had stored, and the legacy V1→V2 routine lift in
/// `From<RoutineStateV1> for RoutineStateV2` still writes `Manual`
/// explicitly so V1 saves keep their pre-Auto behaviour.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum BattleActionPolicy {
    /// No auto-queue. Legacy AFK behaviour — used by routines lifted
    /// from V1 saves; no longer the `Default`.
    Manual,
    /// Auto-queue potion when HP drops below `potion_below_hp_pct`
    /// percent (0..=100) of max HP and at least one potion is
    /// available. Auto-queue fireball every `fireball_per_n_turns`
    /// turns (0 disables fireball auto-cast).
    Auto {
        potion_below_hp_pct: u8,
        fireball_per_n_turns: u32,
    },
}

impl Default for BattleActionPolicy {
    fn default() -> Self {
        // Fresh-install policy. See struct docs for the rationale and
        // why this doesn't affect existing players.
        Self::Auto {
            potion_below_hp_pct: 40,
            fireball_per_n_turns: 5,
        }
    }
}

/// Frozen original Routine shape — single field. Embedded in
/// `InventoryV14..V16` to keep their bincode wire format byte-
/// identical to what production currently writes. Do NOT add
/// fields here; extensions go in `RoutineStateV2` and ship inside
/// a new `InventoryV(N+1)`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RoutineStateV1 {
    pub estate_targets: BTreeMap<u8, u64>,
}

/// Current Routine shape — adds 5 fields over V1 covering gear /
/// consumables / skills / per-zone activity / battle policy.
/// `base: RoutineStateV1` keeps the V1 portion of the bincode
/// layout byte-identical (same additive pattern as
/// `InventoryV(N+1) { base: V(N), … }`). Deref to V1 means
/// `routine.estate_targets` still works without `.base.` prefix.
/// `RoutineState` is a type alias to this — consumer code is
/// unchanged.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RoutineStateV2 {
    pub base: RoutineStateV1,
    /// Desired equipped-gear tier per slot (`slot_idx → tier`,
    /// `tier ∈ 1..=3`). Auto-equip + auto-buy fires only on slots
    /// the current form's mask allows. Missing key = no target.
    pub gear_targets: BTreeMap<u8, u8>,
    /// Desired stockpile per consumable kind (`CONSUMABLE_POTION /
    /// CONSUMABLE_FIREBALL → keep_count`). Auto-buy fires when
    /// `inv.<kind> < target` and gold permits.
    pub consumable_targets: BTreeMap<u8, u32>,
    /// When true, auto-buy any unlocked skill the moment the
    /// player has enough gold. Off by default.
    pub auto_skill_unlock: bool,
    /// Per-zone activity preference (`area_id → activity_id`).
    /// When the player sets `current_area` to a key in this map
    /// and the area's activities include the targeted activity,
    /// the routine pump flips `idle_action` to
    /// `IDLE_ACTION_ACTIVITY` and starts the chosen one.
    pub auto_activity_at_zone: BTreeMap<u8, u8>,
    /// Battle auto-action policy. Default `Manual` preserves the
    /// legacy AFK-without-actions behavior.
    pub battle_action_policy: BattleActionPolicy,
}

impl std::ops::Deref for RoutineStateV2 {
    type Target = RoutineStateV1;
    fn deref(&self) -> &RoutineStateV1 {
        &self.base
    }
}

impl std::ops::DerefMut for RoutineStateV2 {
    fn deref_mut(&mut self) -> &mut RoutineStateV1 {
        &mut self.base
    }
}

impl From<RoutineStateV1> for RoutineStateV2 {
    fn from(v1: RoutineStateV1) -> Self {
        Self {
            base: v1,
            gear_targets: BTreeMap::new(),
            consumable_targets: BTreeMap::new(),
            auto_skill_unlock: false,
            auto_activity_at_zone: BTreeMap::new(),
            battle_action_policy: BattleActionPolicy::Manual,
        }
    }
}

/// V3 adds the global `auto_equip_best_on_drop` toggle — when ON,
/// the pump-driver runs the same "best stash piece per form-
/// allowed slot" sweep that the manual Auto-Equip Best button
/// does, on every state mutation. Gear drops auto-equip without
/// needing to configure per-slot tier targets. Additive over V2
/// (same `base: V<N-1>` pattern as `Inventory V(N+1)`); Deref V3
/// → V2 → V1 keeps every accessor on prior versions working
/// transparently.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RoutineStateV3 {
    pub base: RoutineStateV2,
    /// When true, the routine pump auto-equips the best stash
    /// piece for every form-allowed slot whose current piece is
    /// inferior (score = atk + def + hp). No shop-buy — just
    /// shuffles existing inventory.
    pub auto_equip_best_on_drop: bool,
}

impl std::ops::Deref for RoutineStateV3 {
    type Target = RoutineStateV2;
    fn deref(&self) -> &RoutineStateV2 {
        &self.base
    }
}

impl std::ops::DerefMut for RoutineStateV3 {
    fn deref_mut(&mut self) -> &mut RoutineStateV2 {
        &mut self.base
    }
}

impl From<RoutineStateV2> for RoutineStateV3 {
    fn from(v2: RoutineStateV2) -> Self {
        Self {
            base: v2,
            auto_equip_best_on_drop: false,
        }
    }
}

/// V4 — offline-cap config (§8 B6) + auto-mission area cycle
/// (§8 B7). Both additive over V3 with the standard
/// `base: V<N-1>` shape; Deref V4 → V3 → V2 → V1 keeps every
/// prior accessor intact. `From<V3> for V4` defaults
/// `offline_cap_hours = 1` (legacy `MAX_CATCHUP_TICKS / 3600`)
/// and `mission_cycle_mode = Static` (legacy behavior).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RoutineStateV4 {
    pub base: RoutineStateV3,
    /// Max wall-clock hours of offline auto-mission to simulate
    /// in one `catch_up_auto`. `0` falls back to the hard-coded
    /// default (1 hour). Capped server-side at 24 to keep the
    /// catchup loop bounded.
    pub offline_cap_hours: u8,
    /// Per-area auto-mission cycle mode (§8 B7). `0` = Static
    /// (legacy — always farm `inv.current_area`), `1` = Cycle
    /// through `mission_cycle_areas` in order, `2` = Boss-first
    /// (prefer any boss-eligible area first, fall back to cycle).
    pub mission_cycle_mode: u8,
    /// Ordered list of area ids to cycle through when
    /// `mission_cycle_mode != 0`. Empty = no cycling regardless
    /// of mode (degrades to Static behavior).
    pub mission_cycle_areas: Vec<u8>,
    /// Index into `mission_cycle_areas` of the next area to use
    /// when the current mission ends. Advanced by the pump
    /// driver on each completed mission.
    pub mission_cycle_idx: u8,
    /// Combat-speed multiplier in basis points (§8 D6). `0` (or
    /// 10_000) keeps the default 1.0×; 5_000 = 0.5×, 20_000 = 2×.
    /// Applied as `TURN_COOLDOWN_MS * 10_000 / mult_bp` so values
    /// > 10_000 shorten the cooldown (faster combat). Capped
    /// server-side at 30_000 (3×) to avoid pathological tick
    /// counts in catchup.
    pub combat_speed_bp: u32,
}

impl std::ops::Deref for RoutineStateV4 {
    type Target = RoutineStateV3;
    fn deref(&self) -> &RoutineStateV3 {
        &self.base
    }
}

impl std::ops::DerefMut for RoutineStateV4 {
    fn deref_mut(&mut self) -> &mut RoutineStateV3 {
        &mut self.base
    }
}

impl From<RoutineStateV3> for RoutineStateV4 {
    fn from(v3: RoutineStateV3) -> Self {
        Self {
            base: v3,
            offline_cap_hours: 0,
            mission_cycle_mode: 0,
            mission_cycle_areas: Vec::new(),
            mission_cycle_idx: 0,
            combat_speed_bp: 0,
        }
    }
}

/// V5 — §E-tier public cosmetics. `base: V4` keeps the V4 portion
/// of the bincode bytes byte-identical so v=4 blobs decode
/// cleanly. New fields publish into `PresencePayloadV3` on the
/// presence-contract heartbeat.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RoutineStateV5 {
    pub base: RoutineStateV4,
    /// §E2 public motto. Empty = don't publish a motto.
    pub public_motto: String,
    /// §E3 public leaderboard accent id. `0` = no accent.
    pub public_accent: u8,
    /// §E1 public leaderboard frame id. `0` = no frame.
    pub public_frame: u8,
    /// §P3 daily-streak: UTC day-index (`now_ms / 86_400_000`) of
    /// the most recent successful check-in. `0` = never claimed.
    pub last_checkin_day: u64,
    /// Current consecutive-day streak length. Resets to 1 on
    /// claim if `now_day != last_checkin_day + 1`; increments
    /// otherwise. Caps at 30 — beyond is irrelevant for the
    /// reward curve.
    pub streak_days: u32,
}

impl std::ops::Deref for RoutineStateV5 {
    type Target = RoutineStateV4;
    fn deref(&self) -> &RoutineStateV4 {
        &self.base
    }
}

impl std::ops::DerefMut for RoutineStateV5 {
    fn deref_mut(&mut self) -> &mut RoutineStateV4 {
        &mut self.base
    }
}

impl From<RoutineStateV4> for RoutineStateV5 {
    fn from(v4: RoutineStateV4) -> Self {
        Self {
            base: v4,
            public_motto: String::new(),
            public_accent: 0,
            public_frame: 0,
            last_checkin_day: 0,
            streak_days: 0,
        }
    }
}

/// §P3: reward curve for the daily check-in. Returns essence
/// gained for a check-in at streak day `n`. Flat baseline plus
/// modest linear-up-to-day-7 ramp; flat after that. Sized so a
/// 30-day streak hands out roughly the same essence as an hour
/// of late-game grinding.
pub fn daily_checkin_reward_essence(streak: u32) -> u64 {
    let n = streak.min(30) as u64;
    let ramp = n.min(7);
    50 + ramp.saturating_mul(25)
}

pub const MISSION_CYCLE_STATIC: u8 = 0;
pub const MISSION_CYCLE_ROTATE: u8 = 1;
pub const MISSION_CYCLE_BOSS_FIRST: u8 = 2;

/// Public alias. Consumer code reads/writes `RoutineState` — the
/// V1/V2/V3/V4 freezes are only relevant inside the Inventory
/// version chain.
pub type RoutineState = RoutineStateV5;

impl RoutineStateV1 {
    pub fn target_for(&self, tier_id: u8) -> Option<u64> {
        self.estate_targets.get(&tier_id).copied()
    }

    pub fn set_target(&mut self, tier_id: u8, target: u64) {
        if target == 0 {
            self.estate_targets.remove(&tier_id);
        } else {
            self.estate_targets.insert(tier_id, target);
        }
    }
}

impl RoutineStateV2 {
    pub fn gear_target_for(&self, slot_idx: u8) -> Option<u8> {
        self.gear_targets.get(&slot_idx).copied()
    }

    pub fn set_gear_target(&mut self, slot_idx: u8, tier: u8) {
        if tier == 0 {
            self.gear_targets.remove(&slot_idx);
        } else {
            self.gear_targets.insert(slot_idx, tier);
        }
    }

    pub fn consumable_target_for(&self, kind: u8) -> Option<u32> {
        self.consumable_targets.get(&kind).copied()
    }

    pub fn set_consumable_target(&mut self, kind: u8, target: u32) {
        if target == 0 {
            self.consumable_targets.remove(&kind);
        } else {
            self.consumable_targets.insert(kind, target);
        }
    }

    pub fn activity_for_zone(&self, area_id: u8) -> Option<u8> {
        self.auto_activity_at_zone.get(&area_id).copied()
    }

    pub fn set_activity_for_zone(&mut self, area_id: u8, activity_id: u8) {
        if activity_id == 0 {
            self.auto_activity_at_zone.remove(&area_id);
        } else {
            self.auto_activity_at_zone.insert(area_id, activity_id);
        }
    }
}
