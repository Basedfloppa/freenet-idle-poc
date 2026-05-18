#!/usr/bin/env bash
# Approach A — lockfile isolation gate for identity-delegate.
#
# The delegate's `code_hash` namespaces every per-player SecretsStore
# write (`<base>/<delegate.code_hash>/<secret_id>`). If `code_hash`
# rotates between releases, every player's inventory becomes
# unreachable — effectively a silent wipe.
#
# This script rebuilds `identity_delegate.wasm` from source and
# `cmp`s it against `published-delegate/identity_delegate.wasm`.
# Drift here = a workspace dep, rustc bump, or =x.y.z pin change
# has leaked into the delegate. Fix the regression OR consciously
# accept the rotation by regenerating the snapshot (see
# `published-delegate/README.md`).
#
# Canonical host: linux/amd64 with the rustc pin in
# `identity-delegate/rust-toolchain.toml`. Other host arch/OS combos
# (macOS arm64, etc.) can produce different wasm bytes from the same
# source, so the check skips with a warning so local devs aren't
# blocked.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
WASM_OUT="$ROOT/identity-delegate/target/wasm32-unknown-unknown/release/identity_delegate.wasm"
SNAPSHOT_SHA256="$ROOT/published-delegate/sha256.txt"

if [ ! -f "$SNAPSHOT_SHA256" ]; then
    echo "warn: $SNAPSHOT_SHA256 missing — first run? Skipping byte-equality check." >&2
    exit 0
fi

HOST_OS=$(uname -s)
HOST_ARCH=$(uname -m)
if [ "$HOST_OS" != "Linux" ] || [ "$HOST_ARCH" != "x86_64" ]; then
    echo "warn: delegate byte-equality check is canonical only on linux/amd64."
    echo "      This host is $HOST_OS/$HOST_ARCH — skipping rebuild + compare."
    echo "      To rebuild the snapshot deliberately, see"
    echo "      published-delegate/README.md."
    exit 0
fi

(
    cd "$ROOT/identity-delegate"
    cargo build --release --target wasm32-unknown-unknown
)

WANT_SHA="$(cat "$SNAPSHOT_SHA256")"
GOT_SHA="$(sha256sum "$WASM_OUT" | cut -d' ' -f1)"

if [ "$WANT_SHA" != "$GOT_SHA" ]; then
    echo "FAIL: identity_delegate.wasm drift detected." >&2
    echo "  built:     $WASM_OUT ($(wc -c < "$WASM_OUT") bytes, sha256 $GOT_SHA)" >&2
    echo "  committed: snapshot sha256 $WANT_SHA" >&2
    echo "" >&2
    echo "A drift here rotates the delegate code_hash. That re-namespaces" >&2
    echo "every SecretsStore write, so every player's inventory becomes" >&2
    echo "unreachable. Investigate before publishing." >&2
    echo "" >&2
    echo "If this drift is intentional (deliberate dep bump / rustc pin" >&2
    echo "rotation / behaviour fix), regenerate the snapshot per" >&2
    echo "published-delegate/README.md, commit the new sha256.txt," >&2
    echo "and pair the release with a migration story for stranded" >&2
    echo "inventories." >&2
    exit 1
fi

echo "ok: identity_delegate.wasm matches committed snapshot ($(wc -c < "$WASM_OUT") bytes, sha256 ${GOT_SHA:0:12}…)"
