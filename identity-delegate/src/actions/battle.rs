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
    // Drain every idle loop since idle actions are mutually
    // exclusive (§5.6) — flipping auto-mission implies leaving
    // Estate / Activity too.
    catch_up_auto(&mut inv, now_ms);
    super::estate::tick_estate(&mut inv, now_ms);
    super::activity::tick_activity(&mut inv, now_ms);
    // Flipping auto-mission also clears any selected activity
    // (kept symmetric with `set_idle_action`).
    inv.active_activity = shared::ACTIVITY_NONE;
    inv.activity_last_tick_ms = 0;
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

/// Default hard cap on how much offline time we'll simulate in
/// one call — roughly an hour at the current 1 s tick. The
/// player can lift this up to `MAX_CATCHUP_CAP_HOURS_BASE` (24h)
/// via `routine.offline_cap_hours` (§8 B6), or up to
/// `MAX_CATCHUP_CAP_HOURS_LHF` (168h / 7 days) when the
/// LongHaulForeman token perk is owned. The hard ceiling exists so
/// a catchup window can't burn the delegate's CPU budget — see
/// `catch_up_auto` for the chunked-execution and analytical
/// fast-path that keep wall-time bounded.
const DEFAULT_CATCHUP_HOURS: u64 = 1;
const MAX_CATCHUP_CAP_HOURS_BASE: u64 = 24;
const MAX_CATCHUP_CAP_HOURS_LHF: u64 = 168;
/// One delegate call processes at most this many simulated hours.
/// Frontend re-fires the touch RPC (or any inventory-mutating
/// action) until `auto_last_tick_ms >= now_ms`, drawing a progress
/// bar in between. Keeps individual `process()` invocations inside
/// the host's per-call time budget even for week-long catchups.
const CATCHUP_CHUNK_HOURS: u64 = 24;
const CATCHUP_TICK_MS: u64 = 1_000;
/// Above this threshold the catchup switches from tick-by-tick
/// simulation to an analytical per-hour-average computation
/// (`analytical_catchup_chunk`). 4h tick-by-tick keeps the
/// "interesting moments" feel of a short return; longer windows
/// are too expensive to simulate per-tick and the player won't
/// notice individual ticks anyway.
const ANALYTICAL_THRESHOLD_HOURS: u64 = 4;
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

    // Effective catchup cap. `0` is the legacy-default sentinel
    // (1h); explicit values are clamped server-side. The ceiling
    // depends on the LongHaulForeman token perk: 24h without,
    // 168h (7 days) with.
    let ceiling_hours = if inv.tokens.long_haul() {
        MAX_CATCHUP_CAP_HOURS_LHF
    } else {
        MAX_CATCHUP_CAP_HOURS_BASE
    };
    let cap_hours = if inv.routine.offline_cap_hours == 0 {
        DEFAULT_CATCHUP_HOURS
    } else {
        (inv.routine.offline_cap_hours as u64).min(ceiling_hours)
    };
    let cap_ms = cap_hours.saturating_mul(3_600_000);

    // Skip the un-catchable prefix once at the start of a long
    // return: if the missed window exceeds `cap_ms`, the player
    // forfeits the older part and catchup starts `cap_ms` before
    // `now_ms`. Recomputed only once per catchup chain — once
    // `auto_last_tick_ms` is inside the cap window, subsequent
    // chunks don't slide the floor.
    if now_ms - inv.auto_last_tick_ms > cap_ms {
        inv.auto_last_tick_ms = now_ms - cap_ms;
    }

    // Single-chunk wall: process at most `CATCHUP_CHUNK_HOURS` per
    // call. Frontend re-fires the touch RPC until
    // `auto_last_tick_ms >= now_ms`, drawing a progress bar between
    // chunks. Keeps each `process()` invocation inside the host's
    // per-call CPU budget even for week-long catchups.
    let remaining_ms = now_ms - inv.auto_last_tick_ms;
    let chunk_ms = remaining_ms.min(CATCHUP_CHUNK_HOURS.saturating_mul(3_600_000));
    if chunk_ms == 0 {
        return;
    }

    let started_ms = inv.auto_last_tick_ms;
    let gold_before = inv.gold;
    let essence_before = inv.essence;
    let xp_before = inv.experience;
    let boss_before = inv.boss_damage;
    let mission_before = inv.mission_count;

    // Split into a tick-by-tick prefix (up to ANALYTICAL_THRESHOLD_HOURS
    // of *this chunk*) and an analytical tail. The tail extrapolates
    // per-hour averages from the tick prefix, so a player who returns
    // after a week sees per-tick "drama" for the first 4h of catchup
    // and a smoothed average after that.
    let tick_portion_ms = chunk_ms.min(ANALYTICAL_THRESHOLD_HOURS.saturating_mul(3_600_000));
    let analytical_portion_ms = chunk_ms.saturating_sub(tick_portion_ms);

    let tick_count = tick_portion_ms / CATCHUP_TICK_MS;
    let mut simulated_at = started_ms;
    for _ in 0..tick_count {
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

    // Analytical tail: extrapolate per-hour averages from the tick
    // prefix. Sampled rates are zero for brand-new players (no
    // tick prefix sampled in this chunk) — analytical extrapolation
    // then awards nothing, which is the correct fallback. If a
    // future chunk catches up enough tick prefix, subsequent
    // analytical tails use the fresh sample.
    if analytical_portion_ms > 0 && tick_portion_ms > 0 {
        let sampled_hours = tick_portion_ms / 3_600_000;
        if sampled_hours > 0 {
            let analytical_hours = analytical_portion_ms / 3_600_000;
            let gold_per_h = inv.gold.saturating_sub(gold_before) / sampled_hours;
            let essence_per_h = inv.essence.saturating_sub(essence_before) / sampled_hours;
            let xp_per_h = inv.experience.saturating_sub(xp_before) / sampled_hours;
            let boss_per_h = inv.boss_damage.saturating_sub(boss_before) / sampled_hours;
            let mission_per_h = inv.mission_count.saturating_sub(mission_before) / sampled_hours;
            inv.gold = inv.gold.saturating_add(gold_per_h.saturating_mul(analytical_hours));
            inv.essence = inv.essence.saturating_add(essence_per_h.saturating_mul(analytical_hours));
            inv.experience = inv.experience.saturating_add(xp_per_h.saturating_mul(analytical_hours));
            inv.boss_damage = inv.boss_damage.saturating_add(boss_per_h.saturating_mul(analytical_hours));
            inv.mission_count = inv.mission_count.saturating_add(mission_per_h.saturating_mul(analytical_hours));
        }
        simulated_at = simulated_at.saturating_add(analytical_portion_ms);
    }

    inv.auto_last_tick_ms = simulated_at;
    inv.last_action_ms = simulated_at;
    check_achievements(inv, now_ms);

    let elapsed_simulated = simulated_at.saturating_sub(started_ms);
    let report_threshold_ms = CATCHUP_REPORT_THRESHOLD_TICKS.saturating_mul(CATCHUP_TICK_MS);
    if elapsed_simulated >= report_threshold_ms {
        let missions_total = inv.mission_count.saturating_sub(mission_before);
        // `ticks_simulated` reports wall-clock ms / CATCHUP_TICK_MS
        // so the legacy UI math (ticks → seconds) keeps working for
        // both the tick-by-tick prefix and the analytical tail.
        // No clean "missions_lost" count in tick mode — a defeat
        // ends one mission early so the diff between "missions
        // started" and `missions_total` would be a rough proxy.
        // For the v1 summary we just report wins.
        inv.last_catchup = Some(CatchupSummary {
            started_ms,
            ended_ms: simulated_at,
            ticks_simulated: (elapsed_simulated / CATCHUP_TICK_MS) as u32,
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
