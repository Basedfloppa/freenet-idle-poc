//! Mailbox contract — a payload-agnostic signed message log.
//!
//! Each entry is a `MessagePayload` (from / to / kind / body / ts)
//! signed by the sender's identity key. The contract validates
//! signatures + size bounds, prunes entries older than
//! `MAILBOX_MAX_STALE_MS`, and caps active entries at
//! `MAX_MAILBOX_MESSAGES`. It does NOT route — recipients filter
//! `to == my_pubkey` client-side. Multiple recipients on the same
//! contract is fine; mailboxes don't need to be addressable
//! individually until we hit scale.
//!
//! This is the substrate for guild invites, gifts, trade offers,
//! and ad-hoc chat. The contract itself doesn't know what the
//! `kind` byte means — it just stores and replicates.

use freenet_stdlib::prelude::*;
use shared::{MailboxDelta, MailboxState, MAILBOX_STATE_VERSION, MAX_MAILBOX_PAYLOAD_BYTES};

struct Mailbox;

#[contract]
impl ContractInterface for Mailbox {
    /// Same lenient validation as presence: bad signatures fail the
    /// whole state (they could only have been direct-injected, not
    /// arrived through `apply`); structural over-size also fails.
    /// Anything else is just a stale-but-honest message.
    fn validate_state(
        _parameters: Parameters<'static>,
        state: State<'static>,
        _related: RelatedContracts<'static>,
    ) -> Result<ValidateResult, ContractError> {
        let parsed: MailboxState = match bincode::deserialize(state.as_ref()) {
            Ok(s) => s,
            Err(_) => return Ok(ValidateResult::Invalid),
        };
        if parsed.version != MAILBOX_STATE_VERSION {
            return Ok(ValidateResult::Invalid);
        }
        if parsed.entries.len() > shared::MAX_MAILBOX_MESSAGES {
            return Ok(ValidateResult::Invalid);
        }
        for entry in parsed.entries.iter() {
            if entry.payload.len() > MAX_MAILBOX_PAYLOAD_BYTES {
                return Ok(ValidateResult::Invalid);
            }
            if entry.verify().is_err() {
                return Ok(ValidateResult::Invalid);
            }
        }
        Ok(ValidateResult::Valid)
    }

    fn update_state(
        _parameters: Parameters<'static>,
        state: State<'static>,
        data: Vec<UpdateData<'static>>,
    ) -> Result<UpdateModification<'static>, ContractError> {
        let mut current: MailboxState = bincode::deserialize(state.as_ref())
            .map_err(|e| ContractError::Deser(e.to_string()))?;

        for update in data {
            match update {
                UpdateData::Delta(d) => {
                    let delta: MailboxDelta = bincode::deserialize(d.as_ref())
                        .map_err(|e| ContractError::Deser(e.to_string()))?;
                    for entry in delta.entries {
                        current.apply(entry);
                    }
                }
                UpdateData::State(s) => {
                    let incoming: MailboxState = bincode::deserialize(s.as_ref())
                        .map_err(|e| ContractError::Deser(e.to_string()))?;
                    for entry in incoming.entries {
                        current.apply(entry);
                    }
                }
                UpdateData::StateAndDelta { state: s, delta: d } => {
                    let incoming: MailboxState = bincode::deserialize(s.as_ref())
                        .map_err(|e| ContractError::Deser(e.to_string()))?;
                    for entry in incoming.entries {
                        current.apply(entry);
                    }
                    let delta: MailboxDelta = bincode::deserialize(d.as_ref())
                        .map_err(|e| ContractError::Deser(e.to_string()))?;
                    for entry in delta.entries {
                        current.apply(entry);
                    }
                }
                _ => {}
            }
        }

        current.prune_stale();

        let bytes =
            bincode::serialize(&current).map_err(|e| ContractError::Other(e.to_string()))?;
        Ok(UpdateModification::valid(State::from(bytes)))
    }

    /// Mailbox is delta-friendly — `summarize` returns the freshest
    /// timestamp we've seen, and `get_state_delta` ships every entry
    /// strictly newer than that. Suffices because mailbox entries
    /// are append-only (no LWW rewrites within an entry).
    fn summarize_state(
        _parameters: Parameters<'static>,
        state: State<'static>,
    ) -> Result<StateSummary<'static>, ContractError> {
        let current: MailboxState = bincode::deserialize(state.as_ref())
            .map_err(|e| ContractError::Deser(e.to_string()))?;
        let newest: u64 = current
            .entries
            .iter()
            .filter_map(|e| e.decode().map(|p| p.timestamp_ms))
            .max()
            .unwrap_or(0);
        let bytes = bincode::serialize(&newest)
            .map_err(|e| ContractError::Other(e.to_string()))?;
        Ok(StateSummary::from(bytes))
    }

    fn get_state_delta(
        _parameters: Parameters<'static>,
        state: State<'static>,
        summary: StateSummary<'static>,
    ) -> Result<StateDelta<'static>, ContractError> {
        let current: MailboxState = bincode::deserialize(state.as_ref())
            .map_err(|e| ContractError::Deser(e.to_string()))?;
        let cutoff: u64 = bincode::deserialize(summary.as_ref()).unwrap_or(0);
        let delta = MailboxDelta {
            entries: current
                .entries
                .into_iter()
                .filter(|e| {
                    e.decode()
                        .map(|p| p.timestamp_ms > cutoff)
                        .unwrap_or(false)
                })
                .collect(),
        };
        let bytes = bincode::serialize(&delta)
            .map_err(|e| ContractError::Other(e.to_string()))?;
        Ok(StateDelta::from(bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use shared::{
        MailboxEntry, MessagePayload, MAX_MAILBOX_MESSAGES, MSG_KIND_CHAT,
    };

    fn sign(sk: &SigningKey, to: [u8; 32], kind: u8, body: &[u8], ts: u64) -> MailboxEntry {
        let payload = MessagePayload::new(
            sk.verifying_key().to_bytes(),
            to,
            kind,
            body.to_vec(),
            ts,
        );
        let bytes = bincode::serialize(&payload).unwrap();
        let sig: ed25519_dalek::Signature = sk.sign(&bytes);
        MailboxEntry {
            payload: bytes,
            signature: sig.to_bytes(),
        }
    }

    fn run_delta(prior: &MailboxState, entries: Vec<MailboxEntry>) -> MailboxState {
        let initial = bincode::serialize(prior).unwrap();
        let delta = MailboxDelta { entries };
        let m = Mailbox::update_state(
            Parameters::from(vec![]),
            State::from(initial),
            vec![UpdateData::Delta(StateDelta::from(
                bincode::serialize(&delta).unwrap(),
            ))],
        )
        .unwrap();
        bincode::deserialize(m.new_state.unwrap().as_ref()).unwrap()
    }

    #[test]
    fn signed_message_lands_in_log() {
        let sender = SigningKey::from_bytes(&[1u8; 32]);
        let recipient = SigningKey::from_bytes(&[2u8; 32]);
        let s = run_delta(
            &MailboxState::default(),
            vec![sign(&sender, recipient.verifying_key().to_bytes(), MSG_KIND_CHAT, b"hi", 1_000)],
        );
        assert_eq!(s.entries.len(), 1);
        let p = s.entries[0].decode().unwrap();
        assert_eq!(p.kind, MSG_KIND_CHAT);
        assert_eq!(p.body, b"hi");
    }

    #[test]
    fn unsigned_message_rejected() {
        let alice = SigningKey::from_bytes(&[10u8; 32]);
        let mallory = SigningKey::from_bytes(&[11u8; 32]);
        let payload = MessagePayload::new(
            alice.verifying_key().to_bytes(),
            [0u8; 32],
            MSG_KIND_CHAT,
            b"forged".to_vec(),
            1_000,
        );
        let bytes = bincode::serialize(&payload).unwrap();
        let sig: ed25519_dalek::Signature = mallory.sign(&bytes);
        let bad = MailboxEntry { payload: bytes, signature: sig.to_bytes() };
        let s = run_delta(&MailboxState::default(), vec![bad]);
        assert!(s.entries.is_empty());
    }

    #[test]
    fn duplicate_message_deduped() {
        let sk = SigningKey::from_bytes(&[20u8; 32]);
        let same = sign(&sk, [0u8; 32], MSG_KIND_CHAT, b"a", 5_000);
        let s = run_delta(&MailboxState::default(), vec![same.clone(), same]);
        assert_eq!(s.entries.len(), 1, "(from, to, ts) must dedupe");
    }

    #[test]
    fn stale_message_pruned() {
        let sk = SigningKey::from_bytes(&[30u8; 32]);
        let stale_ts = 1_000;
        let fresh_ts = stale_ts + shared::MAILBOX_MAX_STALE_MS + 1;
        let s = run_delta(
            &MailboxState::default(),
            vec![
                sign(&sk, [0u8; 32], MSG_KIND_CHAT, b"old", stale_ts),
                sign(&sk, [0u8; 32], MSG_KIND_CHAT, b"new", fresh_ts),
            ],
        );
        assert_eq!(s.entries.len(), 1);
        assert_eq!(s.entries[0].decode().unwrap().body, b"new");
    }

    #[test]
    fn cap_evicts_oldest() {
        // Build state at cap, then push one more — oldest should
        // fall out. Use a small body so we don't pay for 5000 large
        // entries during the test.
        let sk = SigningKey::from_bytes(&[40u8; 32]);
        let mut prior = MailboxState::default();
        for i in 0..MAX_MAILBOX_MESSAGES {
            // Spread ts evenly across the non-stale window so prune
            // doesn't fire and corrupt the eviction expectations.
            let ts = 1_000_000 + i as u64;
            assert!(prior.apply(sign(&sk, [0u8; 32], MSG_KIND_CHAT, b".", ts)));
        }
        assert_eq!(prior.entries.len(), MAX_MAILBOX_MESSAGES);
        let oldest_ts = 1_000_000u64;
        // New message arrives with the freshest timestamp.
        let new_ts = 1_000_000 + MAX_MAILBOX_MESSAGES as u64 + 1;
        let s = run_delta(
            &prior,
            vec![sign(&sk, [0u8; 32], MSG_KIND_CHAT, b"new", new_ts)],
        );
        assert_eq!(s.entries.len(), MAX_MAILBOX_MESSAGES);
        // The oldest entry should be gone.
        assert!(!s
            .entries
            .iter()
            .any(|e| e.decode().map(|p| p.timestamp_ms == oldest_ts).unwrap_or(false)));
    }
}
