//! Insight currency + spend tree (backlog B5, MVP scope).
//!
//! Earn rate: +1 insight every `INSIGHT_PER_MISSIONS` missions
//! plus whatever "Decode sigils" (Astral activity) drips in.
//! Currency is rare on purpose — the spend tree is small and the
//! nodes are noticeable (+1% gold drop is real money over the
//! long run).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Award one insight every N completed missions. Conservative —
/// rare enough that picking which node to spend on is a real
/// choice, common enough that a casual player gets a couple per
/// session.
pub const INSIGHT_PER_MISSIONS: u64 = 25;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum InsightNode {
    /// +1 HP per hero level per node level. Stacks linearly with
    /// the base `lvl * 5` curve, so it's most effective late.
    HpPerLevel = 0,
    /// +1% gold drop on every encounter win. Capped indirectly
    /// via the spend curve.
    GoldDropPct = 1,
    /// Lifts the active form's affinity buff/penalty toward
    /// neutral or beyond — converts ×1.75 → ×1.75 + N×0.1, and
    /// the penalty side ×0.75 → ×0.75 + N×0.05 (so penalties
    /// soften faster than buffs grow, encouraging spec play).
    FormAffinity = 2,
    /// +0.5% chance per node level to land a critical strike
    /// (double damage). Capped at 25% (50 lvl effective cap).
    CriticalStrike = 3,
    /// +1% per node level to every encounter's essence reward.
    /// Multiplicative with `area.essence_mult` and the token
    /// EssenceWeaver perk.
    EssenceSurge = 4,
    /// −10 ms per node level on `TURN_COOLDOWN_MS`. Capped at
    /// −500 ms (50 lvl effective cap), so the minimum tick stays
    /// at 500 ms.
    BattleCadence = 5,
    /// +5 HP restored at the end of each won encounter per node
    /// level. Soft heal between fights without forcing a potion.
    HealingTouch = 6,
    /// +1% per node level chance bonus to gear drops at
    /// `GEAR_DROP_EVERY` milestones. Caps softly via the spend
    /// curve.
    TreasureHunter = 7,
    /// −1% per node level on the next-worker price exponential
    /// curve. Capped at −50% (50 lvl effective cap).
    EstateFrugality = 8,
    /// +5% per node level to per-zone activity yield rate.
    /// Multiplicative against the activity's base yield.
    ActivityYield = 9,
    /// +1 bonus boss damage per encounter per node level, applied
    /// only on boss-contact areas (`damage_mult > 0`).
    BossStriker = 10,
}

impl InsightNode {
    pub const ALL: &'static [InsightNode] = &[
        InsightNode::HpPerLevel,
        InsightNode::GoldDropPct,
        InsightNode::FormAffinity,
        InsightNode::CriticalStrike,
        InsightNode::EssenceSurge,
        InsightNode::BattleCadence,
        InsightNode::HealingTouch,
        InsightNode::TreasureHunter,
        InsightNode::EstateFrugality,
        InsightNode::ActivityYield,
        InsightNode::BossStriker,
    ];

    pub fn id(self) -> u8 {
        self as u8
    }

    pub fn from_id(id: u8) -> Option<InsightNode> {
        match id {
            0 => Some(InsightNode::HpPerLevel),
            1 => Some(InsightNode::GoldDropPct),
            2 => Some(InsightNode::FormAffinity),
            3 => Some(InsightNode::CriticalStrike),
            4 => Some(InsightNode::EssenceSurge),
            5 => Some(InsightNode::BattleCadence),
            6 => Some(InsightNode::HealingTouch),
            7 => Some(InsightNode::TreasureHunter),
            8 => Some(InsightNode::EstateFrugality),
            9 => Some(InsightNode::ActivityYield),
            10 => Some(InsightNode::BossStriker),
            _ => None,
        }
    }

    /// Stable i18n key for `insight_node_name.<key>` /
    /// `insight_node_desc.<key>`. Lowercase snake-case, never
    /// reordered.
    pub fn key(self) -> &'static str {
        match self {
            InsightNode::HpPerLevel => "hp_per_level",
            InsightNode::GoldDropPct => "gold_drop_pct",
            InsightNode::FormAffinity => "form_affinity",
            InsightNode::CriticalStrike => "critical_strike",
            InsightNode::EssenceSurge => "essence_surge",
            InsightNode::BattleCadence => "battle_cadence",
            InsightNode::HealingTouch => "healing_touch",
            InsightNode::TreasureHunter => "treasure_hunter",
            InsightNode::EstateFrugality => "estate_frugality",
            InsightNode::ActivityYield => "activity_yield",
            InsightNode::BossStriker => "boss_striker",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            InsightNode::HpPerLevel => "+1 HP / level",
            InsightNode::GoldDropPct => "+1% gold drop",
            InsightNode::FormAffinity => "form affinity bias",
            InsightNode::CriticalStrike => "Critical Strike",
            InsightNode::EssenceSurge => "Essence Surge",
            InsightNode::BattleCadence => "Battle Cadence",
            InsightNode::HealingTouch => "Healing Touch",
            InsightNode::TreasureHunter => "Treasure Hunter",
            InsightNode::EstateFrugality => "Estate Frugality",
            InsightNode::ActivityYield => "Activity Yield",
            InsightNode::BossStriker => "Boss Striker",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            InsightNode::HpPerLevel =>
                "Adds 1 HP per hero level per node level. Stacks on top of the base 5 HP/lvl curve, so it's most effective in the late game.",
            InsightNode::GoldDropPct =>
                "Adds 1% per node level to every encounter's gold reward. Compounds multiplicatively with Legacy MissionGold and area gold_mult.",
            InsightNode::FormAffinity =>
                "Stretches the current form's Estate-tier affinity table — buffs grow by +10% per node level, penalties shrink by +5%. Encourages specialisation.",
            InsightNode::CriticalStrike =>
                "Adds 0.5% critical-strike chance per node level (×2 damage on the swing). Capped at 25% (50 levels). Deterministic per-turn roll.",
            InsightNode::EssenceSurge =>
                "Adds 1% per node level to every encounter's essence reward. Compounds with area essence_mult and the token EssenceWeaver perk.",
            InsightNode::BattleCadence =>
                "Subtracts 10 ms per node level from the turn cooldown (default 1000 ms). Capped at −500 ms (50 levels), so the floor stays a half-second.",
            InsightNode::HealingTouch =>
                "Heals 5 HP at the end of each won encounter per node level. Refreshes between fights without spending a potion.",
            InsightNode::TreasureHunter =>
                "Adds 1% per node level to the gear-drop chance at every GEAR_DROP_EVERY milestone. Independent of the base cadence.",
            InsightNode::EstateFrugality =>
                "Subtracts 1% per node level from the next-worker price on every Estate tier. Capped at −50% (50 levels).",
            InsightNode::ActivityYield =>
                "Adds 5% per node level to the yield rate of the currently active per-zone activity (Tend farm, Forage, Mine, etc.).",
            InsightNode::BossStriker =>
                "Adds 1 bonus boss damage per encounter per node level. Applies only on boss-contact areas (damage_mult > 0).",
        }
    }

    /// Cost in insight for the *next* level. Strictly monotonically
    /// rising (no soft cap) — insight is a permanent currency that
    /// keeps trickling in, so the curve has to keep growing to avoid
    /// trivialising late-game nodes. Linear ramp `lvl + 1` for the
    /// first 10 levels, then a quadratic tail beyond so a level-100
    /// node still costs proportionally more than a level-10 one.
    pub fn next_cost(self, current_level: u64) -> u64 {
        if current_level < 10 {
            return current_level + 1;
        }
        let over = current_level.saturating_sub(9);
        10u64.saturating_add(over.saturating_mul(over))
    }
}

/// Frozen V1 shape of `InsightState`. Embedded in `InventoryV14`.
///
/// **Wire-format rule:** do NOT add fields here. bincode 1 doesn't
/// honour `#[serde(default)]` on truncated input, so growing this
/// struct in place breaks every older blob. To extend, freeze V1
/// and define `InsightStateV2 { base: V1, … }` inside a new
/// `InventoryV(N+1)`. See `RoutineStateV1`/`V2` for the canonical
/// shape change.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct InsightStateV1 {
    /// Unspent insight in the player's pocket.
    pub balance: u64,
    /// Levels purchased per node id.
    pub nodes: BTreeMap<u8, u64>,
    /// High-watermark of `mission_count` that has already paid
    /// out a milestone insight — same idempotency trick as
    /// `LegacyState::last_awarded_level`.
    pub last_awarded_mission: u64,
}

/// Public alias. Consumer code reads/writes `InsightState`; the V1
/// freeze is only relevant when extending the schema (see above).
pub type InsightState = InsightStateV1;

impl InsightState {
    pub fn node_level(&self, node: InsightNode) -> u64 {
        self.nodes.get(&node.id()).copied().unwrap_or(0)
    }

    /// Critical-strike chance in basis points (10_000 = 100%).
    /// +50 bp per CriticalStrike level, capped at 2500 (=25%).
    pub fn crit_chance_bp(&self) -> u64 {
        let lvl = self.node_level(InsightNode::CriticalStrike);
        lvl.saturating_mul(50).min(2_500)
    }

    /// Essence multiplier in basis points (+1% per EssenceSurge
    /// level, no hard cap — spend curve self-throttles).
    pub fn essence_mult_bp(&self) -> u64 {
        let lvl = self.node_level(InsightNode::EssenceSurge);
        10_000u64.saturating_add(lvl.saturating_mul(100))
    }

    /// Turn-cooldown delta in ms: −10 ms per BattleCadence level,
    /// capped at −500 ms.
    pub fn cadence_delta_ms(&self) -> u64 {
        let lvl = self.node_level(InsightNode::BattleCadence);
        lvl.saturating_mul(10).min(500)
    }

    /// Bonus HP restored after each won encounter (+5 HP per
    /// HealingTouch level).
    pub fn heal_per_encounter(&self) -> u64 {
        self.node_level(InsightNode::HealingTouch).saturating_mul(5)
    }

    /// Bonus gear-drop chance in basis points (+100 bp per
    /// TreasureHunter level). Caller rolls against this on top
    /// of the base GEAR_DROP_EVERY cadence.
    pub fn treasure_bonus_bp(&self) -> u64 {
        self.node_level(InsightNode::TreasureHunter).saturating_mul(100)
    }

    /// Estate next-price multiplier in basis points (−100 bp per
    /// EstateFrugality level, floor at 5_000 = 50% off).
    pub fn frugality_mult_bp(&self) -> u64 {
        let lvl = self.node_level(InsightNode::EstateFrugality);
        let off = lvl.saturating_mul(100).min(5_000);
        10_000u64.saturating_sub(off)
    }

    /// Activity-yield multiplier in basis points (+500 bp per
    /// ActivityYield level).
    pub fn activity_yield_bp(&self) -> u64 {
        let lvl = self.node_level(InsightNode::ActivityYield);
        10_000u64.saturating_add(lvl.saturating_mul(500))
    }

    /// Bonus boss damage per encounter on boss-contact areas
    /// (+1 dmg per BossStriker level).
    pub fn boss_striker_bonus(&self) -> u64 {
        self.node_level(InsightNode::BossStriker)
    }
}
