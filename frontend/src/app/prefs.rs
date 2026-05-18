//! User preferences — visual theme + the JSON blob persisted in
//! localStorage covering sync cadence, HP-pause threshold,
//! publish-on-mission toggle, leaderboard filters, the WS URL
//! override, and the active UI locale.

use wasm_bindgen::JsCast;

use super::i18n::{detect_browser_locale, Locale};

/// Default theme code on first paint / when the persisted value is
/// missing or unknown. Theme catalog itself is now discovered at
/// runtime from `frontend/themes/*.json` via
/// `super::theme_loader::available_codes()` — the old hardcoded
/// `pub const THEMES` is removed.
pub const DEFAULT_THEME: &str = "parchment";
pub const DEFAULT_NAME: &str = "player";

/// Display list for the Settings UI — pairs of (code, endonym).
/// Backed by `theme_loader`; falls back to `DEFAULT_THEME` if no
/// JSON file was bundled.
pub fn themes_list() -> Vec<(&'static str, &'static str)> {
    let codes = super::theme_loader::available_codes();
    if codes.is_empty() {
        return vec![(DEFAULT_THEME, "Parchment")];
    }
    codes
        .iter()
        .map(|c| (*c, super::theme_loader::endonym(c)))
        .collect()
}

pub fn theme_is_known(code: &str) -> bool {
    super::theme_loader::is_known(code)
}

/// Reflect visual prefs that need DOM-level CSS hooks
/// (reduced-motion / reduced-flash / stash-density / overlay-mode
/// — anything where CSS selectors `[data-reduced-motion="true"]`
/// or similar gate the rule). Call this once at boot and after
/// every pref mutation that touches these knobs.
/// Render gold display, honouring `hide_gold` (§8 C1). Returns
/// "***" when hidden so the UI keeps shape but a streamer's audience
/// can't pixel-peek the number. Pass through to `format_si` otherwise.
pub fn render_gold(hide: bool, value: u64) -> String {
    if hide {
        "***".to_string()
    } else {
        shared::format_si(value)
    }
}

/// Render boss-damage display, honouring `hide_boss_damage` (§8 C2).
pub fn render_boss_damage(hide: bool, value: u64) -> String {
    if hide {
        "***".to_string()
    } else {
        shared::format_si(value)
    }
}

pub fn apply_visual_prefs(prefs: &UserPrefs) {
    let Some(window) = web_sys::window() else { return };
    let Some(doc) = window.document() else { return };
    let Some(root) = doc.document_element() else { return };
    let _ = root.set_attribute(
        "data-reduced-motion",
        if prefs.reduced_motion { "true" } else { "false" },
    );
    let _ = root.set_attribute(
        "data-reduced-flash",
        if prefs.reduced_flash { "true" } else { "false" },
    );
    let _ = root.set_attribute(
        "data-stash-density",
        match prefs.stash_density {
            1 => "compact",
            2 => "tight",
            _ => "comfortable",
        },
    );
    let _ = root.set_attribute(
        "data-overlay-mode",
        if prefs.overlay_mode { "true" } else { "false" },
    );
}

/// Inject a `--font-scale: N%;` declaration into the same
/// `<style id="dynamic-theme">` element used by `apply_theme`. The
/// CSS reads it as a multiplier on `body { font-size }`. Clamped
/// by the caller; this fn just renders the override.
pub fn apply_font_scale(percent: u8) {
    let Some(window) = web_sys::window() else { return };
    let Some(doc) = window.document() else { return };
    let head = match doc.head() {
        Some(h) => h,
        None => return,
    };
    let existing = doc.get_element_by_id("dynamic-font-scale");
    let style: web_sys::HtmlStyleElement = if let Some(e) = existing {
        match e.dyn_into() {
            Ok(s) => s,
            Err(_) => return,
        }
    } else {
        let created = match doc.create_element("style") {
            Ok(e) => e,
            Err(_) => return,
        };
        let _ = created.set_attribute("id", "dynamic-font-scale");
        if head.append_child(&created).is_err() {
            return;
        }
        match created.dyn_into() {
            Ok(s) => s,
            Err(_) => return,
        }
    };
    let pct = percent.clamp(50, 200);
    style.set_inner_html(&format!(":root {{ --font-scale: {pct}%; }}"));
}

/// Where the unified prefs blob lives in `localStorage`. NOTE: the
/// sandboxed Freenet webapp iframe has a "null" document origin in
/// the default Tier-1 sandbox, so writes here don't survive a page
/// reload. The blob is kept for non-critical UI knobs (cadence,
/// HP-pause threshold, etc.) that can tolerate resetting on reload;
/// load-bearing settings (display name, theme) are persisted on the
/// delegate via `crate::freenet::actions::settings`.
const PREFS_STORAGE_KEY: &str = "freenet-idle-prefs";

/// Push the requested theme id to `<html data-theme=...>`.
///
/// The persistent copy lives on the delegate (see
/// `freenet::actions::settings::save_settings_once`) — the sandboxed
/// iframe's null origin breaks localStorage across reloads, so we
/// cannot rely on it. The pre-WASM inline script in `index.html`
/// still attempts a localStorage read for first-paint speed; in the
/// sandbox it silently no-ops and we fall back to the parchment
/// default until the delegate's `LoadUiPrefs` response lands a few
/// hundred ms later.
pub fn apply_theme(theme_id: &str) {
    let Some(window) = web_sys::window() else { return };
    let Some(doc) = window.document() else { return };
    // Keep `data-theme=…` on <html> for DevTools-inspection and so
    // any CSS that wants to target a named scheme via
    // `[data-theme="dusk"] foo` still has a hook. The actual CSS
    // vars are now injected through `<style id="dynamic-theme">`
    // so untargeted theme JSON files just work.
    if let Some(root) = doc.document_element() {
        let _ = root.set_attribute("data-theme", theme_id);
    }
    let css = super::theme_loader::render_root_css(theme_id);
    let head = match doc.head() {
        Some(h) => h,
        None => return,
    };
    let existing = doc.get_element_by_id("dynamic-theme");
    let style: web_sys::HtmlStyleElement = if let Some(existing) = existing {
        match existing.dyn_into() {
            Ok(s) => s,
            Err(_) => return,
        }
    } else {
        let created = match doc.create_element("style") {
            Ok(e) => e,
            Err(_) => return,
        };
        let _ = created.set_attribute("id", "dynamic-theme");
        if head.append_child(&created).is_err() {
            return;
        }
        match created.dyn_into() {
            Ok(s) => s,
            Err(_) => return,
        }
    };
    style.set_inner_html(&css);
}

/// How often the presence contract gets a fresh signed entry, and
/// how often we pull `LoadInventory` from the delegate. Both knobs
/// move together so a player picks one stance for "this tab".
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SyncCadence {
    /// 5 s heartbeat / 5 s pull — leaderboard updates fast, more node
    /// traffic.
    Aggressive,
    /// 10 s / 10 s — current MVP defaults.
    Normal,
    /// 30 s / 30 s — for backgrounded or low-bandwidth tabs. Players
    /// vanish from leaderboards on the contract's 60 s prune unless
    /// they refresh at least once a minute, so 30 s is the slowest
    /// that still keeps you visible.
    Easy,
}

impl SyncCadence {
    pub fn heartbeat_ms(self) -> u64 {
        match self {
            SyncCadence::Aggressive => 5_000,
            SyncCadence::Normal => 10_000,
            SyncCadence::Easy => 30_000,
        }
    }
    pub fn pull_ms(self) -> u64 {
        match self {
            SyncCadence::Aggressive => 5_000,
            SyncCadence::Normal => 10_000,
            SyncCadence::Easy => 30_000,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            SyncCadence::Aggressive => "Aggressive (5s)",
            SyncCadence::Normal => "Normal (10s)",
            SyncCadence::Easy => "Easy (30s)",
        }
    }
}

/// All user-facing tuning knobs. Serialized as one JSON blob in
/// localStorage so adding a field doesn't multiply storage keys.
/// New fields must have a `#[serde(default)]` so older blobs still
/// deserialize cleanly after an update.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserPrefs {
    #[serde(default = "default_sync_cadence")]
    pub sync_cadence: SyncCadence,
    /// Auto-mission pauses when current HP falls below this fraction
    /// of max HP. `0` = original behavior (only pause at 0 HP). New
    /// installs default to 25 so a string of bad rolls can't death-
    /// loop a fresh player; existing users keep whatever value is
    /// already in their localStorage blob.
    #[serde(default = "default_auto_pause_hp_pct")]
    pub auto_pause_hp_pct: u8,
    /// If false, only the periodic heartbeat publishes presence;
    /// the post-mission reactive publish is skipped. Useful on
    /// slow/expensive links.
    #[serde(default = "default_true")]
    pub reactive_publish: bool,
    /// Hide ed25519 public key in Hero panel + Settings tab — for
    /// screenshots / privacy.
    #[serde(default)]
    pub hide_pubkey: bool,
    /// Filter stale (>30 s) entries out of the leaderboard so only
    /// currently-live publishers show.
    #[serde(default)]
    pub hide_stale_players: bool,
    /// Free-form WebSocket URL override. Empty string falls through
    /// to `?ws=` query param, then `DEFAULT_WS`. Takes effect after
    /// a page reload (or a forced reconnect).
    #[serde(default)]
    pub ws_url_override: String,
    /// Active UI locale. Persisted alongside the other knobs so a
    /// player's language choice survives reloads even when the
    /// delegate's `UiPrefs` round-trip hasn't completed yet. New
    /// installations seed this from the browser's
    /// `navigator.language` via `detect_browser_locale` — see
    /// `Default` impl below.
    #[serde(default = "default_locale")]
    pub locale: Locale,
    /// Number format (`compact` / `full` / `raw`). See §8 A1.
    #[serde(default = "default_number_format")]
    pub number_format: String,
    /// Body font-size scale in percent (50..=200). `100` = unchanged.
    #[serde(default = "default_font_scale")]
    pub font_scale: u8,
    /// Spoiler-safe mode — hides plot / chapter / endings copy.
    #[serde(default)]
    pub spoiler_safe: bool,
    /// Confirm-before-destructive — wraps Ascend / form-change /
    /// sell-all in an extra confirm dialog.
    #[serde(default = "default_true")]
    pub confirm_destructive: bool,
    /// Stash filter (§8 B1). `0xFF` = show every slot; any other
    /// value filters the stash to that slot index. `u8` so a
    /// future "by tier" or "by form" extension can re-use the
    /// same field — keep the wire payload tight.
    #[serde(default = "default_stash_filter")]
    pub stash_filter: u8,
    /// Stash sort mode (§8 B1). `0` = catalog order (legacy),
    /// `1` = tier descending, `2` = score descending (atk+def+hp).
    #[serde(default)]
    pub stash_sort: u8,
    /// Stash density (§8 B4). `0` = comfortable (default), `1` =
    /// compact, `2` = tight. Sets a CSS class on `.stash-grouped`
    /// which adjusts row spacing / font size.
    #[serde(default)]
    pub stash_density: u8,
    /// Hide gold amount everywhere (§8 C1). For streamer / privacy
    /// sessions — replaces numbers with `***`. Doesn't affect
    /// gameplay logic, only display.
    #[serde(default)]
    pub hide_gold: bool,
    /// Hide boss-damage counter (§8 C2). Same display-only suppress
    /// as `hide_gold`. Useful when streaming PR runs you don't want
    /// to spoil for viewers.
    #[serde(default)]
    pub hide_boss_damage: bool,
    /// Reduced-motion mode (§8 D2). Strips slide-in / fade
    /// animations from toasts, panels, reveals. Honours
    /// `prefers-reduced-motion: reduce` as the system-level
    /// default; this pref is the explicit override.
    #[serde(default)]
    pub reduced_motion: bool,
    /// Reduced-flash mode (§8 D3). Disables Champion-badge glow,
    /// ascension flash, era-advance pulse. Layered on top of
    /// `reduced_motion`; some animations only show flashes.
    #[serde(default)]
    pub reduced_flash: bool,
    /// Bitmask of toast categories to surface (§8 B5). Bit set =
    /// show, bit clear = silently drop. Bits in `ToastKind` order
    /// (see `core.rs::ToastKind`). Default = all-on (`u32::MAX`).
    #[serde(default = "default_toast_filter")]
    pub toast_filter: u32,
    /// Free-form player motto shown under the Hero name (§8 C6).
    /// Max 64 chars; longer strings are truncated client-side.
    /// Auto-published into `inv.routine.public_motto` on every
    /// change (Settings input `onchange`), then carried by the
    /// next PresencePayloadV3 heartbeat so other players see it
    /// on the leaderboard. The Force-Republish button forces
    /// the SetPublicCosmetics RPC explicitly for retry cases.
    #[serde(default)]
    pub motto: String,
    /// Leaderboard row accent (§8 C7 + §E3). 0 = no accent
    /// (default), 1..=6 = one of six preset hues. Applied as
    /// (a) text colour on name + motto and (b) 4px inset
    /// left-border ribbon on the player's row — both work
    /// against light AND dark theme backgrounds. Auto-published
    /// via `SetPublicCosmetics` so other clients render the
    /// same accent on this player's leaderboard row.
    #[serde(default)]
    pub row_accent: u8,
    /// Bitmask of collapsed panel ids (§8 B2). Each panel that
    /// supports collapse owns a stable u8 id (see
    /// `widgets::collapse::PanelId`); bit set = collapsed.
    /// `u64` so we have room for ~64 distinct collapsibles.
    #[serde(default)]
    pub collapsed_panels: u64,
    /// Bitmask of hidden tab indices (§8 B3). Bit `i` set = hide
    /// `Tab` variant with discriminant `i`. Settings and Home
    /// can't be hidden — those bits are ignored at render time.
    #[serde(default)]
    pub hidden_tabs: u32,
    /// Numerical-assist toggles (§8 D5). Bit 0 = show enemy HP as
    /// %, bit 1 = hide hero HP numbers (bar only), bit 2 =
    /// hide damage numbers in combat feed.
    #[serde(default)]
    pub numerical_assists: u32,
    /// Override gear sprite for the player (§8 C5). When `Some`,
    /// the rendered form-name prefix uses this string instead of
    /// the default per-form emoji. Restricted to a whitelist at
    /// render time so a stray locale string can't smuggle HTML.
    #[serde(default)]
    pub hero_skin: String,
    /// Pubkey display variant (§8 C4). 0 = Full (legacy), 1 =
    /// Short (8-char prefix only), 2 = Hidden ("***"). Falls back
    /// to `hide_pubkey` on load so existing localStorage doesn't
    /// reset the user's choice — see `load_prefs`.
    #[serde(default)]
    pub pubkey_display: u8,
    /// OBS-friendly compact overlay (§8 C3). When on, the page
    /// switches to a streamlined layout: nav-tabs collapse to a
    /// thin header, panels max-width 600px, advanced
    /// debug-overlay forcibly hidden. Useful for embedding as
    /// a browser-source in OBS.
    #[serde(default)]
    pub overlay_mode: bool,
    /// Keyboard shortcuts toggle (§8 D4). When on, document-level
    /// keydown listener routes hotkeys to game actions: `M` =
    /// Run Mission, `A` = toggle Auto, `E` = Auto-Equip Best,
    /// `1..8` = switch tabs by index. Off by default so a player
    /// who didn't ask doesn't get surprised by stray keypresses
    /// in form inputs.
    #[serde(default)]
    pub keyboard_shortcuts: bool,
    /// §8 B8 theme schedule. Empty = disabled (use the static
    /// `theme_id` from the delegate Settings blob). Otherwise:
    /// `day_code`/`night_code` are theme code values from
    /// `themes/*.json`, `night_hour` (0..=23) is the local-time
    /// hour at which we flip to the night theme; we flip back at
    /// `night_hour + 12 mod 24`. Frontend-only; the boot timer
    /// in main.rs re-applies on tab-load + ticks every minute.
    #[serde(default)]
    pub theme_schedule_day: String,
    #[serde(default)]
    pub theme_schedule_night: String,
    /// Hour of day (0..23) when night theme kicks in. `0xFF` =
    /// schedule disabled (treat empty `day`/`night` as the same
    /// disable signal). Held as a u8 so legacy localStorage
    /// without the field decodes cleanly.
    #[serde(default = "default_night_hour")]
    pub theme_night_hour: u8,
}

pub const THEME_NIGHT_HOUR_DISABLED: u8 = 0xFF;

pub const PUBKEY_DISPLAY_FULL: u8 = 0;
pub const PUBKEY_DISPLAY_SHORT: u8 = 1;
pub const PUBKEY_DISPLAY_HIDDEN: u8 = 2;

// §8 B2 panel-collapse bit positions inside `collapsed_panels: u64`.
// Each panel that supports collapse owns a stable bit. Don't
// reorder — these are persisted in localStorage. Append new
// collapsibles at the end and never re-use a retired slot.
pub const PANEL_BIT_PLOT: u8 = 0;
pub const PANEL_BIT_STASH: u8 = 1;
pub const PANEL_BIT_BUY_GEAR: u8 = 2;
pub const PANEL_BIT_SAGE: u8 = 3;
pub const PANEL_BIT_RESOURCES: u8 = 4;

#[inline]
pub fn is_panel_collapsed(bits: u64, panel: u8) -> bool {
    (bits & (1u64 << panel)) != 0
}

/// Sentinel "no filter" value for `stash_filter` — kept out of
/// the legal slot index range (`0..SLOT_COUNT == 0..8`) so the
/// match against actual slot ids stays simple.
pub const STASH_FILTER_NONE: u8 = 0xFF;

fn default_number_format() -> String {
    "compact".to_string()
}

fn default_font_scale() -> u8 {
    100
}

fn default_locale() -> Locale {
    detect_browser_locale()
}

fn default_sync_cadence() -> SyncCadence {
    SyncCadence::Normal
}
fn default_true() -> bool {
    true
}
fn default_auto_pause_hp_pct() -> u8 {
    25
}
fn default_stash_filter() -> u8 {
    STASH_FILTER_NONE
}
fn default_toast_filter() -> u32 {
    u32::MAX
}
fn default_night_hour() -> u8 {
    THEME_NIGHT_HOUR_DISABLED
}

impl Default for UserPrefs {
    fn default() -> Self {
        Self {
            sync_cadence: default_sync_cadence(),
            auto_pause_hp_pct: default_auto_pause_hp_pct(),
            reactive_publish: true,
            hide_pubkey: false,
            hide_stale_players: false,
            ws_url_override: String::new(),
            locale: default_locale(),
            number_format: default_number_format(),
            font_scale: default_font_scale(),
            spoiler_safe: false,
            confirm_destructive: true,
            stash_filter: default_stash_filter(),
            stash_sort: 0,
            stash_density: 0,
            hide_gold: false,
            hide_boss_damage: false,
            reduced_motion: false,
            reduced_flash: false,
            toast_filter: default_toast_filter(),
            motto: String::new(),
            row_accent: 0,
            collapsed_panels: 0,
            hidden_tabs: 0,
            numerical_assists: 0,
            hero_skin: String::new(),
            pubkey_display: PUBKEY_DISPLAY_FULL,
            overlay_mode: false,
            keyboard_shortcuts: false,
            theme_schedule_day: String::new(),
            theme_schedule_night: String::new(),
            theme_night_hour: THEME_NIGHT_HOUR_DISABLED,
        }
    }
}

/// §8 B8: which theme code should be active right now given the
/// player's schedule + local clock? Returns `None` when the
/// schedule is disabled (empty day/night codes OR sentinel
/// hour) — caller falls back to the static theme.
pub fn schedule_theme_for_now(prefs: &UserPrefs) -> Option<String> {
    if prefs.theme_night_hour == THEME_NIGHT_HOUR_DISABLED
        || prefs.theme_schedule_day.is_empty()
        || prefs.theme_schedule_night.is_empty()
    {
        return None;
    }
    let hour = local_hour_now()?;
    let night_start = prefs.theme_night_hour;
    let day_start = (night_start as u16 + 12) % 24;
    // Night window is [night_start, day_start) on the 24-hr clock.
    // Handles the wrap (e.g. night_start = 20, day_start = 8): an
    // hour in [20, 24) ∪ [0, 8) is night.
    let is_night = if night_start < day_start as u8 {
        hour >= night_start && (hour as u16) < day_start
    } else {
        hour >= night_start || (hour as u16) < day_start
    };
    Some(if is_night {
        prefs.theme_schedule_night.clone()
    } else {
        prefs.theme_schedule_day.clone()
    })
}

fn local_hour_now() -> Option<u8> {
    let date = js_sys::Date::new_0();
    Some(date.get_hours() as u8)
}

/// Pull prefs from localStorage. Malformed / missing JSON →
/// defaults. Forward compat: missing fields fall to `#[serde(default)]`.
pub fn load_prefs() -> UserPrefs {
    let Some(window) = web_sys::window() else { return UserPrefs::default() };
    let Ok(Some(storage)) = window.local_storage() else { return UserPrefs::default() };
    let Ok(Some(raw)) = storage.get_item(PREFS_STORAGE_KEY) else {
        return UserPrefs::default();
    };
    let mut prefs: UserPrefs = serde_json::from_str(&raw).unwrap_or_default();
    // §8 C4 migration: if the legacy `hide_pubkey` is true but the
    // new `pubkey_display` is still at its default (Full), promote
    // the user's previous "hide" choice into the new enum so the
    // upgrade doesn't silently re-reveal a pubkey they had hidden.
    if prefs.hide_pubkey && prefs.pubkey_display == PUBKEY_DISPLAY_FULL {
        prefs.pubkey_display = PUBKEY_DISPLAY_HIDDEN;
    }
    prefs
}

pub fn save_prefs(prefs: &UserPrefs) {
    let Some(window) = web_sys::window() else { return };
    let Ok(Some(storage)) = window.local_storage() else { return };
    if let Ok(json) = serde_json::to_string(prefs) {
        let _ = storage.set_item(PREFS_STORAGE_KEY, &json);
    }
}

/// Wipe every UI preference back to defaults — theme key, prefs blob.
/// Reloads the page so the inline boot script re-evaluates without us.
pub fn clear_all_prefs() {
    let Some(window) = web_sys::window() else { return };
    if let Ok(Some(storage)) = window.local_storage() {
        let _ = storage.remove_item(PREFS_STORAGE_KEY);
    }
    if let Some(location) = window.location().href().ok() {
        let _ = window.location().set_href(&location);
    }
}
