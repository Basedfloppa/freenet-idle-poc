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
#   FDEV               default: ../freenet-core/target/debug/fdev
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
FDEV="${FDEV:-$HERE/../freenet-core/target/debug/fdev}"
WEBSITE_KEY="${WEBSITE_KEY:-idle-poc}"
SSH_HOST="${SSH_HOST:-orange}"
WEBAPP_CACHE_DIR="${WEBAPP_CACHE_DIR:-/root/.cache/freenet/webapp_cache}"

if [[ ! -x "$FDEV" ]]; then
    echo "[prod-update] fdev not found at: $FDEV"
    exit 1
fi

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
trunk build --release

echo
echo "[prod-update] fdev website update"
"$FDEV" "${NODE_ARGS[@]}" network website update \
    --key "$WEBSITE_KEY" "$HERE/frontend/dist"

if [[ -n "$SSH_HOST" ]]; then
    echo
    echo "[prod-update] clearing unpacked webapp cache on $SSH_HOST"
    # Guard against accidentally wiping the whole cache dir if
    # WEBAPP_ID is somehow empty — earlier check should have caught
    # it, but defence-in-depth is cheap.
    if [[ -z "$WEBAPP_ID" ]]; then
        echo "[prod-update] refusing to ssh: WEBAPP_ID empty"
        exit 1
    fi
    ssh "$SSH_HOST" "rm -rf '${WEBAPP_CACHE_DIR}/${WEBAPP_ID}/'"
    echo "[prod-update] cache cleared: ${WEBAPP_CACHE_DIR}/${WEBAPP_ID}/"
else
    echo
    echo "[prod-update] SSH_HOST empty — skipping webapp cache rm."
    echo "[prod-update] if you see the old version, clear it manually:"
    echo "    ssh <node> 'rm -rf ${WEBAPP_CACHE_DIR}/${WEBAPP_ID}/'"
fi

echo
echo "[prod-update] DONE"
