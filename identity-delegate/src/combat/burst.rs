//! Legacy burst-mode mission resolver — runs a whole encounter in a
//! single call with no cooldowns. Replaced by the live `tick_battle`
//! path for player-facing play; kept for unit tests that want a
//! deterministic one-shot resolution.

use shared::{
    area_of, enemy_def, enemy_roster_for_area, gear_template, CombatLog, EncounterLog,
    Inventory, COMBAT_OUTCOME_LOSS, COMBAT_OUTCOME_WIN, ENCOUNTERS_PER_MISSION,
    FIREBALL_DROP_EVERY, GEAR_DROP_EVERY, MISSION_DAMAGE, MISSION_ESSENCE, POTION_DROP_EVERY,
};

use crate::derived::{attack_of, defence_of, max_hp_of, player_speed_evasion};
use crate::progression::{check_endings, check_skill_unlocks};

use super::{enforce_form_slot_mask, push_combat_history};

/// Legacy burst-mode mission chain — currently unreferenced after
/// the catch-up loop migrated to `start_battle` + `tick_battle`, but
/// kept around as a deterministic one-shot resolver for unit tests
/// that don't want to thread a wall-clock cooldown through.
#[allow(dead_code)]
pub fn run_mission_chain(inv: &mut Inventory, now_ms: u64) -> Result<(u32, u32), String> {
    if inv.current_hp == 0 {
        return Err("at 0 HP — heal or wait for regen first".into());
    }
    let roster = enemy_roster_for_area(inv.current_area);
    if roster.is_empty() {
        return Err("no enemies defined for current area".into());
    }
    let chain_seed = inv.mission_count;
    let mut won = 0u32;
    let mut lost = 0u32;
    for step in 0..ENCOUNTERS_PER_MISSION {
        if inv.current_hp == 0 {
            break;
        }
        let pick = (chain_seed.wrapping_add(step as u64) as usize) % roster.len();
        let enemy_id = roster[pick];
        let (outcome, _gained) = resolve_encounter_burst(inv, enemy_id, now_ms);
        if outcome == COMBAT_OUTCOME_LOSS {
            lost += 1;
            break;
        }
        won += 1;
    }
    Ok((won, lost))
}

/// Burst resolver — runs a whole encounter in one go (no cooldowns).
/// Used by `run_mission_chain` for tests; live play funnels through
/// `start_battle` + `tick_battle` instead.
#[allow(dead_code)]
fn resolve_encounter_burst(
    inv: &mut Inventory,
    enemy_id: u16,
    now_ms: u64,
) -> (u8, u64) {
    let area = *area_of(inv.current_area);
    let Some(enemy) = enemy_def(enemy_id).copied() else {
        return (COMBAT_OUTCOME_LOSS, 0);
    };
    let player_hp_start = inv.current_hp;
    let player_atk = attack_of(inv);
    let player_def = defence_of(inv);
    let (player_speed, player_evasion) = player_speed_evasion(inv);
    let mut player_hp = player_hp_start;
    let mut enemy_hp = enemy.hp;
    let raw_player_dmg = (player_atk as i64 - enemy.def as i64).max(1) as u64;
    let raw_enemy_dmg = (enemy.atk as i64 - player_def as i64).max(1) as u64;
    let player_hit = raw_player_dmg
        .saturating_mul(100u64.saturating_sub(enemy.evasion.min(95)))
        / 100;
    let enemy_hit = raw_enemy_dmg
        .saturating_mul(100u64.saturating_sub(player_evasion.min(95)))
        / 100;
    let player_hit = player_hit.max(1);
    let enemy_hit = enemy_hit.max(1);
    let player_first = player_speed >= enemy.speed;
    let mut turns: u32 = 0;
    let mut dmg_dealt: u64 = 0;
    let mut dmg_taken: u64 = 0;
    while turns < 200 {
        turns += 1;
        if player_first {
            let d = player_hit.min(enemy_hp);
            enemy_hp = enemy_hp.saturating_sub(d);
            dmg_dealt = dmg_dealt.saturating_add(d);
            if enemy_hp == 0 { break; }
            let d2 = enemy_hit.min(player_hp);
            player_hp = player_hp.saturating_sub(d2);
            dmg_taken = dmg_taken.saturating_add(d2);
            if player_hp == 0 { break; }
        } else {
            let d2 = enemy_hit.min(player_hp);
            player_hp = player_hp.saturating_sub(d2);
            dmg_taken = dmg_taken.saturating_add(d2);
            if player_hp == 0 { break; }
            let d = player_hit.min(enemy_hp);
            enemy_hp = enemy_hp.saturating_sub(d);
            dmg_dealt = dmg_dealt.saturating_add(d);
            if enemy_hp == 0 { break; }
        }
    }
    let outcome = if enemy_hp == 0 && player_hp > 0 {
        COMBAT_OUTCOME_WIN
    } else {
        COMBAT_OUTCOME_LOSS
    };
    let gold_gained = if outcome == COMBAT_OUTCOME_WIN {
        enemy.gold_reward.saturating_mul(area.gold_mult)
    } else {
        0
    };
    inv.current_hp = player_hp;
    if outcome == COMBAT_OUTCOME_WIN {
        inv.mission_count = inv.mission_count.saturating_add(1);
        // Per-area clear counter — feeds the unlock-gate for the
        // next area (A3 in `docs/gameplay-backlog.md`).
        inv.area_clears_inc(area.id);
        inv.gold = inv.gold.saturating_add(gold_gained);
        inv.essence = inv
            .essence
            .saturating_add(MISSION_ESSENCE.saturating_mul(area.essence_mult));
        inv.boss_damage = inv
            .boss_damage
            .saturating_add(MISSION_DAMAGE.saturating_mul(area.damage_mult));
        inv.experience = inv.experience.saturating_add(enemy.xp_reward);
        if inv.mission_count % GEAR_DROP_EVERY == 0 {
            let drop_index = inv.mission_count / GEAR_DROP_EVERY;
            let slot = (drop_index as u16) % 8;
            let tier_bias = (area.id as u16).min(shared::TIER_COUNT as u16 - 1);
            let catalog_id = slot + tier_bias * 8;
            if gear_template(catalog_id).is_some() {
                inv.unequipped.push(catalog_id);
            }
        }
        if inv.mission_count % POTION_DROP_EVERY == 0 {
            inv.potions = inv.potions.saturating_add(1);
        }
        if inv.mission_count % FIREBALL_DROP_EVERY == 0 {
            inv.fireballs = inv.fireballs.saturating_add(1);
        }
    } else if enemy.transform_to != inv.current_form {
        inv.current_form = enemy.transform_to;
        inv.forms_visited
            .entry(enemy.transform_to)
            .or_insert(now_ms);
        enforce_form_slot_mask(inv);
        let cap = max_hp_of(inv);
        inv.current_hp = (cap / 4).max(1);
    }
    let log = EncounterLog {
        area_id: area.id,
        enemy_id,
        player_hp_start,
        player_hp_end: inv.current_hp,
        turns,
        dmg_dealt,
        dmg_taken,
        gold_gained,
        outcome,
        form_after: inv.current_form,
        timestamp_ms: now_ms,
    };
    push_combat_history(inv, log);
    inv.last_combat = Some(CombatLog {
        area_id: area.id,
        player_hp_start,
        player_hp_end: inv.current_hp,
        enemy_hp_start: enemy.hp,
        turns,
        dmg_dealt,
        dmg_taken,
        outcome,
    });
    check_skill_unlocks(inv, now_ms);
    let killed = if outcome == COMBAT_OUTCOME_WIN {
        Some(enemy_id)
    } else {
        None
    };
    check_endings(inv, now_ms, killed);
    (outcome, gold_gained)
}
