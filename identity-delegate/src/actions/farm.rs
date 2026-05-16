//! Wheat farm — slow gold drip that doesn't require combat. Caps
//! the "Quiet Farmer" ending milestone.

use freenet_stdlib::prelude::*;

use shared::{Inventory, WHEAT_PER_GOLD};

use crate::progression::{check_achievements, check_endings};
use crate::state::{enter_action, load_inventory_raw, save_inventory};

pub fn work_farm(ctx: &mut DelegateCtx, now_ms: u64) -> Result<Inventory, String> {
    let mut inv = load_inventory_raw(ctx);
    enter_action(&mut inv, now_ms)?;
    inv.wheat = inv.wheat.saturating_add(1);
    check_achievements(&mut inv, now_ms);
    save_inventory(ctx, &mut inv)?;
    Ok(inv)
}

pub fn sell_wheat(
    ctx: &mut DelegateCtx,
    amount: u64,
    now_ms: u64,
) -> Result<Inventory, String> {
    let mut inv = load_inventory_raw(ctx);
    enter_action(&mut inv, now_ms)?;
    let to_sell = if amount == 0 { inv.wheat } else { amount };
    if to_sell == 0 {
        return Err("no wheat to sell".into());
    }
    if inv.wheat < to_sell {
        return Err(format!(
            "not enough wheat: need {to_sell}, have {}",
            inv.wheat
        ));
    }
    let gold_gain = to_sell / WHEAT_PER_GOLD;
    inv.wheat -= to_sell;
    inv.gold = inv.gold.saturating_add(gold_gain);
    inv.wheat_sold_total = inv.wheat_sold_total.saturating_add(to_sell);
    check_achievements(&mut inv, now_ms);
    check_endings(&mut inv, now_ms, None);
    save_inventory(ctx, &mut inv)?;
    Ok(inv)
}
