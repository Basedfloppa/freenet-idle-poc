//! Delegate-side wire wrappers + secret-store ids. Lives in
//! `idle-shared` (not `idle-shared-wire`) because the envelopes
//! reference `crate::rpc::DelegateRequest`/`DelegateResponse`,
//! and `rpc.rs` carries Inventory + game enums that have no
//! business on the contract side.
//!
//! Contracts never see this — they only deserialize their own
//! `ContractState` / `MailboxState` / `GuildsState`.

use serde::{Deserialize, Serialize};

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
/// UI-only persistence (display name, theme id, future cosmetic
/// prefs). Separate from inventory so a schema bump on either side
/// doesn't reset the other. The delegate stores a bincode'd
/// [`crate::rpc::UiPrefs`] under this key.
///
/// **Legacy.** The new path is `BLOB_SECRET_ID_SETTINGS` (opaque JSON)
/// — see `crate::rpc::BlobKind`. This key is retained so the new
/// delegate can serve a one-time migration read for users who saved
/// under it before the blob protocol existed.
pub const UI_PREFS_SECRET_ID: &[u8] = b"ui-prefs-v1";

/// Per-domain secret ids for the blob protocol (`crate::rpc::BlobKind`).
/// Names match the enum discriminants so the mapping is obvious; the
/// `-v1` suffix lets us re-key a domain if we ever need a hard reset
/// of just that slice without rotating the whole delegate.
pub const BLOB_SECRET_ID_SETTINGS: &[u8] = b"blob/settings-v1";
pub const BLOB_SECRET_ID_GAMESTATE: &[u8] = b"blob/gamestate-v1";
pub const BLOB_SECRET_ID_CHARACTER: &[u8] = b"blob/character-v1";
pub const BLOB_SECRET_ID_INVENTORY: &[u8] = b"blob/inventory-v1";
