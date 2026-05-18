//! Runtime state — `Core` (the single mutable cell every callback
//! threads through), the `CoreCell`/`PendingCell` aliases that
//! wrap it in an `Rc<RefCell<…>>`, the achievement-toast diff in
//! `ingest_inventory`, and the localStorage flag for the first-
//! visit onboarding wizard.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use freenet_stdlib::prelude::{ContractKey, DelegateKey};
use shared::{Inventory, PresencePayload, PubKey};

use crate::delegate_client::{Pending, WsCell};

use super::i18n_shared;
use super::prefs::UserPrefs;
use super::types::{Tab, Toast, MAX_TOASTS};
use super::util::now_ms;

pub type CoreCell = Rc<RefCell<Option<Core>>>;
pub type PendingCell = Rc<RefCell<Pending>>;

pub struct Core {
    /// Filled once the delegate's `GetPubkey` response arrives.
    pub pubkey: Option<PubKey>,
    pub name: String,
    /// Authoritative inventory — copied from the delegate on
    /// `LoadInventory` / `RunMission` responses. The webapp never
    /// mutates this directly except through delegate calls.
    pub inventory: Inventory,
    /// The inventory snapshot that's already been broadcast to the
    /// presence contract. Used to decide whether the next heartbeat
    /// has anything new to say.
    pub last_published: Inventory,
    pub last_published_ms: Option<u64>,
    /// True while a `RunMission` is in flight — used to disable the
    /// button and prevent overlapping calls in auto-mode.
    pub mission_in_flight: bool,
    /// Chunked offline catchup progress (`Some` only while the
    /// catchup is in-flight). `start_gap_ms` is the wall-clock gap
    /// at the moment we first observed the catchup; `current_gap_ms`
    /// is the gap after the most recent `LoadInventory` response.
    /// Progress = 1 - current/start. Cleared when current_gap drops
    /// below `CATCHUP_DONE_GAP_MS` (single tick worth).
    ///
    /// While `Some`, the polling loop overrides the user's
    /// `sync_cadence.pull_ms()` and fires back-to-back
    /// `LoadInventory` calls so the catchup completes in seconds,
    /// not in minutes-of-heartbeat. A modal blocks every other
    /// interaction until the catchup ends.
    pub catchup_progress: Option<CatchupProgress>,
    // The persistent auto-run flag now lives in `inventory.auto_run_enabled`
    // — single source of truth, delegate-authoritative, survives reloads.
    // Click handlers mutate it optimistically and reconcile via the
    // SetAutoRun response.
    pub others: BTreeMap<PubKey, (PresencePayload, u64)>,
    /// Persistent World Boss ledger as published by the contract.
    /// Per-pubkey high-watermark of cumulative `boss_damage`; survives
    /// presence-entry pruning so the boss aggregate cannot regress
    /// when contributors go idle. Repopulated on every full-state
    /// frame from the contract.
    pub cumulative_damage: BTreeMap<PubKey, u64>,
    pub ws: Option<WsCell>,
    /// Parsed presence contract key — `None` if not configured (empty
    /// constants AND no `dev-keys.json` override). When `Some`, the
    /// frontend subscribes on connect and heartbeats publish signed
    /// Inventory deltas. When `None`, the app runs in single-player
    /// mode: delegate still owns the inventory locally, but no other
    /// players or World Boss aggregate are visible.
    pub contract_key: Option<ContractKey>,
    pub delegate_key: DelegateKey,
    pub status: String,
    /// Flipped to `true` once `freenet::actions::settings::load_settings_once`
    /// has merged the delegate's persisted display name / theme /
    /// tutorial flag into Core. Heartbeats are gated on this so the
    /// first presence publish ships the player's actual name instead
    /// of the cold-load `DEFAULT_NAME` placeholder — without this,
    /// the leaderboard briefly shows `"player"` for returning users
    /// every reload.
    pub prefs_loaded: bool,
    /// Currently-visible section. UI-only state; the delegate has
    /// no notion of tabs.
    pub current_tab: Tab,
    /// Active visual theme id (one of `THEMES`). Persisted in
    /// localStorage; mirrored on `<html data-theme="…">`. UI-only.
    pub current_theme: String,
    /// User-facing tuning knobs. Persisted as one JSON blob in
    /// localStorage; pulled in on init, written through on every
    /// edit. Drives cadence + behavioral toggles for the unified
    /// polling tick below.
    pub prefs: UserPrefs,
    /// Bookkeeping for the unified polling tick — last wall-clock at
    /// which we fired each periodic action. Compared against the
    /// matching cadence from `prefs` to gate the next fire.
    pub last_auto_tick_ms: u64,
    pub last_heartbeat_tick_ms: u64,
    pub last_pull_tick_ms: u64,
    /// Holds the most recent `ExportSeed` reveal so the Settings
    /// panel can show it. `None` = nothing revealed; `Some(hex)` =
    /// raw 32-byte seed encoded as hex. **Never persisted** — lives
    /// in RAM only, cleared on tab close and on manual "Hide".
    pub exported_seed_hex: Option<String>,
    /// Parsed mailbox contract key — `None` if not configured (empty
    /// constants AND no `dev-keys.json` override). When `Some`, the
    /// frontend subscribes on connect and maintains a local mirror
    /// of messages addressed to `pubkey`.
    pub mailbox_key: Option<ContractKey>,
    /// Local mirror of mailbox messages addressed to *this* player.
    /// Populated from contract `GetResponse` / `UpdateNotification`.
    /// Filtered by `to == c.pubkey` at merge time so we don't store
    /// other players' mail.
    pub mailbox: Vec<shared::MessagePayload>,
    /// Parsed guilds contract key (optional, same shape as mailbox).
    pub guilds_key: Option<ContractKey>,
    /// Local mirror of the guilds-contract state.
    pub guilds: shared::GuildsState,
    /// Draft text in the "create guild" input on the Guilds tab.
    /// UI-only — never persisted. Cleared after a successful CREATE.
    pub new_guild_name_input: String,
    /// Achievement-unlock toast queue. Each entry holds its
    /// creation timestamp; render filters by age, the unified
    /// tick prunes expired ones.
    pub toasts: Vec<Toast>,
    /// Achievement ids we've already shown a toast for in this
    /// session. `None` until the first inventory load — the
    /// initial load establishes the baseline silently (no flood
    /// of "you unlocked X 200 missions ago" toasts on reconnect).
    pub shown_achievements: Option<std::collections::BTreeSet<u8>>,
    /// Last hero level a toast was already shown for. `None` on
    /// cold load — first ingest establishes the baseline silently
    /// (no toast on reconnect for the level you were already at).
    /// Bumps fire a Toast { "Уровень N → N+1" } the next ingest.
    pub last_level_shown: Option<u64>,
    /// Last `inv.current_form` value surfaced to the player via the
    /// form-change toast. `None` = first-load baseline (no toast).
    /// Compared in `ingest_inventory`; any change fires one toast
    /// listing the new form's allowed slot mask so the player
    /// understands why some gear just slid into the stash.
    pub last_form_shown: Option<u8>,
    /// §P3: set of skill ids that should get the `skill-unlock-anim`
    /// CSS class on the upcoming render pass. Populated by
    /// `ingest_inventory` when `skills_unlocked` grows; cleared by
    /// the next ingest so the animation fires exactly once.
    pub animate_skills: std::collections::BTreeSet<u8>,
    /// Baseline of skill ids the player has already seen. `None` on
    /// first load — establishes the baseline silently so a returning
    /// player doesn't see every skill animate.
    pub last_skills_shown: Option<std::collections::BTreeSet<u8>>,
    /// Reveal-key bits that have already played their slide-in
    /// animation in this session. Initialised on first inventory
    /// load to the full `revealed` bitmask — returning players
    /// don't see every section flash on reload. Subsequent
    /// reveal-bit flips during the session animate once and
    /// then land in this set so tab navigation doesn't replay.
    pub revealed_animated: Option<u64>,
    /// Reveal-key bits that should animate on the *current*
    /// render pass — populated by `ingest_inventory` whenever a
    /// new bit appears in `inv.revealed` for the first time this
    /// session. Render reads it to gate the `reveal-anim` CSS
    /// class. Naturally cleared by the next inventory update
    /// (which recomputes from an updated `revealed_animated`),
    /// so an in-flight reveal animates exactly once across the
    /// short window before the next delegate tick.
    pub animate_reveal: u64,
    /// Current step of the first-visit onboarding wizard. `None`
    /// = dismissed (or never shown). `Some(0..ONBOARDING_STEPS)`
    /// shows that step's modal. Persisted-as-dismissed via
    /// localStorage key `freenet-idle-onboarded`.
    pub onboarding_step: Option<u8>,
    /// Last `CARGO_PKG_VERSION` the player acknowledged via the
    /// catchup modal's "Got it" button (backlog B4). Loaded from
    /// the delegate-stored `Settings` blob on connect; compared
    /// against the current build's version to decide whether the
    /// "What's new" section of the modal should appear.
    pub last_seen_version: Option<String>,
    /// Was the catchup / patchnotes modal dismissed in this
    /// session? Resets to `false` on every reload so the modal
    /// surfaces once per offline-return rather than blocking the
    /// UI permanently after the first save.
    pub catchup_modal_dismissed: bool,
    /// `started_ms` watermark of the most-recently-acknowledged
    /// catchup summary. Loaded from the delegate's Settings blob
    /// on connect, written back via `save_settings_once` when the
    /// player dismisses the modal. Persistent across reloads so
    /// the same catchup window doesn't pop the modal twice — the
    /// pre-modal banner used to be cleared by `run_mission` on
    /// the delegate, but that path raced the unified auto-tick
    /// and the modal vanished before the player could click.
    pub last_catchup_acked_started_ms: u64,
    /// Currently-selected World Map view — Linear chain or the
    /// procedural Wilds graph. UI-only; doesn't touch the
    /// delegate or the inventory. Default Linear because Wilds
    /// is gated on level 10+ anyway.
    pub map_view: super::types::MapView,
    /// Pending custom confirm-modal (§8.A8). When `Some`, the modal
    /// blocks every other interaction until the user accepts or
    /// cancels. UI-only — never persisted.
    pub pending_confirm: Option<PendingConfirm>,
}

/// Snapshot of in-progress offline catchup. See
/// `Core::catchup_progress` for the lifecycle and rationale.
#[derive(Clone, Copy, Debug)]
pub struct CatchupProgress {
    /// Wall-clock gap (`now_ms - auto_last_tick_ms`) observed when
    /// the catchup first started. Used as the denominator for the
    /// progress fraction.
    pub start_gap_ms: u64,
    /// Most-recently observed gap. Equals `start_gap_ms` on the
    /// first frame and shrinks toward zero with each chunk.
    pub current_gap_ms: u64,
    /// Effective offline cap (hours) at the time the catchup
    /// started — surfaced in the modal copy so the player knows
    /// the simulated window length even after the visible gap has
    /// shrunk.
    pub cap_hours: u8,
}

/// Gap (ms) at or below which the catchup is considered "done".
/// One catchup tick is `CATCHUP_TICK_MS = 1_000`; we treat anything
/// under a few seconds of drift as caught up so the modal closes
/// cleanly even if the delegate's clock is slightly ahead.
pub const CATCHUP_DONE_GAP_MS: u64 = 5_000;
/// Gap (ms) at or above which the catchup modal opens. Anything
/// shorter is normal heartbeat drift and shouldn't pop a modal.
pub const CATCHUP_OPEN_GAP_MS: u64 = 60_000;

/// Confirm-modal staging slot. Each destructive callsite that wants
/// a custom confirm pushes one of these into `core.pending_confirm`
/// instead of calling `window.confirm()` directly. The
/// `<ConfirmModal>` reads it, renders the message, and runs
/// `on_confirm` when the user accepts.
#[derive(Clone)]
pub struct PendingConfirm {
    /// Pre-localized message body. The modal renders it verbatim.
    pub message: String,
    /// Action to dispatch on OK. Captures whatever context the
    /// originating callsite needs (RPC sender clones, payload ids).
    pub on_confirm: std::rc::Rc<dyn Fn()>,
}

/// Filtered toast-push. Drops the toast if `prefs.toast_filter`
/// has the bit for `kind` cleared (§8 B5); otherwise enforces the
/// `MAX_TOASTS` cap by evicting the oldest. Centralises the cap +
/// filter logic so individual callsites stay readable.
pub fn push_toast(c: &mut Core, kind: super::types::ToastKind, toast: Toast) {
    if (c.prefs.toast_filter & kind.bit()) == 0 {
        return;
    }
    if c.toasts.len() >= MAX_TOASTS {
        c.toasts.remove(0);
    }
    c.toasts.push(toast);
}

/// Apply a fresh `Inventory` from the delegate into `Core`,
/// surfacing any newly-unlocked achievements as toasts. The first
/// invocation in a session establishes the baseline silently —
/// existing achievements are noted as "seen", no toasts fire.
/// Subsequent calls diff the new set against `shown_achievements`
/// and push one toast per genuinely new id.
pub fn ingest_inventory(c: &mut Core, inv: Inventory) {
    let now = now_ms();
    let current: std::collections::BTreeSet<u8> =
        inv.achievement_unlocks.keys().copied().collect();
    let locale = c.prefs.locale;
    let new_achievement_ids: Vec<u8> = match c.shown_achievements.as_mut() {
        None => {
            c.shown_achievements = Some(current);
            Vec::new()
        }
        Some(seen) => {
            let news: Vec<u8> = current.difference(seen).copied().collect();
            for id in &news {
                seen.insert(*id);
            }
            news
        }
    };
    for id in new_achievement_ids {
        push_toast(c, super::types::ToastKind::Achievement, Toast {
            label: format!("🏆 {}", i18n_shared::achievement_label(locale, id)),
            body: i18n_shared::achievement_reason(locale, id),
            created_ms: now,
        });
    }
    // Level-up toast. First-load baselines silently; later ingests
    // diff the level and push one toast per crossing.
    let cur_level = shared::level_of(&inv);
    match c.last_level_shown {
        None => {
            c.last_level_shown = Some(cur_level);
        }
        Some(prev) if cur_level > prev => {
            for new_lvl in (prev + 1)..=cur_level {
                push_toast(c, super::types::ToastKind::LevelUp, Toast {
                    label: format!("⬆ {}", locale.tr_key("toast.level_up_title")
                        .replace("{lvl}", &new_lvl.to_string())),
                    body: locale.tr_key("toast.level_up_body")
                        .replace("{lvl}", &new_lvl.to_string()),
                    created_ms: now,
                });
            }
            c.last_level_shown = Some(cur_level);
        }
        _ => {}
    }
    // Form-change toast. First-load baselines silently; later
    // ingests fire one toast per transition so the player knows
    // why some gear was just moved to the stash and which slots
    // the new form locks them out of.
    let cur_form = inv.current_form;
    match c.last_form_shown {
        None => {
            c.last_form_shown = Some(cur_form);
        }
        Some(prev) if prev != cur_form => {
            let form_label = crate::app::i18n_shared::form_name(locale, cur_form);
            let mask = shared::form_slot_mask(cur_form);
            let mut allowed = Vec::with_capacity(shared::SLOT_COUNT);
            for slot_idx in 0..shared::SLOT_COUNT {
                if mask[slot_idx] {
                    allowed.push(crate::app::i18n_shared::slot_name(locale, slot_idx));
                }
            }
            let slots_str = allowed.join(", ");
            push_toast(c, super::types::ToastKind::FormChange, Toast {
                label: format!(
                    "🔁 {}",
                    locale
                        .tr_key("toast.form_change.title")
                        .replace("{form}", form_label)
                ),
                body: locale
                    .tr_key("toast.form_change.body")
                    .replace("{slots}", &slots_str),
                created_ms: now,
            });
            c.last_form_shown = Some(cur_form);
        }
        _ => {}
    }
    // Idle-potion feedback toast. The "Use" button outside combat
    // is otherwise a silent click — HP bar fills, count -1, no
    // confirmation. Fire when potions decremented while no battle
    // was running on either side of the ingest. `healed > 0` skips
    // the at-full-HP no-op case where the heal returned no delta.
    if c.inventory.current_battle.is_none()
        && inv.current_battle.is_none()
        && c.inventory.potions > inv.potions
    {
        let healed = inv.current_hp.saturating_sub(c.inventory.current_hp);
        if healed > 0 {
            push_toast(c, super::types::ToastKind::PotionIdle, Toast {
                label: format!("🧪 {}", locale.tr_key("toast.potion_idle.title")),
                body: locale
                    .tr_key("toast.potion_idle.body")
                    .replace("{hp}", &healed.to_string()),
                created_ms: now,
            });
        }
    }
    // §P3 skill-up animation. Diff `skills_unlocked` against the
    // baseline; freshly unlocked ids land in `animate_skills` for
    // the upcoming render pass. Baseline is set silently on first
    // ingest so returning players don't see every skill pulse.
    let cur_skills: std::collections::BTreeSet<u8> =
        inv.skills_unlocked.keys().copied().collect();
    match c.last_skills_shown.as_ref() {
        None => {
            c.last_skills_shown = Some(cur_skills);
            c.animate_skills.clear();
        }
        Some(seen) => {
            let new_skills: Vec<u8> = cur_skills.difference(seen).copied().collect();
            c.animate_skills = new_skills.iter().copied().collect();
            c.last_skills_shown = Some(seen.union(&cur_skills).copied().collect());
        }
    }
    // Reveal-bit animation gating. First load is silent: the
    // baseline is set to whatever the delegate already had
    // unlocked, so a returning player doesn't see every section
    // flash on reconnect. After that, any bit that's in
    // `inv.revealed` but not yet in `revealed_animated` is
    // "newly revealed" — surface it via `animate_reveal` for
    // the upcoming render pass, then promote it into the
    // baseline so the next ingest doesn't replay it.
    match c.revealed_animated {
        None => {
            c.revealed_animated = Some(inv.revealed);
            c.animate_reveal = 0;
        }
        Some(prev) => {
            let newly = inv.revealed & !prev;
            c.animate_reveal = newly;
            c.revealed_animated = Some(prev | inv.revealed);
        }
    }
    // Chunked offline catchup tracking. The delegate's
    // `catch_up_auto` processes at most one chunk
    // (CATCHUP_CHUNK_HOURS=24h) per call and advances
    // `auto_last_tick_ms` toward `now_ms`. If a gap is still
    // visible here, the catchup isn't finished — store a snapshot
    // for the modal and let the polling loop fire another
    // LoadInventory immediately. When the gap drops below
    // CATCHUP_DONE_GAP_MS we clear the slot.
    let gap_ms = now.saturating_sub(inv.auto_last_tick_ms);
    if inv.auto_last_tick_ms == 0 || !inv.auto_run_enabled {
        // Either fresh save (auto-run was never started) or the
        // player disabled auto-run — no catchup to do.
        c.catchup_progress = None;
    } else if gap_ms <= CATCHUP_DONE_GAP_MS {
        c.catchup_progress = None;
    } else {
        match c.catchup_progress {
            None if gap_ms >= CATCHUP_OPEN_GAP_MS => {
                c.catchup_progress = Some(CatchupProgress {
                    start_gap_ms: gap_ms,
                    current_gap_ms: gap_ms,
                    cap_hours: if inv.routine.offline_cap_hours == 0 {
                        1
                    } else {
                        inv.routine.offline_cap_hours
                    },
                });
            }
            Some(mut p) => {
                p.current_gap_ms = gap_ms;
                if gap_ms > p.start_gap_ms {
                    // Player's clock drifted forward, or auto-run
                    // was re-enabled mid-session — reset the
                    // denominator so progress stays in [0, 1].
                    p.start_gap_ms = gap_ms;
                }
                c.catchup_progress = Some(p);
            }
            _ => {}
        }
    }
    c.inventory = inv;
}

/// How many steps in the first-visit wizard.
pub const ONBOARDING_STEPS: u8 = 4;

// `onboarding_dismissed` / `dismiss_onboarding` (localStorage
// key `freenet-idle-onboarded`) retired — the sandboxed iframe's
// null origin makes localStorage reload-ephemeral. The dismissed
// flag now lives in `UiPrefs.tutorial_dismissed` on the delegate
// and is loaded via `freenet::actions::settings::load_settings_once`.
