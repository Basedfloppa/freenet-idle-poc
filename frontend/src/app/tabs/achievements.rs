//! Achievements tab — endings list, skill milestones + forms
//! visited, achievement chips with hover criterion, plus the
//! global World Boss bar and leaderboard.

use shared::{
    ending_blurb, ending_name, form_name, form_sprite, format_si, skill_blurb, skill_name,
    Inventory, PresencePayload, PubKey, ACHIEVEMENT_TABLE, ENDINGS_TOTAL, ENDING_DRAGON_LORD,
    ENDING_PILGRIM, ENDING_QUIET_FARMER, ENDING_VICTORY,
};
use yew::prelude::*;

use crate::app::widgets::row_view;

pub fn render_achievements_tab(
    inv: &Inventory,
    now: u64,
    boss_era: u64,
    boss_hp: u64,
    boss_max_hp: u64,
    boss_pct: u64,
    total_dmg: u64,
    rows: &[(PubKey, PresencePayload, u64, bool)],
) -> Html {
    html! {
        <>
            <section class="panel endings">
                <h2>{ format!("endings ({}/{})", inv.ending_unlocks.len(), ENDINGS_TOTAL) }</h2>
                <p class="muted small">
                    { "Terminal-state milestones. Unlocking one doesn't end your run — keep playing past every one. Mutually reachable in any order." }
                </p>
                {
                    if inv.ending_unlocks.is_empty() {
                        html! { <p class="muted">{ "no endings unlocked yet — Pilgrim is the easiest: visit all 5 forms" }</p> }
                    } else {
                        html! {
                            <ul class="ending-list">
                                { for [ENDING_VICTORY, ENDING_DRAGON_LORD, ENDING_PILGRIM, ENDING_QUIET_FARMER].iter().filter_map(|eid| {
                                    inv.ending_unlocks.get(eid).map(|_| html! {
                                        <li class="ending-row">
                                            <span class="ending-name">{ ending_name(*eid) }</span>
                                            <span class="ending-blurb muted small">{ ending_blurb(*eid) }</span>
                                        </li>
                                    })
                                }) }
                            </ul>
                        }
                    }
                }
            </section>

            <section class="panel skills">
                <h2>{ format!("skills ({}/6) · forms visited ({}/5)", inv.skills_unlocked.len(), inv.forms_visited.len()) }</h2>
                <p class="muted small">
                    { "Skills are permanent passive bonuses. Each form you've taken leaves a mark on you — they don't reset when you change back. Level 10 and 20 unlock veteran milestones." }
                </p>
                {
                    if inv.skills_unlocked.is_empty() {
                        html! { <p class="muted">{ "no skills yet — lose to a non-Human enemy to learn one" }</p> }
                    } else {
                        html! {
                            <ul class="skill-list">
                                { for inv.skills_unlocked.keys().map(|id| html! {
                                    <li class="skill-row">
                                        <span class="skill-name">{ skill_name(*id) }</span>
                                        <span class="skill-blurb muted small">{ skill_blurb(*id) }</span>
                                    </li>
                                }) }
                            </ul>
                        }
                    }
                }
                <h3>{ "forms visited" }</h3>
                <div class="badges">
                    { for inv.forms_visited.keys().map(|f| html! {
                        <span class="achievement">{ format!("{} {}", form_sprite(*f), form_name(*f)) }</span>
                    }) }
                </div>
            </section>

            <section class="panel achievements">
                <h2>{ format!("achievements ({}/{})", inv.achievement_unlocks.len(), ACHIEVEMENT_TABLE.len()) }</h2>
                {
                    if inv.achievement_unlocks.is_empty() {
                        html! { <p class="muted">{ "no badges yet — run a mission to start" }</p> }
                    } else {
                        html! {
                            <div class="badges">
                                { for ACHIEVEMENT_TABLE.iter().filter_map(|(id, _)| {
                                    inv.achievement_unlocks.get(id).map(|ts| {
                                        let age = now.saturating_sub(*ts);
                                        let age_str = if age < 60_000 {
                                            format!("{}s ago", age / 1000)
                                        } else if age < 3_600_000 {
                                            format!("{}m ago", age / 60_000)
                                        } else {
                                            format!("{}h ago", age / 3_600_000)
                                        };
                                        // Tooltip = unlock criterion + when. Hover
                                        // shows what you did to get it.
                                        let tooltip = format!(
                                            "{}\nUnlocked {age_str}",
                                            shared::achievement_reason(*id)
                                        );
                                        html! {
                                            <span class="achievement" title={tooltip}>
                                                { shared::achievement_label(*id) }
                                            </span>
                                        }
                                    })
                                })}
                            </div>
                        }
                    }
                }
            </section>
            <section class="panel boss">
                <h2>{ "World Boss" }</h2>
                <div class="hp-bar">
                    <div class="hp-fill" style={format!("width: {boss_pct}%")}></div>
                </div>
                <p class="muted">
                    { format!("Era {boss_era} · {} / {} HP — {} total damage from {} players",
                              format_si(boss_hp), format_si(boss_max_hp), format_si(total_dmg), rows.len()) }
                </p>
            </section>
            <section class="panel leaderboard">
                <h2>{ format!("active players ({})", rows.len()) }</h2>
                <table>
                    <thead>
                        <tr>
                            <th>{"#"}</th>
                            <th>{"name"}</th>
                            <th class="num">{"gold"}</th>
                            <th class="num">{"damage"}</th>
                            <th>{"area"}</th>
                            <th>{"seen"}</th>
                            <th></th>
                        </tr>
                    </thead>
                    <tbody>
                        { for rows.iter().enumerate().map(|(i, (pk, p, recv_ms, is_me))| row_view(i, pk, p, *recv_ms, *is_me, now)) }
                    </tbody>
                </table>
            </section>
        </>
    }
}
