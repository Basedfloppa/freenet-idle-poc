#!/usr/bin/env bash
#
# Full first-time deploy to a Freenet node running in `network`
# mode (e.g. orange / baka). Sister of `dev-publish.sh`, but:
#   - talks to a remote node (env: NODE_URL, or NODE_ADDRESS+NODE_PORT)
#   - adds the `--release` flag so puts actually propagate into the
#     DHT instead of being executed locally
#   - patches `frontend/src/app/keys.rs` so the released webapp
#     ships with the prod contract / delegate ids baked in
#   - rebuilds the frontend with `trunk build --release`
#   - publishes the bundle as a website contract via `fdev website`
#
# This script is for FIRST-TIME deploys (or after a contract /
# delegate ABI change). For webapp-only iteration without touching
# the supporting contracts, use `prod-update-webapp.sh` — it skips
# the heavy contract publish and only bumps the website version.
#
# Required env / defaults:
#   FDEV             default: ../freenet-core/target/release/fdev, then
#                    target/debug/fdev. NOT $PATH/fdev — the system one
#                    is 0.3.151 and produces broken tarballs. Must
#                    support `website` subcommand (fdev ≥ 0.3.218).
#   NODE_URL         full ws URL of the prod node (overrides NODE_ADDRESS+NODE_PORT)
#                    typical SSH-tunnel form: ws://127.0.0.1:17509
#   NODE_ADDRESS     default 127.0.0.1   (used when NODE_URL is unset)
#   NODE_PORT        default 7509
#   WEBSITE_KEY      default idle-poc    (`fdev website init` slot)
#   PATCH_KEYS       default 1           (set to 0 to leave keys.rs alone)
#   STAGE_WEBAPP     default 1           (set to 0 to stop after contracts)
#   FORCE_REPUBLISH  default 0           (set to 1 to skip the
#                                        "code hash matches keys.rs"
#                                        optimization and re-publish
#                                        every contract / delegate
#                                        unconditionally — needed when
#                                        the node lost its store)
#
# Usage examples:
#   # via SSH tunnel forwarding orange's 7509 → local 17509
#   ssh -fNT -L 17509:127.0.0.1:7509 orange
#   NODE_URL=ws://127.0.0.1:17509 scripts/prod-publish.sh
#
#   # direct (rare — assumes the prod node WS API is reachable from this host)
#   NODE_ADDRESS=145.249.246.115 NODE_PORT=7509 scripts/prod-publish.sh
#
# Output:
#   - frontend/src/app/keys.rs updated in place (unless PATCH_KEYS=0)
#   - frontend/dev-keys.json overwritten with the prod ids too, so
#     `trunk build --release` in this checkout produces a coherent
#     bundle. Re-run `dev-publish.sh` afterwards to switch back to
#     local-node ids.
#   - frontend/prod-webapp-id.txt — the website contract id; the
#     update script reads it for subsequent version bumps.

set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WEBSITE_KEY="${WEBSITE_KEY:-idle-poc}"
PATCH_KEYS="${PATCH_KEYS:-1}"
STAGE_WEBAPP="${STAGE_WEBAPP:-1}"
FORCE_REPUBLISH="${FORCE_REPUBLISH:-0}"

# Resolve fdev — explicit FDEV wins, else prefer release over debug.
# $PATH is NOT consulted: the system fdev on this machine is 0.3.151
# and silently produces broken webapp tarballs.
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
    echo "[prod-publish] fdev not found. Build first:"
    echo "    cd $HERE/../freenet-core && cargo build --release --bin fdev"
    echo "[prod-publish] or set FDEV=/path/to/fdev (must support 'website' subcommand)."
    exit 1
fi

if ! "$FDEV" website --help >/dev/null 2>&1; then
    echo "[prod-publish] $FDEV does not support 'website' subcommand."
    echo "[prod-publish] need fdev ≥ 0.3.218. Got: $("$FDEV" --version 2>&1 | head -1)"
    exit 1
fi
echo "[prod-publish] using fdev: $FDEV ($("$FDEV" --version 2>&1 | head -1))"

# Resolve node connection flags once. fdev accepts either --node-url
# (full ws URL) or --address+--port (host pair). Stored as an array so
# we can splat it into each invocation without re-evaluating quoting.
NODE_ARGS=()
if [[ -n "${NODE_URL:-}" ]]; then
    NODE_ARGS+=(--node-url "$NODE_URL")
    echo "[prod-publish] target node: $NODE_URL"
else
    NODE_ADDRESS="${NODE_ADDRESS:-127.0.0.1}"
    NODE_PORT="${NODE_PORT:-7509}"
    NODE_ARGS+=(--address "$NODE_ADDRESS" --port "$NODE_PORT")
    echo "[prod-publish] target node: ws://${NODE_ADDRESS}:${NODE_PORT}"
fi

# Per-contract empty initial state (same as dev-publish.sh).
PRESENCE_STATE="$(mktemp)"
MAILBOX_STATE="$(mktemp)"
GUILDS_STATE="$(mktemp)"
printf '\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00' > "$PRESENCE_STATE"
printf '\x01\x00\x00\x00\x00\x00\x00\x00\x00' > "$MAILBOX_STATE"
printf '\x01\x00\x00\x00\x00\x00\x00\x00\x00' > "$GUILDS_STATE"

extract() {
    local pattern="$1" file="$2"
    sed -E 's/\x1b\[[0-9;]*m//g' "$file" | grep -oP "$pattern" | tail -1 || true
}

# Read the string value of a `pub const NAME: &str = "…";` line from
# keys.rs. Empty string when the constant is missing or empty.
read_keys_const() {
    local name="$1"
    sed -nE "s/^pub const ${name}: &str = \"([^\"]+)\";.*/\\1/p" \
        "$HERE/frontend/src/app/keys.rs"
}

build_and_publish_contract() {
    local crate="$1" artefact="$2" state_file="$3" label="$4"
    local out_hash_var="$5" out_id_var="$6"
    local hash_const="$7" id_const="$8"

    echo "[prod-publish] building $label"
    cd "$HERE/$crate"

    local build_log pub_log code_hash instance_id prev_hash prev_id
    build_log="$(mktemp)"
    CARGO_TARGET_DIR="$PWD/target" "$FDEV" build 2>&1 | tee "$build_log"
    code_hash="$(extract 'code hash: \K\S+' "$build_log")"
    if [[ -z "$code_hash" ]]; then
        echo "[prod-publish] could not parse $label code hash"; exit 1
    fi

    # Skip the publish when the freshly-built code hash matches what's
    # already baked into keys.rs — the on-network contract is
    # byte-identical so re-publishing would just re-issue the same
    # instance id and waste a PUT round-trip. Override via
    # FORCE_REPUBLISH=1 if the node lost its store.
    prev_hash="$(read_keys_const "$hash_const")"
    prev_id="$(read_keys_const "$id_const")"
    if [[ "$FORCE_REPUBLISH" != "1" \
          && -n "$prev_hash" && "$prev_hash" == "$code_hash" \
          && -n "$prev_id" ]]; then
        echo "[prod-publish] $label code hash unchanged ($code_hash) — skipping publish, reusing id $prev_id"
        instance_id="$prev_id"
    else
        echo "[prod-publish] publishing $label to prod"
        pub_log="$(mktemp)"
        CARGO_TARGET_DIR="$PWD/target" "$FDEV" "${NODE_ARGS[@]}" publish \
            --code "build/freenet/$artefact" \
            contract --state "$state_file" 2>&1 | tee "$pub_log"
        instance_id="$(extract 'Publishing contract \K[1-9A-HJ-NP-Za-km-z]{30,}' "$pub_log")"
        if [[ -z "$instance_id" ]]; then
            echo "[prod-publish] could not parse $label instance id"; exit 1
        fi
    fi

    printf -v "$out_hash_var" '%s' "$code_hash"
    printf -v "$out_id_var" '%s' "$instance_id"
}

###############################################################################
build_and_publish_contract \
    presence-contract presence_contract "$PRESENCE_STATE" "presence-contract" \
    CODE_HASH CONTRACT_ID \
    CODE_HASH_B58 CONTRACT_ID_B58

build_and_publish_contract \
    mailbox-contract mailbox_contract "$MAILBOX_STATE" "mailbox-contract" \
    MAILBOX_CODE_HASH MAILBOX_ID \
    MAILBOX_CODE_HASH_B58 MAILBOX_CONTRACT_ID_B58

build_and_publish_contract \
    guilds-contract guilds_contract "$GUILDS_STATE" "guilds-contract" \
    GUILDS_CODE_HASH GUILDS_ID \
    GUILDS_CODE_HASH_B58 GUILDS_CONTRACT_ID_B58

###############################################################################
# Delegate — no initial state; `key:` line instead of `Publishing
# contract …`.
echo "[prod-publish] building identity-delegate"
cd "$HERE/identity-delegate"

DELEGATE_BUILD_LOG="$(mktemp)"
CARGO_TARGET_DIR="$PWD/target" "$FDEV" build --package-type delegate 2>&1 \
    | tee "$DELEGATE_BUILD_LOG"
DELEGATE_CODE_HASH="$(extract 'code hash: \K\S+' "$DELEGATE_BUILD_LOG")"
if [[ -z "$DELEGATE_CODE_HASH" ]]; then
    echo "[prod-publish] could not parse delegate code hash"; exit 1
fi

# Same skip-if-unchanged optimization as for contracts.
PREV_DELEGATE_HASH="$(read_keys_const DELEGATE_CODE_HASH_B58)"
PREV_DELEGATE_KEY="$(read_keys_const DELEGATE_KEY_B58)"
if [[ "$FORCE_REPUBLISH" != "1" \
      && -n "$PREV_DELEGATE_HASH" && "$PREV_DELEGATE_HASH" == "$DELEGATE_CODE_HASH" \
      && -n "$PREV_DELEGATE_KEY" ]]; then
    echo "[prod-publish] delegate code hash unchanged ($DELEGATE_CODE_HASH) — skipping publish, reusing key $PREV_DELEGATE_KEY"
    DELEGATE_KEY="$PREV_DELEGATE_KEY"
else
    echo "[prod-publish] publishing identity-delegate to prod"
    DELEGATE_PUB_LOG="$(mktemp)"
    CARGO_TARGET_DIR="$PWD/target" "$FDEV" "${NODE_ARGS[@]}" publish \
        --code build/freenet/identity_delegate \
        delegate 2>&1 | tee "$DELEGATE_PUB_LOG"
    DELEGATE_KEY="$(extract 'key: \K[1-9A-HJ-NP-Za-km-z]{30,}' "$DELEGATE_PUB_LOG")"
    if [[ -z "$DELEGATE_KEY" ]]; then
        echo "[prod-publish] could not parse delegate key"; exit 1
    fi
fi

# Stage the versioned delegate WASM into the frontend so trunk's
# copy-file rule bundles it into dist/. The frontend fetches this
# at startup and auto-registers the delegate on whichever node is
# serving the webapp — required for self-hosted users whose nodes
# don't have the delegate pre-installed (delegates are NOT
# replicated through the DHT). The fdev publish above still
# registers on the target node so the very first user (the
# publisher) doesn't hit a register-then-call race on first load.
cp "$HERE/identity-delegate/build/freenet/identity_delegate" \
   "$HERE/frontend/identity_delegate.wasm"
echo "[prod-publish] copied identity_delegate to frontend/identity_delegate.wasm"

# Stage the freshly-built presence-contract WASM into the frontend
# the same way. The webapp Puts the bundled container on connect
# (and on every heartbeat, as a workaround for the freenet-core
# Update-silently-dropped bug) — without this copy the bundle ships
# with the previous run's contract code, so the Put lands under the
# OLD contract_id while the Get/Subscribe targets the NEW id, and
# heartbeats never reach the new contract's state store.
cp "$HERE/presence-contract/build/freenet/presence_contract" \
   "$HERE/frontend/presence_contract.wasm"
echo "[prod-publish] copied presence_contract to frontend/presence_contract.wasm"

###############################################################################
# Patch frontend/src/app/keys.rs so the compile-time defaults match
# what we just published. The release build picks these up — even if
# `dev-keys.json` is later stripped or fails to load, the webapp
# still resolves the right contracts.
###############################################################################
KEYS_RS="$HERE/frontend/src/app/keys.rs"
if [[ "$PATCH_KEYS" == "1" ]]; then
    echo
    echo "[prod-publish] patching $KEYS_RS"
    # Backup once per run so a botched sed is recoverable.
    cp "$KEYS_RS" "$KEYS_RS.bak"
    sed -i -E \
        -e "s|^(pub const CONTRACT_ID_B58: &str =).*|\1 \"$CONTRACT_ID\";|" \
        -e "s|^(pub const CODE_HASH_B58: &str =).*|\1 \"$CODE_HASH\";|" \
        -e "s|^(pub const DELEGATE_KEY_B58: &str =).*|\1 \"$DELEGATE_KEY\";|" \
        -e "s|^(pub const DELEGATE_CODE_HASH_B58: &str =).*|\1 \"$DELEGATE_CODE_HASH\";|" \
        -e "s|^(pub const MAILBOX_CONTRACT_ID_B58: &str =).*|\1 \"$MAILBOX_ID\";|" \
        -e "s|^(pub const MAILBOX_CODE_HASH_B58: &str =).*|\1 \"$MAILBOX_CODE_HASH\";|" \
        -e "s|^(pub const GUILDS_CONTRACT_ID_B58: &str =).*|\1 \"$GUILDS_ID\";|" \
        -e "s|^(pub const GUILDS_CODE_HASH_B58: &str =).*|\1 \"$GUILDS_CODE_HASH\";|" \
        "$KEYS_RS"
    echo "[prod-publish] keys.rs updated (backup at keys.rs.bak)"
else
    echo "[prod-publish] PATCH_KEYS=0 — leaving keys.rs alone"
fi

# Mirror the prod ids into dev-keys.json too, so an immediate `trunk
# build --release` here ships a self-consistent bundle. After deploy,
# `dev-publish.sh` will overwrite this with local-node ids again.
cat > "$HERE/frontend/dev-keys.json" <<EOF
{
  "contract_id_b58": "$CONTRACT_ID",
  "code_hash_b58": "$CODE_HASH",
  "delegate_key_b58": "$DELEGATE_KEY",
  "delegate_code_hash_b58": "$DELEGATE_CODE_HASH",
  "mailbox_contract_id_b58": "$MAILBOX_ID",
  "mailbox_code_hash_b58": "$MAILBOX_CODE_HASH",
  "guilds_contract_id_b58": "$GUILDS_ID",
  "guilds_code_hash_b58": "$GUILDS_CODE_HASH"
}
EOF

if [[ "$STAGE_WEBAPP" != "1" ]]; then
    echo
    echo "[prod-publish] STAGE_WEBAPP=0 — stopping after contracts/delegate."
    echo "[prod-publish] prod ids:"
    echo "  contract:        $CONTRACT_ID"
    echo "  delegate:        $DELEGATE_KEY"
    echo "  mailbox:         $MAILBOX_ID"
    echo "  guilds:          $GUILDS_ID"
    exit 0
fi

###############################################################################
# Build frontend in release mode.
###############################################################################
echo
echo "[prod-publish] trunk build --release"
cd "$HERE/frontend"
trunk build --release

###############################################################################
# Webapp signing key. `init` only needs to run once per machine; if
# the toml already exists we skip it. The store path
# (`~/.config/freenet/website-keys/<name>.toml`) is documented by
# operator-nodes / webrtc-poc-deployed memory.
###############################################################################
WEBKEY_FILE="${XDG_CONFIG_HOME:-$HOME/.config}/freenet/website-keys/${WEBSITE_KEY}.toml"
if [[ ! -f "$WEBKEY_FILE" ]]; then
    echo "[prod-publish] generating website signing key '$WEBSITE_KEY'"
    "$FDEV" website init "$WEBSITE_KEY"
else
    echo "[prod-publish] reusing existing website signing key '$WEBSITE_KEY'"
fi

###############################################################################
# Publish webapp. fdev emits a contract id for the freshly published
# website — same grammar as contract publish. Captured for the update
# script.
###############################################################################
echo "[prod-publish] publishing webapp via fdev website publish"
WEBSITE_PUB_LOG="$(mktemp)"
WEBSITE_PUB_EXIT=0
"$FDEV" "${NODE_ARGS[@]}" website publish \
    --key "$WEBSITE_KEY" "$HERE/frontend/dist" 2>&1 | tee "$WEBSITE_PUB_LOG" \
    || WEBSITE_PUB_EXIT=$?

# Same retry-race as in prod-update-webapp.sh — see note there.
if [[ "$WEBSITE_PUB_EXIT" -ne 0 ]]; then
    if grep -qE 'New state version ([0-9]+) must be higher than current version \1' "$WEBSITE_PUB_LOG"; then
        echo "[prod-publish] fdev returned $WEBSITE_PUB_EXIT — retry-race (state is in DB)"
    else
        echo "[prod-publish] fdev website publish failed (exit $WEBSITE_PUB_EXIT)"
        echo "[prod-publish] full log: $WEBSITE_PUB_LOG"
        exit "$WEBSITE_PUB_EXIT"
    fi
fi

# Capture the webapp contract id. Patterns in priority order:
#   "Publishing website as contract <id> (version <n>)" — current fdev
#   "Publishing contract <id>"                          — older fdev
#   "contract id: <id>"                                 — legacy
WEBAPP_ID="$(extract 'Publishing website as contract \K[1-9A-HJ-NP-Za-km-z]{30,}' "$WEBSITE_PUB_LOG")"
if [[ -z "$WEBAPP_ID" ]]; then
    WEBAPP_ID="$(extract 'Publishing contract \K[1-9A-HJ-NP-Za-km-z]{30,}' "$WEBSITE_PUB_LOG")"
fi
if [[ -z "$WEBAPP_ID" ]]; then
    WEBAPP_ID="$(extract 'contract id: \K[1-9A-HJ-NP-Za-km-z]{30,}' "$WEBSITE_PUB_LOG")"
fi
if [[ -z "$WEBAPP_ID" ]]; then
    echo "[prod-publish] WARNING: couldn't parse webapp contract id from output."
    echo "[prod-publish] grep the publish log manually: $WEBSITE_PUB_LOG"
else
    echo "$WEBAPP_ID" > "$HERE/frontend/prod-webapp-id.txt"
    echo
    echo "[prod-publish] webapp contract id: $WEBAPP_ID"
    echo "[prod-publish] saved to frontend/prod-webapp-id.txt"
fi

echo
echo "[prod-publish] DONE"
echo "  presence:  $CONTRACT_ID    (code $CODE_HASH)"
echo "  mailbox:   $MAILBOX_ID     (code $MAILBOX_CODE_HASH)"
echo "  guilds:    $GUILDS_ID      (code $GUILDS_CODE_HASH)"
echo "  delegate:  $DELEGATE_KEY   (code $DELEGATE_CODE_HASH)"
[[ -n "$WEBAPP_ID" ]] && echo "  webapp:    $WEBAPP_ID"
echo
echo "  Open the webapp at the prod node's gateway, e.g."
echo "    http://orange.local:50509/v1/contract/web/$WEBAPP_ID/"
echo "  (the actual gateway host/port depends on your node config)."
