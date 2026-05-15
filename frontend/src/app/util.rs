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

/// Webapp contract id parsed out of `window.location.pathname` when the
/// page is served from a freenet gateway under
/// `/v1/contract/web/<id>/...`. Returns `None` for trunk dev (the URL
/// is just `/`) or any other off-gateway host. The id is the same one
/// captured in `frontend/prod-webapp-id.txt` at publish time and acts
/// as the load-bearing identifier for which build of the webapp is
/// currently being served — `fdev website publish` rotates it on every
/// re-publish (and on every contracts/delegate change), so showing it
/// in the UI gives the user a visible cue when the running version
/// shifts.
pub fn webapp_contract_id() -> Option<String> {
    let win = web_sys::window()?;
    let pathname = win.location().pathname().ok()?;
    let marker = "/v1/contract/web/";
    let start = pathname.find(marker)? + marker.len();
    let rest = &pathname[start..];
    let end = rest.find('/').unwrap_or(rest.len());
    let id = rest[..end].trim();
    if id.is_empty() {
        None
    } else {
        Some(id.to_string())
    }
}

pub fn ws_url() -> String {
    // Priority:
    //   1. `?ws=…` query param  — shareable explicit override
    //   2. prefs.ws_url_override — user-configured in Settings
    //   3. same-host derivation when served from a freenet gateway
    //      (URL path contains `/v1/contract/web/`): the node that
    //      handed us this HTML is exposing its WS API on the same
    //      host/port, so default to that.
    //   4. DEFAULT_WS — `ws://127.0.0.1:7509/...`, used for trunk
    //      dev mode (page on :9003) and any other off-gateway host.
    if let Some(win) = web_sys::window() {
        let location = win.location();
        if let Ok(search) = location.search() {
            if let Some(start) = search.find("ws=") {
                let rest = &search[start + 3..];
                let end = rest.find('&').unwrap_or(rest.len());
                return rest[..end].to_string();
            }
        }
        let prefs = load_prefs();
        if !prefs.ws_url_override.is_empty() {
            return prefs.ws_url_override;
        }
        if let Some(derived) = derive_same_host_ws(&location) {
            return derived;
        }
        return DEFAULT_WS.to_string();
    }
    DEFAULT_WS.to_string()
}

/// Build `ws[s]://<host>/v1/contract/command?encodingProtocol=native`
/// from `window.location` IF the current page was served from a
/// freenet gateway path (`…/v1/contract/web/<id>/…`). Returns `None`
/// otherwise — trunk's dev server, file://, or anything else falls
/// through to `DEFAULT_WS` so local dev keeps hitting `:7509`.
fn derive_same_host_ws(location: &web_sys::Location) -> Option<String> {
    let pathname = location.pathname().ok()?;
    if !pathname.contains("/v1/contract/web/") {
        return None;
    }
    let host = location.host().ok().filter(|h| !h.is_empty())?;
    let scheme = match location.protocol().ok().as_deref() {
        Some("https:") => "wss",
        _ => "ws",
    };
    Some(format!(
        "{scheme}://{host}/v1/contract/command?encodingProtocol=native"
    ))
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
