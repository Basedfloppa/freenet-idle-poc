#!/usr/bin/env bash
#
# Watcher loop: re-runs scripts/dev-publish.sh whenever any source
# file outside frontend/ changes (shared/, presence-contract/,
# identity-delegate/). Trunk's own watcher already handles
# frontend/src + style + index.html for hot reload, and it
# automatically picks up the new dev-keys.json that dev-publish.sh
# writes on every run — so editing UI feels seamless and editing
# the contract or delegate re-bakes + re-publishes them without
# any manual step.
#
# Uses inotifywait if available (event-driven, instant), otherwise
# falls back to a 1-second polling loop using `find -printf '%T@'`
# (no external deps required).
#
# Run alongside `trunk serve`, OR use `scripts/dev.sh` which starts
# both in one shot.

set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# Watch the source trees of the non-frontend crates plus their
# Cargo.toml. Leaf paths only — we deliberately don't watch target/
# or build/ subdirs to avoid the cargo-build-touches-files feedback
# loop.
WATCH_DIRS=(
    "$HERE/shared/src"
    "$HERE/shared-wire/src"
    "$HERE/presence-contract/src"
    "$HERE/mailbox-contract/src"
    "$HERE/guilds-contract/src"
    "$HERE/identity-delegate/src"
)
WATCH_FILES=(
    "$HERE/shared/Cargo.toml"
    "$HERE/shared-wire/Cargo.toml"
    "$HERE/presence-contract/Cargo.toml"
    "$HERE/mailbox-contract/Cargo.toml"
    "$HERE/guilds-contract/Cargo.toml"
    "$HERE/identity-delegate/Cargo.toml"
)

republish() {
    echo
    echo "[watch] $(date +%H:%M:%S) change detected — republishing"
    if "$HERE/scripts/dev-publish.sh" > /tmp/idle-watch.log 2>&1; then
        echo "[watch] ok. dev-keys.json:"
        sed -n 's/.*"\([a-z_]*_b58\)": *"\([^"]*\)".*/    \1 = \2/p' \
            "$HERE/frontend/dev-keys.json"
    else
        echo "[watch] FAILED — see /tmp/idle-watch.log"
    fi
}

# Initial publish so dev-keys.json reflects the current code before
# any change.
republish

if command -v inotifywait >/dev/null 2>&1; then
    echo
    echo "[watch] using inotifywait (event-driven)"
    while true; do
        inotifywait -r -q -e modify,create,delete,move \
            "${WATCH_DIRS[@]}" "${WATCH_FILES[@]}" >/dev/null 2>&1
        # Debounce: drain any further events that hit in the next
        # second (cargo fmt, multi-file save, etc.) so we trigger
        # exactly once per logical "save".
        while inotifywait -r -q -t 1 -e modify,create,delete,move \
            "${WATCH_DIRS[@]}" "${WATCH_FILES[@]}" >/dev/null 2>&1; do
            :
        done
        republish
    done
else
    echo
    echo "[watch] inotifywait not found — falling back to 1s polling"
    echo "[watch] (install inotify-tools for instant detection)"
    prev_state=""
    while true; do
        # mtime + path for every watched file, sorted for stable
        # comparison. find -printf is GNU find — works on Linux,
        # also on Debian/Ubuntu/Arch by default.
        state="$(
            { find "${WATCH_DIRS[@]}" -type f \
                  \( -name '*.rs' -o -name 'Cargo.toml' \) -printf '%T@ %p\n'
              stat -c '%Y %n' "${WATCH_FILES[@]}" 2>/dev/null
            } 2>/dev/null | sort
        )"
        if [[ -n "$prev_state" && "$state" != "$prev_state" ]]; then
            republish
        fi
        prev_state="$state"
        sleep 1
    done
fi
