//! Achievements tab — endings list, skill milestones + forms
//! visited, achievement chips with hover criterion, plus the
//! global World Boss bar and leaderboard.

use shared::{
    form_base_bonuses, form_speed_evasion, form_sprite, format_si, Inventory,
    PresencePayload, PubKey, ACHIEVEMENT_TABLE, ENDINGS_TOTAL, ENDING_DRAGON_LORD,
    ENDING_PILGRIM, ENDING_QUIET_FARMER, ENDING_VICTORY,
};
use yew::prelude::*;

use crate::app::i18n::{Locale, MessageId};
use crate::app::i18n_shared;
use crate::app::widgets::row_view;

pub fn render_achievements_tab(
    locale: Locale,
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
                <h2>{ locale.fmt_count_of(locale.tr(MessageId::PanelEndings), inv.ending_unlocks.len(), ENDINGS_TOTAL) }</h2>
                <p class="muted small">
                    { match locale {
                        Locale::Ru => "Финальные вехи. Открытие одной не прекращает прохождение — продолжай играть после любой. Достижимы в любом порядке.",
                        _ => "Terminal-state milestones. Unlocking one doesn't end your run — keep playing past every one. Mutually reachable in any order.",
                    } }
                </p>
                {
                    if inv.ending_unlocks.is_empty() {
                        html! { <p class="muted">{ match locale {
                            Locale::Ru => "финалов пока нет — самый простой Пилигрим: посети все 5 форм",
                            _ => "no endings unlocked yet — Pilgrim is the easiest: visit all 5 forms",
                        } }</p> }
                    } else {
                        html! {
                            <ul class="ending-list">
                                { for [ENDING_VICTORY, ENDING_DRAGON_LORD, ENDING_PILGRIM, ENDING_QUIET_FARMER].iter().filter_map(|eid| {
                                    inv.ending_unlocks.get(eid).map(|_| html! {
                                        <li class="ending-row">
                                            <span class="ending-name">{ i18n_shared::ending_name(locale, *eid) }</span>
                                            <span class="ending-blurb muted small">{ i18n_shared::ending_blurb(locale, *eid) }</span>
                                        </li>
                                    })
                                }) }
                            </ul>
                        }
                    }
                }
            </section>

            <section class="panel skills">
                <h2>{ format!("{} · {}",
                    locale.fmt_count_of(locale.tr(MessageId::PanelSkillsLine), inv.skills_unlocked.len(), 6),
                    locale.fmt_count_of(locale.tr(MessageId::PanelFormsVisited), inv.forms_visited.len(), 5),
                ) }</h2>
                <p class="muted small">
                    { match locale {
                        Locale::Ru => "Навыки — постоянные пассивные бонусы. Каждая принятая форма оставляет свой след — он не сбрасывается при возврате. Уровни 10 и 20 открывают вехи ветерана.",
                        _ => "Skills are permanent passive bonuses. Each form you've taken leaves a mark on you — they don't reset when you change back. Level 10 and 20 unlock veteran milestones.",
                    } }
                </p>
                {
                    if inv.skills_unlocked.is_empty() {
                        html! { <p class="muted">{ match locale {
                            Locale::Ru => "пока без навыков — проиграй не-Человеку, чтобы выучить первый",
                            _ => "no skills yet — lose to a non-Human enemy to learn one",
                        } }</p> }
                    } else {
                        html! {
                            <ul class="skill-list">
                                { for inv.skills_unlocked.keys().map(|id| html! {
                                    <li class="skill-row">
                                        <span class="skill-name">{ i18n_shared::skill_name(locale, *id) }</span>
                                        <span class="skill-blurb muted small">{ i18n_shared::skill_blurb(locale, *id) }</span>
                                    </li>
                                }) }
                            </ul>
                        }
                    }
                }
                <h3>{ locale.tr(MessageId::PanelFormsVisited) }</h3>
                <div class="badges">
                    { for inv.forms_visited.keys().map(|f| {
                        // Hover tooltip = the form's stat bundle so
                        // the player can scan the Achievements tab
                        // for "which form gives me what". Mirrors
                        // the live computation used in combat
                        // (form_base_bonuses + form_speed_evasion).
                        let (atk, def, hp) = form_base_bonuses(*f);
                        let (speed, eva) = form_speed_evasion(*f);
                        let mut parts: Vec<String> = Vec::new();
                        if atk > 0 { parts.push(format!("+{atk} atk")); }
                        if def > 0 { parts.push(format!("+{def} def")); }
                        if hp > 0 { parts.push(format!("+{hp} hp")); }
                        if speed != 100 { parts.push(format!("speed {speed}")); }
                        if eva > 0 { parts.push(format!("+{eva}% eva")); }
                        let tooltip = if parts.is_empty() {
                            i18n_shared::form_name(locale, *f).to_string()
                        } else {
                            format!("{} — {}",
                                i18n_shared::form_name(locale, *f),
                                parts.join(", "))
                        };
                        html! {
                            <span class="achievement" title={tooltip}>
                                { format!("{} {}", form_sprite(*f), i18n_shared::form_name(locale, *f)) }
                            </span>
                        }
                    }) }
                </div>
            </section>

            <section class="panel achievements">
                <h2>{ locale.fmt_count_of(locale.tr(MessageId::PanelAchievementsLow), inv.achievement_unlocks.len(), ACHIEVEMENT_TABLE.len()) }</h2>
                {
                    if inv.achievement_unlocks.is_empty() {
                        html! { <p class="muted">{ match locale {
                            Locale::Ru => "значков ещё нет — запусти миссию",
                            _ => "no badges yet — run a mission to start",
                        } }</p> }
                    } else {
                        html! {
                            <div class="badges">
                                { for ACHIEVEMENT_TABLE.iter().filter_map(|(id, _)| {
                                    inv.achievement_unlocks.get(id).map(|ts| {
                                        let age = now.saturating_sub(*ts);
                                        let age_str = match locale {
                                            Locale::Ru => {
                                                if age < 60_000 {
                                                    format!("{} с назад", age / 1000)
                                                } else if age < 3_600_000 {
                                                    format!("{} мин назад", age / 60_000)
                                                } else {
                                                    format!("{} ч назад", age / 3_600_000)
                                                }
                                            }
                                            _ => {
                                                if age < 60_000 {
                                                    format!("{}s ago", age / 1000)
                                                } else if age < 3_600_000 {
                                                    format!("{}m ago", age / 60_000)
                                                } else {
                                                    format!("{}h ago", age / 3_600_000)
                                                }
                                            }
                                        };
                                        // Tooltip = unlock criterion + when. Hover
                                        // shows what you did to get it.
                                        let unlocked_prefix = match locale {
                                            Locale::Ru => "Открыто",
                                            _ => "Unlocked",
                                        };
                                        let tooltip = format!(
                                            "{}\n{unlocked_prefix} {age_str}",
                                            i18n_shared::achievement_reason(locale, *id)
                                        );
                                        html! {
                                            <span class="achievement" title={tooltip}>
                                                { i18n_shared::achievement_label(locale, *id) }
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
                <h2>{ locale.tr(MessageId::PanelWorldBoss) }</h2>
                <div class="hp-bar">
                    <div class="hp-fill" style={format!("width: {boss_pct}%")}></div>
                </div>
                <p class="muted">
                    { locale.fmt_boss_summary(
                        boss_era,
                        &format_si(boss_hp),
                        &format_si(boss_max_hp),
                        &format_si(total_dmg),
                        rows.len(),
                    ) }
                </p>
            </section>
            <section class="panel leaderboard">
                <h2>{ locale.fmt_active_players(rows.len()) }</h2>
                <table>
                    <thead>
                        <tr>
                            <th>{"#"}</th>
                            <th>{ locale.tr(MessageId::ColName) }</th>
                            <th class="num">{ locale.tr(MessageId::ResGold) }</th>
                            <th class="num">{ locale.tr(MessageId::ColDamage) }</th>
                            <th>{ locale.tr(MessageId::ColArea) }</th>
                            <th>{ locale.tr(MessageId::ColSeen) }</th>
                            <th></th>
                        </tr>
                    </thead>
                    <tbody>
                        { for rows.iter().enumerate().map(|(i, (pk, p, recv_ms, is_me))| row_view(locale, i, pk, p, *recv_ms, *is_me, now)) }
                    </tbody>
                </table>
            </section>
        </>
    }
}
