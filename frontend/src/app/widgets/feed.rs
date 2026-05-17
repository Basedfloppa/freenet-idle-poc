//! Read-only feed widgets — combat-history scrollback, the
//! per-leaderboard row, and the mailbox inbox panel.

use shared::{
    enemy_def, format_si, EncounterLog, PresencePayload, PubKey, COMBAT_OUTCOME_WIN,
};
use yew::prelude::*;

use crate::app::core::Core;
use crate::app::i18n::{Locale, MessageId};
use crate::app::i18n_shared;
use crate::app::util::short_id;

pub fn render_combat_history(locale: Locale, history: &[EncounterLog]) -> Html {
    if history.is_empty() {
        return html! {
            <p class="muted small">{ locale.tr(MessageId::BattleNoEncounters) }</p>
        };
    }
    // Show up to 8 latest (newest first).
    let n = history.len();
    let take = n.min(8);
    let rows = history[n - take..]
        .iter()
        .rev()
        .map(|e| {
            let won = e.outcome == COMBAT_OUTCOME_WIN;
            let cls = if won { "encounter win" } else { "encounter loss" };
            let verdict = if won {
                locale.tr(MessageId::TermWin)
            } else {
                locale.tr(MessageId::TermDefeat)
            };
            let enemy_name = enemy_def(e.enemy_id)
                .map(|d| i18n_shared::enemy_name(locale, d))
                .unwrap_or_else(|| locale.tr_key("term.unknown_fallback"));
            let detail = if won {
                format!(
                    "+{}g · turn {} · dealt {} · taken {} · hp {} → {}",
                    e.gold_gained, e.turns, e.dmg_dealt, e.dmg_taken,
                    e.player_hp_start, e.player_hp_end,
                )
            } else {
                let blurb = enemy_def(e.enemy_id)
                    .map(|d| i18n_shared::enemy_death_blurb(locale, d))
                    .unwrap_or("…");
                format!("dealt {} · taken {} · {}", e.dmg_dealt, e.dmg_taken, blurb)
            };
            html! {
                <div class={cls}>
                    <span class="encounter-verdict">{ verdict }</span>
                    <span class="encounter-enemy">{ enemy_name }</span>
                    <span class="encounter-detail muted small">{ detail }</span>
                </div>
            }
        });
    html! {
        <div class="combat-history">
            { for rows }
        </div>
    }
}

pub fn row_view(
    locale: Locale,
    rank: usize,
    pk: &PubKey,
    p: &PresencePayload,
    received_ms: u64,
    is_me: bool,
    now: u64,
    champion: bool,
) -> Html {
    let age_s = now.saturating_sub(received_ms) / 1000;
    let live = age_s < 30;
    let badge_cls = if live { "badge live" } else { "badge stale" };
    let badge_text = if is_me {
        locale.tr(MessageId::TermYouBadge).to_string()
    } else if live {
        locale.tr(MessageId::TermLive).to_string()
    } else {
        format!("{age_s}s")
    };
    let cls = if is_me { "you" } else { "" };
    let name_cell = if p.name.is_empty() { short_id(pk) } else { p.name.clone() };
    html! {
        <tr class={cls}>
            <td>{ rank + 1 }</td>
            <td>
                { name_cell }
                {
                    if champion {
                        html! {
                            <span
                                class="champion-badge"
                                title={ locale.tr_key("token_perk_name.champion_badge") }
                            >{ "🏆" }</span>
                        }
                    } else { html! {} }
                }
            </td>
            <td class="num">{ format_si(p.gold) }</td>
            <td class="num">{ format_si(p.boss_damage) }</td>
            <td>{ &p.area }</td>
            <td>{ locale.fmt_seconds_ago(age_s) }</td>
            <td><span class={badge_cls}>{ badge_text }</span></td>
        </tr>
    }
}

/// Tiny inbox panel for the mailbox sub-section in Settings. Lists
/// the most recent 5 messages addressed to us so a feature dev can
/// see traffic without instrumenting the console. Empty mailbox or
/// missing mailbox_key both render distinct copy.
pub fn render_mailbox_panel(
    locale: Locale,
    c: &Core,
    on_self_test: Callback<MouseEvent>,
) -> Html {
    if c.mailbox_key.is_none() {
        return html! {
            <p class="muted small">
                { locale.tr(MessageId::MailboxNotConfiguredHead) }
                <code>{ "mailbox-contract" }</code>
                { locale.tr(MessageId::MailboxNotConfiguredVia) }
                <code>{ "scripts/dev-publish.sh" }</code>
                { locale.tr(MessageId::MailboxNotConfiguredTail) }
                <code>{ "mailbox_contract_id_b58" }</code>
                { locale.tr(MessageId::MailboxNotConfiguredIn) }
                <code>{ "dev-keys.json" }</code>
                { "." }
            </p>
        };
    }
    let inbox_n = c.mailbox.len();
    html! { <>
        <div class="action-row">
            <button onclick={on_self_test} disabled={c.pubkey.is_none()}>
                { locale.tr(MessageId::BtnSendTestSelf) }
            </button>
            <span class="muted small">{ locale.fmt_inbox_count(inbox_n) }</span>
        </div>
        {
            if c.mailbox.is_empty() {
                html! { <p class="muted small">{ locale.tr(MessageId::MailboxEmpty) }</p> }
            } else {
                let take = c.mailbox.len().min(5);
                html! {
                    <ul class="mailbox-list">
                        { for c.mailbox[..take].iter().map(|m| {
                            let kind_label = match m.kind {
                                shared::MSG_KIND_CHAT => locale.tr(MessageId::MailboxKindChat),
                                shared::MSG_KIND_GIFT => locale.tr(MessageId::MailboxKindGift),
                                shared::MSG_KIND_GUILD_INVITE => locale.tr(MessageId::MailboxKindGuildInvite),
                                shared::MSG_KIND_TRADE_OFFER => locale.tr(MessageId::MailboxKindTradeOffer),
                                _ => "?",
                            };
                            let preview = String::from_utf8_lossy(&m.body).into_owned();
                            let preview = if preview.len() > 80 {
                                format!("{}…", &preview[..80])
                            } else { preview };
                            html! {
                                <li class="mailbox-row">
                                    <span class="badge">{ kind_label }</span>
                                    <span class="muted small">{ short_id(&m.from) }</span>
                                    <span class="mailbox-preview">{ preview }</span>
                                </li>
                            }
                        })}
                    </ul>
                }
            }
        }
    </>}
}
