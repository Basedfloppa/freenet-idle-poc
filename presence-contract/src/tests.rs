//! Apply-path tests for the presence aggregator. Each test seeds a
//! known `ContractState`, runs a delta through `update_state`, then
//! asserts the post-image. Signature verification and the per-key
//! monotone-counter invariants are covered, plus the structural
//! caps that defend the prune pivot.

use super::*;
use ed25519_dalek::{Signer, SigningKey};
use shared::{PresencePayload, SignedEntry, MAX_FORWARD_SKEW_MS};

fn sign(sk: &SigningKey, name: &str, gold: u64, boss_damage: u64, ts: u64) -> SignedEntry {
    let payload = PresencePayload::new(
        sk.verifying_key().to_bytes(),
        name.into(),
        gold,
        boss_damage,
        "lobby".into(),
        ts,
    );
    let bytes = bincode::serialize(&payload).unwrap();
    let sig: ed25519_dalek::Signature = sk.sign(&bytes);
    SignedEntry { payload: bytes, signature: sig.to_bytes() }
}

fn run_delta(prior: &ContractState, entries: Vec<SignedEntry>) -> ContractState {
    let initial = bincode::serialize(prior).unwrap();
    let delta = ContractDelta { entries };
    let m = Presence::update_state(
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
fn delta_applies_and_lww_wins() {
    let sk = SigningKey::from_bytes(&[7u8; 32]);
    let s = run_delta(
        &ContractState::default(),
        vec![
            sign(&sk, "alice", 10, 3, 100),
            sign(&sk, "alice", 5, 1, 50),
        ],
    );
    assert_eq!(s.entries.len(), 1);
    let p = s.entries[&sk.verifying_key().to_bytes()].decode().unwrap();
    assert_eq!(p.gold, 10);
    assert_eq!(p.boss_damage, 3);
}

#[test]
fn unsigned_entry_rejected() {
    let sk_a = SigningKey::from_bytes(&[1u8; 32]);
    let sk_b = SigningKey::from_bytes(&[2u8; 32]);
    // Construct a payload claiming key A but signed by B.
    let payload = PresencePayload::new(
        sk_a.verifying_key().to_bytes(),
        "spoof".into(),
        9999,
        1234,
        "lobby".into(),
        100,
    );
    let bytes = bincode::serialize(&payload).unwrap();
    let sig: ed25519_dalek::Signature = sk_b.sign(&bytes);
    let bad = SignedEntry { payload: bytes, signature: sig.to_bytes() };

    let s = run_delta(&ContractState::default(), vec![bad]);
    assert!(s.entries.is_empty(), "forged entry must be rejected");
}

#[test]
fn stale_entries_pruned_when_three_or_more() {
    // Three publishers — the oldest one is past MAX_STALE_MS
    // behind the second-largest, so it should be pruned. With
    // only two entries the new outlier-resistant pivot keeps
    // both alive intentionally (single-attacker scenario).
    let sk_old = SigningKey::from_bytes(&[3u8; 32]);
    let sk_mid = SigningKey::from_bytes(&[4u8; 32]);
    let sk_new = SigningKey::from_bytes(&[5u8; 32]);
    let s = run_delta(
        &ContractState::default(),
        vec![
            sign(&sk_old, "old", 1, 0, 1_000),
            sign(&sk_mid, "mid", 1, 0, 70_000),
            sign(&sk_new, "new", 1, 0, 80_000),
        ],
    );
    // Pivot = second-largest = 70_000, cutoff = 10_000.
    // sk_old@1000 < 10000 → pruned. The other two survive.
    assert_eq!(s.entries.len(), 2);
    assert!(s.entries.contains_key(&sk_mid.verifying_key().to_bytes()));
    assert!(s.entries.contains_key(&sk_new.verifying_key().to_bytes()));
}

#[test]
fn u64_max_timestamp_rejected() {
    // The headline DoS: a single entry with `timestamp_ms` near
    // u64::MAX used to push the prune cutoff into the far future
    // and wipe every legitimate publisher. The absolute ceiling
    // (`MAX_TIMESTAMP_MS`, year 2100) now rejects it on apply.
    let sk = SigningKey::from_bytes(&[6u8; 32]);
    let s = run_delta(
        &ContractState::default(),
        vec![sign(&sk, "poison", 0, 0, u64::MAX)],
    );
    assert!(
        s.entries.is_empty(),
        "u64::MAX timestamp must be rejected by absolute ceiling"
    );
}

#[test]
fn outlier_does_not_dominate_prune() {
    // Two honest publishers at "now"; a third stamps near the
    // absolute ceiling. Pivot must remain "now"-relative so the
    // honest entries survive the next prune cycle. (Re-publishes
    // bring them across the median.)
    let sk_a = SigningKey::from_bytes(&[10u8; 32]);
    let sk_b = SigningKey::from_bytes(&[11u8; 32]);
    let sk_c = SigningKey::from_bytes(&[12u8; 32]);
    let absurd_ts = MAX_TIMESTAMP_MS - 1;
    let s = run_delta(
        &ContractState::default(),
        vec![
            sign(&sk_a, "alice", 1, 0, 100_000),
            sign(&sk_b, "bob", 1, 0, 100_500),
            sign(&sk_c, "mallory", 0, 0, absurd_ts),
        ],
    );
    // With the relative forward-skew check, mallory's absurd_ts
    // is also rejected (it's > 100_500 + skew). So state holds 2
    // honest entries.
    assert_eq!(s.entries.len(), 2);
    assert!(s.entries.contains_key(&sk_a.verifying_key().to_bytes()));
    assert!(s.entries.contains_key(&sk_b.verifying_key().to_bytes()));
}

#[test]
fn forward_skew_caps_relative_jumps() {
    let sk_seed = SigningKey::from_bytes(&[20u8; 32]);
    let sk_jumper = SigningKey::from_bytes(&[21u8; 32]);
    let mut prior = ContractState::default();
    prior.apply(sign(&sk_seed, "seed", 1, 0, 100_000));

    // Honest small drift: well within the skew, accepted.
    let s = run_delta(
        &prior,
        vec![sign(&sk_jumper, "drifty", 1, 0, 100_000 + MAX_FORWARD_SKEW_MS / 2)],
    );
    assert_eq!(s.entries.len(), 2);

    // Hostile big jump: just over the skew, rejected.
    let s = run_delta(
        &prior,
        vec![sign(
            &sk_jumper,
            "jumper",
            1,
            0,
            100_000 + MAX_FORWARD_SKEW_MS + 1,
        )],
    );
    assert_eq!(
        s.entries.len(),
        1,
        "entry exceeding forward-skew window must be rejected"
    );
}

#[test]
fn monotonicity_blocks_gold_regression() {
    let sk = SigningKey::from_bytes(&[30u8; 32]);
    let s = run_delta(
        &ContractState::default(),
        vec![
            sign(&sk, "honest", 100, 5, 1_000),
            // Later timestamp but lower gold — would be a wipe
            // by a compromised webapp. Rejected.
            sign(&sk, "compromised", 0, 5, 2_000),
        ],
    );
    let p = s.entries[&sk.verifying_key().to_bytes()].decode().unwrap();
    assert_eq!(p.gold, 100, "gold must not regress under same key");
}

#[test]
fn monotonicity_blocks_boss_damage_regression() {
    let sk = SigningKey::from_bytes(&[31u8; 32]);
    let s = run_delta(
        &ContractState::default(),
        vec![
            sign(&sk, "honest", 0, 100, 1_000),
            sign(&sk, "rollback", 0, 99, 2_000),
        ],
    );
    let p = s.entries[&sk.verifying_key().to_bytes()].decode().unwrap();
    assert_eq!(p.boss_damage, 100);
}

#[test]
fn cumulative_damage_survives_pruning() {
    // The World Boss ledger must persist across entry pruning so
    // the boss does not regress when contributors go idle.
    let sk_idle = SigningKey::from_bytes(&[40u8; 32]);
    let sk_mid = SigningKey::from_bytes(&[41u8; 32]);
    let sk_fresh = SigningKey::from_bytes(&[42u8; 32]);
    let s = run_delta(
        &ContractState::default(),
        vec![
            sign(&sk_idle, "idle", 5, 50, 1_000),
            sign(&sk_mid, "mid", 5, 30, 70_000),
            sign(&sk_fresh, "fresh", 5, 20, 80_000),
        ],
    );
    // The idle entry got pruned (ts=1000, far below cutoff).
    assert_eq!(s.entries.len(), 2);
    assert!(!s
        .entries
        .contains_key(&sk_idle.verifying_key().to_bytes()));
    // But its damage contribution remains in the cumulative
    // ledger and the world-boss aggregate.
    assert_eq!(
        s.cumulative_damage.get(&sk_idle.verifying_key().to_bytes()),
        Some(&50)
    );
    assert_eq!(s.world_boss_total_damage(), 50 + 30 + 20);
}

#[test]
fn wrong_payload_version_rejected() {
    // Future client publishes v=2 but contract speaks v=1 only.
    let sk = SigningKey::from_bytes(&[60u8; 32]);
    let mut payload = PresencePayload::new(
        sk.verifying_key().to_bytes(),
        "from_future".into(),
        5,
        5,
        "lobby".into(),
        1_000,
    );
    payload.version = 2;
    let bytes = bincode::serialize(&payload).unwrap();
    let sig: ed25519_dalek::Signature = sk.sign(&bytes);
    let entry = SignedEntry { payload: bytes, signature: sig.to_bytes() };
    let s = run_delta(&ContractState::default(), vec![entry]);
    assert!(
        s.entries.is_empty(),
        "payload from a future schema version must be rejected"
    );
}

#[test]
fn live_entries_cap_blocks_new_publishers() {
    // Fill the live map to capacity, then verify a fresh key is
    // refused while an existing one can still refresh its slot.
    let mut prior = ContractState::default();
    for i in 0..MAX_LIVE_ENTRIES {
        // Distinct 32-byte seeds. The bottom u16 derives the seed,
        // the rest stays zero — that's still well-formed for
        // ed25519's secret key.
        let mut seed = [0u8; 32];
        seed[0] = (i & 0xFF) as u8;
        seed[1] = ((i >> 8) & 0xFF) as u8;
        seed[2] = 0xAA; // disambiguate from `existing` and `newcomer` below
        let sk = SigningKey::from_bytes(&seed);
        assert!(prior.apply(sign(&sk, "p", 1, 0, 100_000 + i as u64)));
    }
    assert_eq!(prior.entries.len(), MAX_LIVE_ENTRIES);

    let existing = SigningKey::from_bytes(&{
        let mut s = [0u8; 32];
        s[0] = 0;
        s[1] = 0;
        s[2] = 0xAA;
        s
    });
    let newcomer = SigningKey::from_bytes(&[0xBBu8; 32]);

    let s = run_delta(
        &prior,
        vec![
            // Existing pubkey refreshes — must succeed.
            sign(
                &existing,
                "still here",
                2,
                1,
                100_000 + MAX_LIVE_ENTRIES as u64 + 1,
            ),
            // New pubkey shows up — must be refused.
            sign(
                &newcomer,
                "rejected",
                1,
                1,
                100_000 + MAX_LIVE_ENTRIES as u64 + 1,
            ),
        ],
    );
    assert_eq!(s.entries.len(), MAX_LIVE_ENTRIES);
    // The existing key's refresh did land.
    let refreshed = s.entries[&existing.verifying_key().to_bytes()].decode().unwrap();
    assert_eq!(refreshed.gold, 2);
    // The newcomer is absent.
    assert!(!s
        .entries
        .contains_key(&newcomer.verifying_key().to_bytes()));
}

/// Build a state with `cumulative_damage` pre-filled to one slot
/// below capacity at watermark 100, plus a fixed "weak" key at
/// watermark 1. Used by the cap-eviction and order-independence
/// tests to exercise the boundary case quickly.
fn cap_minus_one_state(filler_dmg: u64, weak_pk: Option<[u8; 32]>) -> ContractState {
    let mut s = ContractState::default();
    let mut seeded = 0usize;
    if let Some(pk) = weak_pk {
        s.cumulative_damage.insert(pk, 1);
        seeded += 1;
    }
    let target_total = MAX_CUMULATIVE_KEYS - 1;
    let mut i = 0usize;
    while seeded < target_total {
        let mut pk = [0u8; 32];
        pk[0] = (i & 0xFF) as u8;
        pk[1] = ((i >> 8) & 0xFF) as u8;
        pk[2] = ((i >> 16) & 0xFF) as u8;
        pk[3] = 0xAA;
        if !s.cumulative_damage.contains_key(&pk) {
            s.cumulative_damage.insert(pk, filler_dmg);
            seeded += 1;
        }
        i += 1;
    }
    s
}

#[test]
fn cumulative_cap_evicts_lowest_watermark() {
    // Hand-rig a state at cap-1 with a known minimum watermark,
    // then admit one new publisher whose watermark beats it.
    let weak_pk = [0xCCu8; 32];
    let prior = cap_minus_one_state(1_000, Some(weak_pk));
    assert_eq!(prior.cumulative_damage.len(), MAX_CUMULATIVE_KEYS - 1);
    let newcomer = SigningKey::from_bytes(&[0xEEu8; 32]);
    let s = run_delta(&prior, vec![sign(&newcomer, "arrival", 10, 42, 100_000)]);
    // Newcomer added; we're now at cap. weak_pk is still here
    // (the cap wasn't hit during the single insert) — eviction
    // only fires when admitting a NEW key into an already-full map.
    assert_eq!(s.cumulative_damage.len(), MAX_CUMULATIVE_KEYS);
    assert!(s.cumulative_damage.contains_key(&weak_pk));
    // A second newcomer at watermark > 1 now triggers eviction of
    // weak_pk.
    let second = SigningKey::from_bytes(&[0xEFu8; 32]);
    let s2 = run_delta(&s, vec![sign(&second, "second", 10, 50, 100_001)]);
    assert_eq!(s2.cumulative_damage.len(), MAX_CUMULATIVE_KEYS);
    assert!(!s2.cumulative_damage.contains_key(&weak_pk));
    assert_eq!(
        s2.cumulative_damage.get(&second.verifying_key().to_bytes()),
        Some(&50)
    );
}

#[test]
fn cumulative_cap_rejects_below_min_watermark() {
    // When the cumulative map is full and the incoming watermark
    // is ≤ the current minimum, the entire `apply` must reject.
    // Otherwise the live entry would be committed without a
    // matching cumulative slot — a state-invariant violation
    // (validate_state checks `watermark >= live boss_damage`).
    let weak_pk = [0xCCu8; 32];
    let prior = cap_minus_one_state(100, None);
    let newcomer = SigningKey::from_bytes(&[0xFFu8; 32]);
    // First publish fills the map to cap.
    let s = run_delta(&prior, vec![sign(&newcomer, "first", 1, 50, 100_000)]);
    assert_eq!(s.cumulative_damage.len(), MAX_CUMULATIVE_KEYS);
    // A different new key with watermark below the current min
    // (50 — the newcomer's slot) is rejected.
    let too_weak = SigningKey::from_bytes(&[0x11u8; 32]);
    let s2 = run_delta(
        &s,
        vec![sign(&too_weak, "too_weak", 1, 10, 100_001)],
    );
    // Neither entries nor cumulative was touched.
    assert_eq!(s2.cumulative_damage.len(), MAX_CUMULATIVE_KEYS);
    assert!(!s2
        .cumulative_damage
        .contains_key(&too_weak.verifying_key().to_bytes()));
    assert!(!s2
        .entries
        .contains_key(&too_weak.verifying_key().to_bytes()));
    // Force-use unused weak_pk to silence dead-code lint.
    let _ = weak_pk;
}

#[test]
fn cumulative_eviction_is_order_independent() {
    // Three candidates with distinct watermarks land on a
    // contract that's one slot below cap. The final cumulative
    // map must be the same across every permutation of arrival
    // order — required for freenet's CRDT convergence.
    let make_prior = || cap_minus_one_state(100, None);
    let sk_a = SigningKey::from_bytes(&[0x90u8; 32]);
    let sk_b = SigningKey::from_bytes(&[0x91u8; 32]);
    let sk_c = SigningKey::from_bytes(&[0x92u8; 32]);
    // a: below filler watermark → eventually rejected.
    // b: highest → always lands.
    // c: above filler → always lands, evicts a filler.
    let entries = [
        sign(&sk_a, "a", 1, 50, 100_000),
        sign(&sk_b, "b", 1, 200, 100_001),
        sign(&sk_c, "c", 1, 150, 100_002),
    ];
    let orders: &[[usize; 3]] = &[
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ];
    let mut results = Vec::new();
    for ord in orders {
        let s = run_delta(
            &make_prior(),
            ord.iter().map(|&i| entries[i].clone()).collect(),
        );
        results.push(s.cumulative_damage);
    }
    let first = &results[0];
    for (i, other) in results.iter().enumerate().skip(1) {
        assert_eq!(
            first, other,
            "cumulative_damage diverged between orderings 0 and {i}",
        );
    }
    // Sanity-check the shape: a was rejected (watermark below min);
    // b and c landed, evicting two fillers.
    assert!(!first.contains_key(&sk_a.verifying_key().to_bytes()));
    assert_eq!(first.get(&sk_b.verifying_key().to_bytes()), Some(&200));
    assert_eq!(first.get(&sk_c.verifying_key().to_bytes()), Some(&150));
    assert_eq!(first.len(), MAX_CUMULATIVE_KEYS);
}

#[test]
fn oversized_payload_rejected() {
    // Append trailing bytes to a valid payload. The signature
    // covers them, so `verify` would pass — but the payload-size
    // gate must reject before deserialization.
    let sk = SigningKey::from_bytes(&[50u8; 32]);
    let payload = PresencePayload::new(
        sk.verifying_key().to_bytes(),
        "padding".into(),
        0,
        0,
        "lobby".into(),
        1_000,
    );
    let mut bytes = bincode::serialize(&payload).unwrap();
    bytes.extend(std::iter::repeat(0xAB).take(MAX_PAYLOAD_BYTES));
    let sig: ed25519_dalek::Signature = sk.sign(&bytes);
    let huge = SignedEntry { payload: bytes, signature: sig.to_bytes() };
    let s = run_delta(&ContractState::default(), vec![huge]);
    assert!(s.entries.is_empty(), "oversized payload must be rejected");
}
