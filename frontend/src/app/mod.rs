//! Webapp top-level — every component of the browser-side runtime
//! that isn't WebSocket plumbing or game derivation. Each submodule
//! is one cohesive concern; everything is flat-re-exported through
//! the crate root via `pub use app::*;` in `main.rs` so existing
//! `crate::SomeType` imports keep working.

pub mod core;
pub mod keys;
pub mod prefs;
pub mod render;
pub mod tabs;
pub mod types;
pub mod util;
pub mod widgets;

pub use core::*;
pub use keys::*;
pub use prefs::*;
pub use render::*;
pub use tabs::*;
pub use types::*;
pub use util::*;
pub use widgets::*;
