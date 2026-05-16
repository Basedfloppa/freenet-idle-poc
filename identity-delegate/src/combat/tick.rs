//! Interactive tick-based combat loop. `tick_battle` advances the
//! active `BattleState` by however many `TURN_COOLDOWN_MS` have
//! elapsed since the last resolved turn; `run_one_turn` resolves a
//! single swing-and-counter; the `end_*` helpers finalize encounter
//! and battle outcomes and update progression state.

use shared::{
    area_of, enemy_def, enemy_roster_for_area, gear_template, BattleState,
    BattleTurn, CombatLog, EncounterLog, Inventory, BATTLE_ACTION_FIREBALL,
    BATTLE_ACTION_NONE, BATTLE_ACTION_POTION, COMBAT_OUTCOME_LOSS, COMBAT_OUTCOME_WIN,
    ENCOUNTERS_PER_MISSION, FIREBALL_BOSS_DAMAGE, FIREBALL_DROP_EVERY, GEAR_DROP_EVERY,
    MISSION_DAMAGE, MISSION_ESSENCE, POTION_DROP_EVERY, TURN_COOLDOWN_MS,
};

use crate::derived::{attack_of, defence_of, max_hp_of, player_speed_evasion};
use crate::progression::{check_endings, check_skill_unlocks};

use super::{enforce_form_slot_mask, push_combat_history, push_turn};

/// Start a fresh battle against the first enemy in the chain. If a
/// battle is already in progress, returns `Ok(false)` (idempotent —
/// callers re-invoke this on every Run-Mission click without having
/// to check `current_battle` first). Returns `Err` on bad inventory
/// state (zero HP, no enemies for the current area).
pub fn start_battle(inv: &mut Inventory, now_ms: u64) -> Result<bool, String> {
    if inv.current_battle.is_some() {
        return Ok(false);
    }
    if inv.current_hp == 0 {
        return Err("at 0 HP — heal or wait for regen first".into());
    }
    let roster = enemy_roster_for_area(inv.current_area);
    if roster.is_empty() {
        return Err("no enemies defined for current area".into());
    }
    let chain_seed = inv.mission_count;
    let pick = (chain_seed as usize) % roster.len();
    let enemy_id = roster[pick];
    let enemy = enemy_def(enemy_id)
        .copied()
        .ok_or_else(|| format!("unknown enemy id {enemy_id}"))?;
    inv.current_battle = Some(BattleState {
        encounter_idx: 0,
        enemy_id,
        enemy_hp: enemy.hp,
        enemy_max_hp: enemy.hp,
        // Anchor the first turn to "now − cooldown" so the very first
        // `tick_battle` call already produces a turn — otherwise
        // there's a confusing 1 s no-op delay after Start.
        last_turn_ms: now_ms.saturating_sub(TURN_COOLDOWN_MS),
        queued_action: BATTLE_ACTION_NONE,
        started_ms: now_ms,
        chain_seed,
        recent_turns: Vec::new(),
    });
    Ok(true)
}

/// Set the action the player wants to take at the start of the
/// next turn. Single-slot queue — clicking a second action before
/// the next turn fires overwrites the first. Returns `false` if
/// there's no active battle to queue against.
pub fn queue_action(inv: &mut Inventory, action: u8) -> bool {
    let Some(ref mut battle) = inv.current_battle else { return false };
    battle.queued_action = action;
    true
}

/// Advance the active battle by however many `TURN_COOLDOWN_MS`
/// have elapsed since the last resolved turn. Returns the number of
/// turns actually resolved. Cleans up `current_battle` on
/// mission-complete or player-defeat.
///
/// Idempotent against the wall-clock: called twice with the same
/// `now_ms` it resolves the same set of turns. Safe to call from
/// `touch_inventory` on every pull-refresh.
pub fn tick_battle(inv: &mut Inventory, now_ms: u64) -> u32 {
    let mut turns_resolved = 0u32;
    // Hard cap so a delegate that wakes up after a long sleep
    // doesn't burn CPU budget burning through a year of accumulated
    // turns. Catch-up needs a separate ceiling anyway.
    const MAX_TURNS_PER_TICK: u32 = 600;
    while turns_resolved < MAX_TURNS_PER_TICK {
        let Some(battle) = inv.current_battle.as_ref() else { break };
        let next_turn_at = battle
            .last_turn_ms
            .saturating_add(TURN_COOLDOWN_MS);
        if next_turn_at > now_ms {
            break;
        }
        let outcome = run_one_turn(inv, next_turn_at);
        turns_resolved += 1;
        match outcome {
            TurnOutcome::Continue => {}
            TurnOutcome::EncounterWon { enemy_id } => {
                end_encounter_win(inv, enemy_id, next_turn_at);
                advance_to_next_encounter(inv, next_turn_at);
            }
            TurnOutcome::BattleLost { enemy_id } => {
                end_battle_loss(inv, enemy_id, next_turn_at);
                break;
            }
        }
    }
    if turns_resolved > 0 {
        // Achievements may unlock from cumulative counters (gold,
        // missions, etc.) — sweep once at the end of the tick batch
        // rather than after every turn.
        crate::progression::check_achievements(inv, now_ms);
    }
    turns_resolved
}

enum TurnOutcome {
    Continue,
    EncounterWon { enemy_id: u16 },
    BattleLost { enemy_id: u16 },
}

/// Resolve exactly one combat turn against the current enemy.
/// Mutates `inv.current_hp`, the battle's enemy HP / log / action
/// queue. Returns whether the encounter (or battle) is now over.
fn run_one_turn(inv: &mut Inventory, turn_ms: u64) -> TurnOutcome {
    // Pull the enemy snapshot up front — `enemy_def` is static.
    let enemy_id = inv
        .current_battle
        .as_ref()
        .map(|b| b.enemy_id)
        .unwrap_or(0);
    let Some(enemy) = enemy_def(enemy_id).copied() else {
        // Misconfigured roster — treat as a non-fatal loss so the
        // battle clears and the player isn't stuck.
        return TurnOutcome::BattleLost { enemy_id };
    };

    // Derive player stats now (immutable borrow OK before mutation).
    let player_atk = attack_of(inv);
    let player_def = defence_of(inv);
    let (player_speed, player_evasion) = player_speed_evasion(inv);
    let max_hp = max_hp_of(inv);

    // Pre-compute per-side hits using the same evasion-as-damage-
    // scaling rule the burst-mode resolver used.
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

    // Apply queued action and basic swings, accumulating damage
    // counters. Borrow `battle` for the duration of this turn so we
    // can mutate it inline.
    let mut player_dmg_this_turn = 0u64;
    let mut enemy_dmg_this_turn = 0u64;
    let mut bonus_enemy_dmg = 0u64;
    let mut healed = false;
    // Pop the queued action out of the battle so the next turn
    // starts clean. Done in its own scope to release the &mut
    // borrow on `inv.current_battle` before we mutate `inv` below.
    let queued = {
        let battle = inv
            .current_battle
            .as_mut()
            .expect("current_battle present from caller");
        std::mem::replace(&mut battle.queued_action, BATTLE_ACTION_NONE)
    };
    let mut applied_action = queued;
    match applied_action {
        BATTLE_ACTION_POTION if inv.potions > 0 => {
            inv.potions -= 1;
            inv.current_hp = max_hp;
            healed = true;
        }
        BATTLE_ACTION_FIREBALL if inv.fireballs > 0 => {
            inv.fireballs -= 1;
            bonus_enemy_dmg = FIREBALL_BOSS_DAMAGE;
        }
        _ => {
            // Not enough consumables OR no action queued — silent.
            applied_action = BATTLE_ACTION_NONE;
        }
    }

    let mut player_hp = inv.current_hp;
    // Hold enemy HP locally; write it back at the end of the turn.
    let mut enemy_hp = inv
        .current_battle
        .as_ref()
        .map(|b| b.enemy_hp)
        .unwrap_or(0);

    // Fireball bonus damage lands before swings — same as a
    // pre-combat spell would.
    if bonus_enemy_dmg > 0 {
        let d = bonus_enemy_dmg.min(enemy_hp);
        enemy_hp = enemy_hp.saturating_sub(d);
        player_dmg_this_turn = player_dmg_this_turn.saturating_add(d);
    }

    let mut encounter_won = false;
    let mut battle_lost = false;
    if enemy_hp == 0 {
        encounter_won = true;
    } else {
        // Swings with initiative — preserve the legacy ordering.
        if player_first {
            let d = player_hit.min(enemy_hp);
            enemy_hp = enemy_hp.saturating_sub(d);
            player_dmg_this_turn = player_dmg_this_turn.saturating_add(d);
            if enemy_hp == 0 {
                encounter_won = true;
            } else {
                let d2 = enemy_hit.min(player_hp);
                player_hp = player_hp.saturating_sub(d2);
                enemy_dmg_this_turn = enemy_dmg_this_turn.saturating_add(d2);
                if player_hp == 0 {
                    battle_lost = true;
                }
            }
        } else {
            let d2 = enemy_hit.min(player_hp);
            player_hp = player_hp.saturating_sub(d2);
            enemy_dmg_this_turn = enemy_dmg_this_turn.saturating_add(d2);
            if player_hp == 0 {
                battle_lost = true;
            } else {
                let d = player_hit.min(enemy_hp);
                enemy_hp = enemy_hp.saturating_sub(d);
                player_dmg_this_turn = player_dmg_this_turn.saturating_add(d);
                if enemy_hp == 0 {
                    encounter_won = true;
                }
            }
        }
    }

    // Commit player HP. Note: do NOT clamp to max_hp on heal — the
    // potion path already set current_hp = max_hp above.
    inv.current_hp = if healed { max_hp } else { player_hp };

    // Write the turn back to the battle log + update its HP / clock.
    // Reach through `.base` so the borrow checker splits the two
    // field borrows (`current_battle` mut + `current_hp` immut)
    // instead of seeing a single whole-struct borrow through Deref.
    let player_hp_now = inv.current_hp;
    let battle = inv
        .base
        .current_battle
        .as_mut()
        .expect("current_battle present from caller");
    battle.enemy_hp = enemy_hp;
    battle.last_turn_ms = turn_ms;
    push_turn(
        battle,
        BattleTurn {
            ts_ms: turn_ms,
            action: applied_action,
            player_dmg: player_dmg_this_turn.min(u32::MAX as u64) as u32,
            enemy_dmg: enemy_dmg_this_turn.min(u32::MAX as u64) as u32,
            enemy_hp_after: enemy_hp,
            player_hp_after: player_hp_now,
        },
    );

    if battle_lost {
        TurnOutcome::BattleLost { enemy_id }
    } else if encounter_won {
        TurnOutcome::EncounterWon { enemy_id }
    } else {
        TurnOutcome::Continue
    }
}

/// Apply victory rewards for a finished encounter: mission counter,
/// gold/essence/XP, drops, skill+ending unlocks, history entry. Does
/// NOT touch `current_battle` (the caller decides whether to advance
/// the chain or finalize the mission).
fn end_encounter_win(inv: &mut Inventory, enemy_id: u16, turn_ms: u64) {
    let area = *area_of(inv.current_area);
    let Some(enemy) = enemy_def(enemy_id).copied() else { return };
    let gold_gained = enemy.gold_reward.saturating_mul(area.gold_mult);
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

    let battle = inv.current_battle.as_ref();
    let player_hp_start = battle
        .and_then(|b| b.recent_turns.first().map(|t| t.player_hp_after))
        .unwrap_or(inv.current_hp);
    let enemy_hp_start = battle.map(|b| b.enemy_max_hp).unwrap_or(0);
    let turns_count: u32 = battle.map(|b| b.recent_turns.len() as u32).unwrap_or(0);

    let log = EncounterLog {
        area_id: area.id,
        enemy_id,
        player_hp_start,
        player_hp_end: inv.current_hp,
        turns: turns_count,
        dmg_dealt: enemy_hp_start,
        dmg_taken: player_hp_start.saturating_sub(inv.current_hp),
        gold_gained,
        outcome: COMBAT_OUTCOME_WIN,
        form_after: inv.current_form,
        timestamp_ms: turn_ms,
    };
    push_combat_history(inv, log);

    inv.last_combat = Some(CombatLog {
        area_id: area.id,
        player_hp_start,
        player_hp_end: inv.current_hp,
        enemy_hp_start,
        turns: turns_count,
        dmg_dealt: enemy_hp_start,
        dmg_taken: player_hp_start.saturating_sub(inv.current_hp),
        outcome: COMBAT_OUTCOME_WIN,
    });

    check_skill_unlocks(inv, turn_ms);
    check_endings(inv, turn_ms, Some(enemy_id));
}

/// Move the chain pointer forward. If there's another encounter to
/// fight, replace the enemy in `current_battle` with the next pick
/// from the roster. If we've finished `ENCOUNTERS_PER_MISSION`, end
/// the battle (clear `current_battle`).
fn advance_to_next_encounter(inv: &mut Inventory, turn_ms: u64) {
    // Capture the area id before the mutable borrow on
    // `current_battle` — Deref through V11→V10 doesn't field-split,
    // so we'd otherwise borrow-check-fail on `inv.current_area`.
    let area_id = inv.current_area;
    let Some(battle) = inv.base.current_battle.as_mut() else { return };
    battle.encounter_idx = battle.encounter_idx.saturating_add(1);
    if battle.encounter_idx >= ENCOUNTERS_PER_MISSION as u8 {
        inv.current_battle = None;
        return;
    }
    let roster = enemy_roster_for_area(area_id);
    if roster.is_empty() {
        inv.current_battle = None;
        return;
    }
    let pick = (battle
        .chain_seed
        .wrapping_add(battle.encounter_idx as u64) as usize)
        % roster.len();
    let next_enemy_id = roster[pick];
    let Some(next_enemy) = enemy_def(next_enemy_id).copied() else {
        inv.current_battle = None;
        return;
    };
    battle.enemy_id = next_enemy_id;
    battle.enemy_hp = next_enemy.hp;
    battle.enemy_max_hp = next_enemy.hp;
    // The fight continues — the next turn fires after one more
    // cooldown so the player has a beat to glance at the new enemy.
    battle.last_turn_ms = turn_ms;
    // Don't clear `recent_turns` — keep the recent feed continuous
    // across encounter boundaries. The history cap will roll it.
}

/// End a battle by player defeat. Applies transformation (if the
/// losing enemy carries one), pushes the encounter log, and clears
/// `current_battle`. Player HP stays at 0 or the post-transform
/// fraction — same rules the burst-mode resolver applied.
fn end_battle_loss(inv: &mut Inventory, enemy_id: u16, turn_ms: u64) {
    let area = *area_of(inv.current_area);
    let Some(enemy) = enemy_def(enemy_id).copied() else {
        inv.current_battle = None;
        return;
    };
    let (player_hp_start, enemy_hp_start, turns_count) =
        if let Some(battle) = inv.current_battle.as_ref() {
            let player_start = battle
                .recent_turns
                .first()
                .map(|t| t.player_hp_after)
                .unwrap_or(inv.current_hp);
            (player_start, battle.enemy_max_hp, battle.recent_turns.len() as u32)
        } else {
            (inv.current_hp, 0, 0)
        };
    let dmg_dealt = enemy_hp_start.saturating_sub(
        inv.current_battle
            .as_ref()
            .map(|b| b.enemy_hp)
            .unwrap_or(0),
    );

    if enemy.transform_to != inv.current_form {
        inv.current_form = enemy.transform_to;
        inv.forms_visited
            .entry(enemy.transform_to)
            .or_insert(turn_ms);
        enforce_form_slot_mask(inv);
        let cap = max_hp_of(inv);
        inv.current_hp = (cap / 4).max(1);
    }

    let log = EncounterLog {
        area_id: area.id,
        enemy_id,
        player_hp_start,
        player_hp_end: inv.current_hp,
        turns: turns_count,
        dmg_dealt,
        dmg_taken: player_hp_start.saturating_sub(inv.current_hp),
        gold_gained: 0,
        outcome: COMBAT_OUTCOME_LOSS,
        form_after: inv.current_form,
        timestamp_ms: turn_ms,
    };
    push_combat_history(inv, log);

    inv.last_combat = Some(CombatLog {
        area_id: area.id,
        player_hp_start,
        player_hp_end: inv.current_hp,
        enemy_hp_start,
        turns: turns_count,
        dmg_dealt,
        dmg_taken: player_hp_start.saturating_sub(inv.current_hp),
        outcome: COMBAT_OUTCOME_LOSS,
    });

    check_skill_unlocks(inv, turn_ms);
    check_endings(inv, turn_ms, None);
    inv.current_battle = None;
}
