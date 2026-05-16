//! Phased-reveal bitmask — which UI sections are visible to the
//! player. One bit per `RevealKey`. Bits **latch on** the first
//! time their predicate fires; transient state changes (e.g. selling
//! the last potion) don't make the Consumables panel disappear.
//!
//! The predicates live here in shared so the delegate can recompute
//! them on every state mutation. The frontend reads
//! `inventory.revealed_has(...)` to gate `html!` blocks; it never
//! evaluates the predicates itself.
//!
//! ## Adding a new RevealKey
//!
//! 1. Add a variant at the end of `RevealKey` with the next-free
//!    bit position. **Never reorder or delete** — the bitmask is
//!    persisted on disk.
//! 2. Add the matching predicate to `predicate_for(key, inv)`.
//! 3. Add the matching reveal-message to the localiser if you want
//!    a toast.
//! 4. Wrap the section in `render.rs` with
//!    `if inv.revealed_has(RevealKey::Foo) { html! { … } }`.

use super::InventoryV10;
use super::InventoryV11;

/// Each variant is one bit in `inventory.revealed`. The numeric
/// values are **load-bearing** — they index into the bitmask. Add
/// new variants at the end. Never reorder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RevealKey {
    /// Shop tab — buying gear, potions, fireballs.
    Shop = 0,
    /// World Map tab — area-switch UI.
    WorldMap = 1,
    /// Equipment panel — gear slots + Auto-Equip Best.
    Equipment = 2,
    /// Consumables panel — potion / fireball buttons.
    Consumables = 3,
    /// Achievements tab — unlock list.
    Achievements = 4,
    /// World Boss panel — collective HP + era display.
    WorldBoss = 5,
    /// Guilds tab — multiplayer team UI.
    Guilds = 6,
    /// Auto-mission toggle — only useful after a few manual runs.
    AutoMission = 7,
    /// Skills panel — essence-bought permanent buffs.
    Skills = 8,
}

impl RevealKey {
    /// All keys, in declaration order. Used by `recompute_reveals`.
    pub const ALL: &'static [RevealKey] = &[
        RevealKey::Shop,
        RevealKey::WorldMap,
        RevealKey::Equipment,
        RevealKey::Consumables,
        RevealKey::Achievements,
        RevealKey::WorldBoss,
        RevealKey::Guilds,
        RevealKey::AutoMission,
        RevealKey::Skills,
    ];

    #[inline]
    pub fn bit(self) -> u64 {
        1u64 << (self as u8)
    }
}

impl InventoryV11 {
    /// Is the section associated with `key` currently revealed?
    /// Read this from frontend code to gate `html!` blocks.
    #[inline]
    pub fn revealed_has(&self, key: RevealKey) -> bool {
        self.revealed & key.bit() != 0
    }

    /// Latch the given key as revealed. Idempotent; returns true if
    /// this call flipped the bit (caller may want to emit a toast).
    #[inline]
    pub fn reveal_set(&mut self, key: RevealKey) -> bool {
        let already = self.revealed_has(key);
        self.revealed |= key.bit();
        !already
    }
}

/// Per-key reveal threshold. Predicates are intentionally simple
/// (read-only on `Inventory`) so they can be recomputed cheaply on
/// every state mutation. The level argument is passed in separately
/// because computing it here would require a circular import on
/// `xp::level_of`.
pub fn predicate_for(key: RevealKey, inv: &InventoryV11, level: u64) -> bool {
    match key {
        RevealKey::Shop => inv.mission_count >= 1,
        RevealKey::WorldMap => inv.mission_count >= 5,
        RevealKey::Equipment => {
            !inv.unequipped.is_empty() || inv.equipped.iter().any(|s| s.is_some())
        }
        RevealKey::Consumables => inv.potions > 0 || inv.fireballs > 0,
        RevealKey::Achievements => !inv.achievement_unlocks.is_empty(),
        RevealKey::WorldBoss => inv.mission_count >= 10,
        RevealKey::Guilds => level >= 5,
        RevealKey::AutoMission => inv.mission_count >= 25,
        RevealKey::Skills => inv.essence >= 100 || !inv.skills_unlocked.is_empty(),
    }
}

/// Evaluate every predicate against the current state and latch
/// any newly-true keys. Returns the list of keys that flipped on
/// during this call (frontend or delegate can use for toasts /
/// telemetry). Bits that were already set stay set even if their
/// predicate is no longer true — that's the latch semantic.
pub fn recompute_reveals(inv: &mut InventoryV11, level: u64) -> Vec<RevealKey> {
    let mut flipped = Vec::new();
    for &key in RevealKey::ALL {
        if !inv.revealed_has(key) && predicate_for(key, inv, level) {
            inv.reveal_set(key);
            flipped.push(key);
        }
    }
    flipped
}

/// Migration helper: when upgrading V10 → V11, derive the initial
/// `revealed` bitmask from the V10 state. We can't evaluate
/// `predicate_for` directly because V10 doesn't have the `revealed`
/// field, so the predicates are duplicated here against V10 fields.
/// Level is derived from `experience` via `xp::level_from_xp` (the
/// canonical implementation takes raw XP, not `&Inventory`, so the
/// call works on V10 data without a cyclic dependency).
pub fn derive_initial_reveals_v10(v10: &InventoryV10) -> u64 {
    let level = super::xp::level_from_xp(v10.experience);
    let mut bits: u64 = 0;
    if v10.mission_count >= 1 {
        bits |= RevealKey::Shop.bit();
    }
    if v10.mission_count >= 5 {
        bits |= RevealKey::WorldMap.bit();
    }
    if !v10.unequipped.is_empty() || v10.equipped.iter().any(|s| s.is_some()) {
        bits |= RevealKey::Equipment.bit();
    }
    if v10.potions > 0 || v10.fireballs > 0 {
        bits |= RevealKey::Consumables.bit();
    }
    if !v10.achievement_unlocks.is_empty() {
        bits |= RevealKey::Achievements.bit();
    }
    if v10.mission_count >= 10 {
        bits |= RevealKey::WorldBoss.bit();
    }
    if level >= 5 {
        bits |= RevealKey::Guilds.bit();
    }
    if v10.mission_count >= 25 {
        bits |= RevealKey::AutoMission.bit();
    }
    if v10.essence >= 100 || !v10.skills_unlocked.is_empty() {
        bits |= RevealKey::Skills.bit();
    }
    bits
}
