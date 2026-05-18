//! Reset boundaries — every code path that wipes part of an
//! `Inventory` should name itself via one of these variants so an
//! audit / changelog can spot accidental data loss before it ships.
//!
//! See `docs/planned-work-2026-05-17.md §6` for the motivation:
//! 2026-05-17 we lost every player inventory because a delegate
//! rebuild silently rotated the secrets namespace. The script-level
//! safety gate (`ALLOW_DELEGATE_REPUBLISH`) is one half of the fix;
//! this enum is the other half — every reset must be intentional.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResetScope {
    /// Soft-reset run state on opt-in Ascend — clears gold, gear,
    /// area, HP, missions, area_clears, XP. Preserves Legacy
    /// ledger + achievements + tokens + forms_visited + skills +
    /// boss-era watermarks. See `identity-delegate::actions::legacy::ascend`.
    Ascend,

    /// Hard wipe via Settings → Advanced → "Reset progress". Only
    /// triggered behind an explicit UI confirm. Sets the inventory
    /// back to `InventoryV15::default()`. No partial state survives.
    NewPlayer,

    /// Schema migration between Inventory wrapper versions. Pure
    /// field-by-field copy with `From<VN-1>` impls; no data loss.
    /// Documented per-bump in `shared/src/game/inventory.rs`.
    SchemaMigration { from: u8, to: u8 },
}

impl ResetScope {
    /// Short tag for logging / audit trails.
    pub fn tag(self) -> &'static str {
        match self {
            ResetScope::Ascend => "ascend",
            ResetScope::NewPlayer => "new_player",
            ResetScope::SchemaMigration { .. } => "schema_migration",
        }
    }
}
