//! Mailbox contract — durable player-to-player message log.
//! Substrate for gifts, guild invites, trade offers, chat; the
//! mailbox contract itself is payload-agnostic.

use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};

use super::bytes::{byte_array_32, byte_array_64};
use super::presence::MAX_TIMESTAMP_MS;
use super::{PubKey, SIG_LEN};

/// Schema version of `MessagePayload`. Same forward-compat hook as
/// `PRESENCE_PAYLOAD_VERSION`.
pub const MESSAGE_PAYLOAD_VERSION: u8 = 1;
pub const MAILBOX_STATE_VERSION: u8 = 1;

pub const MAX_MESSAGE_BODY_BYTES: usize = 512;
pub const MAX_MAILBOX_PAYLOAD_BYTES: usize = 768;

/// Live message cap on the mailbox contract — beyond this an arrival
/// triggers eviction of the oldest message. Higher than presence
/// because the mailbox is durable communication, not heartbeats.
pub const MAX_MAILBOX_MESSAGES: usize = 5_000;

/// How long mailbox messages live before being pruned. 7 days at
/// the prune pivot's resolution — way longer than presence (60 s),
/// because a player should still get a gift sent while they slept.
pub const MAILBOX_MAX_STALE_MS: u64 = 7 * 24 * 60 * 60 * 1000;

/// Tagged payload kinds. Lets one mailbox carry many *uses*
/// (gifts, guild invites, trade offers, chat) without forking the
/// contract. Recipient-side dispatch reads the tag and parses
/// `body` accordingly.
pub const MSG_KIND_CHAT: u8 = 0;
pub const MSG_KIND_GIFT: u8 = 1;
pub const MSG_KIND_GUILD_INVITE: u8 = 2;
pub const MSG_KIND_TRADE_OFFER: u8 = 3;

/// What gets bincode-serialized into `MailboxEntry::payload` and
/// signed by the sender's identity key. Recipient is plain text;
/// privacy is left to upstream features (e.g. NaCl-encrypted body).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MessagePayload {
    pub version: u8,
    #[serde(with = "byte_array_32")]
    pub from: PubKey,
    #[serde(with = "byte_array_32")]
    pub to: PubKey,
    pub kind: u8,
    pub body: Vec<u8>,
    pub timestamp_ms: u64,
}

impl MessagePayload {
    pub fn new(from: PubKey, to: PubKey, kind: u8, body: Vec<u8>, timestamp_ms: u64) -> Self {
        Self {
            version: MESSAGE_PAYLOAD_VERSION,
            from,
            to,
            kind,
            body,
            timestamp_ms,
        }
    }
}

/// One signed entry on the mailbox contract. Shape mirrors
/// `SignedEntry` from presence — opaque `payload` (bincode'd
/// `MessagePayload`) plus the sender's signature over those bytes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MailboxEntry {
    #[serde(with = "serde_bytes")]
    pub payload: Vec<u8>,
    #[serde(with = "byte_array_64")]
    pub signature: [u8; SIG_LEN],
}

impl MailboxEntry {
    pub fn decode(&self) -> Option<MessagePayload> {
        bincode::deserialize(&self.payload).ok()
    }
    pub fn verify(&self) -> Result<MessagePayload, &'static str> {
        let payload: MessagePayload =
            bincode::deserialize(&self.payload).map_err(|_| "deserialize")?;
        let vk = VerifyingKey::from_bytes(&payload.from).map_err(|_| "bad pubkey")?;
        let sig = Signature::from_bytes(&self.signature);
        vk.verify(&self.payload, &sig).map_err(|_| "bad signature")?;
        Ok(payload)
    }
}

/// V1 of the mailbox contract state. Same wrapper-chain pattern as
/// [`crate::presence::ContractStateV1`] — additive bumps land as
/// `MailboxStateV2 { base: V1, new_field }`. Public alias
/// `MailboxState` tracks the latest.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MailboxStateV1 {
    pub version: u8,
    /// Flat log — order isn't meaningful since each entry has its
    /// own timestamp. Per-recipient filtering is a recipient-side
    /// concern, not the contract's.
    pub entries: Vec<MailboxEntry>,
}

/// Latest `MailboxState`. Bump this alias when a new version lands.
pub type MailboxState = MailboxStateV1;

impl Default for MailboxStateV1 {
    fn default() -> Self {
        Self {
            version: MAILBOX_STATE_VERSION,
            entries: Vec::new(),
        }
    }
}

impl MailboxStateV1 {
    /// Append one entry if it verifies and is within size/timestamp
    /// bounds. Returns true if accepted. Duplicate-detection is by
    /// `(from, to, timestamp_ms)` — the sender's monotonic clock is
    /// the natural ID, no extra fields needed.
    pub fn apply(&mut self, entry: MailboxEntry) -> bool {
        if entry.payload.len() > MAX_MAILBOX_PAYLOAD_BYTES {
            return false;
        }
        let payload = match entry.verify() {
            Ok(p) => p,
            Err(_) => return false,
        };
        if payload.version != MESSAGE_PAYLOAD_VERSION {
            return false;
        }
        if payload.body.len() > MAX_MESSAGE_BODY_BYTES {
            return false;
        }
        if payload.timestamp_ms > MAX_TIMESTAMP_MS {
            return false;
        }
        // Cheap dedup: scan existing entries and skip if a record
        // with the same (from, to, ts) already lives. Mailbox is
        // small enough that linear scan is fine.
        for existing in self.entries.iter() {
            if let Some(p) = existing.decode() {
                if p.from == payload.from
                    && p.to == payload.to
                    && p.timestamp_ms == payload.timestamp_ms
                {
                    return false;
                }
            }
        }
        // Cap enforcement: evict the oldest if full. Sorted by ts
        // ascending means the head is oldest. Linear scan again.
        if self.entries.len() >= MAX_MAILBOX_MESSAGES {
            if let Some(oldest_idx) = self.oldest_entry_index() {
                self.entries.remove(oldest_idx);
            }
        }
        self.entries.push(entry);
        true
    }

    fn oldest_entry_index(&self) -> Option<usize> {
        self.entries
            .iter()
            .enumerate()
            .filter_map(|(i, e)| e.decode().map(|p| (i, p.timestamp_ms)))
            .min_by_key(|(_, ts)| *ts)
            .map(|(i, _)| i)
    }

    /// Drop entries older than `MAILBOX_MAX_STALE_MS` behind the
    /// newest. Mirrors presence prune logic but with a much longer
    /// window — messages aren't ephemeral.
    pub fn prune_stale(&mut self) {
        let Some(newest) = self
            .entries
            .iter()
            .filter_map(|e| e.decode().map(|p| p.timestamp_ms))
            .max()
        else {
            return;
        };
        let cutoff = newest.saturating_sub(MAILBOX_MAX_STALE_MS);
        self.entries.retain(|e| match e.decode() {
            Some(p) => p.timestamp_ms >= cutoff,
            None => false,
        });
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MailboxDelta {
    pub entries: Vec<MailboxEntry>,
}
