#!/bin/bash
set -euo pipefail

# Snapshot test tool for kloc-cli context command
#
# Usage:
#   ./tests/snapshot.sh capture              # Run all cases, save snapshot
#   ./tests/snapshot.sh verify <snapshot>    # Compare current output against snapshot
#   ./tests/snapshot.sh list                 # List available snapshots
#   ./tests/snapshot.sh diff <snapshot>      # Show diffs for failing cases

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
CLI_DIR="$REPO_ROOT/kloc-cli"
CASES_FILE="$SCRIPT_DIR/cases.json"

# Read sot_id from cases.json
SOT_ID=$(python3 -c "import json; print(json.load(open('$CASES_FILE'))['sot_id'])")
SOT_FILE="$REPO_ROOT/artifacts/kloc-dev/$SOT_ID/sot.json"

if [[ ! -f "$SOT_FILE" ]]; then
    echo "Error: sot.json not found at $SOT_FILE"
    echo "Run: ./kloc-dev.sh --id=$SOT_ID  (to generate artifacts first)"
    exit 1
fi

# --- Helpers ---

run_case() {
    local symbol="$1"
    local depth="$2"
    local impl="$3"

    local args=(context "$symbol" --sot "$SOT_FILE" --depth "$depth" --json)
    if [[ "$impl" == "true" ]]; then
        args+=(--impl)
    fi

    cd "$CLI_DIR"
    uv run kloc-cli "${args[@]}" 2>/dev/null
}

run_all_cases() {
    # Outputs a JSON object: { "case_name": <output>, ... }
    local cases
    cases=$(python3 -c "
import json, sys
with open('$CASES_FILE') as f:
    data = json.load(f)
for c in data['cases']:
    print(json.dumps(c))
")

    local total
    total=$(echo "$cases" | wc -l | tr -d ' ')
    local current=0
    local failed=0

    echo "{"
    local first=true

    while IFS= read -r case_json; do
        current=$((current + 1))
        local name symbol depth impl
        name=$(echo "$case_json" | python3 -c "import json,sys; print(json.load(sys.stdin)['name'])")
        symbol=$(echo "$case_json" | python3 -c "import json,sys; print(json.load(sys.stdin)['symbol'])")
        depth=$(echo "$case_json" | python3 -c "import json,sys; print(json.load(sys.stdin)['depth'])")
        impl=$(echo "$case_json" | python3 -c "import json,sys; print(str(json.load(sys.stdin).get('impl', False)).lower())")

        echo "  Running [$current/$total] $name..." >&2

        local output
        if output=$(run_case "$symbol" "$depth" "$impl" 2>&1); then
            # Validate it's valid JSON
            if echo "$output" | python3 -c "import json,sys; json.load(sys.stdin)" 2>/dev/null; then
                if [[ "$first" == "true" ]]; then
                    first=false
                else
                    echo ","
                fi
                echo -n "  \"$name\": "
                echo "$output" | python3 -c "import json,sys; json.dump(json.load(sys.stdin), sys.stdout)"
            else
                echo "  WARN: $name produced invalid JSON" >&2
                failed=$((failed + 1))
            fi
        else
            echo "  FAIL: $name exited with error" >&2
            failed=$((failed + 1))
        fi
    done <<< "$cases"

    echo ""
    echo "}"

    if [[ $failed -gt 0 ]]; then
        echo "  $failed/$total cases failed to produce output" >&2
    fi
    echo "  Completed $((total - failed))/$total cases" >&2
}

# --- Commands ---

cmd_capture() {
    local timestamp
    timestamp=$(date +%d%m%y%H%M)
    local snapshot_file="$SCRIPT_DIR/snapshot-$timestamp.json"

    echo "Capturing snapshot: $snapshot_file" >&2
    echo "SOT: $SOT_FILE" >&2
    echo "" >&2

    run_all_cases > "$snapshot_file"

    echo "" >&2
    echo "Snapshot saved: $snapshot_file" >&2

    # Count cases
    local count
    count=$(python3 -c "import json; print(len(json.load(open('$snapshot_file'))))")
    echo "Cases captured: $count" >&2
}

cmd_verify() {
    local snapshot_file="$1"

    # Resolve relative to tests/ dir if not absolute
    if [[ ! "$snapshot_file" = /* ]]; then
        if [[ -f "$SCRIPT_DIR/$snapshot_file" ]]; then
            snapshot_file="$SCRIPT_DIR/$snapshot_file"
        fi
    fi

    if [[ ! -f "$snapshot_file" ]]; then
        echo "Error: Snapshot file not found: $snapshot_file"
        exit 1
    fi

    echo "Verifying against: $snapshot_file" >&2
    echo "SOT: $SOT_FILE" >&2
    echo "" >&2

    local cases
    cases=$(python3 -c "
import json
with open('$CASES_FILE') as f:
    data = json.load(f)
for c in data['cases']:
    print(json.dumps(c))
")

    local snapshot
    snapshot=$(cat "$snapshot_file")

    local total passed failed errors
    total=$(echo "$cases" | wc -l | tr -d ' ')
    passed=0
    failed=0
    errors=0
    current=0

    while IFS= read -r case_json; do
        current=$((current + 1))
        local name symbol depth impl
        name=$(echo "$case_json" | python3 -c "import json,sys; print(json.load(sys.stdin)['name'])")
        symbol=$(echo "$case_json" | python3 -c "import json,sys; print(json.load(sys.stdin)['symbol'])")
        depth=$(echo "$case_json" | python3 -c "import json,sys; print(json.load(sys.stdin)['depth'])")
        impl=$(echo "$case_json" | python3 -c "import json,sys; print(str(json.load(sys.stdin).get('impl', False)).lower())")

        echo -n "  [$current/$total] $name... " >&2

        # Get expected from snapshot
        local expected
        expected=$(echo "$snapshot" | python3 -c "
import json, sys
snap = json.load(sys.stdin)
name = '$name'
if name in snap:
    print(json.dumps(snap[name], sort_keys=True))
else:
    print('MISSING')
" 2>/dev/null)

        if [[ "$expected" == "MISSING" ]]; then
            echo "SKIP (not in snapshot)" >&2
            continue
        fi

        # Run current
        local actual
        if actual=$(run_case "$symbol" "$depth" "$impl" 2>&1); then
            local actual_sorted
            actual_sorted=$(echo "$actual" | python3 -c "import json,sys; print(json.dumps(json.load(sys.stdin), sort_keys=True))" 2>/dev/null)

            if [[ "$actual_sorted" == "$expected" ]]; then
                echo "PASS" >&2
                passed=$((passed + 1))
            else
                echo "FAIL" >&2
                failed=$((failed + 1))
            fi
        else
            echo "ERROR (command failed)" >&2
            errors=$((errors + 1))
        fi
    done <<< "$cases"

    echo "" >&2
    echo "Results: $passed passed, $failed failed, $errors errors (out of $total)" >&2

    if [[ $failed -gt 0 || $errors -gt 0 ]]; then
        exit 1
    fi
}

cmd_diff() {
    local snapshot_file="$1"

    if [[ ! "$snapshot_file" = /* ]]; then
        if [[ -f "$SCRIPT_DIR/$snapshot_file" ]]; then
            snapshot_file="$SCRIPT_DIR/$snapshot_file"
        fi
    fi

    if [[ ! -f "$snapshot_file" ]]; then
        echo "Error: Snapshot file not found: $snapshot_file"
        exit 1
    fi

    echo "Diffing against: $snapshot_file" >&2
    echo "" >&2

    local cases
    cases=$(python3 -c "
import json
with open('$CASES_FILE') as f:
    data = json.load(f)
for c in data['cases']:
    print(json.dumps(c))
")

    local snapshot
    snapshot=$(cat "$snapshot_file")

    while IFS= read -r case_json; do
        local name symbol depth impl
        name=$(echo "$case_json" | python3 -c "import json,sys; print(json.load(sys.stdin)['name'])")
        symbol=$(echo "$case_json" | python3 -c "import json,sys; print(json.load(sys.stdin)['symbol'])")
        depth=$(echo "$case_json" | python3 -c "import json,sys; print(json.load(sys.stdin)['depth'])")
        impl=$(echo "$case_json" | python3 -c "import json,sys; print(str(json.load(sys.stdin).get('impl', False)).lower())")

        local expected
        expected=$(echo "$snapshot" | python3 -c "
import json, sys
snap = json.load(sys.stdin)
if '$name' in snap:
    print(json.dumps(snap['$name'], indent=2, sort_keys=True))
else:
    print('MISSING')
" 2>/dev/null)

        if [[ "$expected" == "MISSING" ]]; then
            continue
        fi

        local actual
        if actual=$(run_case "$symbol" "$depth" "$impl" 2>&1); then
            local actual_formatted
            actual_formatted=$(echo "$actual" | python3 -c "import json,sys; print(json.dumps(json.load(sys.stdin), indent=2, sort_keys=True))" 2>/dev/null)

            if [[ "$actual_formatted" != "$expected" ]]; then
                echo "=== DIFF: $name ==="
                diff <(echo "$expected") <(echo "$actual_formatted") || true
                echo ""
            fi
        fi
    done <<< "$cases"
}

cmd_list() {
    echo "Available snapshots:"
    ls -1 "$SCRIPT_DIR"/snapshot-*.json 2>/dev/null | while read -r f; do
        local count
        count=$(python3 -c "import json; print(len(json.load(open('$f'))))" 2>/dev/null || echo "?")
        echo "  $(basename "$f")  ($count cases)"
    done
}

# --- Main ---

case "${1:-}" in
    capture)
        cmd_capture
        ;;
    verify)
        if [[ -z "${2:-}" ]]; then
            echo "Usage: $0 verify <snapshot-file>"
            exit 1
        fi
        cmd_verify "$2"
        ;;
    diff)
        if [[ -z "${2:-}" ]]; then
            echo "Usage: $0 diff <snapshot-file>"
            exit 1
        fi
        cmd_diff "$2"
        ;;
    list)
        cmd_list
        ;;
    *)
        echo "Usage: $0 {capture|verify|diff|list}"
        echo ""
        echo "Commands:"
        echo "  capture             Run all cases, save snapshot as snapshot-DDMMYYHHMM.json"
        echo "  verify <snapshot>   Run all cases, compare against snapshot"
        echo "  diff <snapshot>     Show JSON diffs for failing cases"
        echo "  list                List available snapshots"
        exit 1
        ;;
esac
