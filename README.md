# Idle PoC

An idle/RPG on top of Freenet, where:

- Local UI preferences (theme, cadence, prefs) live **only** in the browser.
- Identity (Ed25519 seed) and the entire Inventory live **on the local node**, in the delegate secret store. The webapp is a thin client; "clear site data" resets nothing.
- Players see each other through **three aggregator contracts** (presence / mailbox / guilds): one WS subscription per contract, with Freenet doing the fan-out as the overlay network.

## Layout

```
idle-poc/
├── shared/                   Full game model: Inventory wrapper-chain
│                             (V9..V20 — Estate, Legacy, Activity,
│                             Routine, Insight, Tokens, era-watermark,
│                             RoutineV2 cosmetics, auto-equip-best,
│                             offline-cap + mission cycle, public
│                             cosmetics + streak), RoutineState V1..V5,
│                             reset taxonomy (Ascend / NewPlayer /
│                             SchemaMigration), Wilds procedural-graph
│                             generator, format_si helper.
├── shared-wire/              Wire-only types (PresencePayload V2/V3,
│                             ContractStateV1/MailboxStateV1/GuildsStateV1,
│                             SignedEntry framing). Contracts depend on
│                             this crate, NOT on `shared`, so game-logic
│                             edits don't rotate contract WASMs.
├── presence-contract/        Rust contract: LWW merge + cumulative World
│                             Boss ledger + outlier-resistant prune
│                             (stale-singleton escape hatch); 1k live
│                             entries, 10k ledger cap. Accepts
│                             PresencePayload v2+v3 (§6.4 range).
├── mailbox-contract/         Player-to-player signed log (gift / invite
│                             / trade / chat substrate). 5k entries,
│                             7d TTL.
├── guilds-contract/          Op-sourced cooperative groups
│                             (CREATE/JOIN/LEAVE), one pubkey ≤ one
│                             guild, 256 guilds × 50 members.
├── identity-delegate/        Authoritative seed + Inventory store.
│                             Hosts the tick-based combat resolver,
│                             chunked offline catchup (24h per call,
│                             analytical tail above 4h), routine pump,
│                             auto-mission toggle, export/reset RPCs.
│                             Own workspace + Cargo.lock + rust-
│                             toolchain.toml + `=x.y.z` pins — see
│                             docs/delegate-stability.md.
├── frontend/                 Yew + long-lived WS subscriptions
│                             (presence + optional mailbox + optional
│                             guilds) + unified check-elapsed tick loop.
│                             JSON-driven themes (`themes/*.json`) and
│                             locales (`locales/*.json`), Settings,
│                             onboarding, debug overlay, catchup
│                             progress modal.
├── published-delegate/       Byte-stable identity_delegate.wasm
│                             snapshot — release blocker if the
│                             rebuild drifts. See
│                             docs/delegate-stability.md.
├── published-contract/       Same discipline for presence/mailbox/
│                             guilds contract WASMs (per-contract
│                             subdirectory). Drift = rotated
│                             contract_id = state on the old instance
│                             is orphaned. See
│                             published-contract/README.md.
└── scripts/
    ├── dev-publish.sh        Builds and publishes all 3 contracts +
    │                         delegate, writes 8 keys into
    │                         frontend/dev-keys.json. Runs the
    │                         byte-equality gate per artefact (warning
    │                         only — dev loop isn't blocked).
    ├── prod-publish.sh       Same pipeline against a remote node;
    │                         byte-equality gate is a HARD STOP unless
    │                         the per-artefact `ALLOW_*_REPUBLISH=1`
    │                         override is set.
    ├── prod-update-webapp.sh Webapp-only update (skips contract /
    │                         delegate rebuild).
    ├── dev-watch.sh          Incremental re-publish on changes in
    │                         shared/, shared-wire/, presence/,
    │                         mailbox/, guilds/, delegate/.
    ├── check-delegate-byte-equal.sh
    │                         Rebuilds the delegate WASM and cmp's
    │                         against published-delegate/. Drift =
    │                         rotated code_hash = every player's
    │                         inventory stranded.
    ├── check-contract-byte-equal.sh <name>
    │                         Same gate for one of {presence, mailbox,
    │                         guilds}.
    └── dev.sh                One command: watcher + `trunk serve`.
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
  `prod-update-webapp.sh` on subsequent runs from the same operator.
  Gitignored — different operators publishing to different nodes
  would get different ids; sharing this is what the runtime URL is
  for. `webapp_contract_id()` parses it out of `window.location`.
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
| Inventory (gold, gear, skills, achievements, Estate workers, Legacy stars, Insight, Tokens, era-watermark, Routine cosmetics, daily streak…) | Same delegate secret store (key `inventory-v9`, format `InventoryWire::V20(...)` after the V9→V20 migration chain) | Full game progress | `Reset progress` (full wipe) or `Ascend` (soft-reset run; keeps stars/level/missions/skills) |
| Presence (`anon-XXXX` + gold + boss_damage + ts) | `presence-contract`, one entry per pubkey | Leaderboard + World Boss aggregate | Auto-prune after 60s of silence (live), watermark persists in `cumulative_damage` |
| Cumulative World Boss watermark | `presence-contract.cumulative_damage` | Per-key high watermark | Cap-eviction at 10k unique keys |
| Mailbox messages | `mailbox-contract`, signed log | gift/invite/trade/chat (substrate) | 7-day TTL or 5k-entry cap |
| Guild membership | `guilds-contract`, op-sourced state | Name, leader, members | LEAVE / last LEAVE → dissolve |

## Feature status

### ✅ Done

**Gameplay**
- Tick-based combat (`TURN_COOLDOWN_MS = 1s`, initiative by `speed`, evasion as flat damage scaling). Passive HP regen is gated off during an active battle so sustained fights can't be regen-trivialised.
- Mid-fight queueable actions: `Use Potion` (full heal) / `Use Fireball` (bonus damage)
- 3 encounters per mission (post-B6 rebalance), chain advances automatically
- 6 areas in a **graph** (C3a): Village → Forest Road → {Mountain Pass, Deep Forest} → {Snowfields → Boss's Lair, Mountain Pass → Boss's Lair}. Each area gates on `min_level` AND `clears_required` in any one predecessor (OR semantics).
- 5 forms (Human / Slime / Cat / Dragon / Horse) with transformation on loss → permanent skill (prestige loop). Forms also drive **Estate affinity** — current form buffs/penalises specific worker tiers.
- Shop now sells **forms directly**: cheap Human reset (1.5k g), expensive direct-form purchases (20k–60k g) as alternative to defeat-induced transformation.
- 8 slots × 4 tiers of gear (32 catalog ids), per-form slot mask, tier-coloured borders on equipped slots
- Forge (3-of-a-kind + essence → next tier, up to Legendary)
- 11 achievements, 6 skills (4 form-derived + Veteran/Champion level milestones); hover any visited-form badge for its stat bundle
- 4 endings (Victory / Dragon Lord / Pilgrim / Quiet Farmer)
- Exponential XP leveling (1.5× per level), level-static base stats (post-B6: base atk/def = 2 + lvl×2; HP unchanged)
- HP regen (`HP_FULL_REGEN_MS`) — skipped while a battle is active
- Procedural plot (6×6×6×6×6 = 7776 combinations)
- Wheat farm (10:1 → gold) as safe-mode income
- Shop: pre-rolled gear by slot+tier, potions, fireballs, **Auto-Equip Best** that pre-flights form-mask + per-slot score so the button greys out when nothing in stash beats current gear
- Combat history (ring buffer, `COMBAT_HISTORY_CAP = 30`)
- Sage skill shop (4 form-skills purchasable for essence)
- **Estate** (B2): 4-tier worker economy (Farmhand/Forager/Trader/Sage) with `1.07ⁿ` cost curve. Workers accrue resources passively. **As of 2026-05-18 Estate yield runs in parallel with combat / activity for every player** — no idle-action mutex, no `WorkforceBoss` requirement. Form-affinity multiplier per tier compounds with Legacy multiplier multiplicatively.
- **Legacy / Epoch** (C1, delegate-only MVP): 1 star per 5 earned levels (watermark prevents re-grinding across ascensions). Spend on permanent multipliers (Hero Attack +5%/lvl, Estate Yield +10%/lvl, Mission Gold +5%/lvl). Cost curve `1,2,4,8,…`. **Ascend** soft-resets gold/gear/Estate while keeping stars/level/missions/skills/achievements.
- **Phased reveal** (A2/A5): UI sections latch on by predicate (Shop @ 1 mission, World Boss @ 10, Auto-mission @ 25, Estate @ 50g, Skills @ 100 essence, etc). Once-per-session slide-in animation keyed off `Core::animate_reveal`.
- **Welcome-back modal** (B4): merges offline-catchup summary, Estate accrual breakdown, and per-version patchnotes into a single dismissible modal. Catchup ack persisted via `last_catchup_acked_started_ms` in the Settings blob so the same window doesn't re-pop across reloads.
- **Per-zone activities** (A1): non-combat actions tied to the current area (Tend farm / Pray / Forage / Mine / Channel essence / Decode sigils / etc). 9 activities across the 5 starter zones, producing wheat / gold / essence / **insight**. Active activity is mutually exclusive with auto-mission and Estate (§5.6).
- **Routine auto-hire** (B1): per-Estate-tier headcount target on the Mastery tab. Auto-hire fires inside `save_inventory` on every state mutation — runs regardless of which idle action is active, so gold from any source (auto-mission, Estate, combat win, wheat sale) flows into pending targets.
- **Insight currency** (B5): rare currency earned 1-per-25-missions + via the Astral "Decode sigils" activity. Three spend nodes (HpPerLevel, GoldDropPct, FormAffinity) with descriptions inline.
- **Personal World Boss attack** (C1 partial): `BossAttack` RPC spends 200 essence for +50 boss damage, gated on `mission_count ≥ 100 ∧ level ≥ 10 ∧ ≥1 Estate worker`. No contract change — piggybacks the existing self-attested `boss_damage` path.
- **Token economy** (C2): triangular-growth milestones — N-th token requires `500 * N * (N + 1) / 2` personal `boss_damage`, so the gap scales with era boss-HP. Eight one-shot perks: ChampionBadge (🏆 marker on Hero / leaderboard), GearMastery (×1.20 gear bonuses), BossFury (×2 mission-area boss damage), AlchemistTrust (×2 potion drops), MerchantSeal (+50% encounter gold), IronWill (×1.25 max HP), EssenceWeaver (+30% essence), **LongHaulForeman (offline catchup cap lifts from 24h to 168h)**. All have live gameplay effects through `derived.rs` + `combat/tick.rs`; remote-player badge ownership rides on PresencePayload v2's `champion` field. (`LongHaulForeman` repurposed 2026-05-19 from the original `WorkforceBoss` slot — bincode discriminant `=7` preserved so existing token blobs decode unchanged.)
- **Mastery tab**: dedicated home (⭐ icon) for all permanent upgrades — Legacy / Routine / Insight / Boss Attack / Tokens panels live here instead of cluttering Settings. Revealed once any progression watermark has been touched.
- **Stash as grid**: `repeat(auto-fit, minmax(280px, 1fr))` packs the unequipped-gear list into multi-column cards instead of a tall vertical scroll. Each card has internal `grid-template-areas` for name / tier / stats / actions and a `.stash-actions` flex row that wraps multi-button rows naturally.
- **Bulk-sell**: `SellGearAll` RPC and matching "sell ×N (Mg)" button on multi-copy stash rows. `SellConsumable { kind, amount }` RPC + "sell ×N" button on Shop potion / fireball rows (`amount == 0` is the wire signal for "sell everything").
- **Form-buy clears slot mask**: shop-bought form change now runs `enforce_form_slot_mask` like the defeat-induced path; gear in slots the new form can't wear moves back to stash atomically.
- **Stash card text contrast on Dusk**: `.area-card.current` overrides the global `.muted` colour for blurb / rewards / clear-count so the active-area card stays legible on the gold-tinted dark background.
- **B7 bulk-buy** (`BulkBuyItem { kind, count, now_ms }` + `BulkBuyGearRoll { slot, tier, count, now_ms }`): `+10` / `max` buttons next to single-Buy on consumables and on the gear-slot grid. `count == 0` is the wire signal for "buy as many as gold allows" (delegate caps at 1000 / 100 per call). Skills stay one-shot — they're unlocks, not levels.
- **C1 + C2 era-advance hook** (`ClaimBossKill { era, era_max_hp, rank, now_ms }`): frontend's unified-tick watches `world_boss_state(c)`; when `era > inv.boss_era_witnessed` it computes the player's rank in `c.cumulative_damage` and fires the claim. Delegate validates era-monotonicity, clamps `dmg_share` to `inv.boss_damage - boss_damage_at_era_start`, awards Legacy stars via `boss_kill_stars_for` (sublinear `^0.7` curve, cap 10) AND tokens via `boss_kill_tokens_for_rank` (3/2/1 for top three, 0 otherwise). The personal-milestone token earn rule still runs in parallel; ranked tokens are the bonus.
- **C3b Wilds procedural graph** (`shared::wilds_areas(seed)`): 8-node DAG generated deterministically from `inv.plot_seed` (xorshift32 PRNG). Two branches off the entrance + one confluence node + jittered enemy stats. Node IDs sit in the `100+` namespace; `shared::resolve_area(id, plot_seed)` unifies lookup so `set_area` and combat both reach into the right table. Gated by entrance `min_level: 15` (visible from level 10).
- **Skill bonuses ~½**: `skill_bonuses` + `skill_speed_evasion` halved after playtest showed six skills stacking trivialised the post-B6 baseline (Slime def +5→+3 hp +20→+10, Cat atk +6→+3 speed +30→+15 eva +10→+5, Dragon atk +8→+4 def +6→+3 speed +10→+5, Steed hp +25→+12 def +4→+2 speed +20→+10, Veteran +5/+5→+3/+3, Champion +10/+10/+30→+5/+5/+15). Localised blurbs (EN + RU) regenerated to match.
- **Inventory wrapper-chain V9..V20**: every bump is additive (`V(N+1) { base: V(N), <new_fields> }` with `Deref`/`DerefMut`). V15 added the era watermark, V16 RoutineV2 cosmetics, V17 auto-equip-best toggle, V18 RoutineV4 (offline_cap + mission cycle + combat_speed), V19/V20 RoutineV5 (public cosmetics + daily-streak). Old V9..V19 blobs decode and auto-promote on the next `save_inventory`.
- **Public cosmetics on the leaderboard (§E-tier)**: motto, accent ribbon, frame slot — all surface to other clients through `PresencePayloadV3`. `ACCEPTED_PAYLOAD_VERSIONS = &[2, 3]` so a v2 publisher and a v3 publisher coexist during the rollout.
- **Daily check-in + streak (§P3)**: UTC-rollover streak counter with linear-up-to-7 essence ramp (cap day 30). Persisted in `RoutineStateV5`.
- **Long-haul foreman catchup**: offline catchup ceiling lifts from 24h to 168h (7 days) when the LongHaulForeman token perk is owned. Above 4h the catchup switches to an analytical regime — averaged per-hour yields multiplied by the missing window — so the delegate's CPU budget stays bounded even for week-long returns. Frontend chunks the work in 24h slices with a progress modal; each delegate call writes back the partial state, so closing the tab mid-catchup is recoverable.
- **Lockfile isolation discipline**: `identity-delegate/`, `presence-contract/`, `mailbox-contract/`, `guilds-contract/` are each their own cargo workspace with `=x.y.z` pins, a pinned `rust-toolchain.toml`, and a committed snapshot under `published-delegate/` or `published-contract/<name>/`. The matching `scripts/check-*-byte-equal.sh` is a hard gate on prod-publish and a warning on dev-publish, so accidental workspace dep churn can't rotate `code_hash` and strand player state. See `docs/delegate-stability.md`.

**Multiplayer / Freenet**
- `presence-contract` — World Boss aggregator with a **persistent `cumulative_damage` ledger** (survives entry pruning)
- `mailbox-contract` — signed-log substrate for player-to-player messages (chat/gift/invite/trade — kind tags)
- `guilds-contract` — op-sourced cooperative groups with a "1 pubkey ≤ 1 guild" invariant, auto-handoff leader, dissolve on empty
- Auto-detect: if a key is unconfigured → the feature is disabled gracefully, the rest still works
- Lobby leaderboard, World Boss era progression (`era_max_hp = 500 × (era+1)²`)

**Persistence + Identity**
- `InventoryWire` non-destructive migration framework — chain V9 → V10 → V11 (area_clears + reveal) → V12 (Estate + idle_action) → V13 (Legacy). Old saves auto-promote on next `save_inventory`. Every bump uses additive composition (`pub struct InventoryVN { pub base: InventoryV(N-1), … }` with `Deref`/`DerefMut`) so the wire format stays byte-identical to a flat layout.
- Authoritative delegate (`PublishPresence`, `SendMessage`, `SignGuildOp` — the webapp can't inject numbers)
- Persistent `auto_run_enabled` + chunked offline catch-up. Default cap 1h, user-tunable up to 24h, up to 168h with the `LongHaulForeman` token perk. Above 4h the catchup is analytical (averaged per-hour yields) instead of per-tick. Estate idle accrual runs in parallel with combat / activity for every player; both feed the same Welcome-back modal via `last_catchup`.
- `Settings → Export seed` (Ed25519 hex export, identity migration between nodes)
- `Settings → Reset progress` (wipe Inventory, identity persists)
- `Settings → Legacy` (when revealed): per-node star spend tree + Ascend confirm
- Settings JSON blob (`BlobKind::Settings`) holds display name, theme, locale, tutorial-dismissed, `last_seen_version`, `last_catchup_acked_started_ms`. Frontend owns the schema; delegate stores opaque bytes — adding a field is a frontend-only change.

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
- **Localisation**: all UI strings live in `frontend/locales/<code>.json` and are bundled at compile time via `include_dir!`. Adding a new language is one JSON file drop — the locale picker auto-populates from the directory listing and falls back to English on any missing key. Current set: EN + RU + FR + ES + JA (full coverage) + DE (partial). `navigator.language` auto-pick on first load; explicit picker in Settings stores the locale's short code in the Settings blob.
- **Build-stamped semver**: `frontend/build.rs` runs `git rev-list --count HEAD` and emits `BUILD_VERSION=major.minor.<commit_count>` as a `cargo:rustc-env`. Every push advances the version; catchup modal compares the stamp against `last_seen_version` to fire the "What's new" section even without a curated changelog entry.
- **Reveal animation**: section slide-in plays exactly once per session — `Core::animate_reveal` carries the newly-flipped bits; render stamps `.reveal-anim` class for that single tick; subsequent tab switches see `animate_reveal == 0` and skip the animation.
- **Equipment quality colour-coding**: equipped slots get a 4-px tier-coloured left border + tier-3/4 value-text colour; tier-4 (Legendary) also gets an inset box-shadow glow.
- **Empty-inventory hiding**: 0-count Potion / Fireball rows are pruned from the Consumables panel and the Shop's Resources table. Gold / Essence stay (progress counters, not stash-style).
- **Stable battle log**: `ul.battle-turns` is `min-height: max-height: 4.5em` with internal `overflow-y`, so the page doesn't reflow as turns 0 → 5 accumulate. Queued-action slot also reserves space.
- **World Map as graph**: top-to-bottom rows by predecessor depth, CSS pseudo-element connectors above each non-starter, localised "↑ Predecessor" label. Grows downward as new branch areas ship.

**Infrastructure**
- 28 unit tests across the contract crates (presence 15 with 2 known baseline failures from the stale-singleton escape hatch — see `memory/presence-contract-tests-baseline.md`; mailbox 5; guilds 7; shared fmt 1)
- `dev-publish.sh` builds and publishes **3 contracts + delegate**, writes **8 keys** into `dev-keys.json`, runs the byte-equality gate per artefact (warning only).
- `prod-publish.sh` hard-stops on snapshot drift unless `ALLOW_<NAME>_REPUBLISH=1` is set; cap-ack flow is the same that's been guarding accidental rotation since the 2026-05-17 incident.
- `dev-watch.sh` watches all six source trees (shared + shared-wire + 3 contracts + delegate).

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
- **Full DE translation** — DE currently has only the curated subset (tabs + pills + boot strings) in `frontend/locales/de.json`. Filling out the rest is a JSON-only change — copy keys from `en.json` and translate. A native-speaker review pass is recommended for the FR / ES / JA strings before they're treated as final copy.
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
- **`InventoryWire` is non-destructive schema evolution.** Current chain V9→V20. The on-disk blob is serialised as `InventoryWire::V20(...)` today; older variants decode and auto-promote on first `save_inventory`. **Pattern for purely-additive bumps**: `pub struct InventoryV(N+1) { pub base: InventoryV(N), <new_fields> }` with `Deref`/`DerefMut` to the base. Bincode serialises structs as concatenated fields, so the wire format is byte-identical to a flat layout — old blobs keep decoding even though the type tree got deeper. The same pattern is applied to `RoutineState` (V1..V5), `PresencePayload` (V2/V3), and the three contract states (`ContractStateV1`, `MailboxStateV1`, `GuildsStateV1`). For remove/rename, re-declare flat.
- **Delegate / contract WASM bytes are gated.** Each on-chain artefact has a committed snapshot under `published-delegate/` or `published-contract/<name>/`; `scripts/check-*-byte-equal.sh` rebuilds and `cmp`s against the snapshot. Accidental drift (workspace dep bump, rustc roll-forward) would rotate `code_hash` and strand player inventory or contract state; the gate catches it before `prod-publish.sh` reaches `fdev publish`. See `docs/delegate-stability.md`.
- **Combat is a tick-based state machine in the delegate.** `Inventory.current_battle` persists. The frontend polls `TickBattle` every `POLL_TICK_MS = 1s` during a fight; outside combat — the regular pull cadence (5/10/30s per prefs). `TURN_COOLDOWN_MS = 1s` — one turn iteration = queued action + player swing + enemy swing with initiative by `speed`. Offline catch-up uses the same `tick_battle` procedure — online/offline converge on identical numbers.
- **Auto-mission is persistent.** `Inventory.auto_run_enabled` lives on the node; the toggle button sends `SetAutoRun`. Close the tab, come back later — the delegate simulates the missed window in 24h chunks (frontend re-fires `LoadInventory` until `auto_last_tick_ms` reaches `now_ms`). Default cap 1h, user-tunable up to 24h, up to 168h (7 days) with the `LongHaulForeman` perk. Above 4h the per-tick sim switches to an analytical extrapolation from rolling per-hour averages — the "Welcome back" modal still summarises gold/essence/xp/boss-damage, but individual encounter drama isn't replayed.
- **Mailbox and Guilds are independent contracts.** The frontend subscribes to each in parallel, routes responses by `key.id()`. If the corresponding key isn't configured in `dev-keys.json`, the feature disables gracefully without breaking presence.
- **Identity is portable.** `Settings → Export seed` returns a 32-byte hex. Copy it onto another node = log in under the same pubkey. `Reset progress` wipes the Inventory, but **identity (seed) survives** — leaderboards recognize you.

## Known limitations

- **Wiping `<data-dir>/secrets/` on the node resets the Inventory.** Identity can be pulled out beforehand via **Settings → Export seed**. A production flow needs encrypted import.
- **`boss_damage` is self-attested.** The signature proves "I hold this key", not "these numbers are honest". The contract checks monotonicity (can't shrink), the ts ceiling, and the forward skew, but not growth rate. Witness-based attestation needs freenet-core hooks (see the plan in the `mailbox-contract` comments).
- **Per-key cap on the World Boss ledger.** `cumulative_damage` is capped at 10k unique pubkeys — beyond that, eviction by lowest watermark. New players with `boss_damage=0` don't get into the ledger until someone contributes above the current min.
- **One global presence contract.** Live entries capped at 1k. Once the cap is hit, the plan is sharding via `Parameters: pubkey_hash % N` — not implemented yet.
- **Mailbox / Guilds — optional plumbing.** The contracts are published by the script, but no gameplay logic on top of them yet: guilds — membership only, no shared boss / chat / invites; mailbox — D2D test only in Settings → Advanced.
- **Offline catchup is bounded.** Default 1h. User can lift to 24h, or up to 168h (7 days) with the `LongHaulForeman` token perk. Beyond that, the older part of the missed window is forfeit. Above 4h the catchup uses averaged per-hour yields instead of per-tick simulation — drama moments aren't replayed, only the totals. Frontend shows a progress modal while the chunked catchup runs.
