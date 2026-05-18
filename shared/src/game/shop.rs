//! Mission economy + shop + consumables + farm prices.

use super::{GEAR_CATALOG_SIZE, SLOT_COUNT, TIER_COUNT};

// Mission rewards
pub const MISSION_GOLD: u64 = 10;
pub const MISSION_ESSENCE: u64 = 5;
pub const MISSION_DAMAGE: u64 = 1;

pub const WORLD_BOSS_MAX_HP: u64 = 500;

/// Drop cadences for mission rewards.
pub const GEAR_DROP_EVERY: u64 = 5;
pub const POTION_DROP_EVERY: u64 = 13;
pub const FIREBALL_DROP_EVERY: u64 = 19;

pub const ENCOUNTERS_PER_MISSION: u32 = 3;

// Shop
pub const POTION_PRICE: u64 = 50;
pub const FIREBALL_PRICE: u64 = 80;
pub const POTION_BURST_MISSIONS: u64 = 5;
pub const FIREBALL_BOSS_DAMAGE: u64 = 25;

pub const CONSUMABLE_POTION: u8 = 0;
pub const CONSUMABLE_FIREBALL: u8 = 1;

/// Buy-back price the merchant pays for a single consumable.
/// Half of the buy price (round-down) — same convention as
/// `gear_sell_price`. Players who over-stockpiled potions can
/// liquidate without feeling like the shop is ripping them off
/// at vendor-trash rates.
pub fn consumable_sell_price(kind: u8) -> Option<u64> {
    match kind {
        CONSUMABLE_POTION => Some(POTION_PRICE / 2),
        CONSUMABLE_FIREBALL => Some(FIREBALL_PRICE / 2),
        _ => None,
    }
}

pub fn shop_buy_price(tier: u8) -> u64 {
    match tier {
        1 => 100,
        2 => 250,
        3 => 600,
        _ => u64::MAX,
    }
}

pub fn shop_roll_catalog_id(slot: u8, tier: u8, _counter: u64) -> Option<u16> {
    if slot as usize >= SLOT_COUNT || tier == 0 || tier > TIER_COUNT {
        return None;
    }
    let cid = (slot as u16) + ((tier as u16 - 1) * 8);
    if cid >= GEAR_CATALOG_SIZE {
        return None;
    }
    Some(cid)
}

pub const WHEAT_PER_GOLD: u64 = 10;

/// Essence-to-gold conversion at the merchant. 1 essence → N gold.
/// Sized so a post-Ascend essence pile (~10k after the first
/// Ascend) maps to a couple of T3 gear sets but is still inferior
/// to grinding Boss areas for raw gold — the merchant exists to
/// give essence-rich / gold-poor players a way out of the slump,
/// not to make essence the dominant economy.
pub const ESSENCE_TO_GOLD_RATE: u64 = 5;

/// Gold cost of a shop-bought form change. Returns `None` for the
/// catch-all "unknown form" case so the delegate can reject the
/// purchase. Human is intentionally the cheapest — it's the
/// "panic reset" used when a player gets stuck in a defeat-
/// induced form they didn't want; the other four are *much*
/// more expensive because they're a strategic equip-mask + stat
/// commitment that ICSBAH-style players will want to plan around.
/// Personal opt-in World Boss attack constants (backlog C1).
/// Gates intentionally lift-off after the player has demonstrably
/// committed to the loop: 100 missions ≈ first long session,
/// level 10 = mid-game, owning at least one Estate worker proves
/// they've engaged with the economy spine.
pub const BOSS_ATTACK_MIN_MISSIONS: u64 = 100;
pub const BOSS_ATTACK_MIN_LEVEL: u64 = 10;
pub const BOSS_ATTACK_ESSENCE_COST: u64 = 200;
pub const BOSS_ATTACK_DAMAGE: u64 = 50;

pub fn form_buy_price(form: u8) -> Option<u64> {
    use super::{FORM_CAT, FORM_DRAGON, FORM_HORSE, FORM_HUMAN, FORM_SLIME};
    match form {
        FORM_HUMAN => Some(1_500),
        FORM_SLIME => Some(20_000),
        FORM_CAT => Some(25_000),
        FORM_HORSE => Some(35_000),
        FORM_DRAGON => Some(60_000),
        _ => None,
    }
}
