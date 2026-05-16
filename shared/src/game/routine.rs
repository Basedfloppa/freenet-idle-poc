//! Routine (backlog B1, MVP scope). Today: declarative Estate
//! auto-hire targets keyed by `EstateTierDef.id`. When the player
//! is running Estate as their idle action and gold permits, the
//! delegate's tick advances tier counts toward the configured
//! target. Future iterations extend the target set (skills, gear
//! tiers, consumables stockpile) but the wire format already
//! groups by category so a new map can be added without breaking
//! older blobs.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RoutineState {
    /// Desired headcount per Estate tier. Missing key = no target
    /// set (auto-hire skips that tier). Setting a target lower
    /// than the current count is fine — the delegate just won't
    /// hire more, it never refunds gold by selling workers.
    pub estate_targets: BTreeMap<u8, u64>,
}

impl RoutineState {
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
