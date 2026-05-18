# published-delegate/ — committed delegate snapshot

This directory contains the byte-stable snapshot of the
`identity-delegate` WASM that every release MUST match.

| File           | What it is                                                                                  |
|----------------|----------------------------------------------------------------------------------------------|
| `sha256.txt`   | SHA-256 (hex) of the raw `wasm32-unknown-unknown/release/identity_delegate.wasm`. The byte-equality gate hashes a freshly-built wasm and compares against this — drift = release blocker. |

Neither the raw `.wasm`, the fdev-wrapped form, nor the
base58-Blake3 code_hash is committed. SHA-256 (verifiable with
stock `sha256sum`) is the single drift signal; reproducing the
exact bytes comes from re-running the locked build against the
committed sources. The on-chain `code_hash` (= Blake3 of raw wasm)
is what `fdev build` prints — operators can grep its output during
publish if they want to cross-check.

## Why a snapshot exists

The delegate's `code_hash` namespaces every `SecretsStore` write:

```
<secrets_base>/<delegate.code_hash.encode()>/<secret_id.encode()>
```

If `code_hash` rotates, the node can no longer find the player's
inventory secret — effectively wiping every player's progress. WASM
bytes are sensitive to:

- workspace dep churn (yew, wasm-bindgen, etc.)
- rustc / LLVM version
- minor-version drift of WASM-direct deps (serde, bincode, ed25519-dalek)

Lockfile isolation (`identity-delegate/Cargo.lock` outside the
workspace + `=x.y.z` pins + `rust-toolchain.toml`) makes the build
deterministic given source. The committed snapshot is the gate that
catches accidental drift.

## How the gate works

`scripts/check-delegate-byte-equal.sh` rebuilds the delegate in clean
conditions, runs `sha256sum` on the produced `.wasm`, and compares
the hex digest against `sha256.txt` in this directory. Drift =
release blocker.

Local rebuilds on non-canonical hosts (macOS, arm64) may not match
exactly because of codegen differences — the check skips with a
warning there. The canonical host is **linux/amd64** with the
toolchain pinned in `identity-delegate/rust-toolchain.toml`.

## Regenerating the snapshot deliberately

Any time you intentionally rotate the delegate (a bug fix that
requires a behaviour change, a deliberate dep bump, a rustc pin
bump), pair the change with a snapshot refresh:

```bash
cd identity-delegate
cargo clean
cargo build --release --target wasm32-unknown-unknown
sha256sum target/wasm32-unknown-unknown/release/identity_delegate.wasm \
    | cut -d' ' -f1 > ../published-delegate/sha256.txt
```

Then write a release note: every player's inventory under the old
hash will need a one-time migration (or will be lost — depending on
whether the change is a deliberate reset).

See `docs/delegate-stability.md` for the full discipline.
