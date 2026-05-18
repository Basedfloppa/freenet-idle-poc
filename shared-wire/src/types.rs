//! Cross-cutting wire-only types — pubkey / signature length
//! aliases. Used by every signed wire structure (PresencePayload,
//! mailbox envelopes, guild ops).
//!
//! The delegate-envelope wrappers (`DelegateEnvelopeIn`/`Out`) and
//! secret-store id constants live in `idle-shared` instead — they
//! straddle the delegate↔webapp RPC plane, which is not part of
//! the contract surface. Putting them here would force the wire
//! crate to depend on game-side `rpc::DelegateRequest`/`Response`
//! (the envelope's payload type) and pull the whole game module
//! tree back in.

pub const PUBKEY_LEN: usize = 32;
pub const SIG_LEN: usize = 64;
pub type PubKey = [u8; PUBKEY_LEN];
