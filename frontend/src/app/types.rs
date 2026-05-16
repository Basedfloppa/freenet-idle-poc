//! UI-only enums and the toast notification record. Pure data —
//! no behaviour besides the trivial constructors.

/// Top-level UI section. Each variant maps to one tab button + one
/// content view; switching tabs hides every other view. The Farm tab
/// is the default — it's the main play surface (hero, mission scene,
/// equipment, plot, boss HP, resources). Other tabs are auxiliary
/// tools.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Farm,
    WorldMap,
    Shop,
    Guilds,
    Achievements,
    /// Permanent-upgrades home (Legacy stars, Routine auto-hire,
    /// Insight nodes, Tokens, World Boss attack). Used to live
    /// in Settings — moved out because long-term progression and
    /// volatile UI prefs don't belong on the same tab.
    Mastery,
    Settings,
    Help,
}

/// Boolean prefs the Settings UI exposes as on/off toggles. Lives in
/// one enum so a single closure factory (`mk_toggle_cb`) covers all
/// of them — no copy-pasted `core.borrow_mut() … save_prefs(…)`
/// boilerplate per toggle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToggleField {
    ReactivePublish,
    HidePubkey,
    HideStale,
}

/// One toast notification. Rendered as a corner banner; the
/// unified tick prunes entries older than `TOAST_TTL_MS`.
#[derive(Debug, Clone)]
pub struct Toast {
    pub label: String,
    pub body: String,
    pub created_ms: u64,
}

/// How long an unlock toast stays on screen.
pub const TOAST_TTL_MS: u64 = 6_000;
/// Cap on the in-RAM toast queue. Beyond this we drop the oldest
/// rather than letting the array grow unboundedly (e.g. if 20
/// achievements unlock in one tick from a long offline catch-up).
pub const MAX_TOASTS: usize = 8;
