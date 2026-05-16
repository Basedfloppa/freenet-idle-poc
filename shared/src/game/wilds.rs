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
/// returned `AreaDef`. Easiest way to satisfy the lifetime is
/// to leak into a small interned pool — the table is tiny
/// (8 nodes × ~12 names), the allocation happens once per
/// session, and never freeing is fine because the generator
/// lives for the page lifetime.
fn wilds_name(id: u32, rng: &mut WildsRng) -> &'static str {
    const ROOTS: [&str; 12] = [
        "Thorn", "Glade", "Hollow", "Reach", "Mire", "Veil",
        "Spire", "Wend", "Crag", "Wisp", "Drift", "Pall",
    ];
    const SUFFIXES: [&str; 12] = [
        "Wood", "Hill", "Pass", "Fen", "Crossing", "Cradle",
        "Spur", "Ridge", "Hollow", "Vale", "Knoll", "Cleft",
    ];
    let r = (rng.next() ^ id) as usize;
    let s = (rng.next() ^ id.rotate_left(7)) as usize;
    let root = ROOTS[r % ROOTS.len()];
    let suffix = SUFFIXES[s % SUFFIXES.len()];
    let combined = format!("{root}{suffix}");
    Box::leak(combined.into_boxed_str())
}

fn wilds_blurb(name: &'static str, id: u32) -> &'static str {
    const ATMOS: [&str; 6] = [
        "off-path, unmarked on the village map",
        "moss-thick and breath-quiet between the columns of stone",
        "where the path forks back on itself if you blink wrong",
        "the wind here sounds like other people's footsteps",
        "old battle ground — the ghosts are uninterested but watching",
        "places named here only when you've stayed past dusk",
    ];
    let blurb = format!("{} — {}", name, ATMOS[(id as usize) % ATMOS.len()]);
    Box::leak(blurb.into_boxed_str())
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
