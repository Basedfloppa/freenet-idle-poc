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
