//! Routine auto-hire (backlog B1, MVP scope). The only target
//! category today is Estate tier headcount: when the player runs
//! Estate as their idle action and gold permits, the unified
//! delegate tick advances tier counts toward the configured
//! `routine.estate_targets[tier_id]`. Future targets (skills,
//! consumables, gear) extend the routine state map.

use freenet_stdlib::prelude::*;

use shared::{estate_next_price, estate_tier, Inventory};

use crate::state::{enter_action, load_inventory_raw, save_inventory};

pub fn set_routine_estate_target(
    ctx: &mut DelegateCtx,
    tier_id: u8,
    target: u64,
    now_ms: u64,
) -> Result<Inventory, String> {
    let mut inv = load_inventory_raw(ctx);
    enter_action(&mut inv, now_ms)?;
    // Validate the tier exists before stamping the target — keeps
    // future tier-table edits from leaving orphan rows.
    if estate_tier(tier_id).is_none() {
        return Err(format!("unknown estate tier {tier_id}"));
    }
    inv.routine.set_target(tier_id, target);
    save_inventory(ctx, &mut inv)?;
    Ok(inv)
}

/// Pump the routine — currently only auto-hires Estate workers.
/// Called after `tick_estate` so the gold accrual from the
/// elapsed window is already in the pocket when we try to spend
/// it. Bounded by an explicit per-call cap so a fat catchup
/// window can't burn through the entire treasury in one tick.
///
/// **No idle-action gate.** Setting a routine target is an
/// explicit "spend my gold on this whenever it's available"
/// signal; restricting auto-hire to Estate-idle would mean a
/// player accumulating gold from auto-mission can't ever clear
/// a Sage target without manually switching idle modes. Cheaper
/// for the player to just configure targets and let income from
/// any source drain into them.
pub fn pump_routine(inv: &mut Inventory) {
    if inv.routine.estate_targets.is_empty() {
        return;
    }
    // Cheap cap: at most N hires per pump so a 1-hour catchup
    // doesn't make the click feel like cheating. Tunable.
    const MAX_AUTO_HIRES_PER_PUMP: u64 = 50;
    let targets: Vec<(u8, u64)> = inv
        .routine
        .estate_targets
        .iter()
        .map(|(k, v)| (*k, *v))
        .collect();
    let mut budget = MAX_AUTO_HIRES_PER_PUMP;
    for (tier_id, target) in targets {
        if budget == 0 {
            break;
        }
        let tier = match estate_tier(tier_id) {
            Some(t) => t,
            None => continue,
        };
        while budget > 0 {
            let owned = inv.base.base.estate.workers_of(tier_id);
            if owned >= target {
                break;
            }
            let price = estate_next_price(tier, owned);
            if inv.gold < price {
                break;
            }
            inv.gold -= price;
            inv.base.base.estate.hire(tier_id);
            budget -= 1;
        }
    }
}
