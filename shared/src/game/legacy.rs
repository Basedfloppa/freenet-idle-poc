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
