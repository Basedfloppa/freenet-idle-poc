//! Combat resolver — interactive tick-based fight + multi-encounter
//! mission chain. State persists in `Inventory.current_battle` so
//! closing the tab pauses (not aborts) an in-progress fight, and
//! the offline-catchup loop just walks the same ticks the live UI
//! would have triggered.
//!
//! Submodules:
//!   * [`tick`] — the interactive turn loop (`start_battle`,
//!     `queue_action`, `tick_battle`). Live-play path.
//!   * [`burst`] — legacy `run_mission_chain` resolver. Kept as a
//!     deterministic one-shot for unit tests.
//!
//! Form transformation, achievement+ending unlocks, and combat-
//! history capping live here too — they're closely entwined with
//! encounter outcomes, so co-locating keeps the data flow obvious.

pub mod burst;
pub mod tick;

pub use tick::{queue_action, start_battle, tick_battle};

use freenet_stdlib::prelude::*;

use shared::{
    form_slot_mask, BattleState, BattleTurn, EncounterLog, Inventory, BATTLE_TURN_HISTORY_CAP,
    COMBAT_HISTORY_CAP, SLOT_COUNT,
};

use crate::state::{enter_action, load_inventory_raw, save_inventory};

/// Drop any equipped piece whose slot isn't allowed for the
/// current form. Bumped pieces go back into `unequipped` so nothing
/// is destroyed. Idempotent — safe to call on every action.
pub fn enforce_form_slot_mask(inv: &mut Inventory) {
    let mask = form_slot_mask(inv.current_form);
    for slot_idx in 0..SLOT_COUNT {
        if !mask[slot_idx] {
            if let Some(cid) = inv.equipped[slot_idx].take() {
                inv.unequipped.push(cid);
            }
        }
    }
}

pub(crate) fn push_combat_history(inv: &mut Inventory, log: EncounterLog) {
    if inv.combat_history.len() >= COMBAT_HISTORY_CAP {
        inv.combat_history.remove(0);
    }
    inv.combat_history.push(log);
}

pub(crate) fn push_turn(battle: &mut BattleState, turn: BattleTurn) {
    if battle.recent_turns.len() >= BATTLE_TURN_HISTORY_CAP {
        battle.recent_turns.remove(0);
    }
    battle.recent_turns.push(turn);
}

/// RPC handler for `RunMission`. Now interactive: starts a battle
/// if there isn't one, then ticks. The returned inventory's
/// `current_battle` tells the webapp whether to keep polling.
pub fn run_mission(
    ctx: &mut DelegateCtx,
    now_ms: u64,
) -> Result<Inventory, String> {
    let mut inv = load_inventory_raw(ctx);
    enter_action(&mut inv, now_ms)?;
    crate::actions::catch_up_auto(&mut inv, now_ms);
    if inv.current_battle.is_none() {
        let _ = start_battle(&mut inv, now_ms)?;
    }
    tick_battle(&mut inv, now_ms);
    // Player engaged — clear the "while you were away" banner.
    inv.last_catchup = None;
    save_inventory(ctx, &mut inv)?;
    Ok(inv)
}
