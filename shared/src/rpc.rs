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
    Error(String),
}
