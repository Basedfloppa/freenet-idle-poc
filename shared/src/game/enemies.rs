//! Enemy roster — `EnemyDef` records keyed by stable id, plus the
//! area→roster mapping used by the encounter picker.

use super::{FORM_CAT, FORM_DRAGON, FORM_HORSE, FORM_HUMAN, FORM_SLIME};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnemyDef {
    pub id: u16,
    pub name: &'static str,
    /// Emoji glyph shown in the combat scene. Deliberately distinct
    /// from any player `form_sprite` so the duel reads as two
    /// different entities even when forms overlap visually
    /// (e.g. slime-form player vs unsettling-slime enemy).
    pub sprite: &'static str,
    pub hp: u64,
    pub atk: u64,
    pub def: u64,
    pub gold_reward: u64,
    pub xp_reward: u64,
    pub speed: u64,
    pub evasion: u64,
    pub transform_to: u8,
    pub death_blurb: &'static str,
}

pub const ENEMIES: &[EnemyDef] = &[
    // ---- Village Fields (area 0) ----
    EnemyDef { id: 0, name: "evil elf", sprite: "👺",
        hp: 10, atk: 3, def: 1, gold_reward: 3, xp_reward: 12,
        speed: 110, evasion: 5, transform_to: FORM_HUMAN,
        death_blurb: "The elf overpowers you and leaves you bleeding by the road. You crawl home, bruised but still yourself." },
    EnemyDef { id: 1, name: "medieval lawyer", sprite: "📜",
        hp: 8, atk: 4, def: 0, gold_reward: 1, xp_reward: 10,
        speed: 90, evasion: 0, transform_to: FORM_HUMAN,
        death_blurb: "The lawyer serves you with a writ that flattens your ego. You limp home, mundane as ever." },
    EnemyDef { id: 2, name: "unsettling slime", sprite: "🦠",
        hp: 15, atk: 3, def: 2, gold_reward: 4, xp_reward: 18,
        speed: 50, evasion: 0, transform_to: FORM_SLIME,
        death_blurb: "The slime splits in two, then the new half lunges at you. You are engulfed in it, your body melting and oozing as you become a green shiny blob of dumb slime." },
    // ---- Forest Road (area 1) ----
    EnemyDef { id: 10, name: "feral cat", sprite: "🐈",
        hp: 25, atk: 7, def: 3, gold_reward: 8, xp_reward: 35,
        speed: 140, evasion: 15, transform_to: FORM_CAT,
        death_blurb: "The cat pounces, biting through your soul. As your vision fades you sprout fur, whiskers, and the deep wisdom of an animal that knocks things off tables." },
    EnemyDef { id: 11, name: "thorn wraith", sprite: "👻",
        hp: 30, atk: 9, def: 4, gold_reward: 12, xp_reward: 45,
        speed: 100, evasion: 10, transform_to: FORM_HUMAN,
        death_blurb: "The wraith's thorns leave your veins glowing green for a week, but you stagger home in one piece." },
    // ---- Mountain Pass (area 2) ----
    EnemyDef { id: 20, name: "stone golem", sprite: "🗿",
        hp: 80, atk: 18, def: 12, gold_reward: 30, xp_reward: 110,
        speed: 60, evasion: 0, transform_to: FORM_HUMAN,
        death_blurb: "The golem hammers you flat. You wake up at the trailhead, dented but uncrystallised." },
    EnemyDef { id: 21, name: "warhorse spirit", sprite: "🐎",
        hp: 70, atk: 16, def: 10, gold_reward: 28, xp_reward: 100,
        speed: 150, evasion: 5, transform_to: FORM_HORSE,
        death_blurb: "The warhorse rears, and as its hooves come down you feel your spine lengthen, your hands fuse, your dignity recede. You are now a sturdy quadruped." },
    // ---- Boss's Lair (area 3) ----
    EnemyDef { id: 30, name: "young dragon", sprite: "🐲",
        hp: 200, atk: 45, def: 22, gold_reward: 80, xp_reward: 300,
        speed: 120, evasion: 10, transform_to: FORM_DRAGON,
        death_blurb: "The dragon's fire fuses your bones into scales. When it ends, you cannot remember how to be small. You are dragon now." },
    EnemyDef { id: 31, name: "shadow lord", sprite: "💀",
        hp: 250, atk: 50, def: 25, gold_reward: 120, xp_reward: 400,
        speed: 130, evasion: 20, transform_to: FORM_HUMAN,
        death_blurb: "The shadow lord drains you to a husk, but your skin holds. You stagger back to the village, still human, still alive — barely." },
];

/// Sprite for the area's "default" enemy slot — used by the idle
/// combat scene before any battle has started. Falls back to a
/// generic monster emoji if the roster's first enemy can't be
/// resolved.
pub fn area_default_enemy_sprite(area_id: u8) -> &'static str {
    enemy_roster_for_area(area_id)
        .first()
        .and_then(|id| enemy_def(*id))
        .map(|e| e.sprite)
        .unwrap_or("👹")
}

pub fn enemy_roster_for_area(area_id: u8) -> &'static [u16] {
    match area_id {
        0 => &[0, 1, 2],
        1 => &[10, 11],
        2 => &[20, 21],
        3 => &[30, 31],
        _ => &[0],
    }
}

pub fn enemy_def(id: u16) -> Option<&'static EnemyDef> {
    ENEMIES.iter().find(|e| e.id == id)
}
