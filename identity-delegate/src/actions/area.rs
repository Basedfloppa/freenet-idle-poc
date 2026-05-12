//! Area selection — gates the picker by player level.

use freenet_stdlib::prelude::*;

use shared::{level_of, Inventory, AREAS};

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
    inv.current_area = area_id;
    inv.last_combat = None;
    save_inventory(ctx, &inv)?;
    Ok(inv)
}
