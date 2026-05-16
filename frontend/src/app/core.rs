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
    match c.shown_achievements.as_mut() {
        None => {
            c.shown_achievements = Some(current);
        }
        Some(seen) => {
            let new_ids: Vec<u8> = current.difference(seen).copied().collect();
            for id in new_ids {
                if c.toasts.len() >= MAX_TOASTS {
                    c.toasts.remove(0);
                }
                c.toasts.push(Toast {
                    label: format!("🏆 {}", i18n_shared::achievement_label(locale, id)),
                    body: i18n_shared::achievement_reason(locale, id),
                    created_ms: now,
                });
                seen.insert(id);
            }
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
    c.inventory = inv;
}

/// How many steps in the first-visit wizard.
pub const ONBOARDING_STEPS: u8 = 4;

// `onboarding_dismissed` / `dismiss_onboarding` (localStorage
// key `freenet-idle-onboarded`) retired — the sandboxed iframe's
// null origin makes localStorage reload-ephemeral. The dismissed
// flag now lives in `UiPrefs.tutorial_dismissed` on the delegate
// and is loaded via `freenet::actions::settings::load_settings_once`.
