//! Freenet-side state I/O for the identity delegate.
//!
//! Everything in this module talks to the freenet secret store:
//! loading and saving the player's bincode'd `Inventory`, loading
//! the Ed25519 seed, applying first-touch initialization, and
//! enforcing monotonic `now_ms` across actions.

use ed25519_dalek::SigningKey;
use freenet_stdlib::prelude::*;

use shared::{
    Inventory, InventoryV10, InventoryV9, InventoryWire, FORM_HUMAN, IDENTITY_SECRET_ID,
    INVENTORY_SECRET_ID, STARTING_HP,
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
        return InventoryV10::from(inv_v9);
    }
    Inventory::default()
}

/// Persist inventory always as the latest `InventoryWire` variant.
/// Old bare-`Inventory` saves get promoted to the wrapper format the
/// first time we save after the upgrade.
pub fn save_inventory(ctx: &mut DelegateCtx, inv: &Inventory) -> Result<(), String> {
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
/// `HP_FULL_REGEN_MS` from 0 to max.
pub fn apply_hp_regen(inv: &mut Inventory, now_ms: u64) {
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
