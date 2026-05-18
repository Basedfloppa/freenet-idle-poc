# published-contract/ — committed contract snapshots

Per-contract byte-stable WASM snapshots. Each release MUST match these
bytes or `scripts/check-contract-byte-equal.sh <name>` fails.

```
published-contract/
├── presence/
│   └── sha256.txt                 # SHA-256 (hex) of the raw wasm
├── mailbox/{...}
└── guilds/{...}
```

Neither the raw `.wasm`, the fdev-wrapped form, nor the base58
code_hash is committed. The gate verifies drift via `sha256sum`;
the on-chain `code_hash` (Blake3 of raw wasm in base58) is printed
by `fdev build` if an operator wants to cross-check. Reproducing
the exact bytes is done by re-running the locked build against the
committed sources (`Cargo.lock` + `rust-toolchain.toml` + pinned
deps make rebuilds deterministic).

## Why snapshots exist

Each contract's on-chain identity is `hash(wasm, parameters)`. The
parameters slot is fixed per-contract; the WASM bytes are what move.
If a release rebuilds with different bytes, the contract id rotates
and:

- **presence-contract** orphans `cumulative_damage` (the World Boss
  HP ledger) and the live leaderboard,
- **mailbox-contract** orphans the entire signed-message log,
- **guilds-contract** orphans every guild membership state.

Lockfile isolation (own `Cargo.lock` + `=x.y.z` pins +
`rust-toolchain.toml`) makes the cargo build deterministic given
source; the committed snapshot is the gate that catches drift.

`shared-wire/` is path-dep'd by all three contracts, so any change
to a `*Wire`/`*State`/`*Payload` type rotates the snapshot for the
affected contract(s). The wrapper-chain pattern (V(N) frozen, V(N+1)
adds fields with a `From<V(N-1)>` shim) lets us add fields
*without* rotating the wire bytes — but it does rotate the WASM
bytes, which is exactly what the gate is here to catch.

## How the gate is wired

| Step                                    | What happens                                                                                                                                                                                       |
|-----------------------------------------|----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `scripts/dev-publish.sh`                | Runs `check-contract-byte-equal.sh <name>` for each contract **as a warning** — dev loop still publishes on drift.                                                                                  |
| `scripts/prod-publish.sh`               | Runs the same check **as a hard stop** per contract; aligned with the existing `ALLOW_PRESENCE_REPUBLISH=1` (and equivalent for mailbox/guilds) operator override.                                  |
| Local check on non-canonical hosts      | Skips with a warning. The committed bytes are produced on linux/amd64 with the pinned rustc; other arches can drift even from byte-identical source.                                              |

## Regenerating a snapshot deliberately

When a deliberate behaviour change in the contract ships:

```bash
cd <name>-contract
cargo clean
cargo build --release --target wasm32-unknown-unknown
sha256sum target/wasm32-unknown-unknown/release/<name>_contract.wasm \
    | cut -d' ' -f1 > ../published-contract/<name>/sha256.txt
```

Then verify the gate passes:

```bash
scripts/check-contract-byte-equal.sh <name>
```

Pair the change with `ALLOW_<NAME>_REPUBLISH=1` on prod-publish AND a
release note describing what state is being orphaned. For presence
specifically: the leaderboard repopulates within one heartbeat
(~30s); the cumulative World Boss damage ledger does NOT migrate.

See `docs/delegate-stability.md` for the broader discipline that
covers both the delegate and these three contracts.
