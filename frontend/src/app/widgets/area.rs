//! World-map area card. Three visual states (`current`,
//! `unlocked`, `locked`) drive the disabled/highlight semantics.

use shared::{area_predecessor, AreaDef, Inventory, MISSION_DAMAGE, MISSION_ESSENCE, MISSION_GOLD};
use yew::prelude::*;

use crate::app::i18n::{Locale, MessageId};
use crate::app::i18n_shared::{area_blurb, area_name};

/// Render one area as a clickable card. Three visual states:
///   * `current`  — the active area, button is disabled (no-op
///     click) and gets a highlighted border.
///   * `unlocked` — clickable, shows the payout breakdown.
///   * `locked`   — disabled, shows the level requirement.
///
/// An area is unlocked iff the player's level meets `min_level`
/// AND the predecessor area has been cleared at least
/// `clears_required` times (A3 backlog item).
///
/// `mk_cb` is the closure factory that turns an `area_id` into a
/// Yew `Callback`. The factory is owned by `render_core`'s scope and
/// borrowed here so each card gets a freshly-baked callback.
pub fn render_area_card<F>(
    locale: Locale,
    area: &'static AreaDef,
    current: u8,
    lvl: u64,
    inv: &Inventory,
    mk_cb: &F,
) -> Html
where
    F: Fn(u8) -> Callback<MouseEvent>,
{
    let is_current = area.id == current;
    let level_ok = lvl >= area.min_level;
    let (clears_have, clears_need) = match area_predecessor(area.id) {
        Some(prev_id) => (inv.area_clears_of(prev_id), area.clears_required),
        None => (0, 0),
    };
    let clears_ok = clears_have >= clears_need;
    let unlocked = level_ok && clears_ok;
    let mut classes = vec!["area-card"];
    if is_current {
        classes.push("current");
    } else if unlocked {
        classes.push("unlocked");
    } else {
        classes.push("locked");
    }
    let disabled = is_current || !unlocked;
    let cb = mk_cb(area.id);

    let footer = if is_current {
        html! { <span class="area-tag current-tag">{ locale.tr(MessageId::TermActive) }</span> }
    } else if !unlocked {
        // Show whichever gate the player is missing. If both fail,
        // show the level gate first (it's the longer-term blocker
        // — clears can be earned in one session, levels usually take
        // longer).
        if !level_ok {
            html! { <span class="area-tag lock-tag">{ locale.fmt_lvl_required(area.min_level) }</span> }
        } else {
            html! { <span class="area-tag lock-tag">{ locale.fmt_clears_required(clears_have, clears_need) }</span> }
        }
    } else {
        let gold = MISSION_GOLD.saturating_mul(area.gold_mult);
        let ess = MISSION_ESSENCE.saturating_mul(area.essence_mult);
        let dmg = MISSION_DAMAGE.saturating_mul(area.damage_mult);
        let dmg_badge = if dmg > 0 {
            html! { <span title="boss damage per mission">{ format!("{dmg}d") }</span> }
        } else {
            // Areas 0-2 deal no boss damage — keep the badge slot
            // populated with a hint so the grid stays aligned.
            html! { <span class="muted" title="this area doesn't chip the World Boss">{ "—" }</span> }
        };
        // Show this area's clear-count next to rewards. Renders as
        // `cleared 13` for the player's mastery cue.
        let own_clears = inv.area_clears_of(area.id);
        html! {
            <span class="area-rewards">
                <span title="gold per mission">{ format!("{gold}g") }</span>
                <span title="essence per mission">{ format!("{ess}e") }</span>
                { dmg_badge }
                <span class="muted" title="encounter clears here">
                    { locale.fmt_cleared_count(own_clears) }
                </span>
            </span>
        }
    };

    html! {
        <button class={classes.join(" ")} disabled={disabled} onclick={cb}>
            <span class="area-name">{ area_name(locale, area) }</span>
            <span class="area-blurb muted">{ area_blurb(locale, area) }</span>
            { footer }
        </button>
    }
}
