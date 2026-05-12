//! World-map area card. Three visual states (`current`,
//! `unlocked`, `locked`) drive the disabled/highlight semantics.

use shared::{AreaDef, MISSION_DAMAGE, MISSION_ESSENCE, MISSION_GOLD};
use yew::prelude::*;

/// Render one area as a clickable card. Three visual states:
///   * `current`  — the active area, button is disabled (no-op
///     click) and gets a highlighted border.
///   * `unlocked` — clickable, shows the payout breakdown.
///   * `locked`   — disabled, shows the level requirement.
///
/// `mk_cb` is the closure factory that turns an `area_id` into a
/// Yew `Callback`. The factory is owned by `render_core`'s scope and
/// borrowed here so each card gets a freshly-baked callback.
pub fn render_area_card<F>(area: &'static AreaDef, current: u8, lvl: u64, mk_cb: &F) -> Html
where
    F: Fn(u8) -> Callback<MouseEvent>,
{
    let is_current = area.id == current;
    let unlocked = lvl >= area.min_level;
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
        html! { <span class="area-tag current-tag">{ "active" }</span> }
    } else if !unlocked {
        html! { <span class="area-tag lock-tag">{ format!("lvl {} required", area.min_level) }</span> }
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
        html! {
            <span class="area-rewards">
                <span title="gold per mission">{ format!("{gold}g") }</span>
                <span title="essence per mission">{ format!("{ess}e") }</span>
                { dmg_badge }
            </span>
        }
    };

    html! {
        <button class={classes.join(" ")} disabled={disabled} onclick={cb}>
            <span class="area-name">{ area.name }</span>
            <span class="area-blurb muted">{ area.blurb }</span>
            { footer }
        </button>
    }
}
