//! Wire-only types for the Freenet Idle PoC.
//!
//! Split out from `idle-shared` so the 3 deployed contracts
//! (`presence-contract`, `mailbox-contract`, `guilds-contract`)
//! only rebuild when the wire format actually changes — not when
//! game logic in `idle-shared` is edited. See §6.6 in
//! `docs/planned-work-2026-05-17.md` for the rationale: every
//! contract rebuild changes its `code_hash`, which rotates the
//! on-network instance and wipes leaderboard / mailbox / guilds
//! aggregate state.
//!
//! What lives here:
//!   * [`presence`] — `PresencePayload`, `ContractState`, signed
//!     entries, the wire-version gate (`ACCEPTED_PAYLOAD_VERSIONS`).
//!   * [`mailbox`] — `MailboxDelta`/`MailboxState` + caps.
//!   * [`guilds`] — `GuildsDelta`/`GuildsState`, `GuildOp`, helper
//!     `guild_id_from_name`.
//!   * [`types`] — `PubKey`, `SecretsId` aliases, the
//!     `DelegateEnvelope` in/out wrappers, secret-store ids.
//!   * [`bytes`] — `byte_array_32`/`byte_array_64` serde helpers.
//!
//! Flat re-exports below keep `shared::SomeType` callsites working
//! unchanged after the split.

pub mod bytes;
pub mod guilds;
pub mod mailbox;
pub mod presence;
pub mod types;

pub use bytes::{byte_array_32, byte_array_64};
pub use guilds::*;
pub use mailbox::*;
pub use presence::*;
pub use types::*;
