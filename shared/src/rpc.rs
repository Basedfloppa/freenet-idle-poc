//! Delegate RPC surface: what the webapp can ASK the delegate, and
//! what the delegate can ANSWER. Wrapped on the wire by
//! `freenet::DelegateEnvelopeIn/Out` to round-trip a request id
//! (freenet's `DelegateContext` is wiped on the response leg).
//!
//! Every variant is a game action (run a fight, equip gear, buy a
//! consumable, etc.). The game's authoritative model lives in
//! `crate::game::Inventory`; this module is just the protocol shape.

use serde::{Deserialize, Serialize};

use crate::freenet::{byte_array_32, byte_array_64, PubKey, PUBKEY_LEN, SIG_LEN};
use crate::game::Inventory;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DelegateRequest {
    /// Return the public key derived from the node's seed. The
    /// `seed_if_missing` value is **only** consumed if no seed has
    /// been stored on this node yet — once a seed is on disk, it's
    /// authoritative. This lets the webapp use browser entropy on
    /// first run (when there's no seed to use anyway) without ever
    /// being trusted again on subsequent runs.
    GetPubkey {
        #[serde(with = "byte_array_32")]
        seed_if_missing: [u8; PUBKEY_LEN],
    },
    /// Authoritative presence publish. The delegate constructs the
    /// `PresencePayload` itself — `public_key`, `gold` and
    /// `boss_damage` come straight from the secret store, so a
    /// compromised webapp cannot inject inflated values into the
    /// leaderboard or World Boss aggregate. The webapp supplies only
    /// the presentation fields it owns: display `name`, free-form
    /// `area`, and the current wall-clock.
    ///
    /// Replaces the previous `SignPayload(bytes)` RPC, which was an
    /// open "sign whatever I send" oracle — the principal anti-cheat
    /// hole on the boss/leaderboard side.
    PublishPresence {
        name: String,
        area: String,
        now_ms: u64,
    },
    /// Read the persisted inventory. Also applies HP regen and any
    /// outstanding achievement unlocks for the supplied wall clock.
    LoadInventory { now_ms: u64 },
    /// Run one combat round in the current area. Resolves
    /// turn-by-turn fights against `ENCOUNTERS_PER_MISSION` enemies
    /// from the area's roster. On win: rewards + drops. On loss:
    /// transformation if the killing enemy has a `transform_to`.
    RunMission { now_ms: u64 },
    /// Choose a farming area. Refused if the player's derived level
    /// is below the area's `min_level`.
    SetArea { area_id: u8, now_ms: u64 },
    /// Move a piece of gear from `unequipped` to `equipped[slot]`.
    EquipGear { catalog_id: u16, now_ms: u64 },
    /// Move whatever's in `equipped[slot]` back to `unequipped`.
    UnequipSlot { slot: u8, now_ms: u64 },
    /// Consume one potion (kind=0) or one fireball (kind=1).
    UseConsumable { kind: u8, now_ms: u64 },
    /// Spend gold to acquire a consumable from the shop.
    BuyItem { kind: u8, now_ms: u64 },
    /// Sell a piece of gear from the stash back to the merchant.
    SellGear { catalog_id: u16, now_ms: u64 },
    /// Combine `FORGE_COUNT` copies of the same catalog item +
    /// `forge_essence_cost(tier)` essence → next-tier same slot.
    ForgeUpgrade { catalog_id: u16, now_ms: u64 },
    /// Work the farm — +1 wheat per call. Safe non-combat income.
    WorkFarm { now_ms: u64 },
    /// Convert wheat to gold at `WHEAT_PER_GOLD : 1`. `amount=0`
    /// sells all owned wheat.
    SellWheat { amount: u64, now_ms: u64 },
    /// Buy a pre-rolled gear piece of the requested slot+tier.
    BuyGearRoll { slot: u8, tier: u8, now_ms: u64 },
    /// Walk every form-allowed slot and equip the strongest piece
    /// the player owns for that slot.
    AutoEquipBest { now_ms: u64 },
    /// Buy a skill from the Sage. Costs essence; refused for
    /// Veteran/Champion (level-gated).
    BuySkill { skill_id: u8, now_ms: u64 },
    /// Flip the persistent auto-mission switch and record the
    /// timestamp. While `enabled = true`, every subsequent
    /// inventory-touch call (`LoadInventory`, `RunMission`, …) runs
    /// the delegate's offline-catch-up loop, so closing the tab no
    /// longer pauses the adventure.
    SetAutoRun { enabled: bool, now_ms: u64 },
    /// Return the Ed25519 seed bytes for export / backup. The
    /// returned blob is the *private* signing key — anyone holding
    /// it can impersonate this player on the contract. Webapps
    /// should only surface it through a deliberate "I want to back
    /// up / migrate" flow with a clipboard-clear hint.
    ExportSeed,
    /// Wipe the persisted inventory back to `Inventory::default()`.
    /// Identity (seed + pubkey) is left untouched — leaderboards
    /// still know who this player is, the avatar simply restarts at
    /// level 1. Intended as the "I want a fresh playthrough" knob.
    ResetInventory { now_ms: u64 },
    /// Compose a mailbox message addressed to `to`, sign it with
    /// the player's identity key, return the signed entry bytes.
    /// The webapp publishes the result to the mailbox contract via
    /// the standard ContractOp::Update path — the delegate doesn't
    /// talk to the contract directly. `kind` and `body` are
    /// payload-agnostic; the recipient interprets them.
    SendMessage {
        #[serde(with = "byte_array_32")]
        to: [u8; 32],
        kind: u8,
        body: Vec<u8>,
        now_ms: u64,
    },
    /// Queue a player action (potion / fireball) for the next
    /// turn of the active battle. Returns the updated inventory.
    /// `action` is one of `BATTLE_ACTION_*` constants.
    QueueBattleAction { action: u8, now_ms: u64 },
    /// Advance the active battle to the current wall-clock without
    /// queuing anything. The frontend's tight poll during a fight
    /// calls this; auto-mode + `RunMission` cover the rest.
    TickBattle { now_ms: u64 },
    /// Sign a guild op (CREATE / JOIN / LEAVE) and return its bytes
    /// so the webapp can publish them to the guilds contract. The
    /// delegate stamps the `actor` field with its authoritative
    /// pubkey, just like `PublishPresence` and `SendMessage`.
    /// `name_or_id` is the guild name for CREATE (UTF-8) or the
    /// 32-byte guild id (hex-encoded, exactly 64 chars) for
    /// JOIN/LEAVE — the delegate disambiguates on `op_kind`.
    SignGuildOp {
        op_kind: u8,
        name_or_id: String,
        now_ms: u64,
    },
    /// Read the persisted UI prefs blob (display name + theme).
    /// Returns `[UiPrefs::default()`] if nothing's stored yet.
    ///
    /// **Deprecated** in favour of `LoadBlob { kind: BlobKind::Settings }`.
    /// Kept for one migration cycle so cached webapp builds still talk
    /// to the upgraded delegate. The new path uses JSON-encoded blobs
    /// which let the frontend evolve its schema without re-publishing
    /// the delegate (no `delegate_key` rotation, no identity loss).
    LoadUiPrefs,
    /// Replace the persisted UI prefs blob with the supplied one.
    ///
    /// **Deprecated** — see `LoadUiPrefs` note. Use
    /// `SaveBlob { kind: BlobKind::Settings, payload: <JSON bytes> }`.
    SaveUiPrefs { prefs: UiPrefs },

    /// Read the JSON-encoded blob for `kind`. Returns
    /// `AppResponse::Blob { payload: None }` if nothing's stored yet,
    /// so callers can apply their own defaults. The delegate treats
    /// the bytes as opaque — schema evolution lives entirely on the
    /// caller side via `#[serde(default)]` + ignored unknown fields.
    ///
    /// Adding a new `BlobKind` variant still requires a delegate
    /// rebuild (and `delegate_key` rotation per #4117). Adding a new
    /// field WITHIN an existing kind is frontend-only.
    LoadBlob { kind: BlobKind },
    /// Persist `payload` (JSON bytes, opaque) under `kind`. Read-modify-
    /// write is the caller's responsibility — the delegate does not
    /// merge; each save replaces the entire blob for that kind.
    SaveBlob { kind: BlobKind, payload: Vec<u8> },
    /// Hire one more worker of the given Estate tier. Refused if
    /// the player can't afford the next-worker gold price.
    /// (Backlog B2.)
    BuyEstateWorker { tier_id: u8, now_ms: u64 },
    /// Switch the single active idle action (§5.6). Setting to
    /// `IDLE_ACTION_AUTO_MISSION` mirrors `SetAutoRun(true)`;
    /// `IDLE_ACTION_ESTATE` pauses auto-mission and starts ticking
    /// the Estate; `IDLE_ACTION_NONE` pauses both.
    SetIdleAction { action: u8, now_ms: u64 },
    /// Spend stars to level up a Legacy node (backlog C1). Refused
    /// if the player doesn't have enough stars or the `node_id`
    /// isn't recognised.
    BuyLegacyNode { node_id: u8, now_ms: u64 },
    /// Soft-reset the current run (clear gold, gear, Estate
    /// workers, area, mission battle state). Keeps stars + Legacy
    /// nodes, identity, level, mission_count. Increments
    /// `ascend_count`. (Backlog C1, opt-in personal ascension.)
    Ascend { now_ms: u64 },
}

/// Domain split for blob-encoded persisted state. Each variant maps
/// to a separate secret-store slot on the delegate. Adding a new
/// variant is the only change that requires re-publishing the delegate;
/// growing the JSON inside an existing variant is frontend-only.
///
/// `repr(u8)` + explicit discriminants pin the wire format so the
/// delegate and frontend can be built independently and still agree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum BlobKind {
    /// UI preferences: display name, theme, locale, tutorial-dismissed,
    /// future per-player cosmetic toggles. Delegate stores opaque,
    /// frontend owns the schema.
    Settings = 0,
    /// Per-run game state the webapp wants to survive reloads but
    /// that's NOT part of the authoritative `Inventory` (e.g.
    /// "currently viewing tab", expanded panel state).
    GameState = 1,
    /// Identity / account metadata the webapp wants to attach to the
    /// pubkey but that isn't part of `Inventory` (e.g. avatar choice,
    /// pronouns). Survives reset-progress.
    Character = 2,
    /// Future home for the inventory blob. Currently delegate still
    /// owns typed `Inventory` for `RunMission`; this slot is reserved
    /// for the eventual move to JSON-on-the-wire so frontend can grow
    /// cosmetic fields without delegate cooperation. Not yet wired —
    /// requesting today returns `AppResponse::Error`.
    Inventory = 3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DelegateResponse {
    Pubkey {
        #[serde(with = "byte_array_32")]
        pubkey: PubKey,
    },
    /// Reply to `PublishPresence`: the bincode-serialized
    /// `PresencePayload` the delegate built, plus its Ed25519
    /// signature. Webapp wraps these in a `SignedEntry` and forwards
    /// to the presence contract verbatim — it never sees the inner
    /// fields, so it cannot tamper with them.
    SignedPresence {
        payload: Vec<u8>,
        #[serde(with = "byte_array_64")]
        signature: [u8; SIG_LEN],
    },
    /// Response to read/mutate calls — the post-operation inventory.
    Inventory(Inventory),
    /// Reply to `ExportSeed`. The 32-byte Ed25519 secret key,
    /// suitable for hex/base58 encoding by the webapp.
    Seed {
        #[serde(with = "byte_array_32")]
        seed: [u8; 32],
    },
    /// Reply to `SendMessage`. The bincode-serialized
    /// `MessagePayload` the delegate built plus its signature.
    /// Webapp wraps the pair into a `MailboxEntry` and Updates the
    /// mailbox contract.
    SignedMessage {
        payload: Vec<u8>,
        #[serde(with = "byte_array_64")]
        signature: [u8; SIG_LEN],
    },
    /// Reply to `SignGuildOp`. Same shape as `SignedMessage` — the
    /// webapp wraps these into a `GuildOp` and Updates the guilds
    /// contract.
    SignedGuildOp {
        payload: Vec<u8>,
        #[serde(with = "byte_array_64")]
        signature: [u8; SIG_LEN],
    },
    /// Reply to `LoadUiPrefs` / `SaveUiPrefs` — the canonical prefs
    /// snapshot held by the delegate.
    UiPrefs(UiPrefs),
    /// Reply to `LoadBlob` — the opaque JSON-encoded bytes for the
    /// requested domain, or `None` if no save has happened yet.
    Blob {
        kind: BlobKind,
        payload: Option<Vec<u8>>,
    },
    /// Reply to `SaveBlob` — echoes the kind that was just written
    /// so the caller can correlate concurrent saves. No payload echo
    /// — the caller already has the bytes it just sent.
    BlobSaved { kind: BlobKind },
    Error(String),
}

/// Cosmetic / non-game state persisted on the delegate so it survives
/// browser localStorage being unavailable inside the sandboxed
/// webapp iframe. Loaded once on connect, written on user changes.
///
/// Every field is an `Option<_>` so `Default` is "no preference"
/// (frontend falls back to its own defaults). New fields are added
/// at the **end** of the struct; older blobs serialized with bincode
/// hit EOF on the new trailing field and fall back to the `UiPrefsV1`
/// legacy decoder (see `delegate/state.rs::load_ui_prefs`). Bincode 1
/// is length-prefixed and doesn't honour `#[serde(default)]` for
/// truncated input, so the V1/V2 split is load-bearing.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct UiPrefs {
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub theme: Option<String>,
    /// `Some(true)` once the player has completed (or skipped) the
    /// first-run onboarding wizard. Loaded on connect; the wizard
    /// stays open until either the response confirms `true` or the
    /// user clicks through and saves `true` themselves.
    #[serde(default)]
    pub tutorial_dismissed: Option<bool>,
    /// UI locale short code ("en", "ru"). Persisted on the delegate
    /// so a player's language choice survives moves between browsers
    /// / cleared caches / sandbox null-origin localStorage wipes.
    /// `None` = the delegate has no opinion yet (new install or
    /// V1-blob migration); frontend keeps its own defaults until
    /// the user picks one.
    #[serde(default)]
    pub locale: Option<String>,
}

/// Legacy 3-field shape used by delegate releases before `locale`
/// was added. Kept around so `load_ui_prefs` can fall back when the
/// stored secret was written by an older delegate build. Promotion
/// to the current `UiPrefs` is a lossless field-by-field copy with
/// `locale: None` — the player's first explicit picker click
/// rewrites the blob in the new shape.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UiPrefsV1 {
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub theme: Option<String>,
    #[serde(default)]
    pub tutorial_dismissed: Option<bool>,
}

impl From<UiPrefsV1> for UiPrefs {
    fn from(v: UiPrefsV1) -> Self {
        Self {
            display_name: v.display_name,
            theme: v.theme,
            tutorial_dismissed: v.tutorial_dismissed,
            locale: None,
        }
    }
}

#[cfg(test)]
mod ui_prefs_tests {
    use super::*;

    #[test]
    fn v1_blob_roundtrips_to_v2_with_locale_none() {
        // Simulate an older delegate save: 3-field bincode of the
        // legacy shape with a populated name + theme.
        let v1 = UiPrefsV1 {
            display_name: Some("Alice".into()),
            theme: Some("dusk".into()),
            tutorial_dismissed: Some(true),
        };
        let bytes = bincode::serialize(&v1).unwrap();

        // V2 (current shape) deserialize must reject the truncated
        // blob — bincode is length-prefixed and can't infer the
        // missing trailing `locale` field via `#[serde(default)]`.
        assert!(
            bincode::deserialize::<UiPrefs>(&bytes).is_err(),
            "V2 decode of V1 bytes must fail so the load path falls back to UiPrefsV1"
        );

        // The legacy decoder accepts it cleanly; lifting promotes
        // every populated field and leaves `locale = None`.
        let legacy: UiPrefsV1 = bincode::deserialize(&bytes).unwrap();
        let promoted: UiPrefs = legacy.into();
        assert_eq!(promoted.display_name.as_deref(), Some("Alice"));
        assert_eq!(promoted.theme.as_deref(), Some("dusk"));
        assert_eq!(promoted.tutorial_dismissed, Some(true));
        assert!(promoted.locale.is_none());
    }

    #[test]
    fn v2_blob_decodes_with_locale() {
        let v2 = UiPrefs {
            display_name: Some("Bob".into()),
            theme: None,
            tutorial_dismissed: None,
            locale: Some("ru".into()),
        };
        let bytes = bincode::serialize(&v2).unwrap();
        let round: UiPrefs = bincode::deserialize(&bytes).unwrap();
        assert_eq!(round, v2);
    }

    #[test]
    fn v2_default_blob_decodes_with_all_none() {
        let bytes = bincode::serialize(&UiPrefs::default()).unwrap();
        let round: UiPrefs = bincode::deserialize(&bytes).unwrap();
        assert!(round.display_name.is_none());
        assert!(round.theme.is_none());
        assert!(round.tutorial_dismissed.is_none());
        assert!(round.locale.is_none());
    }
}
