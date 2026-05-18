//! Shared wire-types + game state for the Freenet Idle PoC.
//!
//! Split by what depends on what:
//!
//!   * [`freenet`] — pure Freenet protocol surface. Presence
//!     contract wire types, the delegate envelope (which carries
//!     `request_id` inside the payload because freenet-core wipes
//!     `DelegateContext` on the response leg), secret-store ids,
//!     byte_array serde helpers. No game semantics — these are the
//!     bytes-on-the-wire that talk to freenet-core.
//!
//!   * [`game`] — the game's authoritative model. Inventory, gear
//!     catalog, areas, enemies, forms, skills, achievements,
//!     endings, plot templates, XP curve, status pills. Every
//!     game concept that isn't pure Freenet protocol.
//!
//!   * [`rpc`] — `DelegateRequest` / `DelegateResponse`. Their
//!     variants are game actions but their wire shape is the
//!     freenet RPC protocol — straddles the line, so kept at the
//!     top level rather than nested under either side.
//!
//! Existing import sites pull symbols from the crate root
//! (`shared::SomeType`); the `pub use` re-exports below keep that
//! working unchanged.
//!
//! ## INVENTORY_SECRET_ID ladder
//!
//! Each breaking change to `Inventory`'s bincode layout bumps the
//! secret id so old saves sit unused on disk instead of trying to
//! deserialize into the new shape.
//!
//!   * v1 — initial gold/essence/mission_count/boss_damage.
//!   * v2 — added `current_area`.
//!   * v3 — added `unequipped`/`equipped`/`potions`/`fireballs`.
//!   * v4 — added HP combat + regen + achievements.
//!   * v5 — added forms + combat history.
//!   * v6 — added wheat / plot_seed / shop counter.
//!   * v7 — replaced derived stats with XP + level-static formulae.
//!   * v8 — added endings + wheat_sold_total.
//!   * v9 — added auto_run_enabled + auto_last_tick_ms + last_catchup
//!         for delegate-side offline auto-mission progression.

pub mod envelope;
pub mod fmt;
pub mod game;
pub mod rpc;

// Re-export the wire-only crate at the root so existing
// `shared::PubKey` / `shared::PresencePayload` / etc. callsites
// keep working unchanged after the §6.6 split (see
// `docs/planned-work-2026-05-17.md`). The contracts pull these
// types directly from `idle-shared-wire`; this crate gets them
// transitively via the same re-export.
pub use idle_shared_wire::*;

// Backwards-compatible flat re-exports so existing
// `shared::SomeType` import sites keep working.
pub use envelope::*;
pub use fmt::*;
pub use game::*;
pub use rpc::*;
