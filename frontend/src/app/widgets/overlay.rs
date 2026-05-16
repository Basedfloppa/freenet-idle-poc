//! Transient overlays — toast stack, first-visit onboarding wizard,
//! "while you were away" catch-up banner, debug overlay in Settings.

use shared::format_si;
use yew::prelude::*;

use crate::app::core::{Core, ONBOARDING_STEPS};
use crate::app::i18n::{Locale, MessageId};
use crate::app::types::{Toast, TOAST_TTL_MS};

/// Toast stack — fixed-position corner banners for transient
/// notifications (achievement unlocks today; ending unlocks /
/// boss-era advances later). Older toasts are filtered out at
/// render time; the unified tick prunes the in-RAM Vec separately.
pub fn render_toasts(toasts: &[Toast], now: u64) -> Html {
    let alive: Vec<&Toast> = toasts
        .iter()
        .filter(|t| now.saturating_sub(t.created_ms) < TOAST_TTL_MS)
        .collect();
    if alive.is_empty() {
        return html! {};
    }
    html! {
        <div class="toast-stack">
            { for alive.iter().map(|t| html! {
                <div class="toast">
                    <div class="toast-title">{ &t.label }</div>
                    <div class="toast-body">{ &t.body }</div>
                </div>
            }) }
        </div>
    }
}

/// First-visit walkthrough — a 4-step modal that explains the
/// minimum the player needs to enjoy the game: where state lives,
/// where to fight, the auto loop, and how to find Settings/Help
/// later. Persists "dismissed" in localStorage so it doesn't pester
/// returning players.
pub fn render_onboarding(
    locale: Locale,
    step: Option<u8>,
    on_next: Callback<MouseEvent>,
    on_skip: Callback<MouseEvent>,
) -> Html {
    let Some(step) = step else { return html! {} };
    let (title, body): (&str, Html) = match step {
        0 => (
            locale.tr(MessageId::OnbTitleWelcome),
            html! { <>
                <p>{ locale.tr(MessageId::OnbBodyWelcome1) }</p>
                <p class="muted small">{ locale.tr(MessageId::OnbBodyWelcome2) }</p>
            </> },
        ),
        1 => (
            locale.tr(MessageId::OnbTitleLoop),
            html! { <>
                <p>{ locale.tr(MessageId::OnbBodyLoop1) }</p>
                <p class="muted small">{ locale.tr(MessageId::OnbBodyLoop2) }</p>
            </> },
        ),
        2 => (
            locale.tr(MessageId::OnbTitleAuto),
            html! { <>
                <p>{ locale.tr(MessageId::OnbBodyAuto1) }</p>
                <p class="muted small">{ locale.tr(MessageId::OnbBodyAuto2) }</p>
            </> },
        ),
        _ => (
            locale.tr(MessageId::OnbTitleTabs),
            html! { <>
                <p>{ locale.tr(MessageId::OnbBodyTabs1) }</p>
                <p class="muted small">{ locale.tr(MessageId::OnbBodyTabs2) }</p>
            </> },
        ),
    };
    let last = step + 1 >= ONBOARDING_STEPS;
    let next_label = if last {
        locale.tr(MessageId::BtnStartPlaying)
    } else {
        locale.tr(MessageId::BtnNext)
    };
    html! {
        <div class="onboarding-backdrop">
            <div class="onboarding-modal">
                <p class="muted small onboarding-step">
                    { locale.fmt_onboarding_step(step + 1, ONBOARDING_STEPS) }
                </p>
                <h2>{ title }</h2>
                { body }
                <div class="action-row onboarding-actions">
                    <button class="primary" onclick={on_next}>{ next_label }</button>
                    <button onclick={on_skip}>{ locale.tr(MessageId::BtnSkipIntro) }</button>
                </div>
            </div>
        </div>
    }
}

/// "While you were away" banner — surfaces the delegate's offline
/// catch-up summary when present. Disappears after the next manual
/// mission (the delegate clears `last_catchup` in `run_mission`).
pub fn render_catchup_banner(locale: Locale, catchup: &Option<shared::CatchupSummary>) -> Html {
    let Some(s) = catchup.as_ref() else { return html! {} };
    let elapsed_s = s.ended_ms.saturating_sub(s.started_ms) / 1000;
    let elapsed_human = if elapsed_s >= 3600 {
        format!("{}h {}m", elapsed_s / 3600, (elapsed_s % 3600) / 60)
    } else if elapsed_s >= 60 {
        format!("{}m {}s", elapsed_s / 60, elapsed_s % 60)
    } else {
        format!("{elapsed_s}s")
    };
    html! {
        <section class="panel catchup">
            <h2>{ locale.tr(MessageId::PanelWhileAway) }</h2>
            <p>
                { locale.fmt_catchup_summary(&elapsed_human, s.missions_won, s.missions_lost) }
            </p>
            <p class="muted small">
                { locale.fmt_catchup_rewards(
                    &format_si(s.gold_gained),
                    &format_si(s.essence_gained),
                    &format_si(s.xp_gained),
                    &format_si(s.boss_damage_gained),
                ) }
            </p>
            <p class="muted small">{ locale.tr(MessageId::CatchupClearsHint) }</p>
        </section>
    }
}

/// Patchnotes baked into the bundle, keyed by `BUILD_VERSION` (the
/// commit-count-stamped semver from `build.rs`). The catchup modal
/// compares the active build's version against
/// `Core::last_seen_version` and shows the matching entry — once
/// the player clicks "Got it" the new version is persisted and the
/// notes vanish until the *next* version ships.
///
/// Adding an entry: prepend the new tuple to the slice so the most
/// recent build sits first. Falls back to `None` for versions that
/// don't have notes yet (intermediate builds without an explicit
/// changelog entry just show no "What's new" section — the bumped
/// version still signals to returning players that something
/// shipped).
fn patchnotes_for(version: &str) -> Option<&'static [&'static str]> {
    // Manually-curated changelog. Keys are *literal* version
    // strings (`major.minor.commit_count`) so an autogenerated
    // patch bump won't accidentally re-surface stale notes —
    // when there's nothing meaningful to say, leave the version
    // out of the table and the modal will fire on the version
    // mismatch alone (no notes block).
    const PATCHNOTES: &[(&str, &[&str])] = &[
        // Example entry — uncomment and replace once a stable
        // commit-count is known to ship a documented release.
        // (
        //     "0.1.42",
        //     &[
        //         "Estate panel (Farm tab): hire workers in 4 tiers.",
        //         "Form affinity buffs/penalises Estate yield per tier.",
        //     ],
        // ),
    ];
    PATCHNOTES
        .iter()
        .find(|(v, _)| *v == version)
        .map(|(_, n)| *n)
}

/// Unified post-offline + on-load modal (backlog B4). Surfaces:
///   - the offline-catchup summary from the delegate (`last_catchup`)
///   - the active idle action so the player remembers what was running
///   - the "What's new in v…" patchnotes for the current build, only
///     while it differs from `last_seen_version`
///
/// Returns `html! {}` (no modal) once the player has clicked "Got it"
/// or when none of the trigger conditions hold. Dismissal both:
///   - flips `c.catchup_modal_dismissed` for this session, AND
///   - persists `last_seen_version = CARGO_PKG_VERSION` so the
///     patchnotes section doesn't re-fire on the next reload.
pub fn render_catchup_modal(
    c: &Core,
    locale: Locale,
    on_dismiss: Callback<MouseEvent>,
) -> Html {
    if c.catchup_modal_dismissed {
        return html! {};
    }
    if c.onboarding_step.is_some() {
        // First-run wizard owns the screen; don't stack modals.
        return html! {};
    }
    let inv = &c.inventory;
    let current_version = env!("BUILD_VERSION");
    let version_changed = c.last_seen_version.as_deref() != Some(current_version);
    // Only count a catchup as "fresh" when its started_ms is past
    // the player's stored ack watermark. The delegate keeps the
    // last_catchup field set indefinitely (no longer auto-cleared
    // on `run_mission` because the auto-tick was eating the modal
    // before the player saw it). Frontend acks via `save_settings`
    // bumping `last_catchup_acked_started_ms`, which persists
    // across reloads — same offline window doesn't re-pop after
    // dismiss.
    let has_catchup = inv
        .last_catchup
        .as_ref()
        .map(|s| s.started_ms > c.last_catchup_acked_started_ms)
        .unwrap_or(false);
    if !version_changed && !has_catchup {
        return html! {};
    }
    // Only render the per-section blocks when the catchup is
    // fresh (post-ack-watermark). Avoids stale "while you were
    // away 0s" panes from the same offline window after the
    // player already clicked Got it.
    let catchup_block: Html = match inv.last_catchup.as_ref().filter(|_| has_catchup) {
        Some(s) => {
            let elapsed_s = s.ended_ms.saturating_sub(s.started_ms) / 1000;
            let elapsed_human = if elapsed_s >= 3600 {
                format!("{}h {}m", elapsed_s / 3600, (elapsed_s % 3600) / 60)
            } else if elapsed_s >= 60 {
                format!("{}m {}s", elapsed_s / 60, elapsed_s % 60)
            } else {
                format!("{elapsed_s}s")
            };
            html! {
                <section>
                    <h3>{ locale.tr(MessageId::PanelWhileAway) }</h3>
                    <p>
                        { locale.fmt_catchup_summary(&elapsed_human, s.missions_won, s.missions_lost) }
                    </p>
                    <p class="muted small">
                        { locale.fmt_catchup_rewards(
                            &format_si(s.gold_gained),
                            &format_si(s.essence_gained),
                            &format_si(s.xp_gained),
                            &format_si(s.boss_damage_gained),
                        ) }
                    </p>
                </section>
            }
        }
        None => html! {},
    };
    let estate_block: Html = if has_catchup
        && inv.idle_action == shared::IDLE_ACTION_ESTATE
        && !inv.estate.workers.is_empty()
    {
        let lines: Vec<Html> = shared::ESTATE_TIERS
            .iter()
            .filter_map(|tier| {
                let n = inv.estate.workers_of(tier.id);
                if n == 0 {
                    return None;
                }
                Some(html! { <li>{ locale.fmt_estate_worker_line(tier.name, n) }</li> })
            })
            .collect();
        html! {
            <section>
                <h3>{ locale.tr(MessageId::PanelEstate) }</h3>
                <p class="muted small">
                    { locale.fmt_estate_hint(&shared::form_name(inv.current_form).to_string()) }
                </p>
                <ul class="catchup-estate">{ for lines }</ul>
            </section>
        }
    } else {
        html! {}
    };
    let patchnotes_block: Html = if version_changed {
        match patchnotes_for(current_version) {
            Some(notes) if !notes.is_empty() => html! {
                <section>
                    <h3>{ locale.fmt_whats_new(current_version) }</h3>
                    <ul>
                        { for notes.iter().map(|line| html! { <li>{ *line }</li> }) }
                    </ul>
                </section>
            },
            // Version bumped but no curated notes for this build —
            // surface a one-liner so the player still understands
            // why the modal popped. Commit-count-stamped patch
            // versions move on every push, so this is the common
            // case for incremental rebuilds.
            _ => html! {
                <section>
                    <h3>{ locale.fmt_now_running(current_version) }</h3>
                    <p class="muted small">
                        { locale.tr(MessageId::NewerBuildDesc) }
                    </p>
                </section>
            },
        }
    } else {
        html! {}
    };
    html! {
        <div class="onboarding-backdrop">
            <div class="onboarding-modal catchup-modal">
                <h2>{ locale.tr(MessageId::CatchupModalTitle) }</h2>
                { catchup_block }
                { estate_block }
                { patchnotes_block }
                <div class="action-row onboarding-actions">
                    <button class="primary" onclick={on_dismiss}>{ locale.tr(MessageId::BtnGotIt) }</button>
                </div>
            </div>
        </div>
    }
}

/// Diagnostic readout for the Settings → Advanced section. Surfaces
/// runtime counters that aren't gameplay-relevant but matter when
/// something feels off (state growth, dropped heartbeats, churn).
/// Read-only — purely an observability surface.
pub fn render_debug_overlay(c: &Core, now: u64) -> Html {
    let inv = &c.inventory;
    let ago = |ms: u64| -> String {
        if ms == 0 {
            "never".into()
        } else {
            format!("{}s ago", now.saturating_sub(ms) / 1000)
        }
    };
    html! {
        <details class="debug-overlay">
            <summary>{ "debug overlay (state diagnostics)" }</summary>
            <table class="debug-grid">
                <tbody>
                    <tr><th>{"inventory: gold"}</th><td>{ inv.gold }</td></tr>
                    <tr><th>{"inventory: boss_damage"}</th><td>{ inv.boss_damage }</td></tr>
                    <tr><th>{"inventory: experience"}</th><td>{ inv.experience }</td></tr>
                    <tr><th>{"inventory: mission_count"}</th><td>{ inv.mission_count }</td></tr>
                    <tr><th>{"inventory: equipped slots"}</th>
                        <td>{ inv.equipped.iter().filter(|s| s.is_some()).count() }</td></tr>
                    <tr><th>{"inventory: unequipped items"}</th><td>{ inv.unequipped.len() }</td></tr>
                    <tr><th>{"inventory: skills_unlocked"}</th><td>{ inv.skills_unlocked.len() }</td></tr>
                    <tr><th>{"inventory: forms_visited"}</th><td>{ inv.forms_visited.len() }</td></tr>
                    <tr><th>{"inventory: combat_history rows"}</th><td>{ inv.combat_history.len() }</td></tr>
                    <tr><th>{"inventory: auto_run_enabled"}</th><td>{ if inv.auto_run_enabled { "yes" } else { "no" } }</td></tr>
                    <tr><th>{"contract: others tracked"}</th><td>{ c.others.len() }</td></tr>
                    <tr><th>{"contract: cumulative_damage keys"}</th><td>{ c.cumulative_damage.len() }</td></tr>
                    <tr><th>{"timing: last auto tick"}</th><td>{ ago(c.last_auto_tick_ms) }</td></tr>
                    <tr><th>{"timing: last heartbeat tick"}</th><td>{ ago(c.last_heartbeat_tick_ms) }</td></tr>
                    <tr><th>{"timing: last pull tick"}</th><td>{ ago(c.last_pull_tick_ms) }</td></tr>
                    <tr><th>{"timing: last published"}</th>
                        <td>{ ago(c.last_published_ms.unwrap_or(0)) }</td></tr>
                    <tr><th>{"prefs: sync cadence"}</th><td>{ c.prefs.sync_cadence.label() }</td></tr>
                    <tr><th>{"prefs: theme"}</th><td>{ c.current_theme.clone() }</td></tr>
                </tbody>
            </table>
        </details>
    }
}
