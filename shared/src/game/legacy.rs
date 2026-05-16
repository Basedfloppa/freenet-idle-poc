//! Personal Epoch / Legacy tree (backlog C1). Stars are earned
//! from milestones (every `STARS_PER_N_LEVELS` earned levels grants
//! one, with extras from boss-era thresholds the contract surfaces)
//! and spent on permanent multipliers; the player can choose to
//! Ascend at any point for a stronger second-tier of nodes at the
//! cost of a soft reset.
//!
//! MVP scope: three nodes (Hero attack, Estate yield, Mission gold),
//! all delegate-side. The boss-kill contract-side flow (top-N
//! contributor tokens, network-wide era resets) is deferred —
//! stars come from personal level-up milestones for now so the
//! whole loop is exercisable without contract cooperation.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// One star per this many earned levels. Conservative on purpose:
/// idle ramps mean a player picks up roughly one node level per
/// 30–60 minutes of play in the early game.
pub const STARS_PER_N_LEVELS: u64 = 5;

/// Identifiers for spendable nodes. Variants are pinned to `u8`
/// discriminants so they survive bincode/JSON round-trips without
/// the enum-name overhead. Add new variants at the end; never
/// reorder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum LegacyNode {
    /// +5% hero base attack per level (additive over baseline).
    HeroAttack = 0,
    /// +10% Estate yield per level (multiplicative on tier output).
    EstateYield = 1,
    /// +5% mission gold per level (applies to encounter rewards).
    MissionGold = 2,
}

impl LegacyNode {
    pub const ALL: &'static [LegacyNode] = &[
        LegacyNode::HeroAttack,
        LegacyNode::EstateYield,
        LegacyNode::MissionGold,
    ];

    pub fn id(self) -> u8 {
        self as u8
    }

    pub fn from_id(id: u8) -> Option<LegacyNode> {
        match id {
            0 => Some(LegacyNode::HeroAttack),
            1 => Some(LegacyNode::EstateYield),
            2 => Some(LegacyNode::MissionGold),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            LegacyNode::HeroAttack => "Hero Attack",
            LegacyNode::EstateYield => "Estate Yield",
            LegacyNode::MissionGold => "Mission Gold",
        }
    }

    /// One-line description of what the node does. Plain text so
    /// it can be embedded in a tooltip or as a `<p class="muted
    /// small">` alongside the table. Magnitudes are read off
    /// `bp_per_level` directly so docs can't drift from the
    /// implementation.
    pub fn description(self) -> &'static str {
        match self {
            LegacyNode::HeroAttack =>
                "Multiplies hero base + bonus attack by +5% per level. Stacks multiplicatively after gear / form / skill bonuses.",
            LegacyNode::EstateYield =>
                "Multiplies Estate worker yield by +10% per level. Compounds with form affinity multiplicatively.",
            LegacyNode::MissionGold =>
                "Multiplies gold gained per encounter by +5% per level. Applies to RunMission and auto-mission wins alike.",
        }
    }

    /// Per-level multiplier in basis points. Stacked additively
    /// against 10_000 (×1.0); a HeroAttack at level 4 contributes
    /// `4 * 500 = 2_000` bp → ×1.20.
    pub fn bp_per_level(self) -> u64 {
        match self {
            LegacyNode::HeroAttack => 500,   // +5%
            LegacyNode::EstateYield => 1_000, // +10%
            LegacyNode::MissionGold => 500,  // +5%
        }
    }

    /// Star cost for `current_level → current_level + 1`. Tier-1
    /// curve `1, 2, 4, 8, …` matches the backlog sketch; the long
    /// game runs out of stars before it runs out of ideas.
    pub fn next_cost(self, current_level: u64) -> u64 {
        let base = 1u64;
        base.saturating_mul(1u64 << current_level.min(20))
    }
}

/// Persistent state for the Legacy loop. Lives inside `InventoryV13`
/// (additive composition over V12) so older blobs still decode.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct LegacyState {
    /// Unspent stars in the player's pocket.
    pub stars: u64,
    /// Levels purchased per node (keyed by `LegacyNode::id`).
    pub nodes: BTreeMap<u8, u64>,
    /// High-watermark of the level milestone that has already paid
    /// out stars; ensures a star is only awarded once per crossing.
    pub last_awarded_level: u64,
    /// Number of times the player has ascended. Tier-2 node cost
    /// curve scales on this in a future iteration; recorded today
    /// so the UI can show "Ascensions: N".
    pub ascend_count: u64,
}

/// Star award curve for the era-advance hook (C1 contract-side
/// half). `dmg_share` is the player's contribution this era,
/// `era_max_hp` the era's total HP pool. Stars scale sublinearly
/// (`^0.7`) so a 10% contributor doesn't get 1/10th of the cap
/// — the curve rewards participation more evenly than raw share
/// would. Capped at `BOSS_KILL_STAR_CAP` to keep a single
/// dominant attacker from minting a galaxy.
pub const BOSS_KILL_STAR_CAP: u64 = 10;

pub fn boss_kill_stars_for(dmg_share: u64, era_max_hp: u64) -> u64 {
    if era_max_hp == 0 || dmg_share == 0 {
        return 0;
    }
    // Integer arithmetic on a fixed-point `share_bp` (basis points)
    // — avoids floating-point in WASM and stays deterministic
    // across crate versions. share_bp ∈ [0, 10_000].
    let share_bp = (dmg_share.saturating_mul(10_000) / era_max_hp).min(10_000);
    if share_bp == 0 {
        return 0;
    }
    // Approximate share_bp^0.7 / 10_000^0.7 × CAP. Lookup table
    // covering 10 brackets keeps the math cheap and predictable.
    let bracket = match share_bp {
        0 => 0,
        1..=50 => 1,             // <0.5%   share → 1 star (participation)
        51..=200 => 2,           // <2%     share → 2 stars
        201..=500 => 3,          // <5%     share → 3 stars
        501..=1_000 => 4,        // <10%    share → 4 stars
        1_001..=2_000 => 5,      // <20%    share → 5 stars
        2_001..=4_000 => 6,      // <40%    share → 6 stars
        4_001..=6_000 => 7,      // <60%    share → 7 stars
        6_001..=8_000 => 8,      // <80%    share → 8 stars
        8_001..=9_500 => 9,      // <95%    share → 9 stars
        _ => BOSS_KILL_STAR_CAP, // ≥95%    share → cap
    };
    bracket.min(BOSS_KILL_STAR_CAP)
}

impl LegacyState {
    pub fn node_level(&self, node: LegacyNode) -> u64 {
        self.nodes.get(&node.id()).copied().unwrap_or(0)
    }

    /// Multiplier in basis points for `node` (10_000 = neutral).
    pub fn node_multiplier_bp(&self, node: LegacyNode) -> u64 {
        let lvl = self.node_level(node);
        10_000u64.saturating_add(node.bp_per_level().saturating_mul(lvl))
    }
}
