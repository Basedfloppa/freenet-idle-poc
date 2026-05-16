//! Area selection — gates the picker by player level AND by
//! clear-count in the predecessor area. The clear-count gate is the
//! A3 backlog item: each area's `clears_required` is how many
//! encounters in the predecessor zone the player must have won
//! before this area unlocks. The starter (`Village Fields`) has
//! `clears_required = 0` so a fresh player can enter immediately.

use freenet_stdlib::prelude::*;

use shared::{area_of, area_predecessors, level_of, Inventory, AREAS};

use crate::state::{enter_action, load_inventory_raw, save_inventory};

pub fn set_area(
    ctx: &mut DelegateCtx,
    area_id: u8,
    now_ms: u64,
) -> Result<Inventory, String> {
    let mut inv = load_inventory_raw(ctx);
    enter_action(&mut inv, now_ms)?;
    let area = AREAS
        .iter()
        .find(|a| a.id == area_id)
        .ok_or_else(|| format!("unknown area_id {area_id}"))?;
    let lvl = level_of(&inv);
    if lvl < area.min_level {
        return Err(format!(
            "level {lvl} cannot enter '{}': requires level {}",
            area.name, area.min_level
        ));
    }
    // Graph-walk gate (C3): unlock if *any* predecessor has the
    // required clear count. The error message names whichever
    // predecessor the player is closest in — most actionable
    // suggestion when they need to know what to grind.
    let preds = area_predecessors(area_id);
    if !preds.is_empty() {
        let satisfied = preds
            .iter()
            .any(|p| inv.area_clears_of(*p) >= area.clears_required);
        if !satisfied {
            let (best_id, best_have) = preds
                .iter()
                .map(|p| (*p, inv.area_clears_of(*p)))
                .max_by_key(|(_, h)| *h)
                .unwrap_or((preds[0], 0));
            let prev_name = area_of(best_id).name;
            return Err(format!(
                "cannot enter '{}': need {} clears in '{}' (have {})",
                area.name, area.clears_required, prev_name, best_have
            ));
        }
    }
    let _ = AREAS;
    // Drop any active per-zone activity — A1 activities are
    // location-bound (e.g. "Mine ore" in Mountain Pass), so a
    // zone change implicitly stops them. Tick first so the
    // accrual for the closing window is paid out.
    crate::actions::activity::tick_activity(&mut inv, now_ms);
    if inv.active_activity != shared::ACTIVITY_NONE {
        inv.active_activity = shared::ACTIVITY_NONE;
        inv.activity_last_tick_ms = 0;
        if inv.base.base.idle_action == shared::IDLE_ACTION_ACTIVITY {
            inv.base.base.idle_action = shared::IDLE_ACTION_NONE;
        }
    }
    inv.current_area = area_id;
    inv.last_combat = None;
    save_inventory(ctx, &mut inv)?;
    Ok(inv)
}
