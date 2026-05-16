//! Interactive battle plumbing — auto-run toggle, mid-fight action
//! queueing, routine ticks, and the offline catch-up loop.

use freenet_stdlib::prelude::*;

use shared::{CatchupSummary, Inventory};

use crate::progression::check_achievements;
use crate::state::{enter_action, load_inventory_raw, save_inventory};

/// Flip the persistent auto-run switch. When turning ON we anchor
/// `auto_last_tick_ms = now_ms` so the next catch-up starts from
/// here (not from some ancient timestamp). When turning OFF we
/// clear the anchor — closing auto-mode shouldn't accumulate idle
/// time for a future re-enable.
pub fn set_auto_run(
    ctx: &mut DelegateCtx,
    enabled: bool,
    now_ms: u64,
) -> Result<Inventory, String> {
    let mut inv = load_inventory_raw(ctx);
    enter_action(&mut inv, now_ms)?;
    // Catch up against the OLD setting first, so toggling off
    // doesn't silently drop the last few ticks the player earned.
    // Drain both loops since idle actions are mutually exclusive
    // (§5.6) — flipping auto-mission also implies leaving any
    // other idle action.
    catch_up_auto(&mut inv, now_ms);
    super::estate::tick_estate(&mut inv, now_ms);
    inv.auto_run_enabled = enabled;
    inv.auto_last_tick_ms = if enabled { now_ms } else { 0 };
    inv.idle_action = if enabled {
        shared::IDLE_ACTION_AUTO_MISSION
    } else {
        shared::IDLE_ACTION_NONE
    };
    // Pausing the Estate when auto-mission turns on (and vice
    // versa) — the single-active-action rule means only one
    // accrual clock can advance at a time.
    inv.estate.last_tick_ms = 0;
    save_inventory(ctx, &mut inv)?;
    Ok(inv)
}

/// Hard cap on how much offline time we'll simulate in one call —
/// roughly an hour at the current 1 s tick. Avoids long-tail abuse
/// (years of accumulated idle would crash the delegate's CPU budget)
/// and keeps the catch-up loop's wall-time bounded.
const MAX_CATCHUP_TICKS: u64 = 3_600;
const CATCHUP_TICK_MS: u64 = 1_000;
/// Below this many simulated ticks the catch-up is "routine"
/// (regular online pull at the default 10 s cadence) — we don't
/// overwrite the banner with these. A noticeable offline window
/// is needed before we update the surface text.
const CATCHUP_REPORT_THRESHOLD_TICKS: u64 = 30;

/// Simulate every offline auto-tick that should have happened
/// between `inv.auto_last_tick_ms` and `now_ms`. Bounded by
/// `MAX_CATCHUP_TICKS` so a player returning after a week sees a
/// reasonable-but-not-infinite reward window.
///
/// One catch-up tick = one combat turn (`TURN_COOLDOWN_MS`).
/// Operates through the same `start_battle`/`tick_battle` path the
/// live UI uses — single combat code path means online and offline
/// play converge on the same numbers. Mission boundaries advance
/// naturally as enemy HP hits zero across encounters.
pub fn catch_up_auto(inv: &mut Inventory, now_ms: u64) {
    if !inv.auto_run_enabled || inv.auto_last_tick_ms == 0 {
        return;
    }
    if now_ms <= inv.auto_last_tick_ms {
        inv.auto_last_tick_ms = now_ms;
        return;
    }
    let elapsed = now_ms - inv.auto_last_tick_ms;
    let mut ticks = elapsed / CATCHUP_TICK_MS;
    if ticks == 0 {
        return;
    }
    if ticks > MAX_CATCHUP_TICKS {
        ticks = MAX_CATCHUP_TICKS;
    }
    let started_ms = inv.auto_last_tick_ms;
    let gold_before = inv.gold;
    let essence_before = inv.essence;
    let xp_before = inv.experience;
    let boss_before = inv.boss_damage;
    let mission_before = inv.mission_count;
    let mut simulated_at = started_ms;
    for _ in 0..ticks {
        simulated_at = simulated_at.saturating_add(CATCHUP_TICK_MS);
        crate::state::apply_hp_regen(inv, simulated_at);
        if inv.current_hp == 0 {
            continue;
        }
        if inv.current_battle.is_none() {
            // Errors here are config bugs (empty roster); skip-and-
            // continue keeps catch-up monotonic even if a future
            // schema change drops an area roster.
            if crate::combat::start_battle(inv, simulated_at).is_err() {
                continue;
            }
        }
        crate::combat::tick_battle(inv, simulated_at);
    }
    inv.auto_last_tick_ms = simulated_at;
    inv.last_action_ms = simulated_at;
    check_achievements(inv, now_ms);
    if ticks >= CATCHUP_REPORT_THRESHOLD_TICKS {
        let missions_total = inv.mission_count.saturating_sub(mission_before);
        // No clean "missions_lost" count in tick mode — a defeat
        // ends one mission early so the diff between "missions
        // started" and `missions_total` would be a rough proxy.
        // For the v1 summary we just report wins.
        inv.last_catchup = Some(CatchupSummary {
            started_ms,
            ended_ms: simulated_at,
            ticks_simulated: ticks as u32,
            missions_won: missions_total as u32,
            missions_lost: 0,
            gold_gained: inv.gold.saturating_sub(gold_before),
            essence_gained: inv.essence.saturating_sub(essence_before),
            xp_gained: inv.experience.saturating_sub(xp_before),
            boss_damage_gained: inv.boss_damage.saturating_sub(boss_before),
        });
    }
}

/// Update the active battle's queued action. The next turn-resolve
/// will consume it (potion → full heal, fireball → bonus damage).
/// Returns the post-state inventory so the webapp's optimistic
/// rendering can reconcile.
///
/// **Does NOT call `catch_up_auto`.** Queuing an action is a
/// mid-battle interaction; if the battle has actually ended (e.g.
/// the player clicked Use Potion right as the last encounter
/// resolved), this returns `Err` instead of silently starting a
/// fresh battle. Use-of-item must not cascade into "and also a new
/// fight started" — that's `RunMission`'s job.
pub fn queue_battle_action(
    ctx: &mut DelegateCtx,
    action: u8,
    now_ms: u64,
) -> Result<Inventory, String> {
    let mut inv = load_inventory_raw(ctx);
    enter_action(&mut inv, now_ms)?;
    // Tick the existing battle to the present so the queued action
    // lines up with the right turn boundary.
    crate::combat::tick_battle(&mut inv, now_ms);
    if !crate::combat::queue_action(&mut inv, action) {
        return Err("no active battle to queue an action on".into());
    }
    save_inventory(ctx, &mut inv)?;
    Ok(inv)
}

/// Advance the active battle without queuing anything. The
/// frontend's per-1-s poll calls this so HP bars and the recent-
/// turns feed stay live. No-op if there's no battle in progress.
///
/// **Does NOT call `catch_up_auto`.** Routine ticks must not spawn
/// new battles — `RunMission` (and `touch_inventory` on the slow-
/// poll path) cover that.
pub fn tick_only(ctx: &mut DelegateCtx, now_ms: u64) -> Result<Inventory, String> {
    let mut inv = load_inventory_raw(ctx);
    enter_action(&mut inv, now_ms)?;
    crate::combat::tick_battle(&mut inv, now_ms);
    save_inventory(ctx, &mut inv)?;
    Ok(inv)
}
