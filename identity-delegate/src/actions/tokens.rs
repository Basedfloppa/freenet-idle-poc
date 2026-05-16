//! Token economy handlers (backlog C2, MVP).
//!
//! Earn rule: one token per `TOKEN_PER_BOSS_DAMAGE` of personal
//! `boss_damage` past the watermark. Awarded lazily inside
//! `save_inventory` so any path that bumps `boss_damage`
//! (combat win, BossAttack, fireball) pays out.

use freenet_stdlib::prelude::*;

use shared::{Inventory, TokenPerk, TOKEN_PER_BOSS_DAMAGE};

use crate::state::{enter_action, load_inventory_raw, save_inventory};

pub fn award_pending_tokens(inv: &mut Inventory) {
    let bd = inv.boss_damage;
    let prior = inv.tokens.last_awarded_boss_damage;
    if bd <= prior {
        return;
    }
    let prior_milestone = prior / TOKEN_PER_BOSS_DAMAGE;
    let new_milestone = bd / TOKEN_PER_BOSS_DAMAGE;
    if new_milestone > prior_milestone {
        let gained = new_milestone - prior_milestone;
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
