//! Terminal milestones — the four ending banners shown once the
//! player meets the corresponding criterion. Idempotent: unlocked
//! once, recorded with timestamp, never recomputed.

pub const ENDING_VICTORY: u8 = 0;
pub const ENDING_DRAGON_LORD: u8 = 1;
pub const ENDING_PILGRIM: u8 = 2;
pub const ENDING_QUIET_FARMER: u8 = 3;
pub const ENDINGS_TOTAL: usize = 4;

pub fn ending_name(id: u8) -> &'static str {
    match id {
        ENDING_VICTORY => "Hero's Victory",
        ENDING_DRAGON_LORD => "Dragon Ascendant",
        ENDING_PILGRIM => "Pilgrim of Forms",
        ENDING_QUIET_FARMER => "Quiet Farmer",
        _ => "?",
    }
}

pub fn ending_blurb(id: u8) -> &'static str {
    match id {
        ENDING_VICTORY => "Felled the Shadow Lord with your bare human hands. The kingdom remembers your name.",
        ENDING_DRAGON_LORD => "You came as dragon and left as dragon, but the Shadow Lord's keep is your eyrie now.",
        ENDING_PILGRIM => "You've worn every shape on the map and decided each one was, technically, also you.",
        ENDING_QUIET_FARMER => "Ten thousand bushels of wheat. The Shadow Lord still lurks somewhere, but the harvest is good.",
        _ => "",
    }
}
