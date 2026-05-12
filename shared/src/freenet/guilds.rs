//! Guilds contract — co-op groups. One contract instance holds every
//! guild + member list; ops (CREATE/JOIN/LEAVE/DISBAND) are
//! individually signed and applied in arrival order. Each pubkey is
//! in at most one guild at a time; `apply` enforces that invariant.

use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};

use super::bytes::{byte_array_32, byte_array_64};
use super::presence::MAX_TIMESTAMP_MS;
use super::{PubKey, SIG_LEN};

pub const GUILDS_STATE_VERSION: u8 = 1;
pub const GUILD_OP_VERSION: u8 = 1;
pub const MAX_GUILDS: usize = 256;
pub const MAX_GUILD_MEMBERS: usize = 50;
pub const MAX_GUILD_NAME_BYTES: usize = 32;
pub const MAX_GUILD_OP_PAYLOAD_BYTES: usize = 256;

pub const GUILD_OP_CREATE: u8 = 0;
pub const GUILD_OP_JOIN: u8 = 1;
pub const GUILD_OP_LEAVE: u8 = 2;
/// Leader-only "tear down the whole guild" op. Use case: creator
/// wants to retire the guild without waiting for every member to
/// individually `LEAVE`. Members lose membership atomically and can
/// re-join other guilds immediately.
pub const GUILD_OP_DISBAND: u8 = 3;

/// Stable guild identifier — Blake3-equivalent isn't available in
/// the WASM contract sandbox, but a SHA-256 of the canonical name
/// bytes is. For `GUILD_OP_CREATE` the op carries the name; for
/// `JOIN`/`LEAVE` it carries the target id. Different names with
/// the same id (collision) are vanishingly rare at this scale.
pub type GuildId = [u8; 32];

/// One on-the-wire op. `payload` is bincode of `GuildOpPayload`;
/// the signature must verify against `payload.actor`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GuildOp {
    #[serde(with = "serde_bytes")]
    pub payload: Vec<u8>,
    #[serde(with = "byte_array_64")]
    pub signature: [u8; SIG_LEN],
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GuildOpPayload {
    pub version: u8,
    pub op_kind: u8,
    #[serde(with = "byte_array_32")]
    pub actor: PubKey,
    /// CREATE: the new guild's id (hash of the trimmed name).
    /// JOIN / LEAVE: the target guild id.
    #[serde(with = "byte_array_32")]
    pub guild_id: GuildId,
    /// CREATE: bincode-serialized guild name (UTF-8, ≤ 32 bytes).
    /// JOIN / LEAVE: empty.
    pub data: Vec<u8>,
    pub timestamp_ms: u64,
}

impl GuildOpPayload {
    pub fn new_create(actor: PubKey, name: String, ts: u64) -> Self {
        let id = guild_id_from_name(&name);
        Self {
            version: GUILD_OP_VERSION,
            op_kind: GUILD_OP_CREATE,
            actor,
            guild_id: id,
            data: name.into_bytes(),
            timestamp_ms: ts,
        }
    }
    pub fn new_join(actor: PubKey, guild_id: GuildId, ts: u64) -> Self {
        Self {
            version: GUILD_OP_VERSION,
            op_kind: GUILD_OP_JOIN,
            actor,
            guild_id,
            data: Vec::new(),
            timestamp_ms: ts,
        }
    }
    pub fn new_leave(actor: PubKey, guild_id: GuildId, ts: u64) -> Self {
        Self {
            version: GUILD_OP_VERSION,
            op_kind: GUILD_OP_LEAVE,
            actor,
            guild_id,
            data: Vec::new(),
            timestamp_ms: ts,
        }
    }
    pub fn new_disband(actor: PubKey, guild_id: GuildId, ts: u64) -> Self {
        Self {
            version: GUILD_OP_VERSION,
            op_kind: GUILD_OP_DISBAND,
            actor,
            guild_id,
            data: Vec::new(),
            timestamp_ms: ts,
        }
    }
}

impl GuildOp {
    pub fn decode(&self) -> Option<GuildOpPayload> {
        bincode::deserialize(&self.payload).ok()
    }
    pub fn verify(&self) -> Result<GuildOpPayload, &'static str> {
        let payload: GuildOpPayload =
            bincode::deserialize(&self.payload).map_err(|_| "deserialize")?;
        let vk = VerifyingKey::from_bytes(&payload.actor).map_err(|_| "bad pubkey")?;
        let sig = Signature::from_bytes(&self.signature);
        vk.verify(&self.payload, &sig).map_err(|_| "bad signature")?;
        Ok(payload)
    }
}

/// Deterministic guild id from a trimmed UTF-8 name. SHA-256 truncated
/// to 32 bytes (well, native 32). Used in CREATE to derive the id and
/// in JOIN/LEAVE to address an existing guild.
pub fn guild_id_from_name(name: &str) -> GuildId {
    use sha2::{Digest, Sha256};
    let trimmed = name.trim();
    let bytes: Vec<u8> = trimmed.bytes().take(MAX_GUILD_NAME_BYTES).collect();
    let hash = Sha256::digest(&bytes);
    let mut out = [0u8; 32];
    out.copy_from_slice(&hash);
    out
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Guild {
    pub id: GuildId,
    pub name: String,
    #[serde(with = "byte_array_32")]
    pub leader: PubKey,
    pub members: Vec<PubKey>,
    pub founded_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GuildsState {
    pub version: u8,
    pub guilds: Vec<Guild>,
}

impl Default for GuildsState {
    fn default() -> Self {
        Self {
            version: GUILDS_STATE_VERSION,
            guilds: Vec::new(),
        }
    }
}

impl GuildsState {
    /// Find which guild the given pubkey is currently a member of,
    /// if any. Linear scan — fine at our scale (256 × 50 = 12_800).
    pub fn membership(&self, pk: &PubKey) -> Option<usize> {
        self.guilds
            .iter()
            .position(|g| g.members.iter().any(|m| m == pk))
    }

    /// Apply one signed op to the state. Returns true if the state
    /// changed. Maintains the "each pubkey in at most one guild"
    /// invariant. CREATE collisions, JOIN of full / unknown guilds,
    /// and LEAVE while not a member are silent no-ops (return false).
    pub fn apply(&mut self, op: GuildOp) -> bool {
        if op.payload.len() > MAX_GUILD_OP_PAYLOAD_BYTES {
            return false;
        }
        let payload = match op.verify() {
            Ok(p) => p,
            Err(_) => return false,
        };
        if payload.version != GUILD_OP_VERSION {
            return false;
        }
        if payload.timestamp_ms > MAX_TIMESTAMP_MS {
            return false;
        }
        match payload.op_kind {
            GUILD_OP_CREATE => {
                if self.guilds.len() >= MAX_GUILDS {
                    return false;
                }
                // Name must be valid UTF-8 within the byte cap.
                let name = match String::from_utf8(payload.data.clone()) {
                    Ok(s) if s.len() <= MAX_GUILD_NAME_BYTES && !s.trim().is_empty() => s,
                    _ => return false,
                };
                // Id must match the hash of the name — prevents
                // squatting an id under a different name.
                if guild_id_from_name(&name) != payload.guild_id {
                    return false;
                }
                if self.guilds.iter().any(|g| g.id == payload.guild_id) {
                    return false;
                }
                if self.membership(&payload.actor).is_some() {
                    return false;
                }
                self.guilds.push(Guild {
                    id: payload.guild_id,
                    name,
                    leader: payload.actor,
                    members: vec![payload.actor],
                    founded_ms: payload.timestamp_ms,
                });
                true
            }
            GUILD_OP_JOIN => {
                if self.membership(&payload.actor).is_some() {
                    return false;
                }
                let Some(g) = self.guilds.iter_mut().find(|g| g.id == payload.guild_id) else {
                    return false;
                };
                if g.members.len() >= MAX_GUILD_MEMBERS {
                    return false;
                }
                g.members.push(payload.actor);
                true
            }
            GUILD_OP_LEAVE => {
                let Some(idx) = self.guilds.iter().position(|g| g.id == payload.guild_id) else {
                    return false;
                };
                let g = &mut self.guilds[idx];
                let pos = match g.members.iter().position(|m| *m == payload.actor) {
                    Some(p) => p,
                    None => return false,
                };
                g.members.remove(pos);
                // Leader stepping down → next member inherits. Empty
                // membership → guild dissolved.
                if g.members.is_empty() {
                    self.guilds.remove(idx);
                } else if g.leader == payload.actor {
                    g.leader = g.members[0];
                }
                true
            }
            GUILD_OP_DISBAND => {
                let Some(idx) = self.guilds.iter().position(|g| g.id == payload.guild_id) else {
                    return false;
                };
                // Leader-only — anyone else is a no-op. Note: if the
                // founder handed off via LEAVE before disbanding,
                // they've already lost the right to tear it down.
                if self.guilds[idx].leader != payload.actor {
                    return false;
                }
                self.guilds.remove(idx);
                true
            }
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GuildsDelta {
    pub ops: Vec<GuildOp>,
}
