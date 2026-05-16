//! Presence contract — the public leaderboard / live-player wire.
//! `PresencePayload` is what each player publishes about themselves;
//! `ContractState` is the aggregator state the contract maintains.

use std::collections::BTreeMap;

use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};

use super::bytes::{byte_array_32, byte_array_64};
use super::{PubKey, SIG_LEN};

pub const MAX_NAME_BYTES: usize = 32;
pub const MAX_AREA_BYTES: usize = 24;
pub const MAX_PAYLOAD_BYTES: usize = 256;

/// Schema version of `PresencePayload`. The first byte on the wire —
/// future contracts can dispatch on it instead of rejecting every
/// older payload outright. Bump whenever a field is added or its
/// meaning changes.
pub const PRESENCE_PAYLOAD_VERSION: u8 = 1;

/// Schema version of `ContractState`. Same forward-compat hook as
/// `PRESENCE_PAYLOAD_VERSION`: future contract code that wants to
/// extend the state can dispatch instead of fail-closed.
pub const CONTRACT_STATE_VERSION: u8 = 1;

/// Absolute upper bound on `timestamp_ms` accepted by `apply`. Set to
/// 2100-01-01 UTC in ms — anything beyond is obviously a poisoning
/// attempt (defends against u64::MAX-stamp prune-DoS).
pub const MAX_TIMESTAMP_MS: u64 = 4_102_444_800_000;

/// How far ahead of the current max a new entry's `timestamp_ms` may
/// jump. 5 min absorbs honest clock drift between heterogenous
/// clients while preventing a single entry from dominating the prune
/// pivot.
pub const MAX_FORWARD_SKEW_MS: u64 = 5 * 60 * 1000;

/// Entries silent for longer than this (relative to a freshly observed
/// timestamp) are considered dormant. Used by `apply()` to bypass the
/// forward-skew check when the only existing entry is far older than
/// the incoming payload — without this, a single stale entry that
/// `prune_stale` can't evict (because `prune_stale` only runs with ≥2
/// entries) becomes a permanent skew anchor blocking all new writes.
/// Same value the contract uses for its own `prune_stale` call.
pub const MAX_STALE_MS: u64 = 60 * 1000;

/// Hard cap on the size of the live `entries` map. Once this many
/// distinct publishers are present, additions from *new* pubkeys are
/// refused (existing publishers may still refresh their slot).
/// Bounds state size and makes Sybil floods expensive.
pub const MAX_LIVE_ENTRIES: usize = 1_000;

/// Hard cap on `cumulative_damage`. Past this size, an incoming new
/// publisher evicts the contributor with the *smallest* watermark
/// (the player who has contributed least to the boss yet). Keeps the
/// World Boss ledger from growing without bound across many seasons
/// of unique players.
pub const MAX_CUMULATIVE_KEYS: usize = 10_000;

/// What each player publishes about themselves. Signed by their key
/// before being put on the contract.
///
/// `version` is the first byte on the wire so a future contract that
/// understands multiple schema versions can dispatch on it instead of
/// failing every old payload outright. Today only
/// `PRESENCE_PAYLOAD_VERSION` (=1) is accepted by `ContractState::apply`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PresencePayload {
    pub version: u8,
    #[serde(with = "byte_array_32")]
    pub public_key: PubKey,
    /// Display name. Truncated to 32 bytes by the contract.
    pub name: String,
    /// Cumulative gold this player has earned across all missions
    /// run on their node. Drives the leaderboard.
    pub gold: u64,
    /// Cumulative damage this player has dealt to the current
    /// World Boss. The contract just stores per-player numbers;
    /// the viewer-side aggregates them by summing across entries.
    pub boss_damage: u64,
    /// Free-form area tag (e.g. "lobby", "race-42", "north-bay").
    /// Lets clients filter who they want to see.
    pub area: String,
    /// Wall-clock at publisher. LWW + staleness pivot.
    pub timestamp_ms: u64,
}

impl PresencePayload {
    pub fn new(
        public_key: PubKey,
        name: String,
        gold: u64,
        boss_damage: u64,
        area: String,
        timestamp_ms: u64,
    ) -> Self {
        Self {
            version: PRESENCE_PAYLOAD_VERSION,
            public_key,
            name,
            gold,
            boss_damage,
            area,
            timestamp_ms,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SignedEntry {
    #[serde(with = "serde_bytes")]
    pub payload: Vec<u8>,
    #[serde(with = "byte_array_64")]
    pub signature: [u8; SIG_LEN],
}

impl SignedEntry {
    pub fn decode(&self) -> Option<PresencePayload> {
        bincode::deserialize(&self.payload).ok()
    }

    pub fn verify(&self) -> Result<PresencePayload, &'static str> {
        let payload: PresencePayload =
            bincode::deserialize(&self.payload).map_err(|_| "deserialize")?;
        let vk = VerifyingKey::from_bytes(&payload.public_key).map_err(|_| "bad pubkey")?;
        let sig = Signature::from_bytes(&self.signature);
        vk.verify(&self.payload, &sig).map_err(|_| "bad signature")?;
        Ok(payload)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContractState {
    /// Schema version. First field on the wire — forward-compat hook
    /// for future versions of the state shape. Today the contract
    /// only accepts `CONTRACT_STATE_VERSION`.
    pub version: u8,
    /// Live presence entries — pruned after `MAX_STALE_MS` of silence.
    /// Keyed by signing key; LWW on `timestamp_ms` within a key.
    pub entries: BTreeMap<PubKey, SignedEntry>,
    /// Per-key high-watermark of `boss_damage` ever observed. Capped
    /// at `MAX_CUMULATIVE_KEYS` with lowest-watermark eviction so the
    /// World Boss ledger can't grow indefinitely across seasons.
    pub cumulative_damage: BTreeMap<PubKey, u64>,
}

impl Default for ContractState {
    fn default() -> Self {
        Self {
            version: CONTRACT_STATE_VERSION,
            entries: BTreeMap::new(),
            cumulative_damage: BTreeMap::new(),
        }
    }
}

impl ContractState {
    /// LWW + sanity merge. An entry is accepted iff:
    ///   * the signature verifies against the embedded pubkey,
    ///   * the raw payload fits in `MAX_PAYLOAD_BYTES`,
    ///   * `version == PRESENCE_PAYLOAD_VERSION`,
    ///   * `name` and `area` fit their length caps,
    ///   * `timestamp_ms` ≤ `MAX_TIMESTAMP_MS` (absolute ceiling),
    ///   * `timestamp_ms` ≤ current max + `MAX_FORWARD_SKEW_MS`
    ///     (relative ceiling — defends against a single future-stamp
    ///     entry hijacking the prune pivot),
    ///   * the previous entry for the same key had a strictly older
    ///     `timestamp_ms`, **and** monotone-only counters (`gold`,
    ///     `boss_damage`) do not regress,
    ///   * the live `entries` map has room (≤ `MAX_LIVE_ENTRIES`) or
    ///     the key is already present (existing publishers always
    ///     get to refresh their slot).
    ///
    /// Side-effect: maintains `cumulative_damage` as the per-key
    /// high-watermark of `boss_damage` ever seen. Capped at
    /// `MAX_CUMULATIVE_KEYS`; once full, a *new* publisher is only
    /// admitted if its watermark strictly exceeds the current
    /// minimum, in which case the minimum is evicted. New publishers
    /// whose watermark would not displace anyone are rejected
    /// outright (entry is not committed). This rule makes the
    /// operation **order-independent**: the final state is always
    /// `top-N` watermarks of the union of accepted inputs, regardless
    /// of the order in which deltas arrive — required for freenet's
    /// CRDT convergence guarantees.
    ///
    /// Returns true if the live `entries` map was changed.
    pub fn apply(&mut self, entry: SignedEntry) -> bool {
        if entry.payload.len() > MAX_PAYLOAD_BYTES {
            return false;
        }
        let payload = match entry.verify() {
            Ok(p) => p,
            Err(_) => return false,
        };
        if payload.version != PRESENCE_PAYLOAD_VERSION {
            return false;
        }
        if payload.name.len() > MAX_NAME_BYTES || payload.area.len() > MAX_AREA_BYTES {
            return false;
        }
        if payload.timestamp_ms > MAX_TIMESTAMP_MS {
            return false;
        }
        // Relative ceiling against the freshest existing entry.
        // Bootstrap (no entries yet) trusts the first publisher — the
        // absolute ceiling above still caps abuse.
        //
        // Stale-singleton escape hatch: if `max_existing` is more than
        // `MAX_STALE_MS` behind the incoming `payload.timestamp_ms`,
        // the network is effectively dormant — treat the incoming
        // entry as a bootstrap and skip the skew check. Without this,
        // a single old entry (e.g., a third-party publisher who
        // went idle days ago) becomes a permanent "skew anchor" that
        // silently rejects every fresh entry, because `prune_stale`
        // only runs when ≥2 entries are present and so cannot evict
        // the lone stale singleton. The skew rule's original intent
        // — guard against a clock-skewed adversary publishing into
        // the future — still holds for active networks; only the
        // dormant edge case is relaxed here.
        if let Some(max_existing) = self
            .entries
            .values()
            .filter_map(|e| e.decode().map(|p| p.timestamp_ms))
            .max()
        {
            let max_existing_is_fresh =
                payload.timestamp_ms <= max_existing.saturating_add(MAX_STALE_MS);
            if max_existing_is_fresh
                && payload.timestamp_ms > max_existing.saturating_add(MAX_FORWARD_SKEW_MS)
            {
                return false;
            }
        }
        // Per-key monotonicity. Once a (gold, boss_damage) pair has
        // been published under a key, neither field may regress: a
        // compromised webapp cannot wipe a player's published score
        // back to zero. The delegate already enforces this on its
        // side; the contract enforces it independently here.
        let pk = payload.public_key;
        let key_already_live = self.entries.contains_key(&pk);
        if let Some(existing) = self.entries.get(&pk) {
            if let Some(prev) = existing.decode() {
                if payload.timestamp_ms <= prev.timestamp_ms {
                    return false;
                }
                if payload.gold < prev.gold || payload.boss_damage < prev.boss_damage {
                    return false;
                }
            }
        }
        // Live-entries cap. New publishers are refused when the map
        // is full; existing publishers always get to refresh.
        if !key_already_live && self.entries.len() >= MAX_LIVE_ENTRIES {
            return false;
        }
        // Cumulative-damage cap. Resolved *before* committing the
        // entry so a rejection here keeps `entries` and
        // `cumulative_damage` in sync (no half-state).
        //
        // Policy: «keep top N by watermark».  Equivalent rule, stated
        // operationally: a new pubkey is admitted iff its watermark
        // strictly exceeds the smallest one currently held; if so,
        // the smallest entry is evicted. This is order-independent —
        // the final cumulative map is always the top-N watermarks
        // from the union of inputs, regardless of delta arrival
        // order. (Blind-evict would have been order-dependent: a low
        // watermark could displace a high one when inserted last,
        // diverging across replicas that receive deltas out of order.)
        let new_dmg = payload.boss_damage;
        let key_in_cumulative = self.cumulative_damage.contains_key(&pk);
        let eviction_target = if !key_in_cumulative
            && self.cumulative_damage.len() >= MAX_CUMULATIVE_KEYS
        {
            // Find the smallest watermark; break ties by lowest pubkey
            // for cross-replica determinism.
            let (victim_pk, victim_dmg) = self
                .cumulative_damage
                .iter()
                .min_by(|a, b| a.1.cmp(b.1).then_with(|| a.0.cmp(b.0)))
                .map(|(k, v)| (*k, *v))
                .expect("non-empty when len >= cap > 0");
            if new_dmg <= victim_dmg {
                // Watermark too low to displace anyone — reject the
                // whole apply so `entries` stays in lock-step.
                return false;
            }
            Some(victim_pk)
        } else {
            None
        };
        // All checks passed; commit entries + cumulative atomically.
        self.entries.insert(pk, entry);
        if let Some(victim) = eviction_target {
            self.cumulative_damage.remove(&victim);
        }
        let slot = self.cumulative_damage.entry(pk).or_insert(0);
        if new_dmg > *slot {
            *slot = new_dmg;
        }
        true
    }

    /// Drop entries whose `timestamp_ms` is more than `max_stale_ms`
    /// behind the prune pivot. Pivot is the **second-largest** ts —
    /// not the raw max — so a single future-stamp outlier cannot
    /// shove the cutoff into the future and wipe legitimate state.
    /// State with < 2 entries is never pruned.
    pub fn prune_stale(&mut self, max_stale_ms: u64) {
        let mut all_ts: Vec<u64> = self
            .entries
            .values()
            .filter_map(|e| e.decode().map(|p| p.timestamp_ms))
            .collect();
        if all_ts.len() < 2 {
            return;
        }
        all_ts.sort_unstable();
        // Second-largest. With ≥2 entries this is `len - 2`.
        let pivot = all_ts[all_ts.len() - 2];
        let cutoff = pivot.saturating_sub(max_stale_ms);
        self.entries.retain(|_, e| match e.decode() {
            Some(p) => p.timestamp_ms >= cutoff,
            None => false,
        });
        // `cumulative_damage` is intentionally NOT pruned here — it
        // is the persistent World Boss ledger.
    }

    /// Aggregate World Boss damage across every key that has ever
    /// contributed, regardless of whether the publisher is currently
    /// live in `entries`. Uses `saturating_add` so the sum cannot
    /// overflow even with adversarial inputs.
    pub fn world_boss_total_damage(&self) -> u64 {
        self.cumulative_damage
            .values()
            .copied()
            .fold(0u64, |acc, v| acc.saturating_add(v))
    }

    pub fn summarize(&self) -> ContractSummary {
        ContractSummary {
            latest: self
                .entries
                .iter()
                .filter_map(|(k, e)| e.decode().map(|p| (*k, p.timestamp_ms)))
                .collect(),
        }
    }

    pub fn delta_against(&self, summary: &ContractSummary) -> ContractDelta {
        let entries = self
            .entries
            .iter()
            .filter(|(pk, e)| {
                let Some(p) = e.decode() else { return false };
                summary
                    .latest
                    .get(*pk)
                    .map_or(true, |theirs| p.timestamp_ms > *theirs)
            })
            .map(|(_, e)| e.clone())
            .collect();
        ContractDelta { entries }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContractDelta {
    pub entries: Vec<SignedEntry>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContractSummary {
    pub latest: BTreeMap<PubKey, u64>,
}
