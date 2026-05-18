//! Equipment & stash widgets — one equipped slot, the grouped
//! stash listing, per-row stash rendering, and the gear-stat blurb.

use shared::{
    forge_essence_cost, form_slot_mask, gear_sell_price, gear_template, GearTemplate, Inventory,
    FORGE_COUNT, SLOT_COUNT, TIER_COUNT,
};
use yew::prelude::*;

use crate::app::i18n::{Locale, MessageId};
use crate::app::i18n_shared;

/// One equipment slot. Shows slot label + the equipped piece's name
/// + stat line, with an "x" to send it back to the stash. If the
/// slot isn't allowed for the player's current form (e.g. wearing
/// pants as a slime), the cell is greyed and shows "n/a"; equipping
/// is refused by the delegate anyway.
pub fn render_equipped_slot<F>(
    locale: Locale,
    slot_idx: usize,
    inv: &Inventory,
    mk_unequip: &F,
) -> Html
where
    F: Fn(u8) -> Callback<MouseEvent>,
{
    let slot_u8 = slot_idx as u8;
    let slot_name = i18n_shared::slot_name(locale, slot_idx);
    let allowed = form_slot_mask(inv.current_form)[slot_idx];
    let equipped = inv.equipped[slot_idx];
    // Capture the equipped tier so we can paint the slot with the
    // matching `--tier-N` accent. Stash rows already do this via
    // `tier-{N}` class — mirror the same scheme on the hero panel
    // so quality reads at a glance.
    let mut equipped_tier: Option<u8> = None;
    let (value_text, stat_text, action) = match equipped {
        Some(cid) => match gear_template(cid) {
            Some(t) => {
                equipped_tier = Some(t.tier);
                (
                    i18n_shared::gear_name(locale, &t),
                    stat_blurb(&t),
                    Some(html! {
                        <button
                            class="slot-action"
                            title={locale.tr(MessageId::TipUnequipSlot)}
                            onclick={mk_unequip(slot_u8)}
                        >
                            { "✕" }
                        </button>
                    }),
                )
            }
            None => (locale.tr(MessageId::TermCorrupt).into(), "—".into(), None),
        },
        None if !allowed => (
            locale.tr(MessageId::TermFormNa).into(),
            locale.tr(MessageId::TermFormLocks).into(),
            None,
        ),
        None => (locale.tr(MessageId::TermEmpty).into(), "—".into(), None),
    };
    let cls = match (equipped.is_some(), allowed) {
        (true, _) => {
            let tier = equipped_tier.unwrap_or(0);
            format!("slot filled tier-{}", tier)
        }
        (false, false) => "slot disabled".to_string(),
        (false, true) => "slot".to_string(),
    };
    html! {
        <div class={cls}>
            <span class="slot-name">{ slot_name }</span>
            <span class="slot-value">{ value_text }</span>
            <span class="slot-stats muted small">{ stat_text }</span>
            { for action.into_iter() }
        </div>
    }
}

/// Render the unequipped stash grouped by slot category. Each group
/// header shows the slot name and item count; each row shows tier,
/// stats, and per-row equip + sell buttons. Empty groups are
/// skipped so the listing stays compact.
///
/// Lives on the Shop tab — that's the inventory-management hub: you
/// look at the stash, equip what you want, forge duplicates into
/// the next tier, dump the rest for gold.
pub fn render_stash_grouped<E, S, SA, F>(
    locale: Locale,
    inv: &Inventory,
    filter_slot: u8,
    sort_mode: u8,
    mk_equip: &E,
    mk_sell: &S,
    mk_sell_all: &SA,
    mk_forge: &F,
) -> Html
where
    E: Fn(u16) -> Callback<MouseEvent>,
    S: Fn(u16) -> Callback<MouseEvent>,
    SA: Fn(u16) -> Callback<MouseEvent>,
    F: Fn(u16) -> Callback<MouseEvent>,
{
    if inv.unequipped.is_empty() {
        return html! {
            <p class="muted small">
                { format!("no spare loot — gear drops every {} missions", shared::GEAR_DROP_EVERY) }
            </p>
        };
    }
    let mut by_slot: Vec<Vec<u16>> = (0..SLOT_COUNT).map(|_| Vec::new()).collect();
    for cid in &inv.unequipped {
        if let Some(t) = gear_template(*cid) {
            // Apply user-set slot filter (§8 B1). `STASH_FILTER_NONE`
            // (0xFF) is the "show every slot" sentinel; any other
            // value is a slot index — items in other slots are
            // dropped before grouping.
            if filter_slot != crate::app::prefs::STASH_FILTER_NONE
                && t.slot != filter_slot
            {
                continue;
            }
            by_slot[t.slot as usize].push(*cid);
        }
    }
    // Pre-count duplicates per catalog_id so the forge button can
    // show "have 4 of 3 needed" without re-scanning per row.
    let mut counts_by_id: std::collections::BTreeMap<u16, usize> =
        std::collections::BTreeMap::new();
    for cid in &inv.unequipped {
        *counts_by_id.entry(*cid).or_insert(0) += 1;
    }

    // If filter dropped everything, give the player a visible cue
    // instead of an empty white space — the toolbar above the
    // stash has the "Show all slots" reset.
    let all_empty = by_slot.iter().all(|v| v.is_empty());
    if all_empty {
        return html! {
            <p class="muted small">
                { locale.tr_key("stash.filter_empty") }
            </p>
        };
    }

    html! {
        <div class="stash-grouped">
            { for (0..SLOT_COUNT).filter_map(|slot_idx| {
                let items = &by_slot[slot_idx];
                if items.is_empty() {
                    return None;
                }
                // Distinct catalog ids in this slot, in stable order.
                let mut seen: std::collections::BTreeSet<u16> = std::collections::BTreeSet::new();
                let mut distinct: Vec<u16> = items.iter().filter_map(|c| {
                    if seen.insert(*c) { Some(*c) } else { None }
                }).collect();
                // Apply user-set sort (§8 B1) before render:
                //   * mode 0 = catalog order (legacy, BTreeSet iteration)
                //   * mode 1 = tier descending
                //   * mode 2 = score descending (atk + def + hp)
                match sort_mode {
                    1 => distinct.sort_by(|a, b| {
                        let ta = gear_template(*a).map(|t| t.tier).unwrap_or(0);
                        let tb = gear_template(*b).map(|t| t.tier).unwrap_or(0);
                        tb.cmp(&ta)
                    }),
                    2 => distinct.sort_by(|a, b| {
                        let sa = gear_template(*a).map(|t| t.atk + t.def + t.hp).unwrap_or(0);
                        let sb = gear_template(*b).map(|t| t.atk + t.def + t.hp).unwrap_or(0);
                        sb.cmp(&sa)
                    }),
                    _ => {}
                }
                Some(html! {
                    <div class="stash-group">
                        <h4 class="stash-group-name">
                            { format!("{} ({})", i18n_shared::slot_name(locale, slot_idx), items.len()) }
                        </h4>
                        <div class="stash-items">
                            { for distinct.iter().map(|cid| {
                                let count = *counts_by_id.get(cid).unwrap_or(&0);
                                render_stash_row(
                                    locale, *cid, count, inv.essence,
                                    inv.equipped[slot_idx],
                                    mk_equip, mk_sell, mk_sell_all, mk_forge,
                                )
                            }) }
                        </div>
                    </div>
                })
            })}
        </div>
    }
}

/// One row in the grouped stash listing — name, tier badge, stat
/// blurb, count badge, and per-row equip / sell / forge buttons.
/// Forge button only renders when ≥ FORGE_COUNT copies are owned
/// AND the item isn't already at the max tier; greyed out if
/// essence is insufficient.
pub fn render_stash_row<E, S, SA, F>(
    locale: Locale,
    catalog_id: u16,
    owned_count: usize,
    essence: u64,
    equipped_in_slot: Option<u16>,
    mk_equip: &E,
    mk_sell: &S,
    mk_sell_all: &SA,
    mk_forge: &F,
) -> Html
where
    E: Fn(u16) -> Callback<MouseEvent>,
    S: Fn(u16) -> Callback<MouseEvent>,
    SA: Fn(u16) -> Callback<MouseEvent>,
    F: Fn(u16) -> Callback<MouseEvent>,
{
    let Some(t) = gear_template(catalog_id) else {
        return html! { <div class="stash-item"><span class="muted">{format!("(unknown {catalog_id})") }</span></div> };
    };
    let sell_price = gear_sell_price(t.tier);
    let forge_available = t.tier < TIER_COUNT;
    let forge_cost = forge_essence_cost(t.tier);
    let forge_enough_copies = owned_count >= FORGE_COUNT;
    let forge_enough_essence = essence >= forge_cost;
    let forge_disabled = !forge_enough_copies || !forge_enough_essence;
    let count_text = if owned_count == 1 {
        String::new()
    } else {
        format!("×{}", owned_count)
    };
    // §P3 compare-tool: hover-tooltip showing delta vs equipped.
    // Empty slot or unknown gear → no tooltip. Equip-or-sell call
    // sites already exist; this just informs the choice.
    let compare_tooltip = match equipped_in_slot.and_then(gear_template) {
        Some(eq) => {
            let d_atk = (t.atk as i64) - (eq.atk as i64);
            let d_def = (t.def as i64) - (eq.def as i64);
            let d_hp = (t.hp as i64) - (eq.hp as i64);
            let fmt = |v: i64| if v > 0 { format!("+{v}") } else { v.to_string() };
            format!("vs equipped: atk {} · def {} · hp {}",
                    fmt(d_atk), fmt(d_def), fmt(d_hp))
        }
        None => "no piece equipped in this slot".to_string(),
    };
    html! {
        <div class={format!("stash-item tier-{}", t.tier)} title={compare_tooltip}>
            <span class="stash-name">
                { i18n_shared::gear_name(locale, &t) }
                { if count_text.is_empty() { html!{} } else { html!{<span class="stash-count">{count_text}</span>} } }
            </span>
            <span class="stash-tier">{ format!("T{}", t.tier) }</span>
            <span class="stash-stats muted small">{ stat_blurb(&t) }</span>
            <div class="stash-actions">
                <button class="stash-equip" onclick={mk_equip(catalog_id)}>{ locale.tr(MessageId::BtnEquip) }</button>
                <button class="stash-sell" onclick={mk_sell(catalog_id)}
                        title={locale.tr_key("stash.sell_one_tooltip")
                            .replace("{gold}", &sell_price.to_string())}>
                    { locale.tr_key("stash.sell_one")
                        .replace("{gold}", &sell_price.to_string()) }
                </button>
                {
                    // Bulk-sell only renders for multi-copy rows so
                    // the single-item card stays compact.
                    if owned_count > 1 {
                        let total = sell_price.saturating_mul(owned_count as u64);
                        html! {
                            <button class="stash-sell" onclick={mk_sell_all(catalog_id)}
                                    title={locale.tr_key("stash.sell_all_tooltip")
                                        .replace("{count}", &owned_count.to_string())
                                        .replace("{gold}", &total.to_string())}>
                                { locale.tr_key("stash.sell_all")
                                    .replace("{count}", &owned_count.to_string())
                                    .replace("{gold}", &total.to_string()) }
                            </button>
                        }
                    } else { html! {} }
                }
                {
                    if forge_available {
                        let title = if !forge_enough_copies {
                            locale.tr_key("stash.forge_need_copies")
                                .replace("{n}", &FORGE_COUNT.to_string())
                                .replace("{have}", &owned_count.to_string())
                        } else if !forge_enough_essence {
                            locale.tr_key("stash.forge_need_essence")
                                .replace("{cost}", &forge_cost.to_string())
                                .replace("{have}", &essence.to_string())
                        } else {
                            locale.tr_key("stash.forge_ready_tooltip")
                                .replace("{n}", &FORGE_COUNT.to_string())
                                .replace("{cost}", &forge_cost.to_string())
                                .replace("{tier}", &(t.tier + 1).to_string())
                        };
                        html! {
                            <button class="stash-forge" disabled={forge_disabled}
                                    onclick={mk_forge(catalog_id)} title={title}>
                                { locale.tr_key("stash.forge")
                                    .replace("{cost}", &forge_cost.to_string()) }
                            </button>
                        }
                    } else {
                        html! { <span class="stash-forge-na muted small">{ locale.tr(MessageId::TermMaxTier) }</span> }
                    }
                }
            </div>
        </div>
    }
}

/// Human-readable stat line for a gear piece — only non-zero stats,
/// so a pure-defence Helm doesn't waste pixels on "+0 atk".
pub fn stat_blurb(t: &GearTemplate) -> String {
    let mut parts: Vec<String> = Vec::new();
    if t.atk > 0 { parts.push(format!("+{} atk", t.atk)); }
    if t.def > 0 { parts.push(format!("+{} def", t.def)); }
    if t.hp > 0 { parts.push(format!("+{} hp", t.hp)); }
    if parts.is_empty() {
        "—".into()
    } else {
        parts.join(" · ")
    }
}
