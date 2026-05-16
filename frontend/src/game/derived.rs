//! Pure-function projections of `Inventory` used by the UI. Mirrors
//! the delegate's stat formulae 1-to-1 so the Hero panel shows
//! exactly the same numbers the delegate uses to resolve combat.

use shared::{
    area_of, form_slot_mask, form_speed_evasion, gear_template, level_of,
    skill_speed_evasion, status_label, xp_for_level, xp_total_for_level, Inventory,
    IDLE_ACTION_ESTATE, SLOT_COUNT, STATUS_ADVENTURING, STATUS_DEFEATED, STATUS_ESTATE,
    STATUS_FOCUSING, STATUS_READY, STATUS_RECOVERING,
};

use crate::Core;

/// Would `AutoEquipBest` actually change anything? Mirrors the
/// delegate's `auto_equip_best` decision loop: for every form-
/// allowed slot, find the best stash piece for that slot and
/// compare its score (atk+def+hp) against the currently equipped
/// piece. Returns `true` as soon as one strict improvement is
/// found. Used by the frontend to grey out the button when the
/// player has nothing to upgrade — pressing it would be a no-op
/// otherwise. Keeps the visual cue in sync with what clicking
/// will produce.
pub fn auto_equip_would_change(inv: &Inventory) -> bool {
    let mask = form_slot_mask(inv.current_form);
    for slot_idx in 0..SLOT_COUNT {
        if !mask[slot_idx] {
            continue;
        }
        let best_in_slot = inv
            .unequipped
            .iter()
            .copied()
            .filter_map(|cid| {
                let t = gear_template(cid)?;
                if t.slot as usize != slot_idx {
                    return None;
                }
                Some((t.atk + t.def + t.hp) as u64)
            })
            .max();
        let Some(best_score) = best_in_slot else { continue };
        let current_score = inv.equipped[slot_idx]
            .and_then(gear_template)
            .map(|t| (t.atk + t.def + t.hp) as u64)
            .unwrap_or(0);
        if best_score > current_score {
            return true;
        }
    }
    false
}

/// Equipment-only bonus sum. The level/form/skill layer is added
/// on top in [`total_bonuses_from`].
pub fn equipped_bonuses(inv: &Inventory) -> (u64, u64, u64) {
    let mut atk = 0u64;
    let mut def = 0u64;
    let mut hp = 0u64;
    for slot in inv.equipped.iter() {
        if let Some(cid) = slot {
            if let Some(t) = gear_template(*cid) {
                atk = atk.saturating_add(t.atk as u64);
                def = def.saturating_add(t.def as u64);
                hp = hp.saturating_add(t.hp as u64);
            }
        }
    }
    (atk, def, hp)
}

pub fn total_bonuses_from(inv: &Inventory) -> (u64, u64, u64) {
    let (eq_atk, eq_def, eq_hp) = equipped_bonuses(inv);
    let (f_atk, f_def, f_hp) = shared::form_base_bonuses(inv.current_form);
    let (s_atk, s_def, s_hp) = shared::skill_bonuses(&inv.skills_unlocked);
    (
        eq_atk.saturating_add(f_atk).saturating_add(s_atk),
        eq_def.saturating_add(f_def).saturating_add(s_def),
        eq_hp.saturating_add(f_hp).saturating_add(s_hp),
    )
}

pub fn max_hp_from(inv: &Inventory) -> u64 {
    let lvl = level_of(inv);
    let (_, _, hp_bonus) = total_bonuses_from(inv);
    // Insight HpPerLevel — mirror of `max_hp_of` in the
    // delegate. Frontend reads the same node level so the Hero
    // stats panel matches the combat resolver.
    let insight_hp = inv
        .insight
        .node_level(shared::InsightNode::HpPerLevel)
        .saturating_mul(lvl);
    20u64
        .saturating_add(lvl.saturating_mul(5))
        .saturating_add(hp_bonus)
        .saturating_add(insight_hp)
}

pub fn attack_from(inv: &Inventory) -> u64 {
    let lvl = level_of(inv);
    let (atk_bonus, _, _) = total_bonuses_from(inv);
    let raw = 2u64
        .saturating_add(lvl.saturating_mul(2))
        .saturating_add(atk_bonus);
    let mult_bp = inv
        .legacy
        .node_multiplier_bp(shared::LegacyNode::HeroAttack);
    raw.saturating_mul(mult_bp) / 10_000
}

pub fn defence_from(inv: &Inventory) -> u64 {
    let lvl = level_of(inv);
    let (_, def_bonus, _) = total_bonuses_from(inv);
    2u64.saturating_add(lvl.saturating_mul(2))
        .saturating_add(def_bonus)
}

/// Resolve current XP into `(current_in_level, threshold_for_next)`
/// so the Hero panel can render an "X / Y to next" progress bar.
pub fn xp_in_level(inv: &Inventory) -> (u64, u64) {
    let lvl = level_of(inv);
    let total_for_lvl = xp_total_for_level(lvl);
    let req = xp_for_level(lvl);
    (inv.experience.saturating_sub(total_for_lvl), req)
}

/// Player speed + evasion (mirrors the delegate's `player_speed_evasion`).
pub fn player_speed_evasion(inv: &Inventory) -> (u64, u64) {
    let (f_speed, f_ev) = form_speed_evasion(inv.current_form);
    let (s_speed, s_ev) = skill_speed_evasion(&inv.skills_unlocked);
    (f_speed.saturating_add(s_speed), f_ev.saturating_add(s_ev))
}

/// World Boss damage aggregate — read from the contract's persistent
/// `cumulative_damage` ledger, with our own in-flight `boss_damage`
/// patched in (the delegate may have incremented it locally between
/// heartbeats, before the contract has seen the new value).
///
/// The ledger is per-pubkey high-watermark and is never pruned, so
/// the aggregate is monotonic across player-presence churn —
/// contributing players going offline no longer rolls the boss back.
pub fn world_boss_total_damage(c: &Core) -> u64 {
    let mut total: u64 = 0;
    for (pk, dmg) in c.cumulative_damage.iter() {
        if Some(*pk) == c.pubkey {
            // Our slot in the ledger is replaced by the live
            // delegate-authoritative value to capture damage dealt
            // since the last successful publish.
            continue;
        }
        total = total.saturating_add(*dmg);
    }
    let my_live = c.inventory.boss_damage;
    let my_published = c
        .pubkey
        .and_then(|pk| c.cumulative_damage.get(&pk).copied())
        .unwrap_or(0);
    total.saturating_add(my_live.max(my_published))
}

/// Era-aware boss state. Returns `(era, hp_remaining_in_era,
/// era_max_hp_in_era, total_damage)`.
pub fn world_boss_state(c: &Core) -> (u64, u64, u64, u64) {
    let total = world_boss_total_damage(c);
    let era = shared::era_of_total(total);
    let max_for_era = shared::era_max_hp(era);
    let consumed_into_era = total.saturating_sub(shared::era_threshold(era));
    let remaining = max_for_era.saturating_sub(consumed_into_era);
    (era, remaining, max_for_era, total)
}

/// Status pill code, in priority order:
/// DEFEATED > FOCUSING > ADVENTURING > RECOVERING > READY.
pub fn status_code(c: &Core) -> u8 {
    let max = max_hp_from(&c.inventory);
    if c.inventory.current_hp == 0 {
        STATUS_DEFEATED
    } else if c.mission_in_flight {
        STATUS_FOCUSING
    } else if c.inventory.auto_run_enabled {
        STATUS_ADVENTURING
    } else if c.inventory.idle_action == IDLE_ACTION_ESTATE {
        // Estate is mutually exclusive with auto-mission (§5.6),
        // so a separate pill keeps that clear at a glance.
        STATUS_ESTATE
    } else if max > 0 && c.inventory.current_hp * 2 < max {
        STATUS_RECOVERING
    } else {
        STATUS_READY
    }
}

pub fn status_text(c: &Core) -> &'static str {
    status_label(status_code(c))
}

/// Returns `(chapter number, title, body)` for the player's
/// *currently selected* battlefield. Tied to `inv.current_area` so
/// switching back to a lower area re-narrates that area instead of
/// staying on the deepest one ever reached.
pub fn current_chapter(inv: &Inventory) -> (u8, &'static str, &'static str) {
    match inv.current_area {
        0 => (
            1,
            "Chapter 1 · The Village Fields",
            if inv.mission_count == 0 {
                "Your father points east. \"Be strong, and bring the boss down.\" \
                 The fields outside the village are quiet — for now. Run a mission \
                 to begin."
            } else {
                "You're running errands at the edge of the fields. Each mission \
                 trickles gold and essence into the lockbox the delegate keeps for \
                 you on the node."
            },
        ),
        1 => (
            2,
            "Chapter 2 · The Forest Road",
            "Word of your exploits has reached the next biome. The forest \
             paths yield more essence, but the World Boss begins to stir as \
             every player chips at its HP.",
        ),
        2 => (
            3,
            "Chapter 3 · The Mountain Pass",
            "Merchants pay handsomely at the pass, and the loot scales. \
             Other adventurers across the network are converging on the same \
             foe — every hit is mirrored in the global HP gauge.",
        ),
        _ => (
            4,
            "Chapter 4 · The Boss's Lair",
            "You've reached the inner sanctum. Damage-heavy work — every \
             blow you land is mirrored in the World Boss HP gauge that every \
             connected player sees in real time.",
        ),
    }
}

/// Light-weight name resolver used by status messages outside the
/// renderer (e.g. "area changed to 'Forest Road'"). Pure shim
/// around `shared::area_of` to centralise the lookup site.
pub fn area_of_name(area_id: u8) -> &'static str {
    area_of(area_id).name
}
