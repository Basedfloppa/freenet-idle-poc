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

/// Token award for the era-advance ranked-claim path (C2).
/// Top contributor gets 3 tokens, runner-up 2, third place 1,
/// everyone else 0. The personal-milestone path
/// (`TOKEN_PER_BOSS_DAMAGE`) still runs in parallel — ranked
/// claim is the *bonus* on top of milestone tokens for players
/// who actively chase the contribution leaderboard.
pub fn boss_kill_tokens_for_rank(rank: u8) -> u64 {
    match rank {
        0 => 3,
        1 => 2,
        2 => 1,
        _ => 0,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TokenPerk {
    /// Champion badge — cosmetic marker on the leaderboard row.
    ChampionBadge = 0,
    /// +20% gear stat contribution for every equipped piece.
    /// Stacks multiplicatively after Legacy nodes and Insight.
    GearMastery = 1,
    /// Doubles the World Boss damage dealt by every mission
    /// (mission_damage × 2 from the boss-contact area `damage_mult`).
    BossFury = 2,
    /// +1 potion dropped every `POTION_DROP_EVERY` missions
    /// (so the cadence becomes 2 instead of 1).
    AlchemistTrust = 3,
    /// +50% gold from every encounter. Applies after Legacy
    /// MissionGold and Insight GoldDropPct so it compounds.
    MerchantSeal = 4,
}

impl TokenPerk {
    pub const ALL: &'static [TokenPerk] = &[
        TokenPerk::ChampionBadge,
        TokenPerk::GearMastery,
        TokenPerk::BossFury,
        TokenPerk::AlchemistTrust,
        TokenPerk::MerchantSeal,
    ];

    pub fn id(self) -> u8 {
        self as u8
    }

    pub fn from_id(id: u8) -> Option<TokenPerk> {
        match id {
            0 => Some(TokenPerk::ChampionBadge),
            1 => Some(TokenPerk::GearMastery),
            2 => Some(TokenPerk::BossFury),
            3 => Some(TokenPerk::AlchemistTrust),
            4 => Some(TokenPerk::MerchantSeal),
            _ => None,
        }
    }

    /// JSON i18n key tail (`token_perk_name.<key>` / `token_perk_desc.<key>`).
    pub fn key(self) -> &'static str {
        match self {
            TokenPerk::ChampionBadge => "champion_badge",
            TokenPerk::GearMastery => "gear_mastery",
            TokenPerk::BossFury => "boss_fury",
            TokenPerk::AlchemistTrust => "alchemist_trust",
            TokenPerk::MerchantSeal => "merchant_seal",
        }
    }

    /// English fallback shown if no translation is loaded.
    pub fn name(self) -> &'static str {
        match self {
            TokenPerk::ChampionBadge => "Champion badge",
            TokenPerk::GearMastery => "Gear mastery (+20% gear stats)",
            TokenPerk::BossFury => "Boss fury (×2 mission boss damage)",
            TokenPerk::AlchemistTrust => "Alchemist's trust (×2 potion drops)",
            TokenPerk::MerchantSeal => "Merchant's seal (+50% encounter gold)",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            TokenPerk::ChampionBadge =>
                "Cosmetic marker on your leaderboard row showing you've cleared the boss-damage milestone tree.",
            TokenPerk::GearMastery =>
                "Multiplies the attack/defence/HP bonuses from every equipped piece by 1.20. Stacks multiplicatively with Legacy nodes and Insight.",
            TokenPerk::BossFury =>
                "Doubles the mission boss damage you contribute on every encounter at a boss-contact area.",
            TokenPerk::AlchemistTrust =>
                "Doubles the potion drop cadence — one potion per POTION_DROP_EVERY missions becomes two.",
            TokenPerk::MerchantSeal =>
                "Adds +50% to every encounter's gold reward. Compounds with Legacy MissionGold and Insight GoldDropPct.",
        }
    }

    /// One-shot price in tokens. Higher-impact perks cost more
    /// since they affect gameplay rather than cosmetics.
    pub fn price(self) -> u64 {
        match self {
            TokenPerk::ChampionBadge => 1,
            TokenPerk::AlchemistTrust => 3,
            TokenPerk::MerchantSeal => 5,
            TokenPerk::GearMastery => 8,
            TokenPerk::BossFury => 10,
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

    /// Multiplier in basis points (10_000 = ×1.0) applied to
    /// equipment-bonus totals when GearMastery is owned.
    pub fn gear_mult_bp(&self) -> u64 {
        if self.owns(TokenPerk::GearMastery) { 12_000 } else { 10_000 }
    }

    /// Multiplier in basis points for boss damage per mission.
    pub fn boss_damage_mult_bp(&self) -> u64 {
        if self.owns(TokenPerk::BossFury) { 20_000 } else { 10_000 }
    }

    /// Multiplier in basis points for encounter gold.
    pub fn gold_mult_bp(&self) -> u64 {
        if self.owns(TokenPerk::MerchantSeal) { 15_000 } else { 10_000 }
    }

    /// Number of potions dropped at every `POTION_DROP_EVERY`
    /// milestone (1 by default, 2 with AlchemistTrust).
    pub fn potion_drop_count(&self) -> u32 {
        if self.owns(TokenPerk::AlchemistTrust) { 2 } else { 1 }
    }
}
