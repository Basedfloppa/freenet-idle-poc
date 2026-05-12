//! Gear catalog — 8 slots × 4 tiers = 32 catalog ids — plus the
//! sell-price and forge-cost curves.

pub const SLOT_COUNT: usize = 8;
pub const SLOT_NAMES: [&str; SLOT_COUNT] = [
    "Helm", "Cloak", "Chest", "Pants", "Shield", "Sword", "Boots", "Ring",
];
pub const TIER_COUNT: u8 = 4;
pub const TIER_PREFIXES: [&str; 4] = ["Worn", "Polished", "Runed", "Legendary"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GearTemplate {
    pub catalog_id: u16,
    pub slot: u8,
    pub tier: u8,
    pub atk: u32,
    pub def: u32,
    pub hp: u32,
}

impl GearTemplate {
    pub fn name(&self) -> String {
        format!(
            "{} {}",
            TIER_PREFIXES[(self.tier - 1) as usize],
            SLOT_NAMES[self.slot as usize]
        )
    }
}

pub const GEAR_CATALOG_SIZE: u16 = 32;

pub fn gear_template(catalog_id: u16) -> Option<GearTemplate> {
    if catalog_id >= GEAR_CATALOG_SIZE {
        return None;
    }
    let slot = (catalog_id % 8) as u8;
    let tier = (catalog_id / 8) as u8 + 1;
    let (primary, secondary): (u32, u32) = match tier {
        1 => (2, 1),
        2 => (5, 2),
        3 => (12, 5),
        _ => (30, 12),
    };
    let primary_kind = match slot {
        0 => 1, // Helm — def
        1 => 2, // Cloak — hp
        2 => 1, // Chest — def
        3 => 1, // Pants — def
        4 => 1, // Shield — def
        5 => 0, // Sword — atk
        6 => 0, // Boots — atk
        7 => 0, // Ring — atk
        _ => return None,
    };
    let (atk, def, hp) = match primary_kind {
        0 => (primary, 0, secondary),
        1 => (0, primary, secondary),
        2 => (0, secondary, primary),
        _ => return None,
    };
    Some(GearTemplate { catalog_id, slot, tier, atk, def, hp })
}

pub fn gear_sell_price(tier: u8) -> u64 {
    match tier {
        1 => 5,
        2 => 15,
        3 => 40,
        _ => 100,
    }
}

pub const FORGE_COUNT: usize = 3;

pub fn forge_essence_cost(tier: u8) -> u64 {
    let t = tier as u64;
    50u64.saturating_mul(t).saturating_mul(t)
}
