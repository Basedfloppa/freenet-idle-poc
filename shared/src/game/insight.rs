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

    pub fn description(self) -> &'static str {
        match self {
            InsightNode::HpPerLevel =>
                "Adds 1 HP per hero level per node level. Stacks on top of the base 5 HP/lvl curve, so it's most effective in the late game.",
            InsightNode::GoldDropPct =>
                "Adds 1% per node level to every encounter's gold reward. Compounds multiplicatively with Legacy MissionGold and area gold_mult.",
            InsightNode::FormAffinity =>
                "Stretches the current form's Estate-tier affinity table — buffs grow by +10% per node level, penalties shrink by +5%. Encourages specialisation.",
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
