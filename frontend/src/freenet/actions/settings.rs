//! Settings persistence via the delegate's opaque blob protocol.
//!
//! Replaces the typed `LoadUiPrefs` / `SaveUiPrefs` round-trip with a
//! JSON-encoded blob under `BlobKind::Settings`. The delegate never
//! deserializes the payload, so adding or removing a field below is a
//! frontend-only change — no delegate rebuild, no `delegate_key`
//! rotation, no identity loss.
//!
//! All fields are `Option<T>` so `Default` is "no preference"; the UI
//! falls back to its compile-time defaults when a field is `None`.
//! `#[serde(default)]` on the struct means JSON without a given field
//! decodes successfully, which is the load-bearing forward-compat
//! property the bincode-typed protocol lacked.
//!
//! Operations:
//!   * `load_settings_once` — fire on connect, mirror into `Core`.
//!   * `save_settings_once` — fire after every change. Read-modify-
//!     write: caller passes only the overrides; everything else is
//!     pulled from the current `Core` snapshot so a name change
//!     doesn't clobber the theme.

use serde::{Deserialize, Serialize};
use shared::{
    rpc::BlobKind, DelegateRequest as AppRequest, DelegateResponse as AppResponse,
};
use wasm_bindgen_futures::spawn_local;
use yew::UseStateSetter;

use crate::app::i18n::{locale_code, locale_from_code};
use crate::app::prefs::{save_prefs, DEFAULT_NAME};
use crate::delegate_client;
use crate::{now_ms, CoreCell, PendingCell};

/// JSON-encoded settings blob. Field-level forward/backward compat
/// is provided by `#[serde(default)]` (struct-level) plus `Option<T>`
/// per field — missing fields decode to `None`, unknown fields are
/// ignored. Adding a new field is a frontend-only change.
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct Settings {
    pub display_name: Option<String>,
    pub theme: Option<String>,
    pub tutorial_dismissed: Option<bool>,
    pub locale: Option<String>,
    /// Last `CARGO_PKG_VERSION` the player saw acknowledged via the
    /// catchup modal's "Got it" button (B4). Compared against the
    /// current build's version on cold load — a mismatch surfaces
    /// the "What's new" section of the modal so the player sees
    /// changes shipped while they were away.
    pub last_seen_version: Option<String>,
    /// `started_ms` watermark of the most-recently-acknowledged
    /// catchup summary. The modal only re-fires when the delegate
    /// reports a `last_catchup` with a started_ms newer than this
    /// — survives reloads so a dismissed banner stays dismissed
    /// even though the delegate still has the same offline window
    /// in persisted state.
    pub last_catchup_acked_started_ms: Option<u64>,
    /// Auto-mission HP pause threshold (0..=95). Persisted on the
    /// delegate because the sandboxed webapp iframe's localStorage
    /// is null-origin — without this field the picker resets to 0
    /// after every reload.
    pub auto_pause_hp_pct: Option<u8>,
    /// Number-format preference: `"compact"` (default — engineering
    /// suffix), `"full"` (comma-grouped), `"raw"` (digits only).
    pub number_format: Option<String>,
    /// Body font-size scale in percent (50..=200). `None` =
    /// browser default (100%).
    pub font_scale: Option<u8>,
    /// Spoiler-safe mode — when `true`, frontend hides plot /
    /// chapter / endings text so streamers / first-time players
    /// can show their session without spoiling story beats.
    pub spoiler_safe: Option<bool>,
    /// Confirm-before-destructive — when `true`, frontend wraps
    /// Ascend / form-change / sell-all in an extra confirm
    /// dialog. Already true for Reset Progress and Reveal Seed by
    /// default.
    pub confirm_destructive: Option<bool>,
}

/// Load the delegate-persisted settings blob and mirror its fields
/// into `Core`. Called once after the WS handshake completes. A
/// missing blob (first-time player), an `Error` response, or a JSON
/// parse failure all degrade silently to defaults — the UI stays
/// usable on a clean install.
pub fn load_settings_once(
    core: CoreCell,
    pending: PendingCell,
    bump: UseStateSetter<u64>,
) {
    let (ws, delegate_key) = {
        let g = core.borrow();
        let Some(c) = g.as_ref() else { return };
        let Some(ws) = c.ws.clone() else { return };
        (ws, c.delegate_key.clone())
    };
    spawn_local(async move {
        let result = delegate_client::call(
            ws,
            pending,
            &delegate_key,
            AppRequest::LoadBlob {
                kind: BlobKind::Settings,
            },
        )
        .await;
        if let Some(c) = core.borrow_mut().as_mut() {
            match result {
                Ok(AppResponse::Blob { payload: Some(bytes), .. }) => {
                    match serde_json::from_slice::<Settings>(&bytes) {
                        Ok(settings) => apply_settings(c, settings),
                        Err(e) => {
                            web_sys::console::warn_1(
                                &format!("[settings] parse blob: {e}").into(),
                            );
                        }
                    }
                }
                Ok(AppResponse::Blob { payload: None, .. }) => {
                    // First-time player: nothing stored yet. Leave defaults.
                }
                Ok(AppResponse::Error(e)) => {
                    web_sys::console::warn_1(&format!("[settings] LoadBlob: {e}").into());
                }
                Ok(other) => {
                    web_sys::console::warn_1(
                        &format!("[settings] LoadBlob unexpected: {other:?}").into(),
                    );
                }
                Err(e) => {
                    web_sys::console::warn_1(
                        &format!("[settings] LoadBlob transport: {e}").into(),
                    );
                }
            }
            // Flip the gate regardless — see settings::load_settings_once
            // for the reasoning. Worst case: defaults are published once
            // on first heartbeat; benefit: heartbeats unblock now and
            // boot proceeds even when the delegate is misconfigured.
            c.prefs_loaded = true;
        }
        bump.set(now_ms());
    });
}

/// Persist a new name / theme / tutorial-dismissed / locale override.
/// Whatever override is `None` is filled from the latest `Core`
/// snapshot, so changing a single field never clobbers the others.
pub fn save_settings_once(
    core: CoreCell,
    pending: PendingCell,
    bump: UseStateSetter<u64>,
    name_override: Option<String>,
    theme_override: Option<String>,
    tutorial_dismissed_override: Option<bool>,
    locale_override: Option<String>,
    last_seen_version_override: Option<String>,
    last_catchup_acked_override: Option<u64>,
    auto_pause_hp_pct_override: Option<u8>,
) {
    let (ws, delegate_key, payload) = {
        let g = core.borrow();
        let Some(c) = g.as_ref() else { return };
        let Some(ws) = c.ws.clone() else { return };
        let display_name = name_override.or_else(|| {
            if c.name.is_empty() || c.name == DEFAULT_NAME {
                None
            } else {
                Some(c.name.clone())
            }
        });
        let theme = theme_override.or_else(|| Some(c.current_theme.clone()));
        let tutorial_dismissed = tutorial_dismissed_override.or_else(|| {
            if c.onboarding_step.is_none() {
                Some(true)
            } else {
                None
            }
        });
        let locale = locale_override.or_else(|| Some(locale_code(&c.prefs.locale).to_string()));
        let last_seen_version = last_seen_version_override.or_else(|| c.last_seen_version.clone());
        let last_catchup_acked_started_ms = last_catchup_acked_override
            .map(Some)
            .unwrap_or_else(|| Some(c.last_catchup_acked_started_ms));
        let auto_pause_hp_pct = auto_pause_hp_pct_override
            .or(Some(c.prefs.auto_pause_hp_pct));
        // §8 customization (A1/A2/A5/A8). All mirror the current
        // UserPrefs snapshot — overrides come through the same
        // save_settings_once entry; no separate RPC.
        let number_format = Some(c.prefs.number_format.clone());
        let font_scale = Some(c.prefs.font_scale);
        let spoiler_safe = Some(c.prefs.spoiler_safe);
        let confirm_destructive = Some(c.prefs.confirm_destructive);
        let settings = Settings {
            display_name,
            theme,
            tutorial_dismissed,
            locale,
            last_seen_version,
            last_catchup_acked_started_ms,
            auto_pause_hp_pct,
            number_format,
            font_scale,
            spoiler_safe,
            confirm_destructive,
        };
        let payload = match serde_json::to_vec(&settings) {
            Ok(b) => b,
            Err(e) => {
                web_sys::console::warn_1(
                    &format!("[settings] serialize: {e}").into(),
                );
                return;
            }
        };
        (ws, c.delegate_key.clone(), payload)
    };
    spawn_local(async move {
        let result = delegate_client::call(
            ws,
            pending,
            &delegate_key,
            AppRequest::SaveBlob {
                kind: BlobKind::Settings,
                payload,
            },
        )
        .await;
        match result {
            Ok(AppResponse::BlobSaved { .. }) | Ok(AppResponse::Error(_)) => {}
            Ok(other) => {
                web_sys::console::warn_1(
                    &format!("[settings] SaveBlob unexpected: {other:?}").into(),
                );
            }
            Err(e) => {
                web_sys::console::warn_1(
                    &format!("[settings] SaveBlob transport: {e}").into(),
                );
            }
        }
        let _ = bump;
    });
}

/// Apply a freshly-loaded `Settings` to `Core`. Defaults are honored
/// per-field — an empty name doesn't overwrite, an unknown theme is
/// ignored, an unknown locale code is ignored.
fn apply_settings(c: &mut crate::app::Core, settings: Settings) {
    if let Some(name) = settings.display_name {
        if !name.is_empty() {
            c.name = name;
        }
    }
    if let Some(theme) = settings.theme {
        if crate::app::prefs::theme_is_known(&theme) {
            c.current_theme = theme.clone();
            crate::app::prefs::apply_theme(&theme);
        }
    }
    if let Some(code) = settings.locale.as_deref() {
        if crate::app::i18n_loader::is_known(code) {
            c.prefs.locale = locale_from_code(code);
            save_prefs(&c.prefs);
        }
    }
    if matches!(settings.tutorial_dismissed, Some(true)) {
        c.onboarding_step = None;
    }
    if let Some(v) = settings.last_seen_version {
        c.last_seen_version = Some(v);
    }
    if let Some(t) = settings.last_catchup_acked_started_ms {
        c.last_catchup_acked_started_ms = t;
    }
    if let Some(pct) = settings.auto_pause_hp_pct {
        c.prefs.auto_pause_hp_pct = pct;
        save_prefs(&c.prefs);
    }
    if let Some(fmt) = settings.number_format {
        if shared::NumberFormat::from_code(&fmt).is_some() {
            c.prefs.number_format = fmt;
            save_prefs(&c.prefs);
        }
    }
    if let Some(scale) = settings.font_scale {
        let clamped = scale.clamp(50, 200);
        c.prefs.font_scale = clamped;
        crate::app::prefs::apply_font_scale(clamped);
        save_prefs(&c.prefs);
    }
    if let Some(v) = settings.spoiler_safe {
        c.prefs.spoiler_safe = v;
        save_prefs(&c.prefs);
    }
    if let Some(v) = settings.confirm_destructive {
        c.prefs.confirm_destructive = v;
        save_prefs(&c.prefs);
    }
}
