//! Per-zone activity handlers (backlog A1). Mirrors `estate` —
//! the activity is the active idle loop, ticks accrue resources
//! by elapsed seconds, switching activities drains the previous
//! tick first. Auto-mission and Estate are mutually exclusive
//! with activities (single-active-action rule, §5.6).

use freenet_stdlib::prelude::*;

use shared::{
    activities_for_area, activity_def, ActivityResource, Inventory, ACTIVITY_NONE,
    IDLE_ACTION_ACTIVITY, IDLE_ACTION_NONE,
};

use crate::progression::check_achievements;
use crate::state::{enter_action, load_inventory_raw, save_inventory};

const MAX_ACTIVITY_CATCHUP_SEC: u64 = 3_600;

/// Switch to the given activity. `0` clears the slot and drops
/// idle_action back to NONE (the player explicitly stops idling).
pub fn set_activity(
    ctx: &mut DelegateCtx,
    activity_id: u8,
    now_ms: u64,
) -> Result<Inventory, String> {
    let mut inv = load_inventory_raw(ctx);
    enter_action(&mut inv, now_ms)?;
    // Drain any in-flight loop's accrual before flipping. Estate
    // is similarly drained from `set_idle_action`; auto-mission
    // catch-up runs first by convention so its anchor is fresh.
    crate::actions::battle::catch_up_auto(&mut inv, now_ms);
    crate::actions::estate::tick_estate(&mut inv, now_ms);
    tick_activity(&mut inv, now_ms);
    if activity_id == ACTIVITY_NONE {
        inv.base.base.idle_action = IDLE_ACTION_NONE;
        inv.active_activity = ACTIVITY_NONE;
        inv.activity_last_tick_ms = 0;
        save_inventory(ctx, &mut inv)?;
        return Ok(inv);
    }
    let def = activity_def(activity_id)
        .ok_or_else(|| format!("unknown activity id {activity_id}"))?;
    let current_area = inv.current_area;
    if def.area_id != current_area {
        return Err(format!(
            "activity '{}' is in another area — switch via World Map first",
            def.name
        ));
    }
    let lvl = shared::level_of(&inv);
    if lvl < def.min_level {
        return Err(format!(
            "level {lvl} too low for '{}' (need {})",
            def.name, def.min_level
        ));
    }
    // Activities lock out the other two idle loops. We pause
    // their accrual clocks (workers don't disappear, they just
    // stop earning) so flipping back to Estate later resumes
    // cleanly.
    inv.auto_run_enabled = false;
    inv.auto_last_tick_ms = 0;
    inv.base.base.estate.last_tick_ms = 0;
    inv.base.base.idle_action = IDLE_ACTION_ACTIVITY;
    inv.active_activity = activity_id;
    inv.activity_last_tick_ms = now_ms;
    save_inventory(ctx, &mut inv)?;
    Ok(inv)
}

/// Pay out yield for the elapsed window since
/// `activity_last_tick_ms`. No-op when the player isn't in the
/// activity loop. Capped at `MAX_ACTIVITY_CATCHUP_SEC` of real
/// time.
pub fn tick_activity(inv: &mut Inventory, now_ms: u64) {
    if inv.base.base.idle_action != IDLE_ACTION_ACTIVITY || inv.activity_last_tick_ms == 0 {
        return;
    }
    let def = match activity_def(inv.active_activity) {
        Some(d) => *d,
        None => return,
    };
    if now_ms <= inv.activity_last_tick_ms {
        inv.activity_last_tick_ms = now_ms;
        return;
    }
    let elapsed_ms = now_ms - inv.activity_last_tick_ms;
    let mut elapsed_sec = elapsed_ms / 1_000;
    if elapsed_sec == 0 {
        return;
    }
    if elapsed_sec > MAX_ACTIVITY_CATCHUP_SEC {
        elapsed_sec = MAX_ACTIVITY_CATCHUP_SEC;
    }
    let yield_total = def.yield_per_sec.saturating_mul(elapsed_sec);
    match def.produces {
        ActivityResource::Wheat => {
            inv.wheat = inv.wheat.saturating_add(yield_total);
        }
        ActivityResource::Gold => {
            inv.gold = inv.gold.saturating_add(yield_total);
        }
        ActivityResource::Essence => {
            inv.essence = inv.essence.saturating_add(yield_total);
        }
        ActivityResource::Insight => {
            inv.insight.balance = inv.insight.balance.saturating_add(yield_total);
        }
    }
    inv.activity_last_tick_ms = inv
        .activity_last_tick_ms
        .saturating_add(elapsed_sec.saturating_mul(1_000));
    check_achievements(inv, now_ms);
}

/// Used by `set_area` so changing zone implicitly stops the
/// current activity (it's location-bound). Returns the list of
/// activities exposed in the new zone — frontend uses this to
/// render the per-zone panel.
pub fn activities_visible_in(area_id: u8) -> Vec<u8> {
    activities_for_area(area_id).map(|a| a.id).collect()
}
