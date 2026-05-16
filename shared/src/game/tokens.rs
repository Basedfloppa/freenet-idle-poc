//! Tokens for boss-kill contribution (backlog C2, MVP scope).
//!
//! Earn rule: 1 token per `TOKEN_PER_BOSS_DAMAGE` of personal
//! cumulative `boss_damage`. The original spec was "top-N
//! contributors at era cross", which needs contract-side ranking
//! we don't have yet — milestone-on-personal-watermark gets the
//! token loop into player hands without contract cooperation.
//! Self-attested like `boss_damage` itself; the contract's
//! per-key monotonicity prevents the obvious cheat (regressing
//! `boss_damage` and re-earning the milestone).
//!
//! Spend: cosmetic perks today (leaderboard badge), with the
//! second-auto-mission-preset slot ready when its dependency
//! lands.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub const TOKEN_PER_BOSS_DAMAGE: u64 = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TokenPerk {
    /// Show a "champion" badge on the player's leaderboard row.
    /// Pure cosmetic; one purchase unlocks it forever.
    ChampionBadge = 0,
    /// Extra equipped-gear slot (`SLOT_COUNT + 1`). Currently
    /// not wired into the slot mask — the perk just records the
    /// purchase; the slot extension lands as a follow-up when
    /// gear-mask plumbing supports a 9th slot.
    ExtraSlot = 1,
    /// Second auto-mission preset (the "farm mode vs exp mode"
    /// switcher). Like ExtraSlot, the perk records the purchase
    /// today; the preset switcher is a frontend follow-up.
    SecondAutoPreset = 2,
}

impl TokenPerk {
    pub const ALL: &'static [TokenPerk] = &[
        TokenPerk::ChampionBadge,
        TokenPerk::ExtraSlot,
        TokenPerk::SecondAutoPreset,
    ];

    pub fn id(self) -> u8 {
        self as u8
    }

    pub fn from_id(id: u8) -> Option<TokenPerk> {
        match id {
            0 => Some(TokenPerk::ChampionBadge),
            1 => Some(TokenPerk::ExtraSlot),
            2 => Some(TokenPerk::SecondAutoPreset),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            TokenPerk::ChampionBadge => "Champion badge",
            TokenPerk::ExtraSlot => "Extra gear slot",
            TokenPerk::SecondAutoPreset => "Second auto-mission preset",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            TokenPerk::ChampionBadge =>
                "Cosmetic marker on your leaderboard row showing you've cleared the boss-damage milestone tree.",
            TokenPerk::ExtraSlot =>
                "Reserves a 9th equipment slot for a future gear-mask expansion. Cosmetic for now.",
            TokenPerk::SecondAutoPreset =>
                "Unlocks the slot for a second auto-mission preset (separate HP threshold + area). UI for switching the preset lands as a follow-up.",
        }
    }

    /// One-shot price in tokens. No level curve for cosmetics —
    /// each perk is either unlocked or not.
    pub fn price(self) -> u64 {
        match self {
            TokenPerk::ChampionBadge => 1,
            TokenPerk::ExtraSlot => 5,
            TokenPerk::SecondAutoPreset => 3,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct TokenState {
    /// Unspent tokens.
    pub balance: u64,
    /// Bought perks. Value is the timestamp of purchase for the
    /// achievement log; `contains_key` is the "is owned" check.
    pub perks: BTreeMap<u8, u64>,
    /// High-watermark of personal `boss_damage` already paid out.
    /// Awards happen lazily via `award_pending_tokens` whenever
    /// `boss_damage` grows past the next milestone.
    pub last_awarded_boss_damage: u64,
}

impl TokenState {
    pub fn owns(&self, perk: TokenPerk) -> bool {
        self.perks.contains_key(&perk.id())
    }
}
