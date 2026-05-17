//! Wilds — procedurally-generated alternate map (backlog C3b).
//!
//! Deterministic from a `u32` seed (today: derived from the
//! delegate's pubkey so each player gets a stable personal map
//! without contract cooperation). Same fields as `AreaDef` so
//! the existing graph viewer + activity / mission paths can
//! treat Wilds nodes as just-another-area.
//!
//! The generator emits **8 nodes** in a small DAG with branching
//! + one cycle, gated late (`min_level ≥ 15`) so it doesn't
//! step on the linear progression chain. Node IDs live in a
//! separate namespace (`WILDS_AREA_BASE = 100`) to avoid
//! colliding with the hardcoded areas 0..5.

use super::AreaDef;

pub const WILDS_AREA_BASE: u8 = 100;
pub const WILDS_NODE_COUNT: u8 = 8;

/// Build the Wilds DAG for the given seed. The shape is fixed —
/// only the names + enemy stat noise come from the RNG — so all
/// players have the same connectivity but their flavour text
/// differs. Keeps the design space small enough to balance.
pub fn wilds_areas(seed: u32) -> Vec<AreaDef> {
    // Pre-baked DAG topology (depth × node-at-depth):
    //   D0: 100 (entrance)
    //   D1: 101, 102          (two branches from entrance)
    //   D2: 103 (from 101), 104 (from 102), 105 (from BOTH 101+102)
    //   D3: 106 (from 103), 107 (from 104 and 105 — cycle through 105)
    //   D4: cycle-back edge from 107 → 105 expressed by listing 105
    //       in 107.predecessors too (graph is already DAG above; the
    //       "cycle" is a backward-feel narrative more than a true
    //       loop — keeps `clears_required` semantics intact).
    //
    // Splay enemy stats by depth so the gating + difficulty curve
    // ramps even though the graph is small.
    let mut out = Vec::with_capacity(WILDS_NODE_COUNT as usize);

    // Tiny xorshift RNG seeded by `seed` so naming is
    // deterministic and stable across builds.
    let mut rng = WildsRng::new(seed);

    let templates: [(u8, &[u8], u64, u64, u64, u64, u64); 8] = [
        // (id, preds, min_level, gold_mult, essence_mult, enemy_hp, enemy_atk)
        (100, &[],          15, 5, 3,  180,  35),
        (101, &[100],       17, 6, 4,  220,  45),
        (102, &[100],       17, 4, 6,  220,  42),
        (103, &[101],       19, 7, 4,  280,  55),
        (104, &[102],       19, 5, 7,  280,  52),
        (105, &[101, 102],  20, 8, 8,  340,  62),
        (106, &[103],       22, 9, 5,  420,  75),
        (107, &[104, 105],  22, 6, 9,  420,  72),
    ];
    for &(id, preds, min_level, gm, em, ehp, eatk) in &templates {
        let name = wilds_name(id as u32, &mut rng);
        let blurb = wilds_blurb(name, id as u32);
        // Slight enemy-stat jitter from seed (±15%) so the
        // numbers feel unique per player even when topology is
        // fixed. Within a single player's session the values
        // stay constant — `rng` is rebuilt per call but always
        // from the same seed.
        let mut jitter = |base: u64| -> u64 {
            let drift = (rng.next() % 31) as i32 - 15; // [-15, +15]
            let scaled = base as i64 * (100 + drift as i64) / 100;
            scaled.max(1) as u64
        };
        out.push(AreaDef {
            id,
            name,
            blurb,
            min_level,
            gold_mult: gm,
            essence_mult: em,
            damage_mult: 0, // Wilds doesn't contribute to the boss
            enemy_hp: jitter(ehp),
            enemy_atk: jitter(eatk),
            enemy_def: jitter(eatk / 2),
            clears_required: 15 + (min_level - 15) * 3,
            predecessors: leak_predecessors(preds),
        });
    }
    out
}

/// The Wilds names get baked into `&'static str` slots on the
/// returned `AreaDef`. The string itself is a stable marker
/// `wilds:<root>:<suffix>` — the frontend's i18n layer detects
/// the prefix and assembles a localized name from translated
/// ROOTS/SUFFIXES JSON tables (key `wilds_root.<r>` /
/// `wilds_suffix.<s>`), falling back to the English wordlist
/// when no translation is registered. The pool is tiny (8 nodes
/// per player), allocation happens once per session, and never
/// freeing is fine because the generator lives for the page
/// lifetime.
pub const WILDS_ROOTS_EN: [&str; 12] = [
    "Thorn", "Glade", "Hollow", "Reach", "Mire", "Veil",
    "Spire", "Wend", "Crag", "Wisp", "Drift", "Pall",
];
pub const WILDS_SUFFIXES_EN: [&str; 12] = [
    "Wood", "Hill", "Pass", "Fen", "Crossing", "Cradle",
    "Spur", "Ridge", "Hollow", "Vale", "Knoll", "Cleft",
];

fn wilds_name(id: u32, rng: &mut WildsRng) -> &'static str {
    let r = (rng.next() ^ id) as usize % WILDS_ROOTS_EN.len();
    let s = (rng.next() ^ id.rotate_left(7)) as usize % WILDS_SUFFIXES_EN.len();
    let marker = format!("wilds:{r}:{s}");
    Box::leak(marker.into_boxed_str())
}

/// Reverse of `wilds_name`'s marker encoding — extract
/// `(root_idx, suffix_idx)` so the frontend can rebuild a
/// localized name. Returns `None` if the slot doesn't carry
/// a Wilds marker (linear areas).
pub fn parse_wilds_name(s: &str) -> Option<(usize, usize)> {
    let mut parts = s.strip_prefix("wilds:")?.split(':');
    let r: usize = parts.next()?.parse().ok()?;
    let s: usize = parts.next()?.parse().ok()?;
    Some((r, s))
}

/// Fallback English name assembled from the static wordlists.
pub fn wilds_default_name(root_idx: usize, suffix_idx: usize) -> String {
    let r = WILDS_ROOTS_EN.get(root_idx).copied().unwrap_or("Wilds");
    let s = WILDS_SUFFIXES_EN.get(suffix_idx).copied().unwrap_or("Reach");
    format!("{r}{s}")
}

fn wilds_blurb(_name: &'static str, id: u32) -> &'static str {
    // Encode the atmosphere index as a marker (`wildsblurb:<idx>`)
    // so the i18n layer can swap the body in the active locale.
    // Both name and blurb resolve at display time — keeping the
    // procedural seed deterministic without baking English copy
    // into the AreaDef slot.
    let idx = (id as usize) % 6;
    let marker = format!("wildsblurb:{idx}");
    Box::leak(marker.into_boxed_str())
}

pub fn parse_wilds_blurb(s: &str) -> Option<usize> {
    s.strip_prefix("wildsblurb:")?.parse().ok()
}

fn leak_predecessors(preds: &[u8]) -> &'static [u8] {
    Box::leak(preds.to_vec().into_boxed_slice())
}

/// Cheap reproducible PRNG — xorshift32. `next()` consumes one
/// step. Same seed always yields the same sequence; we lean on
/// that for deterministic name generation across page reloads.
struct WildsRng {
    state: u32,
}

impl WildsRng {
    fn new(seed: u32) -> Self {
        // Avoid the all-zero seed pathology of xorshift.
        let s = if seed == 0 { 0xdead_beef } else { seed };
        Self { state: s }
    }
    fn next(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        x
    }
}
