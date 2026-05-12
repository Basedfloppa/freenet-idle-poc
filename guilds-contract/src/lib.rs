//! Guilds contract — signed-op log for cooperative groups.
//!
//! Holds a flat `Vec<Guild>` whose membership lists evolve through
//! CREATE / JOIN / LEAVE ops. Each op is individually signed; the
//! contract verifies, applies, and stores. The op stream is order-
//! sensitive (LEAVE before JOIN-to-another-guild matters), but
//! freenet's CRDT contract path delivers deltas in arrival order on
//! a per-peer basis — combined with the strict "one pubkey, one
//! guild" invariant, replicas converge.
//!
//! Bookkeeping (member cap, guild cap, unique name) lives in
//! `shared::GuildsState::apply`. This file is just the wire layer.

use freenet_stdlib::prelude::*;
use shared::{GuildsDelta, GuildsState, GUILDS_STATE_VERSION, MAX_GUILDS};

struct Guilds;

#[contract]
impl ContractInterface for Guilds {
    fn validate_state(
        _parameters: Parameters<'static>,
        state: State<'static>,
        _related: RelatedContracts<'static>,
    ) -> Result<ValidateResult, ContractError> {
        let parsed: GuildsState = match bincode::deserialize(state.as_ref()) {
            Ok(s) => s,
            Err(_) => return Ok(ValidateResult::Invalid),
        };
        if parsed.version != GUILDS_STATE_VERSION {
            return Ok(ValidateResult::Invalid);
        }
        if parsed.guilds.len() > MAX_GUILDS {
            return Ok(ValidateResult::Invalid);
        }
        // No per-op verification here — ops live in deltas, not in
        // state. The state is the materialized view. Structural
        // invariants only.
        let mut seen_ids = std::collections::BTreeSet::new();
        let mut all_members: std::collections::BTreeSet<[u8; 32]> = Default::default();
        for g in parsed.guilds.iter() {
            if g.members.len() > shared::MAX_GUILD_MEMBERS {
                return Ok(ValidateResult::Invalid);
            }
            if g.name.len() > shared::MAX_GUILD_NAME_BYTES {
                return Ok(ValidateResult::Invalid);
            }
            if !seen_ids.insert(g.id) {
                return Ok(ValidateResult::Invalid); // duplicate id
            }
            // Each pubkey in at most one guild.
            for m in g.members.iter() {
                if !all_members.insert(*m) {
                    return Ok(ValidateResult::Invalid);
                }
            }
            // Leader must be among the members.
            if !g.members.iter().any(|m| m == &g.leader) {
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
        let mut current: GuildsState = bincode::deserialize(state.as_ref())
            .map_err(|e| ContractError::Deser(e.to_string()))?;

        for update in data {
            match update {
                UpdateData::Delta(d) => {
                    let delta: GuildsDelta = bincode::deserialize(d.as_ref())
                        .map_err(|e| ContractError::Deser(e.to_string()))?;
                    for op in delta.ops {
                        current.apply(op);
                    }
                }
                UpdateData::State(s) => {
                    let incoming: GuildsState = bincode::deserialize(s.as_ref())
                        .map_err(|e| ContractError::Deser(e.to_string()))?;
                    // Trust-then-replace: incoming state is the new
                    // snapshot. Validate before adopting.
                    if incoming.version == GUILDS_STATE_VERSION
                        && incoming.guilds.len() <= MAX_GUILDS
                    {
                        current = incoming;
                    }
                }
                UpdateData::StateAndDelta { state: s, delta: d } => {
                    let incoming: GuildsState = bincode::deserialize(s.as_ref())
                        .map_err(|e| ContractError::Deser(e.to_string()))?;
                    if incoming.version == GUILDS_STATE_VERSION
                        && incoming.guilds.len() <= MAX_GUILDS
                    {
                        current = incoming;
                    }
                    let delta: GuildsDelta = bincode::deserialize(d.as_ref())
                        .map_err(|e| ContractError::Deser(e.to_string()))?;
                    for op in delta.ops {
                        current.apply(op);
                    }
                }
                _ => {}
            }
        }

        let bytes =
            bincode::serialize(&current).map_err(|e| ContractError::Other(e.to_string()))?;
        Ok(UpdateModification::valid(State::from(bytes)))
    }

    /// Summary is just the guild count + a hash-equivalent (last
    /// guild id, if any). Light, lets peers tell "we differ" without
    /// shipping the full member list.
    fn summarize_state(
        _parameters: Parameters<'static>,
        state: State<'static>,
    ) -> Result<StateSummary<'static>, ContractError> {
        let current: GuildsState = bincode::deserialize(state.as_ref())
            .map_err(|e| ContractError::Deser(e.to_string()))?;
        let summary = (current.guilds.len() as u32, current.guilds.last().map(|g| g.id));
        let bytes = bincode::serialize(&summary)
            .map_err(|e| ContractError::Other(e.to_string()))?;
        Ok(StateSummary::from(bytes))
    }

    fn get_state_delta(
        _parameters: Parameters<'static>,
        state: State<'static>,
        _summary: StateSummary<'static>,
    ) -> Result<StateDelta<'static>, ContractError> {
        // Guilds aren't easy to express as an additive delta (LEAVE
        // is removal), so just ship the whole materialized state as
        // a singleton state-delta. Recipient `update_state` will
        // replace.
        let current: GuildsState = bincode::deserialize(state.as_ref())
            .map_err(|e| ContractError::Deser(e.to_string()))?;
        // Empty delta carrier — we'd rather the peer re-fetch State
        // than maintain a custom diff format.
        let delta = GuildsDelta { ops: Vec::new() };
        let _ = current;
        let bytes = bincode::serialize(&delta)
            .map_err(|e| ContractError::Other(e.to_string()))?;
        Ok(StateDelta::from(bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use shared::{guild_id_from_name, GuildOp, GuildOpPayload};

    fn signed(sk: &SigningKey, payload: &GuildOpPayload) -> GuildOp {
        let bytes = bincode::serialize(payload).unwrap();
        let sig: ed25519_dalek::Signature = sk.sign(&bytes);
        GuildOp { payload: bytes, signature: sig.to_bytes() }
    }

    fn run_delta(prior: &GuildsState, ops: Vec<GuildOp>) -> GuildsState {
        let initial = bincode::serialize(prior).unwrap();
        let delta = GuildsDelta { ops };
        let m = Guilds::update_state(
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
    fn create_join_leave_cycle() {
        let founder = SigningKey::from_bytes(&[1u8; 32]);
        let joiner = SigningKey::from_bytes(&[2u8; 32]);
        let leaver = SigningKey::from_bytes(&[3u8; 32]);
        let guild_id = guild_id_from_name("brimstone bears");
        let s = run_delta(
            &GuildsState::default(),
            vec![
                signed(
                    &founder,
                    &GuildOpPayload::new_create(
                        founder.verifying_key().to_bytes(),
                        "brimstone bears".to_string(),
                        1_000,
                    ),
                ),
                signed(
                    &joiner,
                    &GuildOpPayload::new_join(
                        joiner.verifying_key().to_bytes(),
                        guild_id,
                        1_100,
                    ),
                ),
                signed(
                    &leaver,
                    &GuildOpPayload::new_join(
                        leaver.verifying_key().to_bytes(),
                        guild_id,
                        1_200,
                    ),
                ),
                signed(
                    &leaver,
                    &GuildOpPayload::new_leave(
                        leaver.verifying_key().to_bytes(),
                        guild_id,
                        1_300,
                    ),
                ),
            ],
        );
        assert_eq!(s.guilds.len(), 1);
        let g = &s.guilds[0];
        assert_eq!(g.name, "brimstone bears");
        assert_eq!(g.members.len(), 2);
        assert!(g.members.iter().any(|m| *m == founder.verifying_key().to_bytes()));
        assert!(g.members.iter().any(|m| *m == joiner.verifying_key().to_bytes()));
        assert!(!g.members.iter().any(|m| *m == leaver.verifying_key().to_bytes()));
    }

    #[test]
    fn one_pubkey_one_guild() {
        let alice = SigningKey::from_bytes(&[10u8; 32]);
        let g_a = guild_id_from_name("alpha");
        let g_b = guild_id_from_name("beta");
        let s = run_delta(
            &GuildsState::default(),
            vec![
                signed(
                    &alice,
                    &GuildOpPayload::new_create(
                        alice.verifying_key().to_bytes(),
                        "alpha".to_string(),
                        1_000,
                    ),
                ),
                signed(
                    &alice,
                    &GuildOpPayload::new_join(
                        alice.verifying_key().to_bytes(),
                        g_b, // alice tries to also join beta — rejected
                        1_100,
                    ),
                ),
            ],
        );
        assert_eq!(s.guilds.len(), 1);
        assert_eq!(s.guilds[0].id, g_a);
    }

    #[test]
    fn unsigned_op_rejected() {
        let alice = SigningKey::from_bytes(&[20u8; 32]);
        let mallory = SigningKey::from_bytes(&[21u8; 32]);
        // mallory tries to create a guild but signs with alice's pubkey
        // as the actor → signature must fail.
        let payload = GuildOpPayload::new_create(
            alice.verifying_key().to_bytes(),
            "spoof".into(),
            1_000,
        );
        let bytes = bincode::serialize(&payload).unwrap();
        let sig: ed25519_dalek::Signature = mallory.sign(&bytes);
        let bad = GuildOp { payload: bytes, signature: sig.to_bytes() };
        let s = run_delta(&GuildsState::default(), vec![bad]);
        assert!(s.guilds.is_empty());
    }

    #[test]
    fn leader_can_disband_guild_with_members() {
        let founder = SigningKey::from_bytes(&[50u8; 32]);
        let member_a = SigningKey::from_bytes(&[51u8; 32]);
        let member_b = SigningKey::from_bytes(&[52u8; 32]);
        let gid = guild_id_from_name("disband me");
        let s = run_delta(
            &GuildsState::default(),
            vec![
                signed(&founder, &GuildOpPayload::new_create(
                    founder.verifying_key().to_bytes(), "disband me".into(), 1_000)),
                signed(&member_a, &GuildOpPayload::new_join(
                    member_a.verifying_key().to_bytes(), gid, 1_100)),
                signed(&member_b, &GuildOpPayload::new_join(
                    member_b.verifying_key().to_bytes(), gid, 1_200)),
                signed(&founder, &GuildOpPayload::new_disband(
                    founder.verifying_key().to_bytes(), gid, 1_300)),
            ],
        );
        assert!(s.guilds.is_empty(),
                "leader's DISBAND should remove the guild even with members present");
    }

    #[test]
    fn non_leader_cannot_disband() {
        let founder = SigningKey::from_bytes(&[60u8; 32]);
        let usurper = SigningKey::from_bytes(&[61u8; 32]);
        let gid = guild_id_from_name("not yours");
        let s = run_delta(
            &GuildsState::default(),
            vec![
                signed(&founder, &GuildOpPayload::new_create(
                    founder.verifying_key().to_bytes(), "not yours".into(), 1_000)),
                signed(&usurper, &GuildOpPayload::new_join(
                    usurper.verifying_key().to_bytes(), gid, 1_100)),
                // Usurper tries to disband even though they're not leader.
                signed(&usurper, &GuildOpPayload::new_disband(
                    usurper.verifying_key().to_bytes(), gid, 1_200)),
            ],
        );
        assert_eq!(s.guilds.len(), 1, "non-leader DISBAND must be a no-op");
        assert_eq!(s.guilds[0].name, "not yours");
        assert_eq!(s.guilds[0].members.len(), 2);
    }

    #[test]
    fn members_free_after_disband() {
        // After a guild is disbanded, every prior member is free to
        // join another guild — the "one pubkey ≤ one guild"
        // invariant holds because membership of the disbanded guild
        // is gone too.
        let founder = SigningKey::from_bytes(&[70u8; 32]);
        let other = SigningKey::from_bytes(&[71u8; 32]);
        let gid_a = guild_id_from_name("first home");
        let gid_b = guild_id_from_name("second home");
        let s = run_delta(
            &GuildsState::default(),
            vec![
                signed(&founder, &GuildOpPayload::new_create(
                    founder.verifying_key().to_bytes(), "first home".into(), 1_000)),
                signed(&other, &GuildOpPayload::new_join(
                    other.verifying_key().to_bytes(), gid_a, 1_100)),
                signed(&founder, &GuildOpPayload::new_disband(
                    founder.verifying_key().to_bytes(), gid_a, 1_200)),
                // After disband, `other` joins a fresh guild.
                signed(&founder, &GuildOpPayload::new_create(
                    founder.verifying_key().to_bytes(), "second home".into(), 1_300)),
                signed(&other, &GuildOpPayload::new_join(
                    other.verifying_key().to_bytes(), gid_b, 1_400)),
            ],
        );
        assert_eq!(s.guilds.len(), 1);
        assert_eq!(s.guilds[0].name, "second home");
        assert_eq!(s.guilds[0].members.len(), 2);
    }

    #[test]
    fn leader_handoff_on_leave() {
        let founder = SigningKey::from_bytes(&[30u8; 32]);
        let next = SigningKey::from_bytes(&[31u8; 32]);
        let gid = guild_id_from_name("handoffs");
        let s = run_delta(
            &GuildsState::default(),
            vec![
                signed(&founder, &GuildOpPayload::new_create(
                    founder.verifying_key().to_bytes(), "handoffs".to_string(), 1_000)),
                signed(&next, &GuildOpPayload::new_join(
                    next.verifying_key().to_bytes(), gid, 1_100)),
                signed(&founder, &GuildOpPayload::new_leave(
                    founder.verifying_key().to_bytes(), gid, 1_200)),
            ],
        );
        assert_eq!(s.guilds.len(), 1);
        assert_eq!(s.guilds[0].members.len(), 1);
        assert_eq!(s.guilds[0].leader, next.verifying_key().to_bytes());
    }
}
