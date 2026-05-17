//! Token economy handlers (backlog C2, MVP).
//!
//! Earn rule: one token per *growing* boss-damage milestone — the
//! N-th token requires `N * TOKEN_PER_BOSS_DAMAGE` damage past the
//! previous token, so the player who has dealt enough cumulative
//! damage to chip multiple era boss HP totals doesn't trivially
//! farm tokens.  Awarded lazily inside `save_inventory` so any path
//! that bumps `boss_damage` (combat win, BossAttack, fireball)
//! pays out.

use freenet_stdlib::prelude::*;

use shared::{Inventory, TokenPerk, TOKEN_PER_BOSS_DAMAGE};

use crate::state::{enter_action, load_inventory_raw, save_inventory};

/// Cumulative `boss_damage` watermark at which the player should
/// own at least `count` total milestone tokens. The N-th token
/// threshold is `TOKEN_PER_BOSS_DAMAGE * N * (N + 1) / 2` so the
/// gap grows linearly with rank: token #1 at 500, #2 at 1500, #3
/// at 3000, #4 at 5000, … keeping pace with era boss HP scaling
/// (era E cap = WORLD_BOSS_MAX_HP * (E + 1)^2).
fn tokens_threshold(count: u64) -> u64 {
    TOKEN_PER_BOSS_DAMAGE
        .saturating_mul(count)
        .saturating_mul(count.saturating_add(1))
        / 2
}

/// Inverse of `tokens_threshold` — given a cumulative damage
/// watermark, returns how many milestone tokens are due. Stable
/// integer math: increment count while the next threshold is
/// still ≤ damage. Capped at 10_000 so a u64-overflow boss
/// damage can't loop forever.
fn tokens_due_at(damage: u64) -> u64 {
    let mut count = 0u64;
    while count < 10_000 {
        let next = tokens_threshold(count + 1);
        if next > damage {
            break;
        }
        count += 1;
    }
    count
}

pub fn award_pending_tokens(inv: &mut Inventory) {
    let bd = inv.boss_damage;
    let prior = inv.tokens.last_awarded_boss_damage;
    if bd <= prior {
        return;
    }
    let prior_tokens = tokens_due_at(prior);
    let new_tokens = tokens_due_at(bd);
    if new_tokens > prior_tokens {
        let gained = new_tokens - prior_tokens;
        inv.tokens.balance = inv.tokens.balance.saturating_add(gained);
    }
    inv.tokens.last_awarded_boss_damage = bd;
}

pub fn buy_token_perk(
    ctx: &mut DelegateCtx,
    perk_id: u8,
    now_ms: u64,
) -> Result<Inventory, String> {
    let mut inv = load_inventory_raw(ctx);
    enter_action(&mut inv, now_ms)?;
    let perk = TokenPerk::from_id(perk_id)
        .ok_or_else(|| format!("unknown token perk {perk_id}"))?;
    award_pending_tokens(&mut inv);
    if inv.tokens.owns(perk) {
        return Err("you already own this perk".into());
    }
    let price = perk.price();
    if inv.tokens.balance < price {
        return Err(format!("need {price} tokens, have {}", inv.tokens.balance));
    }
    inv.tokens.balance -= price;
    inv.tokens.perks.insert(perk.id(), now_ms);
    save_inventory(ctx, &mut inv)?;
    Ok(inv)
}
