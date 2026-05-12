//! Small leaf-helpers shared across the rest of the app: timing
//! constants used by the unified poll tick, the WS URL resolver, a
//! UTF-8-safe truncator, a pubkey shortener for display, the
//! browser clock, and a single-shot future used by `connect_inner`.

use std::cell::RefCell;
use std::rc::Rc;

use shared::PubKey;

use super::prefs::load_prefs;

pub const DEFAULT_WS: &str =
    "ws://127.0.0.1:7509/v1/contract/command?encodingProtocol=native";

/// Frequency at which the unified polling tick wakes up. Auto-mission,
/// heartbeat and pull-refresh all use the *same* `Interval` and check
/// elapsed time against their own cadence (sourced from `UserPrefs`).
/// One Interval → cadence preferences take effect on the next tick
/// without recreating timers, and the wall-clock checks degrade
/// gracefully if the tab is throttled.
pub const POLL_TICK_MS: u32 = 1_000;
/// Hard floor on auto-mission cadence. The "Reflexes" upgrade ladder
/// will eventually let players buy a faster tick, but until then this
/// is the only auto-mission interval the webapp honors.
pub const AUTO_TICK_MS: u64 = 1_000;
/// Minimum gap between reactive publishes triggered by mission
/// completion. A burst of clicks (or one auto-run second of
/// missions) collapses into a single publish; the heartbeat covers
/// the rest.
pub const REACTIVE_PUBLISH_MIN_MS: u64 = 3_000;
/// Reconnect backoff schedule when the WebSocket drops. Indexed by
/// retry count, capped at the last value.
pub const WS_RECONNECT_BACKOFF_MS: &[u32] = &[500, 1_000, 2_000, 5_000, 10_000];

pub fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        s.chars().take(max).collect()
    }
}

pub fn short_id(pk: &PubKey) -> String {
    let h = hex::encode(&pk[..4]);
    format!("anon-{h}")
}

pub fn now_ms() -> u64 {
    js_sys::Date::now() as u64
}

pub fn ws_url() -> String {
    // Priority: `?ws=…` query param > prefs.ws_url_override > default.
    // The query param wins so a stale prefs entry can't override an
    // explicit URL chosen via shareable link.
    if let Some(win) = web_sys::window() {
        if let Ok(search) = win.location().search() {
            if let Some(start) = search.find("ws=") {
                let rest = &search[start + 3..];
                let end = rest.find('&').unwrap_or(rest.len());
                return rest[..end].to_string();
            }
        }
    }
    let prefs = load_prefs();
    if !prefs.ws_url_override.is_empty() {
        return prefs.ws_url_override;
    }
    DEFAULT_WS.to_string()
}

// --- minimal oneshot<Result<(), String>> for the connect-open path ---

pub fn oneshot() -> (OneshotTx, OneshotRx) {
    let inner = Rc::new(RefCell::new(OneshotState::default()));
    (OneshotTx { inner: inner.clone() }, OneshotRx { inner })
}

#[derive(Default)]
pub struct OneshotState {
    value: Option<Result<(), String>>,
    waker: Option<std::task::Waker>,
}

#[derive(Clone)]
pub struct OneshotTx {
    inner: Rc<RefCell<OneshotState>>,
}
impl OneshotTx {
    pub fn send(&self, v: Result<(), String>) -> Result<(), ()> {
        let mut i = self.inner.borrow_mut();
        if i.value.is_some() {
            return Err(());
        }
        i.value = Some(v);
        if let Some(w) = i.waker.take() {
            w.wake();
        }
        Ok(())
    }
}

pub struct OneshotRx {
    inner: Rc<RefCell<OneshotState>>,
}
impl std::future::Future for OneshotRx {
    type Output = Result<(), String>;
    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let mut i = self.inner.borrow_mut();
        if let Some(v) = i.value.take() {
            std::task::Poll::Ready(v)
        } else {
            i.waker = Some(cx.waker().clone());
            std::task::Poll::Pending
        }
    }
}
