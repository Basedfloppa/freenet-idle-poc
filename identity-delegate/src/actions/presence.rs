//! Presence publishing — delegate stamps the authoritative
//! pubkey/gold/boss_damage onto the payload and signs it before
//! handing the bytes back to the webapp.

use ed25519_dalek::Signer;
use freenet_stdlib::prelude::*;

use shared::{PresencePayload, MAX_AREA_BYTES, MAX_NAME_BYTES, SIG_LEN};

use crate::progression::check_achievements;
use crate::state::{enter_action, load_inventory_raw, load_seed, save_inventory};

/// Truncate `s` to at most `max` *bytes*, snapping back to the
/// previous UTF-8 char boundary so the result is always valid UTF-8.
/// The presence contract enforces a byte cap on `name`/`area`; we
/// stay below it deterministically rather than relying on webapp
/// hygiene.
fn truncate_bytes_at(mut s: String, max: usize) -> String {
    if s.len() <= max {
        return s;
    }
    let mut idx = max;
    while !s.is_char_boundary(idx) {
        idx -= 1;
    }
    s.truncate(idx);
    s
}

/// Build and sign a presence payload from authoritative inventory
/// state. The webapp picks the display fields (`name`, `area`,
/// `now_ms`); `gold` and `boss_damage` come straight from the secret
/// store, so a compromised webapp cannot inflate the leaderboard or
/// World Boss aggregate by asking for arbitrary numbers.
///
/// Returns `(payload_bytes, signature)` ready to wrap into a
/// `SignedEntry` and ship to the contract.
pub fn publish_presence(
    ctx: &mut DelegateCtx,
    name: String,
    area: String,
    now_ms: u64,
) -> Result<(Vec<u8>, [u8; SIG_LEN]), String> {
    let mut inv = load_inventory_raw(ctx);
    enter_action(&mut inv, now_ms)?;
    check_achievements(&mut inv, now_ms);
    save_inventory(ctx, &mut inv)?;
    let sk = load_seed(ctx)?.ok_or_else(|| "no seed installed yet".to_string())?;
    let pubkey = sk.verifying_key().to_bytes();
    let name = truncate_bytes_at(name, MAX_NAME_BYTES);
    let area = truncate_bytes_at(area, MAX_AREA_BYTES);
    let area_id = inv.current_area;
    let champion = inv.tokens.owns(shared::TokenPerk::ChampionBadge);
    let payload = PresencePayload::new(
        pubkey, name, inv.gold, inv.boss_damage, area, now_ms,
        area_id, champion,
    );
    let bytes = bincode::serialize(&payload).map_err(|e| format!("ser payload: {e}"))?;
    let signature: ed25519_dalek::Signature = sk.sign(&bytes);
    Ok((bytes, signature.to_bytes()))
}
