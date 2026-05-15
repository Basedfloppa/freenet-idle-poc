//! Async client for the identity delegate.
//!
//! Tags each request with an 8-byte little-endian request id stored
//! in the `DelegateContext`. The delegate echoes context back
//! unchanged, so the result_handler can route each response back to
//! the awaiting future via a per-id oneshot in `Pending`.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use freenet_stdlib::client_api::{ClientRequest, DelegateRequest};
use freenet_stdlib::prelude::{
    ApplicationMessage, CodeHash, DelegateContainer, DelegateKey, InboundDelegateMsg, Parameters,
};
use shared::{
    DelegateEnvelopeIn, DelegateEnvelopeOut, DelegateRequest as AppRequest,
    DelegateResponse as AppResponse,
};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::js_sys::{ArrayBuffer, Uint8Array};
use web_sys::Response;

use crate::ws_shim::WsShim;

pub type WsCell = Rc<RefCell<WsShim>>;

#[derive(Default)]
pub struct Pending {
    next_id: u64,
    awaiting: HashMap<u64, OneshotTx>,
}

impl Pending {
    pub fn new_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        id
    }

    pub fn register(&mut self, id: u64, tx: OneshotTx) {
        self.awaiting.insert(id, tx);
    }

    /// Fail every awaiting future with the same reason. Called by the
    /// WS result router when the node returns a top-level
    /// `ClientError` (no per-tx routing info), so the awaiting UI
    /// surfaces the actual cause instead of hanging on the status
    /// "asking delegate for identity…" forever.
    pub fn fail_all(&mut self, reason: String) {
        let drained: Vec<(u64, OneshotTx)> = self.awaiting.drain().collect();
        for (id, tx) in drained {
            web_sys::console::warn_1(
                &format!("[delegate-client] failing id={id}: {reason}").into(),
            );
            let _ = tx.fail(reason.clone());
        }
    }

    /// Called from the WS result handler when a `DelegateResponse`
    /// frame arrives. Decodes the embedded `AppResponse` and routes
    /// it to whichever future is awaiting this request id.
    /// Decodes an outbound delegate `ApplicationMessage` payload as
    /// `DelegateEnvelopeOut` and dispatches the response to the
    /// awaiting future. Context bytes are unused — the node wipes
    /// them — so the id travels inside `payload`.
    pub fn deliver(&mut self, payload: &[u8]) {
        let envelope: DelegateEnvelopeOut = match bincode::deserialize(payload) {
            Ok(e) => e,
            Err(e) => {
                web_sys::console::warn_1(
                    &format!("[delegate-client] bad envelope: {e}").into(),
                );
                return;
            }
        };
        let Some(tx) = self.awaiting.remove(&envelope.request_id) else {
            web_sys::console::warn_1(
                &format!(
                    "[delegate-client] no pending entry for id={}",
                    envelope.request_id
                )
                .into(),
            );
            return;
        };
        let _ = tx.send(envelope.response);
    }
}

pub fn parse_delegate_key(key_b58: &str, code_hash_b58: &str) -> Result<DelegateKey, String> {
    let key_bytes = bs58::decode(key_b58)
        .with_alphabet(bs58::Alphabet::BITCOIN)
        .into_vec()
        .map_err(|e| format!("bad delegate key base58: {e}"))?;
    let key_arr: [u8; 32] = key_bytes
        .as_slice()
        .try_into()
        .map_err(|_| "delegate key must be 32 bytes".to_string())?;
    let code_hash_bytes = bs58::decode(code_hash_b58)
        .with_alphabet(bs58::Alphabet::BITCOIN)
        .into_vec()
        .map_err(|e| format!("bad code hash base58: {e}"))?;
    let code_arr: [u8; 32] = code_hash_bytes
        .as_slice()
        .try_into()
        .map_err(|_| "delegate code hash must be 32 bytes".to_string())?;
    Ok(DelegateKey::new(key_arr, CodeHash::from(&code_arr)))
}

/// Fetch the versioned delegate WASM that `dev-publish.sh` /
/// `prod-publish.sh` staged into `frontend/dist/` and register it on
/// whichever node is currently serving the page.
///
/// Delegates are NOT replicated through the DHT — only contracts are
/// — so a self-hosted user who opens the webapp on their own fresh
/// node would otherwise hang forever on "asking delegate for
/// identity…" because the delegate they're trying to call doesn't
/// exist locally. This function closes that gap.
///
/// Called from the **probe-failed** path in `reconnect.rs`: a probe
/// `GetPubkey` is sent first, and only on its failure (typical
/// missing-delegate symptom) does the caller invoke this function
/// and retry. Steady-state nodes that already have the delegate
/// pay zero overhead — no fetch, no register, no extra WS round-
/// trip per (re)connect.
///
/// Fire-and-forget at the WS layer: `WsShim::send` returns once the
/// bytes are on the socket; we do not await a response frame. The
/// node processes WS messages from a single client in arrival order,
/// so the `GetPubkey` retry the caller sends next is guaranteed to
/// see the registration completed first. If the WS pipeline ever
/// switches to out-of-order delivery, this assumption breaks and
/// we'd need an explicit ACK channel — flagged here so the
/// invariant is reviewable.
///
/// Trunk dev mode falls through: when the page is served on
/// `:9003` and the operator hasn't run `dev-publish.sh`, the asset is
/// absent and the fetch returns 404. We log + skip, deferring to the
/// existing manual-publish flow (`fdev publish ... delegate`) that
/// dev workflow already requires.
pub async fn ensure_delegate_registered(ws: WsCell) -> Result<(), String> {
    let bytes = match fetch_bundled_delegate_wasm().await {
        Ok(b) => b,
        Err(e) => {
            // Don't fail the whole connect — let the existing GetPubkey
            // path surface a node-side "delegate not found" error if the
            // node truly doesn't have it. This keeps trunk dev (asset
            // absent) and "operator already registered via fdev" paths
            // working identically.
            web_sys::console::warn_1(
                &format!("[delegate-register] skipping auto-register: {e}").into(),
            );
            return Ok(());
        }
    };

    // The on-disk artefact written by `fdev build --package-type
    // delegate` is the versioned encoding (8-byte BE APIVersion +
    // 32-byte code hash + raw wasm). `DelegateContainer::try_from`
    // for `(Vec<u8>, Parameters)` parses exactly that framing, so
    // we don't need to strip or reconstruct the header here. The
    // empty Parameters mirrors `fdev publish delegate` — the
    // delegate's identity is `hash(wasm) + hash(params)`, and
    // idle-poc registers under empty params on every node.
    let params: Parameters<'static> = Parameters::from(Vec::<u8>::new());
    let container = DelegateContainer::try_from((bytes, &params))
        .map_err(|e| format!("decode versioned delegate: {e}"))?;

    let req = ClientRequest::DelegateOp(DelegateRequest::RegisterDelegate {
        delegate: container,
        cipher: DelegateRequest::DEFAULT_CIPHER,
        nonce: DelegateRequest::DEFAULT_NONCE,
    });

    ws.borrow_mut()
        .send(req)
        .await
        .map_err(|e| format!("ws send register: {e:?}"))?;
    web_sys::console::log_1(
        &"[delegate-register] RegisterDelegate sent (fire-and-forget)".into(),
    );
    Ok(())
}

async fn fetch_bundled_delegate_wasm() -> Result<Vec<u8>, String> {
    let win = web_sys::window().ok_or("no window")?;
    let resp_val = JsFuture::from(win.fetch_with_str("./identity_delegate.wasm"))
        .await
        .map_err(|e| format!("fetch: {e:?}"))?;
    let response: Response = resp_val
        .dyn_into()
        .map_err(|_| "not a Response".to_string())?;
    if !response.ok() {
        return Err(format!("HTTP {}", response.status()));
    }
    let buf_promise = response
        .array_buffer()
        .map_err(|e| format!("array_buffer(): {e:?}"))?;
    let buf_val = JsFuture::from(buf_promise)
        .await
        .map_err(|e| format!("array_buffer body: {e:?}"))?;
    let buf: ArrayBuffer = buf_val
        .dyn_into()
        .map_err(|_| "not an ArrayBuffer".to_string())?;
    Ok(Uint8Array::new(&buf).to_vec())
}

pub async fn call(
    ws: WsCell,
    pending: Rc<RefCell<Pending>>,
    key: &DelegateKey,
    request: AppRequest,
) -> Result<AppResponse, String> {
    let id = pending.borrow_mut().new_id();
    let (tx, rx) = oneshot();
    pending.borrow_mut().register(id, tx);

    let envelope = DelegateEnvelopeIn { request_id: id, request };
    let payload =
        bincode::serialize(&envelope).map_err(|e| format!("ser envelope: {e}"))?;
    let app_msg = ApplicationMessage::new(payload).processed(false);

    let req = ClientRequest::DelegateOp(DelegateRequest::ApplicationMessages {
        key: key.clone(),
        params: Parameters::from(Vec::<u8>::new()),
        inbound: vec![InboundDelegateMsg::ApplicationMessage(app_msg)],
    });

    ws.borrow_mut()
        .send(req)
        .await
        .map_err(|e| format!("ws send: {e:?}"))?;

    rx.await
}

// --- Oneshot tailored for AppResponse -----------------------------
//
// `OneshotState::value` holds the full `Result` so the channel can
// resolve with either a delivered `AppResponse` (Ok) or a fail reason
// (Err). Three completion paths:
//   1. `send()`           — happy path, response decoded from node.
//   2. `fail()`           — caller surfaces a known error (e.g.
//                            `ClientError` arriving on the WS).
//   3. `Drop for Tx`      — Tx leaves scope without resolving (the
//                            `Pending` map is rebuilt mid-call on WS
//                            reconnect → in-flight awaiters used to
//                            hang forever; now they get a "cancelled"
//                            Err and bubble up to the UI status).

fn oneshot() -> (OneshotTx, OneshotRx) {
    let inner = Rc::new(RefCell::new(OneshotState::default()));
    (OneshotTx { inner: inner.clone() }, OneshotRx { inner })
}

#[derive(Default)]
struct OneshotState {
    value: Option<Result<AppResponse, String>>,
    waker: Option<std::task::Waker>,
}

pub struct OneshotTx {
    inner: Rc<RefCell<OneshotState>>,
}
impl OneshotTx {
    fn send(self, v: AppResponse) -> Result<(), ()> {
        self.resolve(Ok(v))
    }

    fn fail(self, reason: String) -> Result<(), ()> {
        self.resolve(Err(reason))
    }

    fn resolve(self, v: Result<AppResponse, String>) -> Result<(), ()> {
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

impl Drop for OneshotTx {
    fn drop(&mut self) {
        let mut i = self.inner.borrow_mut();
        if i.value.is_some() {
            return;
        }
        i.value = Some(Err(
            "delegate call cancelled (WS reconnect or shutdown)".to_string(),
        ));
        if let Some(w) = i.waker.take() {
            w.wake();
        }
    }
}

struct OneshotRx {
    inner: Rc<RefCell<OneshotState>>,
}
impl std::future::Future for OneshotRx {
    type Output = Result<AppResponse, String>;
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
