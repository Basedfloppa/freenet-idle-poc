//! Routine automation (backlog B1, expanded scope). Target
//! categories handled by `pump_routine` (called once per
//! `save_inventory` mutation):
//!
//! * **Estate headcount** — `routine.estate_targets[tier_id] = n`
//!   triggers auto-hire when `gold ≥ next_price`.
//! * **Gear tier** — `routine.gear_targets[slot] = tier` swaps in
//!   the best stash piece if the slot has a lower tier; falls
//!   back to `shop_buy_price` if the stash has nothing better
//!   *and* gold permits.
//! * **Consumables** — `routine.consumable_targets[kind] = N`
//!   auto-buys missing potions / fireballs.
//! * **Skills** — `routine.auto_skill_unlock = true` buys any
//!   skill the moment it's affordable.
//! * **Activities** — `routine.auto_activity_at_zone[area_id] = id`
//!   flips `idle_action` to Activity when the player enters a
//!   matching zone.
//!
//! Per-pump caps stop a fat catch-up window from burning the
//! whole treasury in a single tick.

use freenet_stdlib::prelude::*;

use shared::{
    estate_next_price_with_discount, estate_tier, form_slot_mask, gear_template,
    shop_buy_price, shop_roll_catalog_id, skill_buy_price, Inventory,
    ACTIVITY_NONE, CONSUMABLE_FIREBALL, CONSUMABLE_POTION, FIREBALL_PRICE,
    IDLE_ACTION_ACTIVITY, IDLE_ACTION_AUTO_MISSION, IDLE_ACTION_ESTATE,
    POTION_PRICE, SKILL_CHAMPION, SKILL_DRAGON_SCALES, SKILL_FELINE_GRACE,
    SKILL_SLIME_BODY, SKILL_STEED_HEART, SKILL_VETERAN, SLOT_COUNT,
};

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

pub fn set_routine_gear_target(
    ctx: &mut DelegateCtx,
    slot_idx: u8,
    tier: u8,
    now_ms: u64,
) -> Result<Inventory, String> {
    let mut inv = load_inventory_raw(ctx);
    enter_action(&mut inv, now_ms)?;
    if slot_idx as usize >= SLOT_COUNT {
        return Err(format!("slot {slot_idx} out of range"));
    }
    if tier > shared::TIER_COUNT {
        return Err(format!("tier {tier} out of range"));
    }
    inv.routine.set_gear_target(slot_idx, tier);
    save_inventory(ctx, &mut inv)?;
    Ok(inv)
}

/// One-click preset: lock the routine's per-slot gear-target to
/// whatever's currently equipped. Skips empty slots (their target
/// stays whatever it was). Form-mask is enforced separately by
/// `pump_gear_targets`, so no need to filter here.
pub fn lock_routine_gear_targets_to_equipped(
    ctx: &mut DelegateCtx,
    now_ms: u64,
) -> Result<Inventory, String> {
    let mut inv = load_inventory_raw(ctx);
    enter_action(&mut inv, now_ms)?;
    for s in 0..SLOT_COUNT {
        let Some(cid) = inv.equipped[s] else { continue };
        let Some(t) = shared::gear_template(cid) else { continue };
        inv.routine.set_gear_target(s as u8, t.tier);
    }
    save_inventory(ctx, &mut inv)?;
    Ok(inv)
}

pub fn set_routine_consumable_target(
    ctx: &mut DelegateCtx,
    kind: u8,
    target: u32,
    now_ms: u64,
) -> Result<Inventory, String> {
    let mut inv = load_inventory_raw(ctx);
    enter_action(&mut inv, now_ms)?;
    if kind != CONSUMABLE_POTION && kind != CONSUMABLE_FIREBALL {
        return Err(format!("unknown consumable kind {kind}"));
    }
    inv.routine.set_consumable_target(kind, target);
    save_inventory(ctx, &mut inv)?;
    Ok(inv)
}

pub fn set_routine_auto_skill(
    ctx: &mut DelegateCtx,
    enabled: bool,
    now_ms: u64,
) -> Result<Inventory, String> {
    let mut inv = load_inventory_raw(ctx);
    enter_action(&mut inv, now_ms)?;
    inv.routine.auto_skill_unlock = enabled;
    save_inventory(ctx, &mut inv)?;
    Ok(inv)
}

pub fn set_routine_activity_for_zone(
    ctx: &mut DelegateCtx,
    area_id: u8,
    activity_id: u8,
    now_ms: u64,
) -> Result<Inventory, String> {
    let mut inv = load_inventory_raw(ctx);
    enter_action(&mut inv, now_ms)?;
    inv.routine.set_activity_for_zone(area_id, activity_id);
    save_inventory(ctx, &mut inv)?;
    Ok(inv)
}

pub fn set_routine_battle_policy(
    ctx: &mut DelegateCtx,
    policy: shared::BattleActionPolicy,
    now_ms: u64,
) -> Result<Inventory, String> {
    let mut inv = load_inventory_raw(ctx);
    enter_action(&mut inv, now_ms)?;
    inv.routine.battle_action_policy = policy;
    save_inventory(ctx, &mut inv)?;
    Ok(inv)
}

/// Flip the global `auto_equip_best_on_drop` switch. When ON, the
/// pump-driver auto-equips the best stash piece for every form-
/// allowed slot whose current piece is inferior, on every state
/// mutation (i.e. every gear drop).
pub fn set_routine_auto_equip_best(
    ctx: &mut DelegateCtx,
    enabled: bool,
    now_ms: u64,
) -> Result<Inventory, String> {
    let mut inv = load_inventory_raw(ctx);
    enter_action(&mut inv, now_ms)?;
    inv.routine.auto_equip_best_on_drop = enabled;
    save_inventory(ctx, &mut inv)?;
    Ok(inv)
}

pub fn set_routine_offline_cap_hours(
    ctx: &mut DelegateCtx,
    hours: u8,
    now_ms: u64,
) -> Result<Inventory, String> {
    let mut inv = load_inventory_raw(ctx);
    enter_action(&mut inv, now_ms)?;
    // Server-side clamp. `0` is the legacy-default sentinel; non-zero
    // caps at 24h without the LongHaulForeman perk, 168h (7 days)
    // with it. The hard ceiling is kept in u8 (≤ 255) to avoid a
    // wire-format bump; if a future patch wants 30-day catchup it can
    // add an `offline_cap_hours_extended: u16` in a new RoutineState
    // version. See `docs/customization-followups-2026-05-18.md`.
    let ceiling: u8 = if inv.tokens.long_haul() { 168 } else { 24 };
    inv.routine.offline_cap_hours = hours.min(ceiling);
    save_inventory(ctx, &mut inv)?;
    Ok(inv)
}

pub fn set_routine_combat_speed(
    ctx: &mut DelegateCtx,
    mult_bp: u32,
    now_ms: u64,
) -> Result<Inventory, String> {
    let mut inv = load_inventory_raw(ctx);
    enter_action(&mut inv, now_ms)?;
    inv.routine.combat_speed_bp = mult_bp.min(30_000);
    save_inventory(ctx, &mut inv)?;
    Ok(inv)
}

pub fn set_public_cosmetics(
    ctx: &mut DelegateCtx,
    motto: String,
    accent: u8,
    frame: u8,
    now_ms: u64,
) -> Result<Inventory, String> {
    let mut inv = load_inventory_raw(ctx);
    enter_action(&mut inv, now_ms)?;
    // Truncate motto to the wire cap.
    inv.routine.public_motto = motto
        .chars()
        .take(shared::MAX_MOTTO_BYTES)
        .collect();
    inv.routine.public_accent = accent;
    inv.routine.public_frame = frame;
    save_inventory(ctx, &mut inv)?;
    Ok(inv)
}

/// §P3 daily check-in handler. UTC-day rollover: streak increments
/// if exactly one day elapsed since the last claim, resets to 1
/// otherwise, and same-day re-claims are a no-op.
pub fn claim_daily_checkin(
    ctx: &mut DelegateCtx,
    now_ms: u64,
) -> Result<Inventory, String> {
    let mut inv = load_inventory_raw(ctx);
    enter_action(&mut inv, now_ms)?;
    let day = now_ms / 86_400_000;
    let last = inv.routine.last_checkin_day;
    if last == day {
        // Same UTC day — no-op (idempotent).
        return Err("already checked in today".into());
    }
    let new_streak = if last + 1 == day {
        inv.routine.streak_days.saturating_add(1)
    } else {
        1
    };
    let reward = shared::daily_checkin_reward_essence(new_streak);
    inv.essence = inv.essence.saturating_add(reward);
    inv.routine.streak_days = new_streak.min(30);
    inv.routine.last_checkin_day = day;
    save_inventory(ctx, &mut inv)?;
    Ok(inv)
}

pub fn set_routine_mission_cycle(
    ctx: &mut DelegateCtx,
    mode: u8,
    areas: Vec<u8>,
    now_ms: u64,
) -> Result<Inventory, String> {
    let mut inv = load_inventory_raw(ctx);
    enter_action(&mut inv, now_ms)?;
    if mode > shared::MISSION_CYCLE_BOSS_FIRST {
        return Err(format!("unknown mission cycle mode {mode}"));
    }
    inv.routine.mission_cycle_mode = mode;
    inv.routine.mission_cycle_areas = areas;
    inv.routine.mission_cycle_idx = 0;
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
    pump_estate_hires(inv);
    pump_gear_targets(inv);
    pump_auto_equip_best(inv);
    pump_consumable_targets(inv);
    pump_skill_unlock(inv);
    pump_activity_for_zone(inv);
}

/// Run after gear-target pump: if `auto_equip_best_on_drop` is
/// on, walk every form-allowed slot and equip the highest-score
/// stash piece if it beats the current one. Score is the same
/// `atk + def + hp` total the manual `Auto-Equip Best` button
/// uses. No shop-buy — only shuffles existing inventory.
fn pump_auto_equip_best(inv: &mut Inventory) {
    if !inv.routine.auto_equip_best_on_drop {
        return;
    }
    let mask = shared::form_slot_mask(inv.current_form);
    for slot_idx in 0..SLOT_COUNT {
        if !mask[slot_idx] {
            continue;
        }
        let best_unequipped = inv
            .unequipped
            .iter()
            .copied()
            .filter_map(|cid| {
                let t = gear_template(cid)?;
                if t.slot as usize != slot_idx {
                    return None;
                }
                Some(((t.atk + t.def + t.hp) as u64, cid))
            })
            .max_by_key(|(s, _)| *s);
        let Some((best_score, best_cid)) = best_unequipped else {
            continue;
        };
        let current_score = inv.equipped[slot_idx]
            .and_then(gear_template)
            .map(|t| (t.atk + t.def + t.hp) as u64)
            .unwrap_or(0);
        if best_score > current_score {
            if let Some(pos) = inv.unequipped.iter().position(|c| *c == best_cid) {
                inv.unequipped.swap_remove(pos);
            }
            if let Some(prev) = inv.equipped[slot_idx].take() {
                inv.unequipped.push(prev);
            }
            inv.equipped[slot_idx] = Some(best_cid);
        }
    }
}

fn pump_estate_hires(inv: &mut Inventory) {
    if inv.routine.estate_targets.is_empty() {
        return;
    }
    const MAX_AUTO_HIRES_PER_PUMP: u64 = 50;
    let targets: Vec<(u8, u64)> = inv
        .routine
        .estate_targets
        .iter()
        .map(|(k, v)| (*k, *v))
        .collect();
    let discount_bp = inv.insight.frugality_mult_bp();
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
            let price = estate_next_price_with_discount(tier, owned, discount_bp);
            if inv.gold < price {
                break;
            }
            inv.gold -= price;
            inv.base.base.estate.hire(tier_id);
            budget -= 1;
        }
    }
}

/// Equip the best stash piece for every routine-targeted slot; if
/// the stash has nothing of sufficient tier *and* gold permits,
/// buy a fresh one from the shop. Capped at one shop-buy per pump
/// per slot.
fn pump_gear_targets(inv: &mut Inventory) {
    if inv.routine.gear_targets.is_empty() {
        return;
    }
    let mask = form_slot_mask(inv.current_form);
    let targets: Vec<(u8, u8)> = inv
        .routine
        .gear_targets
        .iter()
        .map(|(k, v)| (*k, *v))
        .collect();
    for (slot_idx, want_tier) in targets {
        let s = slot_idx as usize;
        if s >= SLOT_COUNT || !mask[s] || want_tier == 0 {
            continue;
        }
        // Step 1: best stash piece for this slot reaching `want_tier`.
        let best_in_stash = inv
            .unequipped
            .iter()
            .copied()
            .filter_map(|cid| {
                let t = gear_template(cid)?;
                if t.slot as usize == s && t.tier >= want_tier {
                    Some((cid, t.tier, (t.atk + t.def + t.hp) as u64))
                } else { None }
            })
            .max_by_key(|(_, _, score)| *score);

        let current_score = inv.equipped[s]
            .and_then(gear_template)
            .map(|t| (t.atk + t.def + t.hp) as u64)
            .unwrap_or(0);
        let current_tier = inv.equipped[s]
            .and_then(gear_template)
            .map(|t| t.tier)
            .unwrap_or(0);

        if let Some((cid, _, score)) = best_in_stash {
            if score > current_score {
                // Equip + move the existing piece to stash.
                let idx = inv.unequipped.iter().position(|&x| x == cid);
                if let Some(idx) = idx {
                    inv.unequipped.swap_remove(idx);
                }
                if let Some(prev) = inv.equipped[s].take() {
                    inv.unequipped.push(prev);
                }
                inv.equipped[s] = Some(cid);
                continue;
            }
        }
        // Step 2: nothing in stash; if equipped slot below target,
        // buy from shop. Cap one buy per slot per pump.
        if current_tier < want_tier {
            let price = shop_buy_price(want_tier);
            if inv.gold >= price {
                if let Some(cid) = shop_roll_catalog_id(slot_idx, want_tier, 0) {
                    if gear_template(cid).is_some() {
                        inv.gold -= price;
                        if let Some(prev) = inv.equipped[s].take() {
                            inv.unequipped.push(prev);
                        }
                        inv.equipped[s] = Some(cid);
                    }
                }
            }
        }
    }
}

/// Top up potions / fireballs to the routine-targeted stockpile.
/// One purchase per consumable per pump so a fat catch-up doesn't
/// drain the gold pool instantly.
fn pump_consumable_targets(inv: &mut Inventory) {
    if let Some(target) = inv.routine.consumable_target_for(CONSUMABLE_POTION) {
        if (inv.potions as u32) < target && inv.gold >= POTION_PRICE {
            inv.gold -= POTION_PRICE;
            inv.potions = inv.potions.saturating_add(1);
        }
    }
    if let Some(target) = inv.routine.consumable_target_for(CONSUMABLE_FIREBALL) {
        if (inv.fireballs as u32) < target && inv.gold >= FIREBALL_PRICE {
            inv.gold -= FIREBALL_PRICE;
            inv.fireballs = inv.fireballs.saturating_add(1);
        }
    }
}

/// Auto-buy any priced skill the player doesn't yet own when
/// `routine.auto_skill_unlock` is true and gold permits. Cap at
/// one buy per pump so a backlog doesn't bankrupt the player.
/// Form-locked skills (SLIME/CAT/DRAGON/HORSE) have explicit
/// `skill_buy_price` entries; VETERAN/CHAMPION are level-gated
/// rewards and not pumped here.
fn pump_skill_unlock(inv: &mut Inventory) {
    if !inv.routine.auto_skill_unlock {
        return;
    }
    const PRICED_SKILLS: &[u8] = &[
        SKILL_SLIME_BODY,
        SKILL_FELINE_GRACE,
        SKILL_DRAGON_SCALES,
        SKILL_STEED_HEART,
    ];
    let _ = (SKILL_VETERAN, SKILL_CHAMPION); // documented as level-gated
    for id in PRICED_SKILLS {
        if inv.skills_unlocked.contains_key(id) {
            continue;
        }
        let Some(price) = skill_buy_price(*id) else { continue };
        if inv.gold < price {
            continue;
        }
        inv.gold -= price;
        let ts = inv.last_action_ms;
        inv.skills_unlocked.insert(*id, ts);
        break; // cap: one buy per pump
    }
}

/// On every pump, if a zone preference is set and the player is
/// currently in that zone, flip `idle_action = ACTIVITY` and start
/// the matching activity. Idempotent — won't override an already-
/// running activity.
fn pump_activity_for_zone(inv: &mut Inventory) {
    let area_id = inv.current_area;
    let Some(target_activity) = inv.routine.activity_for_zone(area_id) else {
        return;
    };
    if target_activity == ACTIVITY_NONE {
        return;
    }
    // Don't override an active Estate or auto-mission loop.
    if inv.idle_action == IDLE_ACTION_ESTATE
        || inv.idle_action == IDLE_ACTION_AUTO_MISSION
    {
        return;
    }
    // Validate the activity is available in the current area.
    let in_area = shared::activities_for_area(area_id)
        .any(|a| a.id == target_activity);
    if !in_area {
        return;
    }
    if inv.active_activity == target_activity
        && inv.idle_action == IDLE_ACTION_ACTIVITY
    {
        return;
    }
    inv.active_activity = target_activity;
    inv.activity_last_tick_ms = inv.last_action_ms;
    inv.base.base.idle_action = IDLE_ACTION_ACTIVITY;
}
