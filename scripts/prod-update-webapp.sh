#!/usr/bin/env bash
#
# Incremental webapp update — for iterating on the frontend AFTER
# contracts + delegate are already deployed (i.e. after running
# `prod-publish.sh` at least once). Steps:
#   1. `trunk build --release` in frontend/
#   2. `fdev website update --key <name> dist/` — bumps the version
#      (uses unix_time/60 internally, same as the dashboard webapp)
#      and signs with the website key created by `website init`
#   3. SSH the prod gateway and `rm -rf` the unpacked webapp cache.
#      Without this the gateway keeps serving the OLD version even
#      after the update lands in the DHT (see `webapp-cache-
#      invalidation` memory).
#
# Required env / defaults:
#   FDEV               default: ../freenet-core/target/release/fdev, then
#                      target/debug/fdev. NOT $PATH/fdev — the system one
#                      is 0.3.151 and silently produces broken tarballs.
#                      Must support `website` subcommand (fdev ≥ 0.3.218).
#   NODE_URL           full ws URL (overrides NODE_ADDRESS+NODE_PORT)
#   NODE_ADDRESS       default 127.0.0.1
#   NODE_PORT          default 7509
#   WEBSITE_KEY        default idle-poc
#   SSH_HOST           default orange       (set to "" to skip cache rm)
#   WEBAPP_CACHE_DIR   default /root/.cache/freenet/webapp_cache
#   WEBAPP_ID          default: read from frontend/prod-webapp-id.txt
#
# Use this when:
#   - only the UI code changed (frontend/src/**)
#   - contracts / delegate ABI is unchanged
#   - the prod node is already serving the previous version
#
# If contracts / delegate WERE touched, run `prod-publish.sh` instead
# — `update` would publish a webapp pointing at stale contract ids
# baked into the previous keys.rs.

set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WEBSITE_KEY="${WEBSITE_KEY:-idle-poc}"
SSH_HOST="${SSH_HOST:-orange}"
WEBAPP_CACHE_DIR="${WEBAPP_CACHE_DIR:-/root/.cache/freenet/webapp_cache}"

# Resolve fdev — explicit FDEV wins, else prefer release over debug
# (release is what we actually build for prod). The `$PATH` fdev is
# NOT used as a fallback: PATH-fdev on this machine is 0.3.151 which
# silently lacks the `website` subcommand and would either error out
# or — worse — produce a partial tarball that landed the node in an
# unrecoverable state during the 2026-05-16 publish.
if [[ -z "${FDEV:-}" ]]; then
    for cand in \
        "$HERE/../freenet-core/target/release/fdev" \
        "$HERE/../freenet-core/target/debug/fdev"; do
        if [[ -x "$cand" ]]; then
            FDEV="$cand"
            break
        fi
    done
fi

if [[ -z "${FDEV:-}" || ! -x "$FDEV" ]]; then
    echo "[prod-update] fdev not found. Build first:"
    echo "    cd $HERE/../freenet-core && cargo build --release --bin fdev"
    echo "[prod-update] or set FDEV=/path/to/fdev (must support 'website' subcommand)."
    exit 1
fi

# Verify the chosen binary supports `website update` — catches the
# accidental PATH-fdev / older-release situation before we ship a
# malformed tarball.
if ! "$FDEV" website --help >/dev/null 2>&1; then
    echo "[prod-update] $FDEV does not support 'website' subcommand."
    echo "[prod-update] need fdev ≥ 0.3.218. Got: $("$FDEV" --version 2>&1 | head -1)"
    exit 1
fi
echo "[prod-update] using fdev: $FDEV ($("$FDEV" --version 2>&1 | head -1))"

if [[ -z "${WEBAPP_ID:-}" ]]; then
    if [[ -f "$HERE/frontend/prod-webapp-id.txt" ]]; then
        WEBAPP_ID="$(< "$HERE/frontend/prod-webapp-id.txt")"
    else
        echo "[prod-update] no WEBAPP_ID env and frontend/prod-webapp-id.txt missing."
        echo "[prod-update] run prod-publish.sh first, or set WEBAPP_ID explicitly."
        exit 1
    fi
fi
echo "[prod-update] webapp contract id: $WEBAPP_ID"

NODE_ARGS=()
if [[ -n "${NODE_URL:-}" ]]; then
    NODE_ARGS+=(--node-url "$NODE_URL")
    echo "[prod-update] target node: $NODE_URL"
else
    NODE_ADDRESS="${NODE_ADDRESS:-127.0.0.1}"
    NODE_PORT="${NODE_PORT:-7509}"
    NODE_ARGS+=(--address "$NODE_ADDRESS" --port "$NODE_PORT")
    echo "[prod-update] target node: ws://${NODE_ADDRESS}:${NODE_PORT}"
fi

echo
echo "[prod-update] trunk build --release"
cd "$HERE/frontend"

# Refuse to run while `trunk serve` is active. The dev-server watches
# the same `dist/` dir and races our release build — observed
# 2026-05-16 to produce a 2-file dist (just index.html + dev-mode
# WASM) at the moment fdev packs the tarball, since the dev-server
# overwrites dist before fdev finishes scanning. Result: state in
# DB only contains 2 files, every other asset 404s in browser.
if pgrep -f "trunk serve" >/dev/null 2>&1; then
    echo "[prod-update] trunk serve is running — kill it before publishing."
    echo "[prod-update]   pkill -f 'trunk serve'"
    echo "[prod-update] (the dev-server overwrites dist/ mid-build and produces"
    echo "[prod-update]  an incomplete tarball — see commit-message for context)."
    exit 1
fi

trunk build --release

# Sanity: dist must contain BOTH trunk-emitted assets AND the
# copy-file-staged contract/delegate WASMs. trunk's `data-trunk
# rel="copy-file"` rules silently skip missing source files, and an
# interrupted prior build can leave dist with only the rust-binary
# outputs. We've shipped this exact malformed bundle once — let's
# not do it twice.
EXPECTED_FILES=(
    "dist/index.html"
    "dist/style-"
    "dist/frontend-"
    "dist/identity_delegate.wasm"
    "dist/presence_contract.wasm"
    "dist/dev-keys.json"
)
MISSING=()
for pat in "${EXPECTED_FILES[@]}"; do
    if ! compgen -G "${pat}*" >/dev/null; then
        MISSING+=("$pat")
    fi
done
if (( ${#MISSING[@]} > 0 )); then
    echo "[prod-update] dist/ is missing expected files after trunk build:"
    for m in "${MISSING[@]}"; do echo "    $m*"; done
    echo "[prod-update] re-run scripts/prod-publish.sh to re-stage the contract WASMs,"
    echo "[prod-update] then retry this script."
    exit 1
fi
DIST_FILE_COUNT="$(find dist -maxdepth 1 -type f | wc -l)"
echo "[prod-update] dist/ has $DIST_FILE_COUNT files — looks complete"

echo
echo "[prod-update] fdev website update"
# fdev's `--address`/`--port` resolve the connection target. There is
# no `local`/`network` positional here — the PUT path is the same
# regardless. Capture stdout+stderr so we can disambiguate a real
# failure from the retry-race below.
PUB_LOG="$(mktemp)"
PUB_EXIT=0
"$FDEV" "${NODE_ARGS[@]}" website update \
    --key "$WEBSITE_KEY" "$HERE/frontend/dist" 2>&1 | tee "$PUB_LOG" \
    || PUB_EXIT=$?

# Detect the known retry-race: the first PUT actually lands in the
# node's DB, but the notification channel closes before fdev receives
# the ack, so fdev retries. The retries are rejected with
#   "New state version <N> must be higher than current version <N>"
# where both numbers match the just-uploaded version — i.e. our state
# IS already there. Treat that as success.
if [[ "$PUB_EXIT" -ne 0 ]]; then
    if grep -qE 'New state version ([0-9]+) must be higher than current version \1' "$PUB_LOG"; then
        echo "[prod-update] fdev returned $PUB_EXIT, but the state is already in DB"
        echo "[prod-update] (retry-race: first PUT landed, notification channel closed)."
        echo "[prod-update] proceeding with cache invalidation."
    else
        echo "[prod-update] fdev failed (exit $PUB_EXIT). See $PUB_LOG"
        exit "$PUB_EXIT"
    fi
fi

if [[ -n "$SSH_HOST" ]]; then
    echo
    echo "[prod-update] clearing unpacked webapp cache on $SSH_HOST"
    if [[ -z "$WEBAPP_ID" ]]; then
        echo "[prod-update] refusing to ssh: WEBAPP_ID empty"
        exit 1
    fi
    # Clear BOTH the unpacked directory AND the sibling `.hash`
    # sentinel. Leaving the hash file behind makes the next GET return
    # HTTP 500 ("Contract not cached yet") instead of triggering a
    # fresh unpack — see memory `webapp-cache-invalidation`.
    ssh "$SSH_HOST" "rm -rf '${WEBAPP_CACHE_DIR}/${WEBAPP_ID}/' '${WEBAPP_CACHE_DIR}/${WEBAPP_ID}.hash'"
    echo "[prod-update] cache cleared: ${WEBAPP_CACHE_DIR}/${WEBAPP_ID}/ (+ .hash)"
    # Warm the cache with one outer GET so subsequent users don't
    # race the unpack step.
    ssh "$SSH_HOST" "curl -s -o /dev/null -w 'warm GET status=%{http_code}\n' http://127.0.0.1:7509/v1/contract/web/${WEBAPP_ID}/"
else
    echo
    echo "[prod-update] SSH_HOST empty — skipping webapp cache rm."
    echo "[prod-update] if you see the old version, clear it manually:"
    echo "    ssh <node> \"rm -rf ${WEBAPP_CACHE_DIR}/${WEBAPP_ID}/ ${WEBAPP_CACHE_DIR}/${WEBAPP_ID}.hash\""
fi

echo
echo "[prod-update] DONE"
