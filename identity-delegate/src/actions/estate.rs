//! Estate handlers — hire workers, set the active idle action,
//! and accrue Estate yield during catch-up windows.
//!
//! The Estate sits at the same conceptual level as auto-mission:
//! exactly one idle loop is active at a time (`inv.idle_action`).
//! Each tick of "real time elapsed since `estate.last_tick_ms`"
//! pays out worker yields scaled by the active Form's affinity.

use freenet_stdlib::prelude::*;

use shared::{
    estate_next_price, estate_tier, form_affinity_bp, EstateResource, Inventory,
    IDLE_ACTION_AUTO_MISSION, IDLE_ACTION_ESTATE, IDLE_ACTION_NONE,
};

use crate::progression::check_achievements;
use crate::state::{enter_action, load_inventory_raw, save_inventory};

const MAX_ESTATE_CATCHUP_SEC: u64 = 3_600;

/// Spend gold to hire one more worker of the given tier. Returns
/// the post-mutation inventory so the webapp can re-render.
pub fn buy_estate_worker(
    ctx: &mut DelegateCtx,
    tier_id: u8,
    now_ms: u64,
) -> Result<Inventory, String> {
    let mut inv = load_inventory_raw(ctx);
    enter_action(&mut inv, now_ms)?;
    // Pay out any pending Estate yield before we change the worker
    // count, so the new worker's first tick doesn't backdate.
    tick_estate(&mut inv, now_ms);
    let tier = estate_tier(tier_id).ok_or_else(|| format!("unknown estate tier {tier_id}"))?;
    let owned = inv.estate.workers_of(tier_id);
    let price = estate_next_price(tier, owned);
    if inv.base.base.gold < price {
        return Err(format!("not enough gold: need {price}, have {}", inv.base.base.gold));
    }
    inv.base.base.gold -= price;
    inv.estate.hire(tier_id);
    check_achievements(&mut inv, now_ms);
    save_inventory(ctx, &mut inv)?;
    Ok(inv)
}

/// Switch the active idle action. Mirrors `auto_run_enabled` for
/// back-compat with the existing auto-mission button.
pub fn set_idle_action(
    ctx: &mut DelegateCtx,
    action: u8,
    now_ms: u64,
) -> Result<Inventory, String> {
    let mut inv = load_inventory_raw(ctx);
    enter_action(&mut inv, now_ms)?;
    // Drain both idle loops against the OLD setting first so we
    // don't lose accumulated time on a mode switch.
    crate::actions::battle::catch_up_auto(&mut inv, now_ms);
    tick_estate(&mut inv, now_ms);
    match action {
        IDLE_ACTION_NONE => {
            inv.base.base.auto_run_enabled = false;
            inv.base.base.auto_last_tick_ms = 0;
            inv.estate.last_tick_ms = 0;
        }
        IDLE_ACTION_AUTO_MISSION => {
            inv.base.base.auto_run_enabled = true;
            inv.base.base.auto_last_tick_ms = now_ms;
            inv.estate.last_tick_ms = 0;
        }
        IDLE_ACTION_ESTATE => {
            inv.base.base.auto_run_enabled = false;
            inv.base.base.auto_last_tick_ms = 0;
            inv.estate.last_tick_ms = now_ms;
        }
        _ => return Err(format!("unknown idle action {action}")),
    }
    inv.idle_action = action;
    save_inventory(ctx, &mut inv)?;
    Ok(inv)
}

/// Advance the Estate's accrual clock to `now_ms` and credit per-
/// worker yields. No-op when the player isn't running the Estate.
/// Capped at `MAX_ESTATE_CATCHUP_SEC` of real time so the offline
/// window can't crash the delegate's CPU budget.
pub fn tick_estate(inv: &mut Inventory, now_ms: u64) {
    if inv.idle_action != IDLE_ACTION_ESTATE || inv.estate.last_tick_ms == 0 {
        return;
    }
    if now_ms <= inv.estate.last_tick_ms {
        inv.estate.last_tick_ms = now_ms;
        return;
    }
    let elapsed_ms = now_ms - inv.estate.last_tick_ms;
    let mut elapsed_sec = elapsed_ms / 1_000;
    if elapsed_sec == 0 {
        return;
    }
    if elapsed_sec > MAX_ESTATE_CATCHUP_SEC {
        elapsed_sec = MAX_ESTATE_CATCHUP_SEC;
    }
    let form = inv.base.base.current_form;
    let workers_snapshot: Vec<(u8, u64)> = inv
        .estate
        .workers
        .iter()
        .map(|(k, v)| (*k, *v))
        .collect();
    let mut gold_gain: u64 = 0;
    let mut wheat_gain: u64 = 0;
    let mut essence_gain: u64 = 0;
    for (tier_id, count) in workers_snapshot {
        let tier = match estate_tier(tier_id) {
            Some(t) => t,
            None => continue,
        };
        let aff = form_affinity_bp(form, tier_id);
        // yield_per_sec * count * elapsed * affinity_bp / 10_000
        let raw = tier
            .yield_per_sec
            .saturating_mul(count)
            .saturating_mul(elapsed_sec);
        let scaled = raw.saturating_mul(aff) / 10_000;
        match tier.produces {
            EstateResource::Wheat => wheat_gain = wheat_gain.saturating_add(scaled),
            EstateResource::Gold => gold_gain = gold_gain.saturating_add(scaled),
            EstateResource::Essence => essence_gain = essence_gain.saturating_add(scaled),
        }
    }
    inv.base.base.gold = inv.base.base.gold.saturating_add(gold_gain);
    inv.base.base.wheat = inv.base.base.wheat.saturating_add(wheat_gain);
    inv.base.base.essence = inv.base.base.essence.saturating_add(essence_gain);
    inv.estate.last_tick_ms = inv
        .estate
        .last_tick_ms
        .saturating_add(elapsed_sec.saturating_mul(1_000));
}
