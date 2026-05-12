//! Permanent passive bonuses — unlocked by killing a form-shifting
//! enemy in that form, or bought outright from the shop.

use std::collections::BTreeMap;

use super::{FORM_CAT, FORM_DRAGON, FORM_HORSE, FORM_SLIME};

pub const SKILL_SLIME_BODY: u8 = 0;
pub const SKILL_FELINE_GRACE: u8 = 1;
pub const SKILL_DRAGON_SCALES: u8 = 2;
pub const SKILL_STEED_HEART: u8 = 3;
pub const SKILL_VETERAN: u8 = 4;
pub const SKILL_CHAMPION: u8 = 5;

pub fn skill_name(id: u8) -> &'static str {
    match id {
        SKILL_SLIME_BODY => "Slime Body",
        SKILL_FELINE_GRACE => "Feline Grace",
        SKILL_DRAGON_SCALES => "Dragon Scales",
        SKILL_STEED_HEART => "Steed Heart",
        SKILL_VETERAN => "Veteran",
        SKILL_CHAMPION => "Champion",
        _ => "?",
    }
}

pub fn skill_blurb(id: u8) -> &'static str {
    match id {
        SKILL_SLIME_BODY => "You've been gooey once. The membrane carries over: +20 HP, +5 defence.",
        SKILL_FELINE_GRACE => "Your reflexes remember the cat: +6 attack.",
        SKILL_DRAGON_SCALES => "Stray scales still cling to your skin: +8 attack, +6 defence.",
        SKILL_STEED_HEART => "A horse's lung capacity outlasts the form: +25 HP, +4 defence.",
        SKILL_VETERAN => "Ten levels of combat experience: +5 attack, +5 defence.",
        SKILL_CHAMPION => "Twenty levels in, you've earned the title: +10 atk, +10 def, +30 HP.",
        _ => "",
    }
}

pub fn skill_bonuses(skills: &BTreeMap<u8, u64>) -> (u64, u64, u64) {
    let mut atk = 0u64;
    let mut def = 0u64;
    let mut hp = 0u64;
    for id in skills.keys() {
        match *id {
            SKILL_SLIME_BODY => { def += 5; hp += 20; }
            SKILL_FELINE_GRACE => { atk += 6; }
            SKILL_DRAGON_SCALES => { atk += 8; def += 6; }
            SKILL_STEED_HEART => { def += 4; hp += 25; }
            SKILL_VETERAN => { atk += 5; def += 5; }
            SKILL_CHAMPION => { atk += 10; def += 10; hp += 30; }
            _ => {}
        }
    }
    (atk, def, hp)
}

pub fn skill_for_form(form: u8) -> Option<u8> {
    match form {
        FORM_SLIME => Some(SKILL_SLIME_BODY),
        FORM_CAT => Some(SKILL_FELINE_GRACE),
        FORM_DRAGON => Some(SKILL_DRAGON_SCALES),
        FORM_HORSE => Some(SKILL_STEED_HEART),
        _ => None,
    }
}

pub fn skill_buy_price(id: u8) -> Option<u64> {
    match id {
        SKILL_SLIME_BODY => Some(400),
        SKILL_FELINE_GRACE => Some(600),
        SKILL_DRAGON_SCALES => Some(1000),
        SKILL_STEED_HEART => Some(750),
        _ => None,
    }
}

pub fn skill_speed_evasion(skills: &BTreeMap<u8, u64>) -> (u64, u64) {
    let mut speed = 0u64;
    let mut evasion = 0u64;
    for id in skills.keys() {
        match *id {
            SKILL_FELINE_GRACE => { speed += 30; evasion += 10; }
            SKILL_DRAGON_SCALES => { speed += 10; }
            SKILL_STEED_HEART => { speed += 20; }
            _ => {}
        }
    }
    (speed, evasion)
}
