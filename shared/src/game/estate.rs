//! Estate — multi-tier worker economy (backlog B2). Workers produce
//! resources passively while Estate is the player's selected idle
//! action (§5.6: idle actions are mutually exclusive). Hiring more
//! workers in a tier follows an exponential cost curve so the early
//! ramp is fast and the late game has natural walls.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{
    FORM_CAT, FORM_DRAGON, FORM_HORSE, FORM_HUMAN, FORM_SLIME,
};

/// Single tier of the estate ladder. Each tier produces one
/// resource type at a flat per-worker yield; the player escalates by
/// hiring more of the same tier (cost grows exponentially) and by
/// unlocking deeper tiers.
#[derive(Debug, Clone, Copy)]
pub struct EstateTierDef {
    pub id: u8,
    pub name: &'static str,
    /// First-worker gold cost. Each additional worker costs
    /// `base_cost * ESTATE_PRICE_GROWTH.powi(n)` where `n` is the
    /// number already owned in this tier.
    pub base_cost: u64,
    /// Yield per worker per real-time second. Multiplied by the
    /// active Form's affinity and by the elapsed-tick delta before
    /// being added to the inventory.
    pub yield_per_sec: u64,
    /// Which inventory currency this tier produces.
    pub produces: EstateResource,
}

/// What an estate worker pours into the player's stash. We reuse
/// existing currencies (wheat / gold / essence) instead of inventing
/// food / wood / minerals — keeps the wire format unchanged and
/// avoids the schema churn the original §B2 sketch would have
/// required for new resource fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EstateResource {
    Wheat,
    Gold,
    Essence,
}

/// Number of tiers shipped in the B2 MVP. The original backlog
/// proposed six (Farmhand → Guardian); we ship four for now to keep
/// the balance surface small and add the rest once playtesting
/// settles the curve.
pub const ESTATE_TIER_COUNT: u8 = 4;

/// 1.07 base growth, encoded as basis points (10_000 = ×1.0). Match
/// the Your Chronicle scaling pinned in §5.1 of the backlog.
pub const ESTATE_PRICE_GROWTH_BP: u64 = 10_700;

pub const ESTATE_TIERS: &[EstateTierDef] = &[
    EstateTierDef {
        id: 0,
        name: "Farmhand",
        base_cost: 50,
        yield_per_sec: 1,
        produces: EstateResource::Wheat,
    },
    EstateTierDef {
        id: 1,
        name: "Forager",
        base_cost: 500,
        yield_per_sec: 4,
        produces: EstateResource::Wheat,
    },
    EstateTierDef {
        id: 2,
        name: "Trader",
        base_cost: 5_000,
        yield_per_sec: 1,
        produces: EstateResource::Gold,
    },
    EstateTierDef {
        id: 3,
        name: "Sage",
        base_cost: 50_000,
        yield_per_sec: 2,
        produces: EstateResource::Essence,
    },
];

pub fn estate_tier(id: u8) -> Option<&'static EstateTierDef> {
    ESTATE_TIERS.iter().find(|t| t.id == id)
}

/// Price of *the next* worker for `tier`, given the count already
/// owned. Pure function — used by buy validation and by the
/// frontend's "next price" label.
pub fn estate_next_price(tier: &EstateTierDef, owned: u64) -> u64 {
    // `base_cost * (1.07)^owned` via repeated multiplication in
    // basis points. Caps at u64::MAX on overflow rather than wrapping.
    let mut price = tier.base_cost;
    let mut n = owned;
    while n > 0 {
        price = price.saturating_mul(ESTATE_PRICE_GROWTH_BP) / 10_000;
        n -= 1;
        if price == u64::MAX {
            break;
        }
    }
    price
}

/// Form-affinity table (backlog B3). Multiplier in basis points
/// applied to `yield_per_sec` for the given tier while the hero is
/// in `form`. `10_000` = ×1.0 (neutral); above is a buff, below is
/// a penalty / trade-off. Forms shipped today are Human / Slime /
/// Cat / Dragon / Horse (see `forms.rs`); the backlog's
/// Wolf/Bear/Eagle/Fox names are mapped here to whichever existing
/// form fits the niche most naturally.
pub fn form_affinity_bp(form: u8, tier_id: u8) -> u64 {
    match (form, tier_id) {
        // Human — balanced baseline. All tiers neutral.
        (FORM_HUMAN, _) => 10_000,
        // Slime: capstone — slight buff across the board (the
        // "everything-form" stand-in for the Guardian capstone).
        (FORM_SLIME, _) => 13_000,
        // Cat: dexterity / mind. Buffs Forager + Sage.
        (FORM_CAT, 1) | (FORM_CAT, 3) => 17_500,
        (FORM_CAT, _) => 7_500,
        // Dragon: power / trade. Buffs Trader + Sage.
        (FORM_DRAGON, 2) | (FORM_DRAGON, 3) => 17_500,
        (FORM_DRAGON, _) => 7_500,
        // Horse: labor. Buffs Farmhand + Forager.
        (FORM_HORSE, 0) | (FORM_HORSE, 1) => 17_500,
        (FORM_HORSE, _) => 7_500,
        _ => 10_000,
    }
}

/// Persistent estate state. Lives inside `InventoryV12` and
/// participates in the additive-composition pattern documented on
/// `InventoryV11`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct EstateState {
    /// Worker counts keyed by `tier.id`. Missing key ≡ 0.
    pub workers: BTreeMap<u8, u64>,
    /// Wall-clock of the last tick that paid out yield. Updated by
    /// `tick_estate` on every accrual call.
    pub last_tick_ms: u64,
}

impl EstateState {
    pub fn workers_of(&self, tier_id: u8) -> u64 {
        self.workers.get(&tier_id).copied().unwrap_or(0)
    }

    pub fn hire(&mut self, tier_id: u8) {
        let n = self.workers.entry(tier_id).or_insert(0);
        *n = n.saturating_add(1);
    }
}

/// Single-active-action selector (§5.6). When this is `Estate` the
/// delegate ticks worker yield and the auto-mission loop is gated
/// off; when it's `AutoMission` the opposite. `None` means the
/// player is in active play — neither idle loop runs.
pub const IDLE_ACTION_NONE: u8 = 0;
pub const IDLE_ACTION_AUTO_MISSION: u8 = 1;
pub const IDLE_ACTION_ESTATE: u8 = 2;
