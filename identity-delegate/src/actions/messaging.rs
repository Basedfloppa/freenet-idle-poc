//! Off-presence wire: signed mailbox messages and guild ops.
//! Delegate stamps the authoritative pubkey, signs, and hands the
//! bytes back; the webapp publishes via `ContractOp::Update` on the
//! mailbox / guilds contracts.

use ed25519_dalek::Signer;
use freenet_stdlib::prelude::*;

use shared::{MessagePayload, MAX_MESSAGE_BODY_BYTES, SIG_LEN};

use crate::state::load_seed;

/// Sign a guild op (CREATE / JOIN / LEAVE / DISBAND). The webapp
/// passes the op kind and either the guild name (CREATE) or
/// hex-encoded id (JOIN/LEAVE/DISBAND); the delegate stamps `actor`
/// with the player's pubkey and signs.
pub fn sign_guild_op(
    ctx: &mut DelegateCtx,
    op_kind: u8,
    name_or_id: String,
    now_ms: u64,
) -> Result<(Vec<u8>, [u8; SIG_LEN]), String> {
    use shared::{
        GuildOpPayload, GUILD_OP_CREATE, GUILD_OP_DISBAND, GUILD_OP_JOIN, GUILD_OP_LEAVE,
    };
    let sk = load_seed(ctx)?.ok_or_else(|| "no seed installed yet".to_string())?;
    let actor = sk.verifying_key().to_bytes();
    let payload = match op_kind {
        GUILD_OP_CREATE => {
            let name = name_or_id.trim().to_string();
            if name.is_empty() || name.len() > shared::MAX_GUILD_NAME_BYTES {
                return Err("guild name must be 1..=32 UTF-8 bytes".into());
            }
            GuildOpPayload::new_create(actor, name, now_ms)
        }
        GUILD_OP_JOIN | GUILD_OP_LEAVE | GUILD_OP_DISBAND => {
            // All three of these address an existing guild by id —
            // 32 hex bytes from the webapp.
            let id_bytes = parse_hex_32(&name_or_id)?;
            match op_kind {
                GUILD_OP_JOIN => GuildOpPayload::new_join(actor, id_bytes, now_ms),
                GUILD_OP_LEAVE => GuildOpPayload::new_leave(actor, id_bytes, now_ms),
                GUILD_OP_DISBAND => GuildOpPayload::new_disband(actor, id_bytes, now_ms),
                _ => unreachable!(),
            }
        }
        other => return Err(format!("unknown guild op kind {other}")),
    };
    let bytes = bincode::serialize(&payload).map_err(|e| format!("ser payload: {e}"))?;
    let signature: ed25519_dalek::Signature = sk.sign(&bytes);
    Ok((bytes, signature.to_bytes()))
}

/// Decode a 64-char hex string into 32 bytes. Helper for
/// `sign_guild_op` — keeps the JOIN/LEAVE payload parsing tight.
fn parse_hex_32(s: &str) -> Result<[u8; 32], String> {
    if s.len() != 64 {
        return Err(format!("expected 64 hex chars, got {}", s.len()));
    }
    let mut out = [0u8; 32];
    for (i, byte) in out.iter_mut().enumerate() {
        let hi = hex_digit(s.as_bytes()[i * 2])?;
        let lo = hex_digit(s.as_bytes()[i * 2 + 1])?;
        *byte = (hi << 4) | lo;
    }
    Ok(out)
}

fn hex_digit(c: u8) -> Result<u8, String> {
    match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        b'A'..=b'F' => Ok(c - b'A' + 10),
        _ => Err(format!("bad hex digit {:?}", c as char)),
    }
}

/// Sign a mailbox message from the player to `to`. Mirrors
/// `publish_presence`: webapp supplies the recipient + kind + body,
/// delegate stamps `from` with the authoritative pubkey, signs, and
/// hands the bytes back. The webapp then publishes them via
/// `ContractOp::Update` on the mailbox contract.
pub fn send_message(
    ctx: &mut DelegateCtx,
    to: [u8; 32],
    kind: u8,
    body: Vec<u8>,
    now_ms: u64,
) -> Result<(Vec<u8>, [u8; SIG_LEN]), String> {
    if body.len() > MAX_MESSAGE_BODY_BYTES {
        return Err(format!(
            "body {} bytes exceeds {} cap",
            body.len(),
            MAX_MESSAGE_BODY_BYTES
        ));
    }
    let sk = load_seed(ctx)?.ok_or_else(|| "no seed installed yet".to_string())?;
    let payload = MessagePayload::new(sk.verifying_key().to_bytes(), to, kind, body, now_ms);
    let bytes = bincode::serialize(&payload).map_err(|e| format!("ser payload: {e}"))?;
    let signature: ed25519_dalek::Signature = sk.sign(&bytes);
    Ok((bytes, signature.to_bytes()))
}
