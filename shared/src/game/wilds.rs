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
pub const WILDS_NODE_COUNT: u8 = 12;

/// First-clear landmark reward associated with a specific Wilds
/// node. The watermark of which areas have been claimed lives in
/// `Inventory.landmark_claims` (per-area first-clear timestamp).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WildsLandmark {
    /// One-shot bundle of essence (the value is the amount).
    EssencePool(u64),
    /// A gear-cache drop: tier 1..=3 piece for a random allowed
    /// slot (rolled by the existing `shop_roll_catalog_id`).
    GearCache(u8),
    /// Hidden form scroll — unlocks the named form's slot and adds
    /// it to `forms_visited` immediately.
    HiddenFormScroll(u8),
    /// Bag of tokens (the value is the count).
    TokenBag(u64),
    /// Pool of insight currency.
    InsightTrove(u64),
}

/// Static map of (area_id within the Wilds namespace) → landmark.
/// Independent of the player's seed — every Wilds run has the
/// same landmark layout, only the procedural NAMES differ via
/// the existing seeded RNG. This keeps the design space small
/// enough to balance globally.
pub fn wilds_landmark(area_id: u8) -> Option<WildsLandmark> {
    if area_id < WILDS_AREA_BASE {
        return None;
    }
    match area_id {
        // 100/101/102 — entrance + branch trees, no landmark
        103 => Some(WildsLandmark::EssencePool(150)),
        105 => Some(WildsLandmark::GearCache(2)),
        107 => Some(WildsLandmark::HiddenFormScroll(super::forms::FORM_DRAGON)),
        108 => Some(WildsLandmark::TokenBag(2)),
        109 => Some(WildsLandmark::InsightTrove(40)),
        110 => Some(WildsLandmark::GearCache(3)),
        111 => Some(WildsLandmark::HiddenFormScroll(super::forms::FORM_HORSE)),
        _ => None,
    }
}

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

    let templates: [(u8, &[u8], u64, u64, u64, u64, u64); 12] = [
        // (id, preds, min_level, gold_mult, essence_mult, enemy_hp, enemy_atk)
        // Wilds entrance aligns with Boss's Lair (min_level 10) so a
        // player who finishes the linear chain can step straight in.
        // Inner Wilds ramp up to lvl 65 — combined with the quadratic
        // `scale_by_area_level` factor, the deep nodes stay tough for
        // end-game players carrying full Legacy/Insight/Token spend.
        // 12-node layout (4 landmark slots: 103/105/107/108-111):
        //   D0: 100             entrance
        //   D1: 101, 102        branches
        //   D2: 103, 104, 105   ★ 103 EssencePool, ★ 105 GearCache T2
        //   D3: 106, 107        ★ 107 HiddenFormScroll(Dragon)
        //   D4: 108, 109        ★ 108 TokenBag, ★ 109 InsightTrove
        //   D5: 110             ★ GearCache T3
        //   D6: 111             ★ HiddenFormScroll(Horse) — endgame
        (100, &[],          10,  8,  5,  180,  35),
        (101, &[100],       15, 10,  6,  220,  45),
        (102, &[100],       15,  6, 10,  220,  42),
        (103, &[101],       20, 12,  6,  280,  55),
        (104, &[102],       20,  8, 12,  280,  52),
        (105, &[101, 102],  25, 14, 14,  340,  62),
        (106, &[103],       35, 16,  8,  420,  75),
        (107, &[104, 105],  45, 10, 16,  420,  72),
        (108, &[106],       50, 18, 10,  520,  86),
        (109, &[107],       55, 12, 20,  580,  90),
        (110, &[108, 109],  60, 22, 18,  680, 102),
        (111, &[110],       65, 16, 22,  780, 115),
    ];
    for &(id, preds, min_level, gm, em, ehp, eatk) in &templates {
        let name = wilds_name(id as u32, &mut rng);
        let blurb = wilds_blurb(name, id as u32);
        // Slight enemy-stat jitter from seed (±10%) so numbers
        // feel unique per player even with fixed topology, but
        // the band is tight enough that an early-level enemy
        // can't randomly spike to a one-shot bracket. Tightened
        // from ±15% after the 2026-05-17 UX pass — Wilds
        // entrance occasionally rolled +15% atk on a player just
        // crossing level 10, killing them in two turns. ±10%
        // keeps variance visible without losing the ability to
        // plan around it.
        let mut jitter = |base: u64| -> u64 {
            let drift = (rng.next() % 21) as i32 - 10; // [-10, +10]
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
            // 0 clears required for the entrance (lvl 10); inner
            // nodes ramp linearly with min_level. `saturating_sub`
            // keeps the formula safe when min_level <= 10.
            clears_required: min_level.saturating_sub(10).saturating_mul(3),
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
