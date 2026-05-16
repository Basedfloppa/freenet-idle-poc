//! Area selection — gates the picker by player level AND by
//! clear-count in the predecessor area. The clear-count gate is the
//! A3 backlog item: each area's `clears_required` is how many
//! encounters in the predecessor zone the player must have won
//! before this area unlocks. The starter (`Village Fields`) has
//! `clears_required = 0` so a fresh player can enter immediately.

use freenet_stdlib::prelude::*;

use shared::{area_predecessor, level_of, Inventory, AREAS};

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
    if let Some(prev_id) = area_predecessor(area_id) {
        let have = inv.area_clears_of(prev_id);
        if have < area.clears_required {
            let prev_name = AREAS
                .iter()
                .find(|a| a.id == prev_id)
                .map(|a| a.name)
                .unwrap_or("?");
            return Err(format!(
                "cannot enter '{}': need {} clears in '{}' (have {})",
                area.name, area.clears_required, prev_name, have
            ));
        }
    }
    inv.current_area = area_id;
    inv.last_combat = None;
    save_inventory(ctx, &mut inv)?;
    Ok(inv)
}
