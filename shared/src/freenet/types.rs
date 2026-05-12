//! Cross-cutting types — pubkey/signature length aliases, the
//! delegate envelope that wraps every `DelegateRequest`/`Response`,
//! and the secret-store ids the delegate uses internally.

use serde::{Deserialize, Serialize};

pub const PUBKEY_LEN: usize = 32;
pub const SIG_LEN: usize = 64;
pub type PubKey = [u8; PUBKEY_LEN];

/// Envelope wrapping a `DelegateRequest` with a webapp-chosen
/// `request_id` so responses can be correlated. We carry the id in
/// the payload (not in the `DelegateContext`) because the node wipes
/// context on the response leg — see
/// `freenet-core/crates/core/src/wasm_runtime/delegate.rs:351`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegateEnvelopeIn {
    pub request_id: u64,
    pub request: crate::rpc::DelegateRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegateEnvelopeOut {
    pub request_id: u64,
    pub response: crate::rpc::DelegateResponse,
}

/// Keys used inside the delegate's secrets store. All live in the
/// same delegate namespace so they share the same trust boundary —
/// access one == access them all.
pub const IDENTITY_SECRET_ID: &[u8] = b"identity-seed-v1";
/// Bumped on every breaking change to `Inventory`'s bincode layout.
/// See `crate::lib` docs for the full ladder. v9: added
/// `auto_run_enabled`, `auto_last_tick_ms`, `last_catchup` for
/// offline auto-mission progression.
pub const INVENTORY_SECRET_ID: &[u8] = b"inventory-v9";
