//! Progression bookkeeping: achievement unlocks, skill unlocks
//! (form-based + level milestones), ending detection. All
//! idempotent — safe to call after every state mutation.

use shared::{
    achievement_label, gear_template, level_of, skill_for_form, AchievementCheck, Inventory,
    ACHIEVEMENT_TABLE, ENDING_DRAGON_LORD, ENDING_PILGRIM, ENDING_QUIET_FARMER, ENDING_VICTORY,
    FORM_DRAGON, SKILL_CHAMPION, SKILL_VETERAN, TIER_COUNT,
};

/// Walk every achievement check; if the threshold is met and the
/// id isn't already in `achievement_unlocks`, record the unlock
/// time. One-shot: never overwrites an existing entry.
pub fn check_achievements(inv: &mut Inventory, now_ms: u64) {
    for (id, check) in ACHIEVEMENT_TABLE {
        if inv.achievement_unlocks.contains_key(id) {
            continue;
        }
        let pass = match check {
            AchievementCheck::Missions(n) => inv.mission_count >= *n,
            AchievementCheck::BossDamage(n) => inv.boss_damage >= *n,
            AchievementCheck::Gold(n) => inv.gold >= *n,
            AchievementCheck::Essence(n) => inv.essence >= *n,
            AchievementCheck::WinCount(n) => inv.mission_count >= *n,
            AchievementCheck::LegendaryEquipped => inv.equipped.iter().any(|e| {
                e.and_then(|cid| gear_template(cid))
                    .map_or(false, |t| t.tier >= TIER_COUNT)
            }),
        };
        if pass {
            inv.achievement_unlocks.insert(*id, now_ms);
            let _ = achievement_label(*id);
        }
    }
}

/// One-shot pass — inspects level + forms_visited and adds any
/// skill whose preconditions are now met. Idempotent.
pub fn check_skill_unlocks(inv: &mut Inventory, now_ms: u64) {
    let visited: Vec<u8> = inv.forms_visited.keys().copied().collect();
    for form in visited {
        if let Some(skill_id) = skill_for_form(form) {
            inv.skills_unlocked.entry(skill_id).or_insert(now_ms);
        }
    }
    let lvl = level_of(inv);
    if lvl >= 10 {
        inv.skills_unlocked.entry(SKILL_VETERAN).or_insert(now_ms);
    }
    if lvl >= 20 {
        inv.skills_unlocked.entry(SKILL_CHAMPION).or_insert(now_ms);
    }
}

/// One-shot ending check. Runs after every encounter and after
/// wheat sales. `last_kill` is `Some(enemy_id)` if the trigger
/// was a winning encounter (used to detect Shadow Lord kills),
/// `None` for non-combat actions.
pub fn check_endings(inv: &mut Inventory, now_ms: u64, last_kill: Option<u16>) {
    if !inv.ending_unlocks.contains_key(&ENDING_PILGRIM)
        && inv.forms_visited.len() >= shared::FORM_COUNT
    {
        inv.ending_unlocks.insert(ENDING_PILGRIM, now_ms);
    }
    if !inv.ending_unlocks.contains_key(&ENDING_QUIET_FARMER)
        && inv.wheat_sold_total >= 10_000
    {
        inv.ending_unlocks.insert(ENDING_QUIET_FARMER, now_ms);
    }
    if let Some(killed_id) = last_kill {
        if killed_id == 31 {
            let key = if inv.current_form == FORM_DRAGON {
                ENDING_DRAGON_LORD
            } else {
                ENDING_VICTORY
            };
            inv.ending_unlocks.entry(key).or_insert(now_ms);
        }
    }
}
