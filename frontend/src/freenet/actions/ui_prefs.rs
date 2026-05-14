//! Display name + theme persistence via the delegate.
//!
//! The sandboxed webapp iframe has a "null" document origin (no
//! `allow-same-origin` token in the default sandbox), so localStorage
//! writes don't survive a reload. The delegate is the only place
//! that *can* persist UI prefs across browser sessions, so they live
//! next to the inventory in the secret store.
//!
//! Two operations:
//!   * `load_ui_prefs_once` — fire on connect, mirror into `Core`.
//!   * `save_ui_prefs_once` — fire after every change (name keystroke
//!     or theme click). Read-modify-write semantics: whatever fields
//!     the caller doesn't override are pulled from the current Core
//!     snapshot so a name change doesn't clobber the theme and vice
//!     versa.

use shared::{DelegateRequest as AppRequest, DelegateResponse as AppResponse, UiPrefs};
use wasm_bindgen_futures::spawn_local;
use yew::UseStateSetter;

use crate::app::prefs::DEFAULT_NAME;
use crate::delegate_client;
use crate::{now_ms, CoreCell, PendingCell};

/// Read the delegate's persisted prefs and mirror them into
/// `Core.name` / `Core.current_theme`. Called once after
/// `LoadInventory` succeeds. Failures are silent — the UI keeps its
/// existing defaults so the player can still play.
pub fn load_ui_prefs_once(
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
        let result =
            delegate_client::call(ws, pending, &delegate_key, AppRequest::LoadUiPrefs).await;
        if let Some(c) = core.borrow_mut().as_mut() {
            match result {
                Ok(AppResponse::UiPrefs(prefs)) => {
                    if let Some(name) = prefs.display_name {
                        if !name.is_empty() {
                            c.name = name;
                        }
                    }
                    if let Some(theme) = prefs.theme {
                        if crate::app::prefs::THEMES.iter().any(|(id, _)| *id == theme) {
                            c.current_theme = theme.clone();
                            crate::app::prefs::apply_theme(&theme);
                        }
                    }
                    // Returning players have `tutorial_dismissed = Some(true)`;
                    // close the wizard before the user notices it
                    // flashed open. New players have `None` →
                    // wizard stays at step 0.
                    if matches!(prefs.tutorial_dismissed, Some(true)) {
                        c.onboarding_step = None;
                    }
                }
                Ok(AppResponse::Error(e)) => {
                    web_sys::console::warn_1(&format!("LoadUiPrefs: {e}").into());
                }
                Ok(other) => {
                    web_sys::console::warn_1(
                        &format!("LoadUiPrefs unexpected: {other:?}").into(),
                    );
                }
                Err(e) => {
                    web_sys::console::warn_1(&format!("LoadUiPrefs transport: {e}").into());
                }
            }
            // Flip the gate regardless of success/error/missing data:
            // worst case is that the player has never set a name
            // (default kicks in) and we publish `DEFAULT_NAME`; but
            // we publish it ONCE on the first heartbeat instead of
            // racing every reload. Heartbeats are blocked until this
            // runs.
            c.prefs_loaded = true;
        }
        bump.set(now_ms());
    });
}

/// Persist a new name / theme / tutorial-dismissed flag. Whatever
/// override is `None` is filled from the latest Core snapshot, so
/// changing a single field never clobbers the others (the delegate
/// rewrites the whole blob each call).
pub fn save_ui_prefs_once(
    core: CoreCell,
    pending: PendingCell,
    bump: UseStateSetter<u64>,
    name_override: Option<String>,
    theme_override: Option<String>,
    tutorial_dismissed_override: Option<bool>,
) {
    let (ws, delegate_key, prefs) = {
        let g = core.borrow();
        let Some(c) = g.as_ref() else { return };
        let Some(ws) = c.ws.clone() else { return };
        let display_name = name_override
            .or_else(|| {
                if c.name.is_empty() || c.name == DEFAULT_NAME {
                    None
                } else {
                    Some(c.name.clone())
                }
            });
        let theme = theme_override.or_else(|| Some(c.current_theme.clone()));
        // tutorial state: caller's explicit override wins; otherwise
        // mirror what Core currently shows. `onboarding_step.is_none()`
        // = the wizard isn't on screen = the player has either
        // completed it or never opened it. Distinguishing those is
        // why callers SHOULD pass an explicit override on
        // skip/finish; the implicit path here is a best-effort
        // fallback.
        let tutorial_dismissed = tutorial_dismissed_override.or_else(|| {
            if c.onboarding_step.is_none() {
                Some(true)
            } else {
                None
            }
        });
        (
            ws,
            c.delegate_key.clone(),
            UiPrefs {
                display_name,
                theme,
                tutorial_dismissed,
            },
        )
    };
    spawn_local(async move {
        let result = delegate_client::call(
            ws,
            pending,
            &delegate_key,
            AppRequest::SaveUiPrefs { prefs },
        )
        .await;
        match result {
            Ok(AppResponse::UiPrefs(_)) | Ok(AppResponse::Error(_)) => {}
            Ok(other) => {
                web_sys::console::warn_1(
                    &format!("SaveUiPrefs unexpected: {other:?}").into(),
                );
            }
            Err(e) => {
                web_sys::console::warn_1(&format!("SaveUiPrefs transport: {e}").into());
            }
        }
        let _ = bump;
    });
}
