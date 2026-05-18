//! Estate handlers — hire workers, set the active idle action,
//! and accrue Estate yield during catch-up windows.
//!
//! The Estate sits at the same conceptual level as auto-mission:
//! exactly one idle loop is active at a time (`inv.idle_action`).
//! Each tick of "real time elapsed since `estate.last_tick_ms`"
//! pays out worker yields scaled by the active Form's affinity.

use freenet_stdlib::prelude::*;

use shared::{
    estate_next_price, estate_tier, form_affinity_bp_with_insight, CatchupSummary,
    EstateResource, Inventory, IDLE_ACTION_AUTO_MISSION, IDLE_ACTION_ESTATE,
    IDLE_ACTION_NONE,
};

use crate::progression::check_achievements;
use crate::state::{enter_action, load_inventory_raw, save_inventory};

const MAX_ESTATE_CATCHUP_SEC: u64 = 3_600;
/// Minimum simulated Estate window before the welcome-back modal
/// fires on return. Matches the auto-mission catchup threshold so
/// both paths feel symmetric — short tab-flips don't trigger the
/// modal, anything ≥1 minute does.
const ESTATE_CATCHUP_REPORT_SEC: u64 = 60;

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
    // Insight EstateFrugality node applies a per-worker discount.
    let discount_bp = inv.insight.frugality_mult_bp();
    let price = shared::estate_next_price_with_discount(tier, owned, discount_bp);
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
    // Drain all idle loops against the OLD setting first so we
    // don't lose accumulated time on a mode switch.
    crate::actions::battle::catch_up_auto(&mut inv, now_ms);
    tick_estate(&mut inv, now_ms);
    crate::actions::activity::tick_activity(&mut inv, now_ms);
    // Switching the idle action implicitly clears the activity
    // slot — picking Estate or Auto-Mission shouldn't leave a
    // dormant activity selection lurking.
    inv.active_activity = shared::ACTIVITY_NONE;
    inv.activity_last_tick_ms = 0;
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
    // §⚠️#2 (2026-05-18): Estate yields run concurrent with combat /
    // activity for every player. The WorkforceBoss token perk was
    // previously the only path to parallel Estate yield — gating
    // most players to a mutex with combat made the perk feel
    // mandatory and the Estate panel ignored. Now both states tick
    // Estate unconditionally; WorkforceBoss is retained as a
    // historical token slot (read by `estate_parallel()` but no
    // longer load-bearing here). Bookkeeping still requires
    // `last_tick_ms` to be initialised so a brand-new player who's
    // never visited the Estate doesn't accrue retroactive yield.
    if inv.estate.last_tick_ms == 0 {
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
    let legacy_mult_bp = inv
        .legacy
        .node_multiplier_bp(shared::LegacyNode::EstateYield);
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
        let insight_aff_level = inv
            .insight
            .node_level(shared::InsightNode::FormAffinity);
        let aff = form_affinity_bp_with_insight(form, tier_id, insight_aff_level);
        // yield_per_sec * count * elapsed * affinity_bp / 10_000
        let raw = tier
            .yield_per_sec
            .saturating_mul(count)
            .saturating_mul(elapsed_sec);
        let with_aff = raw.saturating_mul(aff) / 10_000;
        // Layer the Legacy multiplier on top (C1). Compounds with
        // form affinity multiplicatively — small stacks at first,
        // strong by mid-game when a few nodes are bought.
        let scaled = with_aff.saturating_mul(legacy_mult_bp) / 10_000;
        match tier.produces {
            EstateResource::Wheat => wheat_gain = wheat_gain.saturating_add(scaled),
            EstateResource::Gold => gold_gain = gold_gain.saturating_add(scaled),
            EstateResource::Essence => essence_gain = essence_gain.saturating_add(scaled),
        }
    }
    inv.base.base.gold = inv.base.base.gold.saturating_add(gold_gain);
    inv.base.base.wheat = inv.base.base.wheat.saturating_add(wheat_gain);
    inv.base.base.essence = inv.base.base.essence.saturating_add(essence_gain);
    // Populate the welcome-back banner on long Estate windows.
    // The schema's `CatchupSummary` was sized for the auto-mission
    // loop (missions_won / xp_gained / boss_damage_gained), but
    // gold/essence map cleanly onto Estate yield so the frontend
    // surfaces the right "while you were away" copy. `wheat` rides
    // in via `xp_gained` for now — schema extension is a follow-up
    // when more fields are needed.
    let started_ms = inv.estate.last_tick_ms;
    if elapsed_sec >= ESTATE_CATCHUP_REPORT_SEC {
        inv.base.base.last_catchup = Some(CatchupSummary {
            started_ms,
            ended_ms: started_ms.saturating_add(elapsed_sec.saturating_mul(1_000)),
            ticks_simulated: elapsed_sec as u32,
            missions_won: 0,
            missions_lost: 0,
            gold_gained: gold_gain,
            essence_gained: essence_gain,
            xp_gained: wheat_gain,
            boss_damage_gained: 0,
        });
    }
    inv.estate.last_tick_ms = inv
        .estate
        .last_tick_ms
        .saturating_add(elapsed_sec.saturating_mul(1_000));
}
