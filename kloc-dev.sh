#!/bin/bash
set -euo pipefail

# kloc-dev: Full pipeline wrapper for development
# Indexes reference project -> maps to sot.json -> runs kloc-cli
#
# Usage:
#   ./kloc-dev.sh context "App\Service\OrderService" --depth 2 --impl
#   ./kloc-dev.sh context "App\Service\OrderService" --id=my-test --depth 2
#   ./kloc-dev.sh resolve "App\Entity\Order" --id=my-test
#   ./kloc-dev.sh context "App\Service\OrderService" --internal-all
#
# Artifacts are stored in: artifacts/kloc-dev/{id}/
# When --id is provided and artifacts exist, the index/map steps are skipped.

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SCRIPT_NAME="kloc-dev"
ARTIFACTS_BASE="$SCRIPT_DIR/artifacts/$SCRIPT_NAME"

# --- Parse --id flag (extract before passing rest to kloc-cli) ---

RUN_ID=""
INTERNAL_ALL=""
PASSTHROUGH_ARGS=()

for arg in "$@"; do
    case "$arg" in
        --id=*)
            RUN_ID="${arg#--id=}"
            ;;
        --internal-all)
            INTERNAL_ALL="--internal-all"
            ;;
        *)
            PASSTHROUGH_ARGS+=("$arg")
            ;;
    esac
done

# Generate ID if not provided
if [[ -z "$RUN_ID" ]]; then
    RUN_ID="$(date +%Y%m%d-%H%M%S)"
fi

ARTIFACT_DIR="$ARTIFACTS_BASE/$RUN_ID"
INDEX_FILE="$ARTIFACT_DIR/index.json"
SOT_FILE="$ARTIFACT_DIR/sot.json"

echo "== kloc-dev pipeline =="
echo "ID:        $RUN_ID"
echo "Artifacts: $ARTIFACT_DIR"
echo ""

# --- Step 1: Generate SCIP index (if needed) ---

if [[ -f "$INDEX_FILE" ]]; then
    echo "[1/3] Index exists, skipping indexing"
else
    echo "[1/3] Generating SCIP index..."
    mkdir -p "$ARTIFACT_DIR"
    "$SCRIPT_DIR/scip-php/bin/scip-php.sh" \
        -d "$SCRIPT_DIR/kloc-reference-project-php" \
        -o "$ARTIFACT_DIR" \
        $INTERNAL_ALL
    echo ""
fi

if [[ ! -f "$INDEX_FILE" ]]; then
    echo "Error: index.json not found at $INDEX_FILE after indexing"
    exit 1
fi

# --- Step 2: Map to sot.json (if needed) ---

if [[ -f "$SOT_FILE" ]]; then
    echo "[2/3] sot.json exists, skipping mapping"
else
    echo "[2/3] Mapping index to sot.json..."
    cd "$SCRIPT_DIR/kloc-mapper"
    uv run kloc-mapper map "$INDEX_FILE" -o "$SOT_FILE"
    cd "$SCRIPT_DIR"
    echo ""
fi

if [[ ! -f "$SOT_FILE" ]]; then
    echo "Error: sot.json not found at $SOT_FILE after mapping"
    exit 1
fi

# --- Step 3: Run kloc-cli with --sot injected ---

if [[ ${#PASSTHROUGH_ARGS[@]} -eq 0 ]]; then
    echo "[3/3] No kloc-cli command provided. Artifacts ready at:"
    echo "      Index: $INDEX_FILE"
    echo "      SoT:   $SOT_FILE"
    echo ""
    echo "Usage: $0 <command> <symbol> [flags...] [--id=$RUN_ID]"
    exit 0
fi

echo "[3/3] Running kloc-cli..."
echo ""

cd "$SCRIPT_DIR/kloc-cli"
exec uv run kloc-cli "${PASSTHROUGH_ARGS[@]}" --sot "$SOT_FILE"
