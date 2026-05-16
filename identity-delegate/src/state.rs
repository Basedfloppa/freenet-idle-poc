//! Freenet-side state I/O for the identity delegate.
//!
//! Everything in this module talks to the freenet secret store:
//! loading and saving the player's bincode'd `Inventory`, loading
//! the Ed25519 seed, applying first-touch initialization, and
//! enforcing monotonic `now_ms` across actions.

use ed25519_dalek::SigningKey;
use freenet_stdlib::prelude::*;

use shared::{
    rpc::BlobKind, Inventory, InventoryV10, InventoryV9, InventoryWire, UiPrefs, UiPrefsV1,
    BLOB_SECRET_ID_CHARACTER, BLOB_SECRET_ID_GAMESTATE, BLOB_SECRET_ID_INVENTORY,
    BLOB_SECRET_ID_SETTINGS, FORM_HUMAN, IDENTITY_SECRET_ID, INVENTORY_SECRET_ID, STARTING_HP,
    UI_PREFS_SECRET_ID,
};

use crate::derived::max_hp_of;

pub fn load_seed(ctx: &mut DelegateCtx) -> Result<Option<SigningKey>, String> {
    if !ctx.has_secret(IDENTITY_SECRET_ID) {
        return Ok(None);
    }
    let bytes = ctx
        .get_secret(IDENTITY_SECRET_ID)
        .ok_or_else(|| "secret unexpectedly absent after has_secret=true".to_string())?;
    let arr: [u8; 32] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| "stored seed has wrong length".to_string())?;
    Ok(Some(SigningKey::from_bytes(&arr)))
}

pub fn load_or_install_seed(
    ctx: &mut DelegateCtx,
    seed_if_missing: &[u8; 32],
) -> Result<SigningKey, String> {
    if let Some(sk) = load_seed(ctx)? {
        return Ok(sk);
    }
    if !ctx.set_secret(IDENTITY_SECRET_ID, seed_if_missing) {
        return Err("set_secret rejected".into());
    }
    Ok(SigningKey::from_bytes(seed_if_missing))
}

/// Load the persisted inventory.
///
/// Tries formats in order, newest to oldest:
///   1. `InventoryWire` (versioned envelope, current format).
///      `into_latest` migrates older variants up to `Inventory`.
///   2. Bare `InventoryV9` — saves from before the wrapper was
///      introduced. Explicit deserialize-as-V9 here (not "as
///      Inventory") because `Inventory` is now `InventoryV10` and
///      has a different bincode layout. Migrated through `From`.
///   3. `Inventory::default()` — fresh start.
pub fn load_inventory_raw(ctx: &mut DelegateCtx) -> Inventory {
    let Some(bytes) = ctx.get_secret(INVENTORY_SECRET_ID) else {
        return Inventory::default();
    };
    if let Ok(wire) = bincode::deserialize::<InventoryWire>(&bytes) {
        return wire.into_latest();
    }
    if let Ok(inv_v9) = bincode::deserialize::<InventoryV9>(&bytes) {
        return shared::InventoryV15::from(shared::InventoryV14::from(shared::InventoryV13::from(
            shared::InventoryV12::from(shared::InventoryV11::from(InventoryV10::from(inv_v9))),
        )));
    }
    Inventory::default()
}

/// Load legacy UiPrefs blob. Tries the current shape first, falls
/// back to `UiPrefsV1` (pre-locale) — bincode 1 can't apply
/// `#[serde(default)]` to truncated input. Used only by the
/// one-shot migration path in `load_blob`; new code uses
/// `BlobKind::Settings` JSON.
pub fn load_ui_prefs(ctx: &mut DelegateCtx) -> UiPrefs {
    let Some(bytes) = ctx.get_secret(UI_PREFS_SECRET_ID) else {
        return UiPrefs::default();
    };
    if let Ok(prefs) = bincode::deserialize::<UiPrefs>(&bytes) {
        return prefs;
    }
    if let Ok(legacy) = bincode::deserialize::<UiPrefsV1>(&bytes) {
        return legacy.into();
    }
    UiPrefs::default()
}

/// Persist UI prefs. The whole blob is replaced each call — callers
/// are expected to read-modify-write when changing a single field.
pub fn save_ui_prefs(ctx: &mut DelegateCtx, prefs: &UiPrefs) -> Result<(), String> {
    let bytes = bincode::serialize(prefs).map_err(|e| format!("ser ui prefs: {e}"))?;
    if !ctx.set_secret(UI_PREFS_SECRET_ID, &bytes) {
        return Err("set_secret rejected".into());
    }
    Ok(())
}

fn blob_secret_id(kind: BlobKind) -> &'static [u8] {
    match kind {
        BlobKind::Settings => BLOB_SECRET_ID_SETTINGS,
        BlobKind::GameState => BLOB_SECRET_ID_GAMESTATE,
        BlobKind::Character => BLOB_SECRET_ID_CHARACTER,
        BlobKind::Inventory => BLOB_SECRET_ID_INVENTORY,
    }
}

/// Read the opaque blob for `kind`. Returns `None` if nothing's
/// stored — caller applies its own defaults. The delegate doesn't
/// interpret the bytes; schema is the caller's job.
pub fn load_blob(ctx: &mut DelegateCtx, kind: BlobKind) -> Option<Vec<u8>> {
    let id = blob_secret_id(kind);
    if let Some(bytes) = ctx.get_secret(id) {
        return Some(bytes);
    }
    // One-shot migration: legacy bincode `UiPrefs` -> JSON Settings.
    if matches!(kind, BlobKind::Settings) {
        let legacy = load_ui_prefs(ctx);
        if legacy != UiPrefs::default() {
            if let Ok(json) = serde_json::to_vec(&legacy) {
                let _ = ctx.set_secret(id, &json);
                return Some(json);
            }
        }
    }
    None
}

/// Replace the opaque blob for `kind` with `payload`. Caller is
/// responsible for read-modify-write — the delegate does not merge.
pub fn save_blob(ctx: &mut DelegateCtx, kind: BlobKind, payload: &[u8]) -> Result<(), String> {
    let id = blob_secret_id(kind);
    if !ctx.set_secret(id, payload) {
        return Err("set_secret rejected".into());
    }
    Ok(())
}

/// Persist inventory as the latest `InventoryWire` variant.
/// Recomputes the phased-reveal bitmask before serialization so
/// newly-true predicates latch on disk and propagate back to the
/// caller's in-memory copy.
pub fn save_inventory(ctx: &mut DelegateCtx, inv: &mut Inventory) -> Result<(), String> {
    let lvl = shared::level_of(inv);
    let _flipped = shared::recompute_reveals(inv, lvl);
    // Award milestones idempotently on every save — each helper
    // is watermark-anchored so calling them on every mutation is
    // free for paths that didn't move the relevant counter.
    crate::actions::legacy::award_pending_stars(inv);
    crate::actions::insight::award_pending_insight(inv);
    crate::actions::tokens::award_pending_tokens(inv);
    // Pump routine auto-hire here, not just from
    // `touch_inventory`. A player running auto-mission earns
    // gold inside `run_mission`/`tick_only` and never hits the
    // pull-tick path while a battle is running — the previous
    // wiring left their Routine targets dormant for minutes. The
    // pump is cheap (no targets → no-op) and idempotent.
    crate::actions::routine::pump_routine(inv);
    let wire = InventoryWire::from(inv.clone());
    let bytes = bincode::serialize(&wire).map_err(|e| format!("ser inventory: {e}"))?;
    if !ctx.set_secret(INVENTORY_SECRET_ID, &bytes) {
        return Err("set_secret rejected".into());
    }
    Ok(())
}

/// On every action: enforce monotonic `now_ms`, run HP regen for
/// the elapsed interval, set first-touch defaults (plot seed,
/// initial form-visited record, HP filled). Returns the inventory
/// ready for the caller to mutate further.
pub fn enter_action(inv: &mut Inventory, now_ms: u64) -> Result<(), String> {
    if inv.last_action_ms == 0 {
        inv.last_action_ms = now_ms;
        inv.last_hp_tick_ms = now_ms;
        if inv.current_hp == 0 {
            inv.current_hp = STARTING_HP;
        }
        if inv.plot_seed == 0 {
            inv.plot_seed = (now_ms as u32) ^ ((now_ms >> 32) as u32) ^ 0x9E37_79B9;
        }
        // First touch = first "visit" to Human form.
        inv.forms_visited.entry(FORM_HUMAN).or_insert(now_ms);
        return Ok(());
    }
    if now_ms < inv.last_action_ms {
        return Err(format!(
            "non-monotonic now_ms: got {now_ms} < last {}",
            inv.last_action_ms
        ));
    }
    apply_hp_regen(inv, now_ms);
    inv.last_action_ms = now_ms;
    Ok(())
}

/// Top up `current_hp` toward `max_hp_of(inv)` proportional to the
/// elapsed time since the last regen tick. Full regen takes
/// `HP_FULL_REGEN_MS` from 0 to max. **Skipped during an active
/// battle** — passive recovery while the player is being hit would
/// trivialise sustained fights; the battle resolver applies damage
/// every turn and the player can still queue a Potion mid-fight if
/// they need an emergency heal.
pub fn apply_hp_regen(inv: &mut Inventory, now_ms: u64) {
    if inv.current_battle.is_some() {
        // Keep the regen anchor advancing so the moment the battle
        // resolves the clock starts from "now", not "the last
        // pre-fight tick" — without this, finishing a fight would
        // dump a chunk of catch-up regen into the bar.
        inv.last_hp_tick_ms = now_ms;
        return;
    }
    let cap = max_hp_of(inv);
    if inv.current_hp >= cap {
        inv.last_hp_tick_ms = now_ms;
        return;
    }
    let elapsed = now_ms.saturating_sub(inv.last_hp_tick_ms);
    if elapsed == 0 {
        return;
    }
    let regen = cap
        .saturating_mul(elapsed)
        .checked_div(shared::HP_FULL_REGEN_MS)
        .unwrap_or(0);
    inv.current_hp = (inv.current_hp.saturating_add(regen)).min(cap);
    inv.last_hp_tick_ms = now_ms;
}
