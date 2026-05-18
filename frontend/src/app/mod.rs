//! Webapp top-level — every component of the browser-side runtime
//! that isn't WebSocket plumbing or game derivation. Each submodule
//! is one cohesive concern; everything is flat-re-exported through
//! the crate root via `pub use app::*;` in `main.rs` so existing
//! `crate::SomeType` imports keep working.

pub mod core;
pub mod i18n;
pub mod i18n_loader;
pub mod i18n_shared;
pub mod keys;
pub mod prefs;
pub mod render;
pub mod tabs;
pub mod theme_loader;
pub mod types;
pub mod util;
pub mod widgets;

pub use core::*;
pub use i18n::*;
// NOTE: i18n_shared deliberately NOT glob-re-exported — its function
// names (`form_name`, `area_name`, etc.) overlap with the same-name
// re-exports out of the `shared` crate and would create ambiguous
// resolution at call sites. Access it explicitly as
// `crate::app::i18n_shared::form_name(...)`.
pub use keys::*;
pub use prefs::*;
pub use render::*;
pub use tabs::*;
pub use types::*;
pub use util::*;
pub use widgets::*;
