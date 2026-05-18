//! World-map area card. Three visual states (`current`,
//! `unlocked`, `locked`) drive the disabled/highlight semantics.

use shared::{
    area_predecessor_progress, enemy_def, enemy_roster_for_area, scale_by_area_level, AreaDef,
    Inventory, ENCOUNTERS_PER_MISSION, MISSION_DAMAGE, MISSION_ESSENCE, MISSION_GOLD,
};
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
    area: &AreaDef,
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
    // Graph gate (C3): satisfied if *any* predecessor has cleared
    // enough. Show progress on the best one so the badge reads
    // toward the route the player is closest to opening.
    let (clears_have, clears_need) = area_predecessor_progress(area, |id| inv.area_clears_of(id))
        .unwrap_or((0, 0));
    let clears_ok = clears_need == 0 || clears_have >= clears_need;
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
        // Estimated XP/mission: roster's average per-encounter
        // reward scaled by area level, times the mission's
        // encounter count. Rough — actual swing depends on which
        // roster entries roll up — but accurate enough to give the
        // player a meaningful "is this area worth grinding"
        // signal vs the prior step.
        let xp_estimate = {
            let roster = enemy_roster_for_area(area.id);
            if roster.is_empty() {
                0
            } else {
                let total: u64 = roster
                    .iter()
                    .filter_map(|id| enemy_def(*id))
                    .map(|e| scale_by_area_level(e.xp_reward, area.min_level))
                    .sum();
                let avg = total / roster.len() as u64;
                avg.saturating_mul(ENCOUNTERS_PER_MISSION as u64)
            }
        };
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
                <span title="estimated XP per mission (avg of roster, scaled by area level)">
                    { format!("{xp_estimate}xp") }
                </span>
                { dmg_badge }
                <span class="muted" title="encounter clears here">
                    { locale.fmt_cleared_count(own_clears) }
                </span>
            </span>
        }
    };

    html! {
        <button class={classes.join(" ")} disabled={disabled} onclick={cb}>
            <span class="area-name">
                { area_name(locale, area) }
                <span class="area-level-badge" title="recommended hero level">
                    { format!("lv {}", area.min_level) }
                </span>
            </span>
            <span class="area-blurb muted">{ area_blurb(locale, area) }</span>
            { footer }
        </button>
    }
}
