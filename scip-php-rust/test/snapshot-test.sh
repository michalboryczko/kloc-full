#!/usr/bin/env bash
set -euo pipefail

# Usage: ./test/snapshot-test.sh <php-project-path> [--verbose]
# Example: ./test/snapshot-test.sh ../../kloc-reference-project-php/

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

PHP_PROJECT="${1:-}"
VERBOSE="${2:-}"

if [[ -z "$PHP_PROJECT" ]]; then
    echo "Usage: $0 <php-project-path> [--verbose]"
    exit 1
fi

PHP_PROJECT="$(cd "$PHP_PROJECT" && pwd)"
PHP_BIN="${SCIP_PHP_BIN:-$PROJECT_ROOT/../scip-php/bin/scip-php}"
RUST_BIN="${RUST_BIN:-$PROJECT_ROOT/target/debug/scip-php-rust}"

WORK_DIR="$(mktemp -d)"
trap "rm -rf $WORK_DIR" EXIT

echo "=== Snapshot Test ==="
echo "Project:    $PHP_PROJECT"
echo "PHP bin:    $PHP_BIN"
echo "Rust bin:   $RUST_BIN"
echo "Work dir:   $WORK_DIR"
echo ""

# --- Check prerequisites ---
if [[ ! -x "$PHP_BIN" ]]; then
    echo "ERROR: PHP scip-php binary not found at: $PHP_BIN"
    echo "Set SCIP_PHP_BIN env var or build scip-php first."
    exit 1
fi

if [[ ! -x "$RUST_BIN" ]]; then
    echo "ERROR: Rust binary not found at: $RUST_BIN"
    echo "Run: cargo build"
    exit 1
fi

if ! command -v jq &>/dev/null; then
    echo "ERROR: jq not found. Install with: brew install jq"
    exit 1
fi

# --- Run PHP implementation ---
echo "[1/4] Running PHP scip-php..."
PHP_OUT_DIR="$WORK_DIR/php"
mkdir -p "$PHP_OUT_DIR"
"$PHP_BIN" --project-root "$PHP_PROJECT" --output-dir "$PHP_OUT_DIR" 2>/dev/null || true
PHP_INDEX="$PHP_OUT_DIR/index.json"
PHP_CALLS="$PHP_OUT_DIR/calls.json"

if [[ ! -f "$PHP_INDEX" ]]; then
    echo "ERROR: PHP produced no index.json"
    exit 1
fi

# --- Run Rust implementation ---
echo "[2/4] Running Rust scip-php-rust..."
RUST_OUT_DIR="$WORK_DIR/rust"
mkdir -p "$RUST_OUT_DIR"
"$RUST_BIN" --project-root "$PHP_PROJECT" --output-dir "$RUST_OUT_DIR" 2>/dev/null || true
RUST_INDEX="$RUST_OUT_DIR/index.json"

# --- Normalize both outputs ---
echo "[3/4] Normalizing JSON outputs..."

normalize_json() {
    local input="$1"
    local output="$2"
    # Sort keys, normalize absolute paths to relative, sort arrays where order shouldn't matter
    jq --sort-keys \
        'walk(if type == "string" then gsub("'"$PHP_PROJECT"'"; "<PROJECT>") else . end)' \
        "$input" > "$output"
}

if [[ -f "$PHP_INDEX" ]]; then
    normalize_json "$PHP_INDEX" "$WORK_DIR/php-normalized.json"
fi
if [[ -f "$RUST_INDEX" ]]; then
    normalize_json "$RUST_INDEX" "$WORK_DIR/rust-normalized.json"
else
    echo '{}' > "$WORK_DIR/rust-normalized.json"
fi

# --- Diff ---
echo "[4/4] Diffing outputs..."
echo ""

DIFF_OUTPUT="$WORK_DIR/diff.txt"
diff --unified=3 \
    "$WORK_DIR/php-normalized.json" \
    "$WORK_DIR/rust-normalized.json" > "$DIFF_OUTPUT" || true

DIFF_LINES=$(wc -l < "$DIFF_OUTPUT")
if [[ "$DIFF_LINES" -eq 0 ]]; then
    echo "SUCCESS: Outputs are identical!"
else
    echo "DIFF: $DIFF_LINES lines differ"
    echo ""

    # Summary statistics
    ADDED=$(grep -c '^+' "$DIFF_OUTPUT" || true)
    REMOVED=$(grep -c '^-' "$DIFF_OUTPUT" || true)
    echo "  Lines in PHP output only (missing from Rust): $REMOVED"
    echo "  Lines in Rust output only (extra in Rust):    $ADDED"
    echo ""

    if [[ "$VERBOSE" == "--verbose" || "$VERBOSE" == "-v" ]]; then
        cat "$DIFF_OUTPUT"
    else
        echo "Run with --verbose to see full diff."
    fi
fi

# --- Field-level statistics ---
echo ""
echo "=== Field Statistics ==="
if [[ -f "$PHP_INDEX" ]]; then
    PHP_DOCS=$(jq '.documents | length' "$PHP_INDEX" 2>/dev/null || echo 0)
    PHP_OCCS=$(jq '[.documents[].occurrences | length] | add // 0' "$PHP_INDEX" 2>/dev/null || echo 0)
    PHP_SYMS=$(jq '[.documents[].symbols | length] | add // 0' "$PHP_INDEX" 2>/dev/null || echo 0)
    echo "  PHP:  documents=$PHP_DOCS  occurrences=$PHP_OCCS  symbols=$PHP_SYMS"
fi
if [[ -f "$RUST_INDEX" ]]; then
    RUST_DOCS=$(jq '.documents | length' "$RUST_INDEX" 2>/dev/null || echo 0)
    RUST_OCCS=$(jq '[.documents[].occurrences | length] | add // 0' "$RUST_INDEX" 2>/dev/null || echo 0)
    RUST_SYMS=$(jq '[.documents[].symbols | length] | add // 0' "$RUST_INDEX" 2>/dev/null || echo 0)
    echo "  Rust: documents=$RUST_DOCS  occurrences=$RUST_OCCS  symbols=$RUST_SYMS"
fi

exit 0
