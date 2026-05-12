//! Freenet protocol surface: wire types for the presence aggregator,
//! mailbox, and guilds contracts; the delegate-envelope wrapper;
//! byte_array serde helpers; secret-store ids.
//!
//! Nothing in here knows about the game — these are the bytes-on-the-
//! wire types that talk to freenet-core. Game state, RPCs, and
//! progression live in `crate::game` and `crate::rpc`.
//!
//! Submodules are flat-re-exported (`pub use foo::*;`) so existing
//! call sites that say `shared::SomeType` keep working unchanged.

pub mod bytes;
pub mod guilds;
pub mod mailbox;
pub mod presence;
pub mod types;

pub use bytes::{byte_array_32, byte_array_64};
pub use guilds::*;
pub use mailbox::*;
pub use presence::*;
pub use types::*;
