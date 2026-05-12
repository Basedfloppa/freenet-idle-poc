//! Forms — transformation prestige loop. Each form gates which
//! gear slots are available and applies passive stat bonuses.

use super::SLOT_COUNT;

pub const FORM_HUMAN: u8 = 0;
pub const FORM_SLIME: u8 = 1;
pub const FORM_CAT: u8 = 2;
pub const FORM_DRAGON: u8 = 3;
pub const FORM_HORSE: u8 = 4;
pub const FORM_COUNT: usize = 5;

pub fn form_name(form: u8) -> &'static str {
    match form {
        FORM_HUMAN => "Human",
        FORM_SLIME => "Slime",
        FORM_CAT => "Cat",
        FORM_DRAGON => "Dragon",
        FORM_HORSE => "Horse",
        _ => "Unknown",
    }
}

pub fn form_sprite(form: u8) -> &'static str {
    match form {
        FORM_HUMAN => "🧝",
        FORM_SLIME => "🟢",
        FORM_CAT => "🐱",
        FORM_DRAGON => "🐲",
        FORM_HORSE => "🐴",
        _ => "❓",
    }
}

pub fn form_slot_mask(form: u8) -> [bool; SLOT_COUNT] {
    match form {
        FORM_HUMAN => [true, true, true, true, true, true, true, true],
        FORM_SLIME => [true, false, false, false, false, false, false, true],
        FORM_CAT => [true, true, false, false, false, false, true, true],
        FORM_DRAGON => [true, false, true, false, false, false, false, true],
        FORM_HORSE => [true, false, true, true, false, false, true, true],
        _ => [true, true, true, true, true, true, true, true],
    }
}

pub fn form_base_bonuses(form: u8) -> (u64, u64, u64) {
    match form {
        FORM_HUMAN => (0, 0, 0),
        FORM_SLIME => (0, 5, 15),
        FORM_CAT => (4, 0, 0),
        FORM_DRAGON => (10, 0, 10),
        FORM_HORSE => (2, 3, 8),
        _ => (0, 0, 0),
    }
}

pub fn form_speed_evasion(form: u8) -> (u64, u64) {
    match form {
        FORM_HUMAN => (100, 0),
        FORM_SLIME => (50, 0),
        FORM_CAT => (130, 15),
        FORM_DRAGON => (110, 5),
        FORM_HORSE => (140, 10),
        _ => (100, 0),
    }
}
