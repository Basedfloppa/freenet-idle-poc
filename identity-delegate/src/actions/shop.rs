//! Shop transactions — buy consumables, use them, buy passive skills.

use freenet_stdlib::prelude::*;

use shared::{
    skill_buy_price, Inventory, CONSUMABLE_FIREBALL, CONSUMABLE_POTION, FIREBALL_BOSS_DAMAGE,
    FIREBALL_PRICE, POTION_PRICE,
};

use crate::derived::max_hp_of;
use crate::progression::{check_achievements, check_endings};
use crate::state::{enter_action, load_inventory_raw, save_inventory};

pub fn use_consumable(
    ctx: &mut DelegateCtx,
    kind: u8,
    now_ms: u64,
) -> Result<Inventory, String> {
    let mut inv = load_inventory_raw(ctx);
    enter_action(&mut inv, now_ms)?;
    match kind {
        CONSUMABLE_POTION => {
            if inv.potions == 0 {
                return Err("no potions in inventory".into());
            }
            inv.potions -= 1;
            inv.current_hp = max_hp_of(&inv);
            let _ = shared::POTION_BURST_MISSIONS;
        }
        CONSUMABLE_FIREBALL => {
            if inv.fireballs == 0 {
                return Err("no fireballs in inventory".into());
            }
            inv.fireballs -= 1;
            inv.boss_damage = inv.boss_damage.saturating_add(FIREBALL_BOSS_DAMAGE);
        }
        other => return Err(format!("unknown consumable kind {other}")),
    }
    check_achievements(&mut inv, now_ms);
    save_inventory(ctx, &mut inv)?;
    Ok(inv)
}

pub fn buy_item(
    ctx: &mut DelegateCtx,
    kind: u8,
    now_ms: u64,
) -> Result<Inventory, String> {
    let mut inv = load_inventory_raw(ctx);
    enter_action(&mut inv, now_ms)?;
    let (price, is_potion) = match kind {
        CONSUMABLE_POTION => (POTION_PRICE, true),
        CONSUMABLE_FIREBALL => (FIREBALL_PRICE, false),
        other => return Err(format!("unknown shop item kind {other}")),
    };
    if inv.gold < price {
        return Err(format!("not enough gold: need {price}, have {}", inv.gold));
    }
    inv.gold -= price;
    if is_potion {
        inv.potions = inv.potions.saturating_add(1);
    } else {
        inv.fireballs = inv.fireballs.saturating_add(1);
    }
    save_inventory(ctx, &mut inv)?;
    Ok(inv)
}

pub fn buy_skill(
    ctx: &mut DelegateCtx,
    skill_id: u8,
    now_ms: u64,
) -> Result<Inventory, String> {
    let mut inv = load_inventory_raw(ctx);
    enter_action(&mut inv, now_ms)?;
    let price = skill_buy_price(skill_id)
        .ok_or_else(|| format!("skill {skill_id} can't be bought"))?;
    if inv.skills_unlocked.contains_key(&skill_id) {
        return Err("you already know this skill".into());
    }
    if inv.essence < price {
        return Err(format!("need {price} essence, have {}", inv.essence));
    }
    inv.essence -= price;
    inv.skills_unlocked.insert(skill_id, now_ms);
    inv.current_hp = inv.current_hp.min(max_hp_of(&inv));
    check_achievements(&mut inv, now_ms);
    check_endings(&mut inv, now_ms, None);
    save_inventory(ctx, &mut inv)?;
    Ok(inv)
}
