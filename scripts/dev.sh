#!/usr/bin/env bash
#
# One-command dev loop:
#   1. warns if the local node isn't listening,
#   2. runs the watcher (initial publish + auto-republish on
#      shared/contract/delegate edits) in the background,
#   3. runs `trunk serve` in the foreground for the frontend.
#
# Loop semantics:
#   - editing frontend/src/**       → trunk rebuilds + hot-reloads tab
#   - editing shared/, contract/, delegate/ → watcher republishes,
#     rewrites dev-keys.json, trunk notices the JSON change and
#     hot-reloads the tab against the new contract / delegate.
#
# Ctrl-C kills the watcher and trunk together.

set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WS_PORT="${WS_PORT:-7509}"

if ! ss -tnl 2>/dev/null | grep -q ":${WS_PORT}\b"; then
    echo "[dev] WARNING: nothing listening on 127.0.0.1:${WS_PORT}"
    echo "[dev] start a local node first, e.g.:"
    echo "      $HERE/../freenet-core/target/debug/freenet local \\"
    echo "          --ws-api-address 0.0.0.0 --ws-api-port $WS_PORT \\"
    echo "          --data-dir /tmp/freenet-local"
    echo "[dev] continuing anyway — publish will fail loudly if the node isn't up."
fi

WATCH_PID=""
cleanup() {
    if [[ -n "$WATCH_PID" ]] && kill -0 "$WATCH_PID" 2>/dev/null; then
        kill "$WATCH_PID" 2>/dev/null || true
    fi
}
trap cleanup EXIT INT TERM

"$HERE/scripts/dev-watch.sh" &
WATCH_PID=$!

echo "[dev] watcher PID=$WATCH_PID"
echo "[dev] starting trunk serve (Ctrl-C kills both)"
cd "$HERE/frontend"
exec trunk serve
