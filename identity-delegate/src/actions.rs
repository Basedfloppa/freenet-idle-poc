//! Per-RPC handlers — every variant of `DelegateRequest` that
//! isn't `GetPubkey` lives in one of these submodules.
//!
//! Submodules:
//!   * [`inventory`] — pull-refresh + reset
//!   * [`presence`] — `publish_presence`
//!   * [`area`] — `set_area`
//!   * [`gear`] — equip/unequip/sell/forge/buy/auto-equip
//!   * [`shop`] — consumables + skills
//!   * [`farm`] — wheat
//!   * [`messaging`] — mailbox + guild op signing
//!   * [`battle`] — auto-run, queue, tick, catch-up
//!
//! Each handler follows the same pattern: load → `enter_action` →
//! mutate → `check_achievements`/`check_endings` → save.

pub mod area;
pub mod battle;
pub mod estate;
pub mod farm;
pub mod gear;
pub mod inventory;
pub mod messaging;
pub mod presence;
pub mod shop;

pub use area::*;
pub use battle::*;
pub use estate::*;
pub use farm::*;
pub use gear::*;
pub use inventory::*;
pub use messaging::*;
pub use presence::*;
pub use shop::*;
