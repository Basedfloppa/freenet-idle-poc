//! All gameplay state and the formulae that drive it. Inventory,
//! gear catalog, enemy/area roster, forms, skills, achievements,
//! endings, XP curve, plot templates, status pills — every game
//! concept that isn't pure Freenet protocol lives here.
//!
//! The split:
//!   * `crate::freenet` — contract wire types, byte_array helpers,
//!     secret-store ids. No game semantics.
//!   * `crate::game` (this module) — the game's authoritative
//!     model. Inventory is the canonical state; everything else is
//!     pure-function over it.
//!   * `crate::rpc` — the protocol surface between webapp and
//!     delegate; references types from both modules above.
//!
//! Submodules are flat-re-exported (`pub use foo::*;`) so existing
//! call sites that say `shared::SomeType` keep working unchanged.

pub mod achievements;
pub mod activities;
pub mod areas;
pub mod battle;
pub mod combat_log;
pub mod endings;
pub mod enemies;
pub mod estate;
pub mod forms;
pub mod gear;
pub mod insight;
pub mod inventory;
pub mod legacy;
pub mod plot;
pub mod reveal;
pub mod routine;
pub mod shop;
pub mod skills;
pub mod status;
pub mod tokens;
pub mod wilds;
pub mod xp;

pub use achievements::*;
pub use activities::*;
pub use areas::*;
pub use battle::*;
pub use combat_log::*;
pub use endings::*;
pub use enemies::*;
pub use estate::*;
pub use forms::*;
pub use gear::*;
pub use insight::*;
pub use inventory::*;
pub use legacy::*;
pub use plot::*;
pub use reveal::*;
pub use routine::*;
pub use shop::*;
pub use skills::*;
pub use status::*;
pub use tokens::*;
pub use wilds::*;
pub use xp::*;
