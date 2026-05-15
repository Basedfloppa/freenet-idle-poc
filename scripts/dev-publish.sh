#!/usr/bin/env bash
#
# Build + publish every artefact the frontend talks to:
#   1. presence-contract  → leaderboard / World Boss aggregator
#   2. mailbox-contract   → player-to-player signed message log
#   3. guilds-contract    → cooperative group registry
#   4. identity-delegate  → seed + Inventory authority
#
# Captures each instance_id / code_hash / delegate_key and writes
# them all into frontend/dev-keys.json. Trunk's copy-file directive
# picks the file up, the watcher triggers a hot-reload of the tab.
#
# Env overrides:
#   FDEV  — path to the fdev binary (default: locally-built debug)
#   WS    — ws URL of the local node (default: ws://127.0.0.1:7509)

set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FDEV="${FDEV:-$HERE/../freenet-core/target/debug/fdev}"

if [[ ! -x "$FDEV" ]]; then
    echo "[dev-publish] fdev not found at: $FDEV"
    echo "[dev-publish] build it first: cd $HERE/../freenet-core && cargo build --bin fdev"
    exit 1
fi

# Per-contract empty initial state. Each is the bincode-serialized
# `Default::default()` of the contract's `*State` struct in shared/.
# Bincode 1.x uses fixed-int u64 (8 bytes little-endian) for Vec /
# BTreeMap length, plus 1 byte for the `version: u8` prefix.
#
#   presence ContractState  : version(1) + entries(8) + cumulative_damage(8)
#                           = 17 bytes  →  01 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
#   mailbox  MailboxState   : version(1) + entries(8)
#                           = 9 bytes   →  01 00 00 00 00 00 00 00 00
#   guilds   GuildsState    : version(1) + guilds(8)
#                           = 9 bytes   →  01 00 00 00 00 00 00 00 00
#
# Keep these in sync with `Default for *State` in shared/src/freenet.rs.
PRESENCE_STATE="$(mktemp)"
MAILBOX_STATE="$(mktemp)"
GUILDS_STATE="$(mktemp)"
printf '\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00' > "$PRESENCE_STATE"
printf '\x01\x00\x00\x00\x00\x00\x00\x00\x00' > "$MAILBOX_STATE"
printf '\x01\x00\x00\x00\x00\x00\x00\x00\x00' > "$GUILDS_STATE"

extract() {
    # Pull a regex-captured group from a log file. fdev emits ANSI
    # color escapes around field names (e.g. `[1;32mkey[0m: VALUE`),
    # which would prevent a naive `key: ` regex from matching — so
    # strip them first. Empty result is OK; the caller decides
    # whether that's an error.
    local pattern="$1" file="$2"
    sed -E 's/\x1b\[[0-9;]*m//g' "$file" | grep -oP "$pattern" | tail -1 || true
}

# Build + publish a contract crate. Captures code_hash from the
# build log and instance_id from the publish log; both written to
# the named globals (passed by name in $5+).
#   $1: crate dir (under $HERE)
#   $2: built artefact name (under build/freenet/<name>)
#   $3: empty-state file path
#   $4: human label for logs
#   $5: var name to receive code_hash
#   $6: var name to receive instance_id
build_and_publish_contract() {
    local crate="$1" artefact="$2" state_file="$3" label="$4"
    local out_hash_var="$5" out_id_var="$6"

    echo "[dev-publish] building $label"
    cd "$HERE/$crate"

    local build_log pub_log code_hash instance_id
    build_log="$(mktemp)"
    CARGO_TARGET_DIR="$PWD/target" "$FDEV" build 2>&1 | tee "$build_log"
    code_hash="$(extract 'code hash: \K\S+' "$build_log")"
    if [[ -z "$code_hash" ]]; then
        echo "[dev-publish] could not parse $label code hash"; exit 1
    fi

    echo "[dev-publish] publishing $label"
    pub_log="$(mktemp)"
    CARGO_TARGET_DIR="$PWD/target" "$FDEV" publish \
        --code "build/freenet/$artefact" \
        contract --state "$state_file" 2>&1 | tee "$pub_log"
    instance_id="$(extract 'Publishing contract \K[1-9A-HJ-NP-Za-km-z]{30,}' "$pub_log")"
    if [[ -z "$instance_id" ]]; then
        echo "[dev-publish] could not parse $label instance id"; exit 1
    fi

    # Eval-assign back to the named globals so the caller can use
    # them across multiple invocations of this helper.
    printf -v "$out_hash_var" '%s' "$code_hash"
    printf -v "$out_id_var" '%s' "$instance_id"
}

###############################################################################
build_and_publish_contract \
    presence-contract presence_contract "$PRESENCE_STATE" "presence-contract" \
    CODE_HASH CONTRACT_ID

build_and_publish_contract \
    mailbox-contract mailbox_contract "$MAILBOX_STATE" "mailbox-contract" \
    MAILBOX_CODE_HASH MAILBOX_ID

build_and_publish_contract \
    guilds-contract guilds_contract "$GUILDS_STATE" "guilds-contract" \
    GUILDS_CODE_HASH GUILDS_ID

###############################################################################
# identity-delegate has no initial state — `fdev publish delegate`
# emits a `key:` line rather than `Publishing contract …`.
###############################################################################
echo "[dev-publish] building identity-delegate"
cd "$HERE/identity-delegate"

DELEGATE_BUILD_LOG="$(mktemp)"
CARGO_TARGET_DIR="$PWD/target" "$FDEV" build --package-type delegate 2>&1 \
    | tee "$DELEGATE_BUILD_LOG"
DELEGATE_CODE_HASH="$(extract 'code hash: \K\S+' "$DELEGATE_BUILD_LOG")"
if [[ -z "$DELEGATE_CODE_HASH" ]]; then
    echo "[dev-publish] could not parse delegate code hash"; exit 1
fi

echo "[dev-publish] publishing identity-delegate"
DELEGATE_PUB_LOG="$(mktemp)"
CARGO_TARGET_DIR="$PWD/target" "$FDEV" publish \
    --code build/freenet/identity_delegate \
    delegate 2>&1 | tee "$DELEGATE_PUB_LOG"
DELEGATE_KEY="$(extract 'key: \K[1-9A-HJ-NP-Za-km-z]{30,}' "$DELEGATE_PUB_LOG")"
if [[ -z "$DELEGATE_KEY" ]]; then
    echo "[dev-publish] could not parse delegate key"; exit 1
fi

# Stage the versioned delegate WASM into the frontend so trunk's
# copy-file rule picks it up and bundles it into dist/. The frontend
# fetches this at startup and auto-registers the delegate on the
# local node — required for self-hosted users whose nodes don't
# have the delegate pre-installed (delegates are NOT replicated
# through the DHT, only contracts are). `fdev publish ... delegate`
# above still runs because it's the only way to register on remote
# nodes you don't control; auto-register handles the rest.
cp "$HERE/identity-delegate/build/freenet/identity_delegate" \
   "$HERE/frontend/identity_delegate.wasm"
echo "[dev-publish] copied identity_delegate to frontend/identity_delegate.wasm"

###############################################################################
# write dev-keys.json — trunk's copy-file directive picks it up and
# the watcher triggers a hot-reload of the browser tab. Field names
# must mirror `DevKeys` in frontend/src/main.rs.
###############################################################################
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

echo
echo "[dev-publish] wrote frontend/dev-keys.json:"
cat "$HERE/frontend/dev-keys.json"
echo
