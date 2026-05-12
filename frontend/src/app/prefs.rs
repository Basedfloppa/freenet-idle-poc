//! User preferences — visual theme + the JSON blob persisted in
//! localStorage covering sync cadence, HP-pause threshold,
//! publish-on-mission toggle, leaderboard filters, and the WS URL
//! override.

/// Visual themes available in Settings. The id goes into the
/// `data-theme` attribute on `<html>`; `style.css` reads it via a
/// `[data-theme="<id>"]` block. `Parchment` is the default.
pub const THEMES: &[(&str, &str)] = &[
    ("parchment", "Parchment"),
    ("dusk", "Dusk"),
    ("forest", "Forest"),
];
pub const DEFAULT_THEME: &str = "parchment";
const THEME_STORAGE_KEY: &str = "freenet-idle-theme";

/// Where the unified prefs blob lives in `localStorage`. Theme is
/// kept in its own key (`THEME_STORAGE_KEY`) because the
/// pre-WASM inline script in `index.html` needs to read it
/// without parsing JSON.
const PREFS_STORAGE_KEY: &str = "freenet-idle-prefs";

/// Push the requested theme id to `<html data-theme=...>` and
/// persist the choice in localStorage so reloads keep the player's
/// preference. Falls back silently when window/storage are absent
/// (sandbox iframes, SSR-like contexts).
pub fn apply_theme(theme_id: &str) {
    let Some(window) = web_sys::window() else { return };
    if let Some(doc) = window.document() {
        if let Some(root) = doc.document_element() {
            let _ = root.set_attribute("data-theme", theme_id);
        }
    }
    if let Ok(Some(storage)) = window.local_storage() {
        let _ = storage.set_item(THEME_STORAGE_KEY, theme_id);
    }
}

/// Read the persisted theme id, defaulting to `DEFAULT_THEME` if
/// nothing's stored, the stored value isn't in `THEMES`, or
/// localStorage isn't reachable.
pub fn load_theme() -> String {
    let Some(window) = web_sys::window() else { return DEFAULT_THEME.into() };
    if let Ok(Some(storage)) = window.local_storage() {
        if let Ok(Some(saved)) = storage.get_item(THEME_STORAGE_KEY) {
            if THEMES.iter().any(|(id, _)| *id == saved) {
                return saved;
            }
        }
    }
    DEFAULT_THEME.into()
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
    /// of max HP. `0` = original behavior (only pause at 0 HP).
    #[serde(default)]
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
}

fn default_sync_cadence() -> SyncCadence {
    SyncCadence::Normal
}
fn default_true() -> bool {
    true
}

impl Default for UserPrefs {
    fn default() -> Self {
        Self {
            sync_cadence: default_sync_cadence(),
            auto_pause_hp_pct: 0,
            reactive_publish: true,
            hide_pubkey: false,
            hide_stale_players: false,
            ws_url_override: String::new(),
        }
    }
}

/// Pull prefs from localStorage. Malformed / missing JSON →
/// defaults. Forward compat: missing fields fall to `#[serde(default)]`.
pub fn load_prefs() -> UserPrefs {
    let Some(window) = web_sys::window() else { return UserPrefs::default() };
    let Ok(Some(storage)) = window.local_storage() else { return UserPrefs::default() };
    let Ok(Some(raw)) = storage.get_item(PREFS_STORAGE_KEY) else {
        return UserPrefs::default();
    };
    serde_json::from_str(&raw).unwrap_or_default()
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
        let _ = storage.remove_item(THEME_STORAGE_KEY);
    }
    if let Some(location) = window.location().href().ok() {
        let _ = window.location().set_href(&location);
    }
}
