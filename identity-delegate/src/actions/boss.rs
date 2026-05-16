//! Personal opt-in World Boss attack (backlog C1, MVP).
//! Piggybacks on the existing presence-contract `boss_damage`
//! field — no contract changes needed. The delegate validates
//! the gate, spends essence, increments `boss_damage`, and on
//! the next heartbeat that delta lands in the shared ledger
//! exactly like a combat-derived contribution.

use freenet_stdlib::prelude::*;

use shared::{
    level_of, Inventory, BOSS_ATTACK_DAMAGE, BOSS_ATTACK_ESSENCE_COST, BOSS_ATTACK_MIN_LEVEL,
    BOSS_ATTACK_MIN_MISSIONS,
};

use crate::progression::check_achievements;
use crate::state::{enter_action, load_inventory_raw, save_inventory};

/// Returns true if the player has cleared every gate for the
/// personal-attack action. The frontend uses the same check to
/// decide whether to render the button at all (mirrors the
/// `revealed_has` style elsewhere).
pub fn boss_attack_unlocked(inv: &Inventory) -> bool {
    if inv.mission_count < BOSS_ATTACK_MIN_MISSIONS {
        return false;
    }
    if level_of(inv) < BOSS_ATTACK_MIN_LEVEL {
        return false;
    }
    // At least one Estate worker anywhere — the simplest "you've
    // engaged with the economy spine" check. Whole-tier
    // commitment isn't required for the MVP.
    inv.base.base.estate.workers.values().any(|n| *n > 0)
}

/// Frontend-triggered era-advance award (C1 + C2 contract-side
/// half). The frontend has visibility into the cross-player
/// `cumulative_damage` ledger via the presence-contract
/// subscription, so it knows when the global era advanced and
/// what the player's rank was. The delegate trusts those
/// numbers but bounds-checks against authoritative state:
///
///   - `era > inv.boss_era_witnessed` (monotone — same era
///     can't double-claim)
///   - `dmg_share = inv.boss_damage - boss_damage_at_era_start`
///     clamped to `[0, era_max_hp]` so a desync'd frontend
///     can't claim share > total
///
/// On success: awards stars per `boss_kill_stars_for`, tokens
/// per `boss_kill_tokens_for_rank`, then snapshots
/// `boss_damage_at_era_start = inv.boss_damage` for the next
/// era's share calc.
pub fn claim_boss_kill(
    ctx: &mut DelegateCtx,
    era: u64,
    era_max_hp: u64,
    rank: u8,
    now_ms: u64,
) -> Result<Inventory, String> {
    let mut inv = load_inventory_raw(ctx);
    enter_action(&mut inv, now_ms)?;
    if era <= inv.boss_era_witnessed {
        return Err(format!(
            "era {era} already claimed (witnessed {})",
            inv.boss_era_witnessed
        ));
    }
    let dmg_share = inv
        .boss_damage
        .saturating_sub(inv.boss_damage_at_era_start)
        .min(era_max_hp);
    let stars_gained = shared::boss_kill_stars_for(dmg_share, era_max_hp);
    let tokens_gained = shared::boss_kill_tokens_for_rank(rank);
    inv.legacy.stars = inv.legacy.stars.saturating_add(stars_gained);
    inv.tokens.balance = inv.tokens.balance.saturating_add(tokens_gained);
    inv.boss_era_witnessed = era;
    inv.boss_damage_at_era_start = inv.boss_damage;
    check_achievements(&mut inv, now_ms);
    save_inventory(ctx, &mut inv)?;
    Ok(inv)
}

pub fn boss_attack(ctx: &mut DelegateCtx, now_ms: u64) -> Result<Inventory, String> {
    let mut inv = load_inventory_raw(ctx);
    enter_action(&mut inv, now_ms)?;
    if !boss_attack_unlocked(&inv) {
        return Err("World Boss attack is still locked — keep grinding".into());
    }
    if inv.essence < BOSS_ATTACK_ESSENCE_COST {
        return Err(format!(
            "need {BOSS_ATTACK_ESSENCE_COST} essence, have {}",
            inv.essence
        ));
    }
    inv.essence -= BOSS_ATTACK_ESSENCE_COST;
    inv.boss_damage = inv.boss_damage.saturating_add(BOSS_ATTACK_DAMAGE);
    check_achievements(&mut inv, now_ms);
    save_inventory(ctx, &mut inv)?;
    Ok(inv)
}
