# Idle PoC

An idle/RPG on top of Freenet, where:

- Local UI preferences (theme, cadence, prefs) live **only** in the browser.
- Identity (Ed25519 seed) and the entire Inventory live **on the local node**, in the delegate secret store. The webapp is a thin client; "clear site data" resets nothing.
- Players see each other through **three aggregator contracts** (presence / mailbox / guilds): one WS subscription per contract, with Freenet doing the fan-out as the overlay network.

## Layout

```
idle-poc/
├── shared/                   wire types + game model (InventoryV10),
│                             format_si helper, versioned InventoryWire
├── presence-contract/        Rust contract: LWW merge + cumulative World Boss
│                             ledger + outlier-resistant prune; caps 1k entries
├── mailbox-contract/         Player-to-player signed log (gift / invite /
│                             trade / chat substrate). 5k entries, 7d TTL.
├── guilds-contract/          Op-sourced cooperative groups (CREATE/JOIN/LEAVE),
│                             one pubkey ≤ one guild, 256 guilds × 50 members.
├── identity-delegate/        Authoritative seed + Inventory store. Hosts the
│                             tick-based combat resolver, offline catch-up,
│                             auto-mission toggle, export/reset RPCs.
├── frontend/                 Yew + long-lived WS subscriptions (presence +
│                             optional mailbox + optional guilds) + unified
│                             check-elapsed tick loop. 3 themes, onboarding,
│                             debug overlay, Settings prefs.
└── scripts/
    ├── dev-publish.sh        builds and publishes all 3 contracts + delegate,
    │                         writes 8 keys into frontend/dev-keys.json
    ├── dev-watch.sh          incremental re-publish on changes in
    │                         shared/, presence/, mailbox/, guilds/, delegate/
    └── dev.sh                one command: watcher + `trunk serve`
```

## One-time setup

1. **Locally-built `freenet` / `fdev`.** PATH versions (0.1.x / 0.3.151) are wire-format incompatible with what our fdev builds. Pull from the local freenet-core checkout:
   ```fish
   cd ../freenet-core && cargo build --bin freenet --bin fdev
   ```

2. **Start the node.** Bind on `0.0.0.0`, otherwise fdev (IPv4) can't reach it:
   ```fish
   ../freenet-core/target/debug/freenet local \
       --ws-api-address 0.0.0.0 --ws-api-port 7509 \
       --data-dir /tmp/freenet-local
   ```

3. **Install `trunk`** (if not present):
   ```fish
   cargo install trunk
   ```

## Hot-reload dev loop

```fish
cd idle-poc
./scripts/dev.sh
```

The script:
1. Builds and publishes `presence-contract` → captures `instance_id` and `code_hash`.
2. Builds and publishes `mailbox-contract` → captures `instance_id` and `code_hash`.
3. Builds and publishes `guilds-contract` → captures `instance_id` and `code_hash`.
4. Builds and publishes `identity-delegate` → captures `key` and `code_hash`.
5. Writes all eight values into `frontend/dev-keys.json`.
6. Starts `trunk serve` on `http://127.0.0.1:9003/`.

`mailbox` and `guilds` are optional on the frontend side — if their keys are empty, the corresponding features (D2D test in Settings, Guilds tab) show "not configured" but the rest of the game works.

`dev-keys.json` is declared in `index.html` via `<link data-trunk rel="copy-file">`, so trunk:
- copies it into `dist/`,
- watches for changes and **triggers a tab hot-reload** on each re-publish.

The frontend does a `fetch('./dev-keys.json')` on startup and substitutes its values for the `const`s in `src/main.rs`. If a field is empty — fallback to the bake-in constant. No manual "paste this ID into main.rs" — no WASM rebuild needed.

### Re-publish (after editing a contract or the delegate)

In a separate terminal:
```fish
./scripts/dev-publish.sh
```

`trunk serve` notices the `dev-keys.json` change itself, reloads the bundle, and the page updates with the new IDs. WS sessions reopen automatically.

### Publish only (without trunk)

`scripts/dev-publish.sh` is self-contained. Useful when trunk is already running and you only need to refresh a contract / delegate.

### Override the fdev path / node port

```fish
FDEV=/custom/path/fdev WS_PORT=7510 ./scripts/dev.sh
```

## Publishing to a prod node (orange / baka)

The dev scripts above target a local node; prod deploys go through
two dedicated scripts. Versions must align — the local `fdev` from
`freenet-core/target/debug/` produces WASM with imports that need
`freenet-core ≥ 0.2.x` on the receiving node.

### First-time deploy

`scripts/prod-publish.sh` builds + publishes the three contracts +
delegate to a remote node, patches `frontend/src/app/keys.rs` with
the resulting IDs, builds the frontend with `trunk build --release`,
and finally pushes the bundle as a website contract via `fdev
website publish`.

Typical invocation, via SSH local-forward (orange's WS API on the
LAN is `192.168.88.247:7509`, but it's simplest to forward through
SSH to the loopback you control):

```fish
ssh -fNT -L 17509:127.0.0.1:7509 orange
NODE_URL=ws://127.0.0.1:17509 ./scripts/prod-publish.sh
```

What this leaves behind:

- `frontend/src/app/keys.rs` — patched with the new compile-time defaults
  (`.bak` saved alongside). Review the diff and commit.
- `frontend/prod-webapp-id.txt` — the website contract ID. Used by
  `prod-update-webapp.sh` and worth committing so teammates can
  iterate without re-running the full deploy.
- `frontend/dev-keys.json` — mirrored to the prod IDs so the local
  `trunk build --release` is coherent. Re-run `dev-publish.sh` to
  switch back to local-node IDs after deploy.

Set `PATCH_KEYS=0` to skip the `keys.rs` edit (when you want to
inspect the IDs first), or `STAGE_WEBAPP=0` to stop after publishing
the contracts/delegate.

### Subsequent webapp-only updates

When only the UI changed, `scripts/prod-update-webapp.sh` skips the
heavy contract/delegate republish: `trunk build --release` → `fdev
website update` → SSH the gateway and `rm -rf` the unpacked webapp
cache (per the [webapp-cache-invalidation](../) memory — without
this the gateway keeps serving the previous version even after the
new one lands in the DHT).

```fish
NODE_URL=ws://127.0.0.1:17509 ./scripts/prod-update-webapp.sh
```

Reads `frontend/prod-webapp-id.txt` automatically; override with
`WEBAPP_ID=…`. Set `SSH_HOST=""` to skip the cache rm if your
deployment uses a different invalidation path.

## What lives in which layer

| Layer | Where it lives | What goes there | When it resets |
|---|---|---|---|
| UI prefs (theme, sync cadence, hide_pubkey) | Browser `localStorage` | Display settings + onboarding flag | Settings → Reset to defaults, or DevTools → Clear storage |
| Active battle (HP, queue) | `Inventory.current_battle` on the node | Server-side state-machine combat | `Reset progress` or node wipe |
| Identity (Ed25519 seed) | Delegate secret at `/tmp/freenet-local/secrets/local/<delegate-key>/` (key `identity-seed-v1`) | Player's signing key | Wipe the data-dir; identity migrates via **Settings → Export seed** |
| Inventory (gold, gear, skills, achievements, …) | Same delegate secret store (key `inventory-v9`, format `InventoryWire::V10(...)`) | Full game progress | `Reset progress` (via delegate RPC) or wipe the data-dir |
| Presence (`anon-XXXX` + gold + boss_damage + ts) | `presence-contract`, one entry per pubkey | Leaderboard + World Boss aggregate | Auto-prune after 60s of silence (live), watermark persists in `cumulative_damage` |
| Cumulative World Boss watermark | `presence-contract.cumulative_damage` | Per-key high watermark | Cap-eviction at 10k unique keys |
| Mailbox messages | `mailbox-contract`, signed log | gift/invite/trade/chat (substrate) | 7-day TTL or 5k-entry cap |
| Guild membership | `guilds-contract`, op-sourced state | Name, leader, members | LEAVE / last LEAVE → dissolve |

## Feature status

### ✅ Done

**Gameplay**
- Tick-based combat (`TURN_COOLDOWN_MS = 1s`, initiative by `speed`, evasion as flat damage scaling)
- Mid-fight queueable actions: `Use Potion` (full heal) / `Use Fireball` (bonus damage)
- 5 encounters per mission, the chain advances automatically
- 4 areas (Village Fields → Forest Road → Mountain Pass → Boss's Lair) with level gates
- 5 forms (Human / Slime / Cat / Dragon / Horse) with transformation on loss → permanent skill (prestige loop)
- 8 slots × 4 tiers of gear (32 catalog ids), per-form slot mask
- Forge (3-of-a-kind + essence → next tier, up to Legendary)
- 11 achievements, 6 skills (4 form-derived + Veteran/Champion level milestones)
- 4 endings (Victory / Dragon Lord / Pilgrim / Quiet Farmer)
- Exponential XP leveling (1.5× per level), level-static base stats
- HP regen (`HP_FULL_REGEN_MS`)
- Procedural plot (6×6×6×6×6 = 7776 combinations)
- Wheat farm (10:1 → gold) as safe-mode income
- Shop: pre-rolled gear by slot+tier, potions, fireballs, Auto-Equip Best
- Combat history (ring buffer, `COMBAT_HISTORY_CAP = 30`)
- Sage skill shop (4 form-skills purchasable for essence)

**Multiplayer / Freenet**
- `presence-contract` — World Boss aggregator with a **persistent `cumulative_damage` ledger** (survives entry pruning)
- `mailbox-contract` — signed-log substrate for player-to-player messages (chat/gift/invite/trade — kind tags)
- `guilds-contract` — op-sourced cooperative groups with a "1 pubkey ≤ 1 guild" invariant, auto-handoff leader, dissolve on empty
- Auto-detect: if a key is unconfigured → the feature is disabled gracefully, the rest still works
- Lobby leaderboard, World Boss era progression (`era_max_hp = 500 × (era+1)²`)

**Persistence + Identity**
- `InventoryWire` non-destructive migration framework (V9 → V10 done); future bumps don't lose saves
- Authoritative delegate (`PublishPresence`, `SendMessage`, `SignGuildOp` — the webapp can't inject numbers)
- Persistent `auto_run_enabled` + offline catch-up (up to 1 hour of simulation on return)
- `Settings → Export seed` (Ed25519 hex export, identity migration between nodes)
- `Settings → Reset progress` (wipe Inventory, identity persists)

**Anti-cheat / robustness in the contracts**
- Version byte in `PresencePayload`, `ContractState`, `MailboxState`, `GuildsState` — forward-compat hook
- `MAX_TIMESTAMP_MS` (year 2100) — defense against u64::MAX prune-DoS
- `MAX_FORWARD_SKEW_MS` (5 min) — relative ts ceiling
- Per-key monotonicity of `gold` / `boss_damage` (can't regress)
- Outlier-resistant prune (second-largest pivot for `entries`)
- Order-independent cumulative cap (top-N rejection, proven by a 6-permutation test)
- `MAX_LIVE_ENTRIES = 1_000`, `MAX_CUMULATIVE_KEYS = 10_000`, `MAX_MAILBOX_MESSAGES = 5_000`, `MAX_GUILDS = 256`
- Delegate-attested presence / message / guild signatures — the webapp isn't an oracle for signing

**UX**
- 3 themes (Parchment / Dusk / Forest) via CSS custom properties, anti-flash inline script in index.html
- Onboarding wizard (4-step modal, dismiss persists)
- Settings reorg: theme / sync cadence (5/10/30s) / auto-pause HP (0/25/50%) / publish behavior / identity & backup / Advanced collapsible with debug overlay + mailbox D2D test + WS URL override
- `format_si` engineering notation (`1.2k`, `200k`, `1B`) for unbounded counters
- Debug overlay (18 lines of state diagnostics) in Settings → Advanced
- Top-level Guilds tab (`⚔`) with create/join/leave flow

**Infrastructure**
- 28 unit tests across the contract crates (presence 15 + mailbox 5 + guilds 7 + shared fmt 1)
- `dev-publish.sh` builds and publishes **3 contracts + delegate**, writes **8 keys** into `dev-keys.json`
- `dev-watch.sh` watches all five source trees (shared + 3 contracts + delegate)

### 🔜 Deferred / wishlist

**Anti-cheat layer**
- **Witness-based boss_damage attestation** — needs freenet-core hooks for cross-delegate attestation. Without it `boss_damage` is self-attested (any custom delegate can sign whatever it likes).
- **Anomaly detection** in `validate_state` (rate limiter on `boss_damage` growth) — a simple defense that catches ~80% of read-cheating; can be added without freenet-core changes.

**Multiplayer gameplay on top of the existing infrastructure**
- **Guild gameplay** — currently membership only. Not implemented: shared boss with distributed damage tracking, member contributions, guild chat (via the MAILBOX_INVITE kind), invite-only join, kick by leader.
- **Mailbox features** — currently D2D test only. Not implemented: gifts (send gear/gold/potion), trade offers (atomic 2-phase), guild invites via mailbox, in-game chat.
- **Sharding** — the hook is laid out via `Parameters` but isn't needed below ~1k active players.

**Combat depth**
- **Reflexes / Speed upgrades** — `TURN_COOLDOWN_MS` is currently hardcoded. Plan: release it as a Sage-shop skill, or derive from `speed`.
- **Auto-use potion at an HP threshold** — gameplay feature, not a setting. Queue an "auto-defensive" action in the delegate.
- **More queueable abilities** — only potion / fireball today. Possible additions: defensive stance, stun, retreat.

**Identity & persistence**
- **Encrypted seed export** — Export Seed currently returns plain hex. Needs AES-GCM wrap with a passphrase.
- **Import seed flow** — the inverse of export, to install identity on a fresh node.

**UX polish**
- **Mobile-responsive layout** — `grid-3` collapses, but shop/buy-grid/leaderboard break on narrow viewports.
- **i18n** — UI / Help / plot are pure English.
- **Achievement toast** on unlock — the player currently learns through the Achievements tab only. *(Already implemented — see toast queue + `ingest_inventory` diff.)*
- **Reactive notifications** for World Boss era advance, ending unlock.
- **Spectator mode** — view the leaderboard without participating (maybe via `?spectate=1`).
- **Replay shareable link** — export last_combat / boss_damage progress as a URL for sharing.

**Content / narrative**
- Expand the enemy roster (9 enemies across 4 areas today)
- Additional endings (4 → ~10)
- Seasonal events (every N weeks a new area / boss)
- NPC dialogue beyond the Sage descriptors

## Architecture notes worth attention

- **The node wipes `DelegateContext`** before returning the ApplicationMessage to the client ([freenet-core/crates/core/src/wasm_runtime/delegate.rs:351](../freenet-core/crates/core/src/wasm_runtime/delegate.rs#L351)). That's why request-id lives **inside the payload** (`DelegateEnvelopeIn/Out` in `shared/`), not in the context.
- **The WASM delegate has no host RNG.** On first run the webapp offers the delegate a seed candidate via `GetPubkey { seed_if_missing }`; the delegate stores it atomically and ignores all subsequent candidates. The cost is first-run injection: if the first webapp with rights over this delegate is compromised, identity is fixed in a compromised state.
- **Only the locally-built fdev/freenet are API-consistent.** PATH-fdev 0.3.151 + node 0.1.177 fails with `"input bytes aren't valid utf-8"` while compiling WASM. We use `freenet-core/target/debug/{freenet,fdev}` 0.2.55 / 0.3.218.
- **fdev needs `CARGO_TARGET_DIR`** — otherwise it searches for the workspace root via its compile-time `CARGO_MANIFEST_DIR` and panics.
- **The contract pins wire-version 0.6.1** on the frontend side (to talk to node 0.2.55) and uses a path-dep 0.7.0 on the contract/delegate side — the same trick as in `freenet-webrtc-poc`.
- **`InventoryWire` is non-destructive schema evolution.** The Inventory is serialized as `InventoryWire::V10(...)`. Adding fields = a new `V11(InventoryV11)` variant + a `From<V10>` migration in `into_latest`. **`INVENTORY_SECRET_ID` no longer needs to be bumped** — old saves are read and auto-promoted on first save.
- **Combat is a tick-based state machine in the delegate.** `Inventory.current_battle` persists. The frontend polls `TickBattle` every `POLL_TICK_MS = 1s` during a fight; outside combat — the regular pull cadence (5/10/30s per prefs). `TURN_COOLDOWN_MS = 1s` — one turn iteration = queued action + player swing + enemy swing with initiative by `speed`. Offline catch-up uses the same `tick_battle` procedure — online/offline converge on identical numbers.
- **Auto-mission is persistent.** `Inventory.auto_run_enabled` lives on the node; the toggle button sends `SetAutoRun`. Close the tab, come back an hour later — the delegate simulates the missed ticks (capped at 1 hour) and the "while you were away" banner sums it up.
- **Mailbox and Guilds are independent contracts.** The frontend subscribes to each in parallel, routes responses by `key.id()`. If the corresponding key isn't configured in `dev-keys.json`, the feature disables gracefully without breaking presence.
- **Identity is portable.** `Settings → Export seed` returns a 32-byte hex. Copy it onto another node = log in under the same pubkey. `Reset progress` wipes the Inventory, but **identity (seed) survives** — leaderboards recognize you.

## Known limitations

- **Wiping `<data-dir>/secrets/` on the node resets the Inventory.** Identity can be pulled out beforehand via **Settings → Export seed**. A production flow needs encrypted import.
- **`boss_damage` is self-attested.** The signature proves "I hold this key", not "these numbers are honest". The contract checks monotonicity (can't shrink), the ts ceiling, and the forward skew, but not growth rate. Witness-based attestation needs freenet-core hooks (see the plan in the `mailbox-contract` comments).
- **Per-key cap on the World Boss ledger.** `cumulative_damage` is capped at 10k unique pubkeys — beyond that, eviction by lowest watermark. New players with `boss_damage=0` don't get into the ledger until someone contributes above the current min.
- **One global presence contract.** Live entries capped at 1k. Once the cap is hit, the plan is sharding via `Parameters: pubkey_hash % N` — not implemented yet.
- **Mailbox / Guilds — optional plumbing.** The contracts are published by the script, but no gameplay logic on top of them yet: guilds — membership only, no shared boss / chat / invites; mailbox — D2D test only in Settings → Advanced.
- **Field combat catch-up is bounded by `MAX_CATCHUP_TICKS = 3600`** (≈ 1 hour). Longer offline windows — the catch-up window is clipped to one hour, the rest is "lost".
