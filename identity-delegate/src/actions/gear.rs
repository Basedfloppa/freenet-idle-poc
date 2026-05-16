//! Gear management — equip / unequip / sell / forge / buy / auto-equip.

use freenet_stdlib::prelude::*;

use shared::{
    forge_essence_cost, gear_sell_price, gear_template, shop_buy_price, shop_roll_catalog_id,
    Inventory, FORGE_COUNT, SLOT_COUNT, TIER_COUNT,
};

use crate::derived::max_hp_of;
use crate::progression::check_achievements;
use crate::state::{enter_action, load_inventory_raw, save_inventory};

pub fn equip_gear(
    ctx: &mut DelegateCtx,
    catalog_id: u16,
    now_ms: u64,
) -> Result<Inventory, String> {
    let mut inv = load_inventory_raw(ctx);
    enter_action(&mut inv, now_ms)?;
    let tmpl = gear_template(catalog_id)
        .ok_or_else(|| format!("unknown gear id {catalog_id}"))?;
    let slot_idx = tmpl.slot as usize;
    let mask = shared::form_slot_mask(inv.current_form);
    if !mask[slot_idx] {
        return Err(format!(
            "current form can't wear {} (slot {})",
            tmpl.name(),
            slot_idx
        ));
    }
    let pos = inv
        .unequipped
        .iter()
        .position(|c| *c == catalog_id)
        .ok_or_else(|| format!("gear {catalog_id} not in stash"))?;
    inv.unequipped.swap_remove(pos);
    if let Some(prev) = inv.equipped[slot_idx].take() {
        inv.unequipped.push(prev);
    }
    inv.equipped[slot_idx] = Some(catalog_id);
    inv.current_hp = inv.current_hp.min(max_hp_of(&inv));
    check_achievements(&mut inv, now_ms);
    save_inventory(ctx, &mut inv)?;
    Ok(inv)
}

pub fn unequip_slot(
    ctx: &mut DelegateCtx,
    slot: u8,
    now_ms: u64,
) -> Result<Inventory, String> {
    let mut inv = load_inventory_raw(ctx);
    enter_action(&mut inv, now_ms)?;
    let idx = slot as usize;
    if idx >= SLOT_COUNT {
        return Err(format!("slot {slot} out of range"));
    }
    if let Some(prev) = inv.equipped[idx].take() {
        inv.unequipped.push(prev);
    }
    inv.current_hp = inv.current_hp.min(max_hp_of(&inv));
    save_inventory(ctx, &mut inv)?;
    Ok(inv)
}

pub fn sell_gear(
    ctx: &mut DelegateCtx,
    catalog_id: u16,
    now_ms: u64,
) -> Result<Inventory, String> {
    let mut inv = load_inventory_raw(ctx);
    enter_action(&mut inv, now_ms)?;
    let tmpl = gear_template(catalog_id)
        .ok_or_else(|| format!("unknown gear id {catalog_id}"))?;
    let pos = inv
        .unequipped
        .iter()
        .position(|c| *c == catalog_id)
        .ok_or_else(|| format!("gear {catalog_id} not in stash"))?;
    inv.unequipped.swap_remove(pos);
    inv.gold = inv.gold.saturating_add(gear_sell_price(tmpl.tier));
    check_achievements(&mut inv, now_ms);
    save_inventory(ctx, &mut inv)?;
    Ok(inv)
}

/// Bulk-sell every copy of `catalog_id` currently in the stash.
/// One delegate round-trip instead of N — designed for the
/// "I have 50 Worn Helms, just liquidate the lot" case.
/// Atomicity: either all copies are removed and the matching
/// gold credited, or the call fails up-front (unknown id /
/// nothing in stash); no partial-sale state is ever persisted.
pub fn sell_gear_all(
    ctx: &mut DelegateCtx,
    catalog_id: u16,
    now_ms: u64,
) -> Result<Inventory, String> {
    let mut inv = load_inventory_raw(ctx);
    enter_action(&mut inv, now_ms)?;
    let tmpl = gear_template(catalog_id)
        .ok_or_else(|| format!("unknown gear id {catalog_id}"))?;
    let count_before = inv.unequipped.iter().filter(|c| **c == catalog_id).count();
    if count_before == 0 {
        return Err(format!("gear {catalog_id} not in stash"));
    }
    inv.unequipped.retain(|c| *c != catalog_id);
    let unit_price = gear_sell_price(tmpl.tier);
    let total = unit_price.saturating_mul(count_before as u64);
    inv.gold = inv.gold.saturating_add(total);
    check_achievements(&mut inv, now_ms);
    save_inventory(ctx, &mut inv)?;
    Ok(inv)
}

pub fn forge_upgrade(
    ctx: &mut DelegateCtx,
    catalog_id: u16,
    now_ms: u64,
) -> Result<Inventory, String> {
    let mut inv = load_inventory_raw(ctx);
    enter_action(&mut inv, now_ms)?;
    let tmpl = gear_template(catalog_id)
        .ok_or_else(|| format!("unknown gear id {catalog_id}"))?;
    if tmpl.tier >= TIER_COUNT {
        return Err(format!("{} is already at max tier", tmpl.name()));
    }
    let copies = inv
        .unequipped
        .iter()
        .filter(|c| **c == catalog_id)
        .count();
    if copies < FORGE_COUNT {
        return Err(format!(
            "need {FORGE_COUNT} copies of '{}', have {copies}",
            tmpl.name()
        ));
    }
    let cost = forge_essence_cost(tmpl.tier);
    if inv.essence < cost {
        return Err(format!("need {cost} essence, have {}", inv.essence));
    }
    let mut to_remove = FORGE_COUNT;
    let mut i = inv.unequipped.len();
    while i > 0 && to_remove > 0 {
        i -= 1;
        if inv.unequipped[i] == catalog_id {
            inv.unequipped.swap_remove(i);
            to_remove -= 1;
        }
    }
    inv.essence -= cost;
    let next_id = catalog_id + 8;
    if gear_template(next_id).is_some() {
        inv.unequipped.push(next_id);
    }
    check_achievements(&mut inv, now_ms);
    save_inventory(ctx, &mut inv)?;
    Ok(inv)
}

pub fn buy_gear_roll(
    ctx: &mut DelegateCtx,
    slot: u8,
    tier: u8,
    now_ms: u64,
) -> Result<Inventory, String> {
    let mut inv = load_inventory_raw(ctx);
    enter_action(&mut inv, now_ms)?;
    let price = shop_buy_price(tier);
    if price == u64::MAX {
        return Err(format!(
            "tier {tier} is not buyable (legendaries only drop or forge)"
        ));
    }
    if inv.gold < price {
        return Err(format!("need {price}g for tier {tier} {}", slot_label(slot)));
    }
    let Some(cid) = shop_roll_catalog_id(slot, tier, inv.shop_purchase_count) else {
        return Err(format!("invalid (slot {slot}, tier {tier}) — out of catalog"));
    };
    inv.gold -= price;
    inv.unequipped.push(cid);
    inv.shop_purchase_count = inv.shop_purchase_count.saturating_add(1);
    check_achievements(&mut inv, now_ms);
    save_inventory(ctx, &mut inv)?;
    Ok(inv)
}

/// Bulk-buy `count` rolls of (slot, tier). `count == 0` ≡
/// "buy as many as the wallet allows", capped at 100 per call
/// so a single click can't dump the entire treasury into one
/// stash bucket.
pub fn bulk_buy_gear_roll(
    ctx: &mut DelegateCtx,
    slot: u8,
    tier: u8,
    count: u32,
    now_ms: u64,
) -> Result<Inventory, String> {
    const MAX_PER_CALL: u32 = 100;
    let mut inv = load_inventory_raw(ctx);
    enter_action(&mut inv, now_ms)?;
    let price = shop_buy_price(tier);
    if price == u64::MAX {
        return Err(format!(
            "tier {tier} is not buyable (legendaries only drop or forge)"
        ));
    }
    let max_affordable = (inv.gold / price).min(MAX_PER_CALL as u64) as u32;
    let want = if count == 0 { max_affordable } else { count.min(max_affordable) };
    if want == 0 {
        return Err(format!("need {price}g for tier {tier} {}", slot_label(slot)));
    }
    for _ in 0..want {
        let Some(cid) = shop_roll_catalog_id(slot, tier, inv.shop_purchase_count) else {
            break;
        };
        inv.gold = inv.gold.saturating_sub(price);
        inv.unequipped.push(cid);
        inv.shop_purchase_count = inv.shop_purchase_count.saturating_add(1);
    }
    check_achievements(&mut inv, now_ms);
    save_inventory(ctx, &mut inv)?;
    Ok(inv)
}

fn slot_label(slot: u8) -> &'static str {
    shared::SLOT_NAMES
        .get(slot as usize)
        .copied()
        .unwrap_or("?")
}

pub fn auto_equip_best(ctx: &mut DelegateCtx, now_ms: u64) -> Result<Inventory, String> {
    let mut inv = load_inventory_raw(ctx);
    enter_action(&mut inv, now_ms)?;
    let mask = shared::form_slot_mask(inv.current_form);
    for slot_idx in 0..SLOT_COUNT {
        if !mask[slot_idx] {
            continue;
        }
        let best_unequipped = inv
            .unequipped
            .iter()
            .copied()
            .filter_map(|cid| {
                let t = gear_template(cid)?;
                if t.slot as usize != slot_idx {
                    return None;
                }
                let score = (t.atk + t.def + t.hp) as u64;
                Some((score, cid))
            })
            .max_by_key(|(s, _)| *s);

        let Some((best_score, best_cid)) = best_unequipped else {
            continue;
        };
        let current_score = inv.equipped[slot_idx]
            .and_then(|cid| gear_template(cid))
            .map(|t| (t.atk + t.def + t.hp) as u64)
            .unwrap_or(0);

        if best_score > current_score {
            if let Some(pos) = inv.unequipped.iter().position(|c| *c == best_cid) {
                inv.unequipped.swap_remove(pos);
            }
            if let Some(prev) = inv.equipped[slot_idx].take() {
                inv.unequipped.push(prev);
            }
            inv.equipped[slot_idx] = Some(best_cid);
        }
    }
    inv.current_hp = inv.current_hp.min(max_hp_of(&inv));
    check_achievements(&mut inv, now_ms);
    save_inventory(ctx, &mut inv)?;
    Ok(inv)
}
