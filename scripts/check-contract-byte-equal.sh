#!/usr/bin/env bash
# Approach A — lockfile isolation gate for the three on-chain contracts.
#
# Each contract's `code_hash` is `Blake3(raw_wasm)` (see
# freenet-stdlib/rust/src/contract_interface/code.rs::gen_hash). A
# fresh `code_hash` mints a NEW contract instance and orphans
# whatever state lived on the previous one:
#
#   * presence-contract → cumulative_damage + leaderboard aggregate
#   * mailbox-contract  → entire signed-message log
#   * guilds-contract   → every guild membership state
#
# This script rebuilds the contract from source and `cmp`s the
# produced raw `.wasm` against
# `published-contract/<name>/<name>_contract.wasm`. Drift here means
# a workspace dep or rustc pin change leaked into the contract.
# Fix the regression OR consciously accept the rotation (see
# `published-contract/README.md`) and pair with
# `ALLOW_<name>_REPUBLISH=1` on prod-publish.
#
# Canonical host: linux/amd64 with the rustc pin in
# `<contract>/rust-toolchain.toml`. Other host arch/OS combos can
# produce different wasm bytes from the same source, so the check
# skips with a warning so local devs aren't blocked.
#
# Usage:
#   scripts/check-contract-byte-equal.sh <contract_name>
# where <contract_name> ∈ {presence, mailbox, guilds}.

set -euo pipefail

NAME="${1:?usage: $0 <presence|mailbox|guilds>}"
case "$NAME" in
    presence|mailbox|guilds) ;;
    *) echo "unknown contract: $NAME" >&2; exit 2 ;;
esac

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
WASM_OUT="$ROOT/${NAME}-contract/target/wasm32-unknown-unknown/release/${NAME}_contract.wasm"
SNAPSHOT_SHA256="$ROOT/published-contract/${NAME}/sha256.txt"

if [ ! -f "$SNAPSHOT_SHA256" ]; then
    echo "warn: $SNAPSHOT_SHA256 missing — first run? Skipping byte-equality check." >&2
    exit 0
fi

HOST_OS=$(uname -s)
HOST_ARCH=$(uname -m)
if [ "$HOST_OS" != "Linux" ] || [ "$HOST_ARCH" != "x86_64" ]; then
    echo "warn: ${NAME}-contract byte-equality check is canonical only on linux/amd64."
    echo "      This host is $HOST_OS/$HOST_ARCH — skipping rebuild + compare."
    echo "      To rebuild the snapshot deliberately, see"
    echo "      published-contract/README.md."
    exit 0
fi

(
    cd "$ROOT/${NAME}-contract"
    cargo build --release --target wasm32-unknown-unknown
)

WANT_SHA="$(cat "$SNAPSHOT_SHA256")"
GOT_SHA="$(sha256sum "$WASM_OUT" | cut -d' ' -f1)"

if [ "$WANT_SHA" != "$GOT_SHA" ]; then
    echo "FAIL: ${NAME}_contract.wasm drift detected." >&2
    echo "  built:     $WASM_OUT ($(wc -c < "$WASM_OUT") bytes, sha256 $GOT_SHA)" >&2
    echo "  committed: snapshot sha256 $WANT_SHA" >&2
    echo "" >&2
    echo "A drift here rotates the contract code_hash. That mints a new" >&2
    echo "contract instance; whatever state lived on the previous one is" >&2
    echo "orphaned (presence: leaderboard + boss aggregate; mailbox: log;" >&2
    echo "guilds: memberships). Investigate before publishing." >&2
    echo "" >&2
    echo "If this drift is intentional (deliberate dep bump / rustc pin" >&2
    echo "rotation / behaviour fix), regenerate the snapshot per" >&2
    echo "published-contract/README.md, commit the new sha256.txt," >&2
    echo "and pair the release with ALLOW_${NAME^^}_REPUBLISH=1 + a" >&2
    echo "migration story." >&2
    exit 1
fi

echo "ok: ${NAME}_contract.wasm matches committed snapshot ($(wc -c < "$WASM_OUT") bytes, sha256 ${GOT_SHA:0:12}…)"
