//! Presence + leaderboard contract.
//!
//! Holds one entry per player, signed by their Ed25519 key. Anyone
//! can publish, but they can only publish entries signed by their own
//! key — `validate_state` and `apply` both check signatures. Entries
//! older than `MAX_STALE_MS` behind the prune pivot are dropped on
//! every update so the live state stays bounded.
//!
//! `ContractState.cumulative_damage` is the persistent World Boss
//! ledger: it survives entry pruning so the boss aggregate cannot
//! regress when contributing players go idle.

use freenet_stdlib::prelude::*;
use shared::{
    ContractDelta, ContractState, ContractSummary, CONTRACT_STATE_VERSION, MAX_AREA_BYTES,
    MAX_CUMULATIVE_KEYS, MAX_LIVE_ENTRIES, MAX_NAME_BYTES, MAX_PAYLOAD_BYTES, MAX_STALE_MS,
    MAX_TIMESTAMP_MS, PRESENCE_PAYLOAD_VERSION,
};

struct Presence;

#[contract]
impl ContractInterface for Presence {
    /// Validate a serialized state. Lenient: a state is invalid only
    /// if it fails to deserialize, exposes a self-inconsistent
    /// `cumulative_damage` ledger (entries below a published
    /// `boss_damage`), or contains any structurally over-sized entry.
    /// Individual entries with bad signatures or stale timestamps are
    /// silently ignored at apply-time — they cannot have been
    /// authored honestly, so refusing the whole state would let one
    /// bad apple block a healthy delta from landing.
    fn validate_state(
        _parameters: Parameters<'static>,
        state: State<'static>,
        _related: RelatedContracts<'static>,
    ) -> Result<ValidateResult, ContractError> {
        let parsed: ContractState = match bincode::deserialize(state.as_ref()) {
            Ok(s) => s,
            Err(_) => return Ok(ValidateResult::Invalid),
        };
        if parsed.version != CONTRACT_STATE_VERSION {
            return Ok(ValidateResult::Invalid);
        }
        if parsed.entries.len() > MAX_LIVE_ENTRIES
            || parsed.cumulative_damage.len() > MAX_CUMULATIVE_KEYS
        {
            return Ok(ValidateResult::Invalid);
        }
        for (pk, entry) in parsed.entries.iter() {
            if entry.payload.len() > MAX_PAYLOAD_BYTES {
                return Ok(ValidateResult::Invalid);
            }
            let payload = match entry.verify() {
                Ok(p) => p,
                // A live entry with a bad signature should not exist
                // — it could only have arrived via direct state
                // injection, not the apply path. Treat as invalid.
                Err(_) => return Ok(ValidateResult::Invalid),
            };
            if payload.version != PRESENCE_PAYLOAD_VERSION {
                return Ok(ValidateResult::Invalid);
            }
            if &payload.public_key != pk {
                return Ok(ValidateResult::Invalid);
            }
            if payload.name.len() > MAX_NAME_BYTES
                || payload.area.len() > MAX_AREA_BYTES
                || payload.timestamp_ms > MAX_TIMESTAMP_MS
            {
                return Ok(ValidateResult::Invalid);
            }
            // Cumulative ledger must dominate the live entry — it is
            // a *high-watermark*, so falling behind a published
            // `boss_damage` would mean someone tampered with the
            // ledger directly.
            let live_dmg = payload.boss_damage;
            let watermark = parsed.cumulative_damage.get(pk).copied().unwrap_or(0);
            if watermark < live_dmg {
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
        let mut current: ContractState = bincode::deserialize(state.as_ref())
            .map_err(|e| ContractError::Deser(e.to_string()))?;

        for update in data {
            match update {
                UpdateData::Delta(d) => {
                    let delta: ContractDelta = bincode::deserialize(d.as_ref())
                        .map_err(|e| ContractError::Deser(e.to_string()))?;
                    for entry in delta.entries {
                        current.apply(entry);
                    }
                }
                UpdateData::State(s) => {
                    let incoming: ContractState = bincode::deserialize(s.as_ref())
                        .map_err(|e| ContractError::Deser(e.to_string()))?;
                    for (_, entry) in incoming.entries {
                        current.apply(entry);
                    }
                }
                UpdateData::StateAndDelta { state: s, delta: d } => {
                    let incoming: ContractState = bincode::deserialize(s.as_ref())
                        .map_err(|e| ContractError::Deser(e.to_string()))?;
                    for (_, entry) in incoming.entries {
                        current.apply(entry);
                    }
                    let delta: ContractDelta = bincode::deserialize(d.as_ref())
                        .map_err(|e| ContractError::Deser(e.to_string()))?;
                    for entry in delta.entries {
                        current.apply(entry);
                    }
                }
                _ => {}
            }
        }

        current.prune_stale(MAX_STALE_MS);

        let bytes =
            bincode::serialize(&current).map_err(|e| ContractError::Other(e.to_string()))?;
        Ok(UpdateModification::valid(State::from(bytes)))
    }

    fn summarize_state(
        _parameters: Parameters<'static>,
        state: State<'static>,
    ) -> Result<StateSummary<'static>, ContractError> {
        let current: ContractState = bincode::deserialize(state.as_ref())
            .map_err(|e| ContractError::Deser(e.to_string()))?;
        let bytes = bincode::serialize(&current.summarize())
            .map_err(|e| ContractError::Other(e.to_string()))?;
        Ok(StateSummary::from(bytes))
    }

    fn get_state_delta(
        _parameters: Parameters<'static>,
        state: State<'static>,
        summary: StateSummary<'static>,
    ) -> Result<StateDelta<'static>, ContractError> {
        let current: ContractState = bincode::deserialize(state.as_ref())
            .map_err(|e| ContractError::Deser(e.to_string()))?;
        let summary: ContractSummary = bincode::deserialize(summary.as_ref())
            .map_err(|e| ContractError::Deser(e.to_string()))?;
        let bytes = bincode::serialize(&current.delta_against(&summary))
            .map_err(|e| ContractError::Other(e.to_string()))?;
        Ok(StateDelta::from(bytes))
    }
}

#[cfg(test)]
mod tests;
