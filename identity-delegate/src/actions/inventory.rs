//! Inventory lifecycle — pull-refresh and full wipe.

use freenet_stdlib::prelude::*;

use shared::Inventory;

use crate::progression::check_achievements;
use crate::state::{enter_action, load_inventory_raw, save_inventory};

use super::battle::catch_up_auto;

/// Read-and-touch: apply HP regen, simulate any offline auto-mission
/// ticks that elapsed since the last call, run achievement evaluation,
/// persist, return. Used by the frontend's pull heartbeat (and by
/// `connect_inner` on first load) to keep the client view in sync.
pub fn touch_inventory(ctx: &mut DelegateCtx, now_ms: u64) -> Result<Inventory, String> {
    let mut inv = load_inventory_raw(ctx);
    enter_action(&mut inv, now_ms)?;
    catch_up_auto(&mut inv, now_ms);
    check_achievements(&mut inv, now_ms);
    save_inventory(ctx, &mut inv)?;
    Ok(inv)
}

/// Wipe the inventory back to defaults. Identity (seed) is left
/// intact — the player keeps their pubkey, just starts over at
/// level 1 with empty pockets. Useful for re-running the new-player
/// experience without spinning up a fresh node.
pub fn reset_inventory(ctx: &mut DelegateCtx, _now_ms: u64) -> Result<Inventory, String> {
    let mut fresh = Inventory::default();
    save_inventory(ctx, &mut fresh)?;
    Ok(fresh)
}
