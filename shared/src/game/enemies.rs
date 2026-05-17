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
    // ---- Deep Forest (area 4) ----
    EnemyDef { id: 40, name: "moss revenant", sprite: "🌿",
        hp: 50, atk: 11, def: 5, gold_reward: 18, xp_reward: 70,
        speed: 95, evasion: 8, transform_to: FORM_HUMAN,
        death_blurb: "The revenant returns to the moss; you crawl out of the thicket grasping a fistful of essence-soaked leaves." },
    EnemyDef { id: 41, name: "lantern moth", sprite: "🦋",
        hp: 40, atk: 9, def: 3, gold_reward: 14, xp_reward: 55,
        speed: 150, evasion: 25, transform_to: FORM_HUMAN,
        death_blurb: "The moth's wings powder onto your sleeves. You smell of pollen for a week but no one asks why." },
    // ---- Snowfields (area 5) ----
    EnemyDef { id: 50, name: "frost wolf", sprite: "🐺",
        hp: 110, atk: 24, def: 11, gold_reward: 38, xp_reward: 150,
        speed: 145, evasion: 12, transform_to: FORM_HUMAN,
        death_blurb: "The wolf yields its pelt; the wind picks up and you walk back lighter than you came." },
    EnemyDef { id: 51, name: "ice marauder", sprite: "🥶",
        hp: 130, atk: 28, def: 14, gold_reward: 42, xp_reward: 160,
        speed: 105, evasion: 5, transform_to: FORM_HUMAN,
        death_blurb: "The marauder freezes mid-charge. You pry a sigil out of his palm and trudge home through the drifts." },
    // ---- Wilds roster (areas 100+) — procedural areas pick from
    // this pool indexed by their depth tier; HP/atk are base-level
    // (real values are stretched by `scale_by_area_level`).
    EnemyDef { id: 100, name: "thorn hound", sprite: "🪲",
        hp: 60, atk: 13, def: 6, gold_reward: 20, xp_reward: 80,
        speed: 120, evasion: 10, transform_to: FORM_HUMAN,
        death_blurb: "The thorn hound dissolves into bramble. Sap clings to your boots all the way back." },
    EnemyDef { id: 101, name: "whisper kin", sprite: "🫥",
        hp: 70, atk: 15, def: 7, gold_reward: 22, xp_reward: 90,
        speed: 110, evasion: 18, transform_to: FORM_HUMAN,
        death_blurb: "The whisper kin go quiet. You leave with the kind of silence that doesn't lift even at the campfire." },
    EnemyDef { id: 102, name: "fen wraith", sprite: "🌫️",
        hp: 80, atk: 17, def: 8, gold_reward: 25, xp_reward: 100,
        speed: 100, evasion: 14, transform_to: FORM_HUMAN,
        death_blurb: "The fen wraith melts into the marsh. The water remembers you longer than you remember it." },
    EnemyDef { id: 103, name: "stone wanderer", sprite: "🗿",
        hp: 95, atk: 18, def: 12, gold_reward: 30, xp_reward: 110,
        speed: 75, evasion: 4, transform_to: FORM_HUMAN,
        death_blurb: "The wanderer crumbles. You pocket a single warm shard and head out before the rest goes still." },
    EnemyDef { id: 104, name: "drift hawk", sprite: "🦅",
        hp: 65, atk: 16, def: 6, gold_reward: 24, xp_reward: 95,
        speed: 160, evasion: 30, transform_to: FORM_HUMAN,
        death_blurb: "The hawk wheels once and is gone. A single feather lands at your feet — heavier than any feather should be." },
    EnemyDef { id: 105, name: "pall hunter", sprite: "🦇",
        hp: 110, atk: 22, def: 10, gold_reward: 34, xp_reward: 130,
        speed: 130, evasion: 18, transform_to: FORM_HUMAN,
        death_blurb: "The pall hunter folds away under the canopy. The forest exhales and you move on." },
    EnemyDef { id: 106, name: "veil sentinel", sprite: "🛡️",
        hp: 140, atk: 26, def: 16, gold_reward: 40, xp_reward: 150,
        speed: 95, evasion: 6, transform_to: FORM_HUMAN,
        death_blurb: "The sentinel lays down its halberd. You take its tally-stick — already cut with a notch for you." },
    EnemyDef { id: 107, name: "crag broodling", sprite: "🦂",
        hp: 130, atk: 28, def: 12, gold_reward: 38, xp_reward: 140,
        speed: 120, evasion: 14, transform_to: FORM_HUMAN,
        death_blurb: "The broodling rattles once and is still. You climb out of the crag carrying its sigil cooling in your hand." },
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
        4 => &[40, 41, 10, 11],
        5 => &[50, 51, 20, 21],
        // Wilds entrance (area 100) — light tier.
        100 => &[100, 101],
        101 => &[101, 102, 104],
        102 => &[102, 100, 105],
        103 => &[103, 105, 106],
        104 => &[104, 101, 105],
        105 => &[105, 103, 106, 107],
        106 => &[106, 103, 105, 107],
        107 => &[107, 105, 106, 104],
        _ => &[0],
    }
}

pub fn enemy_def(id: u16) -> Option<&'static EnemyDef> {
    ENEMIES.iter().find(|e| e.id == id)
}
