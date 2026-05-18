//! Wheat sell-back. The legacy "Work the Farm" click was removed
//! when the Farm tab became the home tab — wheat is now produced
//! passively by Estate Farmhands, and the merchant in the Shop tab
//! is the only conversion point. Caps the "Quiet Farmer" ending
//! milestone via `wheat_sold_total`.

use freenet_stdlib::prelude::*;

use shared::{Inventory, ESSENCE_TO_GOLD_RATE, WHEAT_PER_GOLD};

use crate::progression::{check_achievements, check_endings};
use crate::state::{enter_action, load_inventory_raw, save_inventory};

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

/// Essence → gold exchange at the merchant. `amount = 0` converts
/// the whole essence pile. The rate (`ESSENCE_TO_GOLD_RATE`) is
/// intentionally inferior to grinding gold from Boss areas — the
/// merchant is a way out of the post-Ascend essence pile, not a
/// preferred income loop.
pub fn convert_essence_to_gold(
    ctx: &mut DelegateCtx,
    amount: u64,
    now_ms: u64,
) -> Result<Inventory, String> {
    let mut inv = load_inventory_raw(ctx);
    enter_action(&mut inv, now_ms)?;
    let to_spend = if amount == 0 { inv.essence } else { amount };
    if to_spend == 0 {
        return Err("no essence to convert".into());
    }
    if inv.essence < to_spend {
        return Err(format!(
            "not enough essence: need {to_spend}, have {}",
            inv.essence
        ));
    }
    let gold_gain = to_spend.saturating_mul(ESSENCE_TO_GOLD_RATE);
    inv.essence -= to_spend;
    inv.gold = inv.gold.saturating_add(gold_gain);
    save_inventory(ctx, &mut inv)?;
    Ok(inv)
}
