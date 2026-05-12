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
    ApplicationMessage, CodeHash, DelegateKey, InboundDelegateMsg, Parameters,
};
use shared::{
    DelegateEnvelopeIn, DelegateEnvelopeOut, DelegateRequest as AppRequest,
    DelegateResponse as AppResponse,
};

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

fn oneshot() -> (OneshotTx, OneshotRx) {
    let inner = Rc::new(RefCell::new(OneshotState::default()));
    (OneshotTx { inner: inner.clone() }, OneshotRx { inner })
}

#[derive(Default)]
struct OneshotState {
    value: Option<AppResponse>,
    waker: Option<std::task::Waker>,
}

pub struct OneshotTx {
    inner: Rc<RefCell<OneshotState>>,
}
impl OneshotTx {
    fn send(self, v: AppResponse) -> Result<(), ()> {
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
            std::task::Poll::Ready(Ok(v))
        } else {
            i.waker = Some(cx.waker().clone());
            std::task::Poll::Pending
        }
    }
}
