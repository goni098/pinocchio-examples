#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$SCRIPT_DIR/.."

cd "$ROOT"

# ─── 1. Build all SBF programs ────────────────────────────────────────────────
echo ">>> Building all SBF programs..."
cargo build-sbf --workspace

# ─── 2. Generate IDLs with shank ──────────────────────────────────────────────
echo ">>> Generating IDLs..."
mkdir -p idl

CRATE_PATHS=("basic/counter" "basic/close-account" "basic/full-shank" "token/basic-mint" "token/vault")
IDL_NAMES=("counter"         "close_account"        "full_shank"        "basic_mint"       "vault")

for i in "${!CRATE_PATHS[@]}"; do
    CRATE_PATH="${CRATE_PATHS[$i]}"
    OUT_NAME="${IDL_NAMES[$i]}"
    echo "  shank idl: $CRATE_PATH -> idl/${OUT_NAME}.json"
    shank idl -o idl --out-filename "${OUT_NAME}.json" -r "$CRATE_PATH" || echo "  (skipped: shank could not process $CRATE_PATH)"
done

# ─── 3. Generate JS clients with codama ───────────────────────────────────────
echo ">>> Generating JS clients..."
node scripts/gen-clients.js

# ─── 4. Format generated code ─────────────────────────────────────────────────
echo ">>> Formatting..."
pnpm biome check . --write --unsafe

echo ""
echo "Done. IDLs in idl/, JS clients in js-client/"
