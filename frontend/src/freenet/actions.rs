//! Delegate-action helpers: one function per `DelegateRequest`
//! variant the UI wants to invoke. Each captures the WS handle,
//! fires the RPC, and merges the response back into local state.
//! All time-bearing RPCs take `now_ms` from the browser clock
//! here; the delegate enforces monotonicity on its side.
//!
//! Submodules group the wrappers by topic — they all flat re-export
//! through here, so call sites keep using `freenet::actions::foo`.

pub mod area;
pub mod battle;
pub mod dispatch;
pub mod farm;
pub mod gear;
pub mod messaging;
pub mod pull_presence;
pub mod seed;
pub mod settings;
pub mod shop;

pub use area::*;
pub use battle::*;
pub use dispatch::*;
pub use farm::*;
pub use gear::*;
pub use messaging::*;
pub use pull_presence::*;
pub use seed::*;
pub use settings::*;
pub use shop::*;
