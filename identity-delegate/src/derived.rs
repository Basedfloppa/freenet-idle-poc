//! Derived stat helpers: how Inventory turns into the numbers used
//! by the combat resolver and the UI. Mirrors `crate::game`
//! formulae used on the frontend; both sides must agree so the
//! number you see in the Hero panel matches the one the delegate
//! uses to swing your sword.

use shared::{
    form_base_bonuses, form_speed_evasion, gear_template, level_of, skill_bonuses,
    skill_speed_evasion, Inventory,
};

/// Equipment-only bonus sum. The level/form/skill layer is added
/// on top in `total_bonuses`.
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

/// Total bonus across equipment + form + skills. Single source of
/// truth for the three derived stats below — they all add this on
/// top of a level-based base.
pub fn total_bonuses(inv: &Inventory) -> (u64, u64, u64) {
    let (eq_atk, eq_def, eq_hp) = equipped_bonuses(inv);
    let (f_atk, f_def, f_hp) = form_base_bonuses(inv.current_form);
    let (s_atk, s_def, s_hp) = skill_bonuses(&inv.skills_unlocked);
    (
        eq_atk.saturating_add(f_atk).saturating_add(s_atk),
        eq_def.saturating_add(f_def).saturating_add(s_def),
        eq_hp.saturating_add(f_hp).saturating_add(s_hp),
    )
}

pub fn max_hp_of(inv: &Inventory) -> u64 {
    let lvl = level_of(inv);
    let (_, _, hp_bonus) = total_bonuses(inv);
    // Insight HpPerLevel node — `+1 HP per hero level per node
    // level`. Read-back path that the original B5 MVP skipped;
    // node was being purchased but the spend did nothing.
    let insight_hp = inv
        .insight
        .node_level(shared::InsightNode::HpPerLevel)
        .saturating_mul(lvl);
    20u64
        .saturating_add(lvl.saturating_mul(5))
        .saturating_add(hp_bonus)
        .saturating_add(insight_hp)
}

pub fn attack_of(inv: &Inventory) -> u64 {
    let lvl = level_of(inv);
    let (atk_bonus, _, _) = total_bonuses(inv);
    let raw = 2u64
        .saturating_add(lvl.saturating_mul(2))
        .saturating_add(atk_bonus);
    // Apply Legacy node multiplier (C1). Neutral when no nodes
    // are unlocked (`node_multiplier_bp` returns 10_000 = ×1.0).
    let mult_bp = inv
        .legacy
        .node_multiplier_bp(shared::LegacyNode::HeroAttack);
    raw.saturating_mul(mult_bp) / 10_000
}

pub fn defence_of(inv: &Inventory) -> u64 {
    let lvl = level_of(inv);
    let (_, def_bonus, _) = total_bonuses(inv);
    2u64.saturating_add(lvl.saturating_mul(2))
        .saturating_add(def_bonus)
}

/// Player initiative + evasion. Layered the same way as
/// atk/def/hp: form base + skill bonuses.
pub fn player_speed_evasion(inv: &Inventory) -> (u64, u64) {
    let (f_speed, f_ev) = form_speed_evasion(inv.current_form);
    let (s_speed, s_ev) = skill_speed_evasion(&inv.skills_unlocked);
    (f_speed.saturating_add(s_speed), f_ev.saturating_add(s_ev))
}
