//! Freenet plumbing: every line of code in this module talks to
//! either the local node (WS shim, delegate RPC, contract
//! Subscribe/Get/Update) or wraps that traffic for the rest of
//! the app. Game-state types come from `shared::game`; UI lives
//! in `crate::game` and `crate::main`. This module is purely
//! "how to move bytes to and from the node".

pub mod actions;
pub mod contract;
pub mod heartbeat;
pub mod reconnect;
