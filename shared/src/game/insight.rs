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
}

impl InsightNode {
    pub const ALL: &'static [InsightNode] = &[
        InsightNode::HpPerLevel,
        InsightNode::GoldDropPct,
        InsightNode::FormAffinity,
    ];

    pub fn id(self) -> u8 {
        self as u8
    }

    pub fn from_id(id: u8) -> Option<InsightNode> {
        match id {
            0 => Some(InsightNode::HpPerLevel),
            1 => Some(InsightNode::GoldDropPct),
            2 => Some(InsightNode::FormAffinity),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            InsightNode::HpPerLevel => "+1 HP / level",
            InsightNode::GoldDropPct => "+1% gold drop",
            InsightNode::FormAffinity => "form affinity bias",
        }
    }

    /// Cost in insight for the *next* level. Linear in node level
    /// for the first ~10 levels, then sublinear cap. Insight is
    /// rare, so we don't need an exponential curve.
    pub fn next_cost(self, current_level: u64) -> u64 {
        // 1, 2, 3, … up to a soft cap of 5 per level after lvl 10.
        match current_level {
            0..=9 => current_level + 1,
            _ => 5,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct InsightState {
    /// Unspent insight in the player's pocket.
    pub balance: u64,
    /// Levels purchased per node id.
    pub nodes: BTreeMap<u8, u64>,
    /// High-watermark of `mission_count` that has already paid
    /// out a milestone insight — same idempotency trick as
    /// `LegacyState::last_awarded_level`.
    pub last_awarded_mission: u64,
}

impl InsightState {
    pub fn node_level(&self, node: InsightNode) -> u64 {
        self.nodes.get(&node.id()).copied().unwrap_or(0)
    }
}
