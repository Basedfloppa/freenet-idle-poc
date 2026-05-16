//! Shop transactions — buy consumables, use them, buy passive skills.

use freenet_stdlib::prelude::*;

use shared::{
    consumable_sell_price, skill_buy_price, Inventory, CONSUMABLE_FIREBALL, CONSUMABLE_POTION,
    FIREBALL_BOSS_DAMAGE, FIREBALL_PRICE, POTION_PRICE,
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

/// Sell `amount` copies of a consumable at half its buy price.
/// `amount == 0` means "sell every copy on hand". The delegate
/// is authoritative on inventory counts, so a tampered webapp
/// asking to sell more than is owned just gets the available
/// stack liquidated (clamped, not rejected).
pub fn sell_consumable(
    ctx: &mut DelegateCtx,
    kind: u8,
    amount: u32,
    now_ms: u64,
) -> Result<Inventory, String> {
    let mut inv = load_inventory_raw(ctx);
    enter_action(&mut inv, now_ms)?;
    let unit_price = consumable_sell_price(kind)
        .ok_or_else(|| format!("unknown consumable kind {kind}"))?;
    let on_hand = match kind {
        CONSUMABLE_POTION => inv.potions,
        CONSUMABLE_FIREBALL => inv.fireballs,
        _ => 0,
    };
    if on_hand == 0 {
        return Err("nothing to sell".into());
    }
    let to_sell = if amount == 0 { on_hand } else { amount.min(on_hand) };
    let gain = unit_price.saturating_mul(to_sell as u64);
    match kind {
        CONSUMABLE_POTION => inv.potions = inv.potions.saturating_sub(to_sell),
        CONSUMABLE_FIREBALL => inv.fireballs = inv.fireballs.saturating_sub(to_sell),
        _ => {}
    }
    inv.gold = inv.gold.saturating_add(gain);
    save_inventory(ctx, &mut inv)?;
    Ok(inv)
}

/// Bulk-buy a stack of consumables in one call. `count == 0`
/// means "buy as many as the wallet allows". Capped at 1000
/// per call so a single click can't drain a deep treasury into
/// a single overflowing stockpile (defensive — tampered webapp
/// could send `u32::MAX` otherwise).
pub fn bulk_buy_item(
    ctx: &mut DelegateCtx,
    kind: u8,
    count: u32,
    now_ms: u64,
) -> Result<Inventory, String> {
    const MAX_PER_CALL: u32 = 1_000;
    let mut inv = load_inventory_raw(ctx);
    enter_action(&mut inv, now_ms)?;
    let unit_price = match kind {
        CONSUMABLE_POTION => POTION_PRICE,
        CONSUMABLE_FIREBALL => FIREBALL_PRICE,
        other => return Err(format!("unknown shop item kind {other}")),
    };
    if unit_price == 0 {
        return Err("zero unit price — refused".into());
    }
    let max_affordable = (inv.gold / unit_price).min(MAX_PER_CALL as u64) as u32;
    let want = if count == 0 { max_affordable } else { count.min(max_affordable) };
    if want == 0 {
        return Err(format!("not enough gold for one {kind}: need {unit_price}, have {}", inv.gold));
    }
    let total = unit_price.saturating_mul(want as u64);
    inv.gold = inv.gold.saturating_sub(total);
    match kind {
        CONSUMABLE_POTION => inv.potions = inv.potions.saturating_add(want),
        CONSUMABLE_FIREBALL => inv.fireballs = inv.fireballs.saturating_add(want),
        _ => unreachable!(),
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

/// Buy a form change from the shop. `FORM_HUMAN` is the cheap
/// "panic reset" path used when a defeat-induced form change
/// leaves the player in something they didn't want; the other
/// four are a strategic stat / equip-mask commitment and cost
/// much more (see `form_buy_price`).
///
/// Side effects mirror the defeat-induced form-change path:
///  * heal to the new form's max HP so the player doesn't end
///    up with a sliver bar in a higher-HP form,
///  * stamp the form into `forms_visited` (idempotent), which is
///    the same set the skill-unlock + Pilgrim ending check
///    consult — buying a form thus also makes its skill available
///    in the Sage.
pub fn buy_form(ctx: &mut DelegateCtx, form: u8, now_ms: u64) -> Result<Inventory, String> {
    let mut inv = load_inventory_raw(ctx);
    enter_action(&mut inv, now_ms)?;
    let price = shared::form_buy_price(form)
        .ok_or_else(|| format!("unknown form id {form}"))?;
    if inv.current_form == form {
        return Err("you are already in this form".into());
    }
    if inv.gold < price {
        return Err(format!("need {price} gold, have {}", inv.gold));
    }
    inv.gold -= price;
    inv.current_form = form;
    inv.forms_visited.entry(form).or_insert(now_ms);
    // Same housekeeping as the defeat-induced transformation:
    // disallowed slots move their gear back into the stash so a
    // Slime doesn't keep a Pants slot equipped. The shop path
    // used to skip this and left ghost gear in place.
    crate::combat::enforce_form_slot_mask(&mut inv);
    inv.current_hp = max_hp_of(&inv);
    check_achievements(&mut inv, now_ms);
    check_endings(&mut inv, now_ms, None);
    save_inventory(ctx, &mut inv)?;
    Ok(inv)
}
