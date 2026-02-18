#!/bin/bash
set -euo pipefail

# Detailed snapshot testing for kloc-cli context command
# Comprehensive regression testing across all symbol types × depths × impl flag
#
# Usage:
#   ./tests/detailed/snapshot.sh generate          # Pick symbols, create cases.json
#   ./tests/detailed/snapshot.sh capture            # Run all cases, save snapshot
#   ./tests/detailed/snapshot.sh verify <snapshot>  # Compare current output vs snapshot
#   ./tests/detailed/snapshot.sh validate [snapshot] # Validate JSON against schema
#   ./tests/detailed/snapshot.sh summary [snapshot]  # Show pass/fail summary by category
#   ./tests/detailed/snapshot.sh diff <snapshot>    # Show diffs for failing cases
#   ./tests/detailed/snapshot.sh list               # List available snapshots

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
CLI_DIR="$REPO_ROOT/kloc-cli"
CASES_FILE="$SCRIPT_DIR/cases.json"
VALIDATE_SCRIPT="$REPO_ROOT/kloc-contracts/validate.py"
GENERATE_SCRIPT="$SCRIPT_DIR/generate_cases.py"

# Read sot_id from cases.json (or use default for generate)
get_sot_id() {
    if [[ -f "$CASES_FILE" ]]; then
        python3 -c "import json; print(json.load(open('$CASES_FILE'))['sot_id'])"
    else
        # Fallback: read from main cases.json
        python3 -c "import json; print(json.load(open('$SCRIPT_DIR/../cases.json'))['sot_id'])"
    fi
}

SOT_ID=$(get_sot_id)
ARTIFACT_DIR="$REPO_ROOT/artifacts/kloc-dev/$SOT_ID"
SOT_FILE="$ARTIFACT_DIR/sot.json"

regenerate_sot() {
    echo "Regenerating sot.json (full pipeline: index -> map)..." >&2
    rm -f "$ARTIFACT_DIR/index.json" "$ARTIFACT_DIR/sot.json"
    "$REPO_ROOT/kloc-dev.sh" --id="$SOT_ID" >&2
    echo "" >&2

    if [[ ! -f "$SOT_FILE" ]]; then
        echo "Error: sot.json not found at $SOT_FILE after regeneration"
        exit 1
    fi
}

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

        # Print progress every 10 cases or on category change
        if (( current % 10 == 1 )) || (( current == total )); then
            echo "  Running [$current/$total] $name..." >&2
        fi

        local output
        if output=$(run_case "$symbol" "$depth" "$impl" 2>&1); then
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

validate_case_output() {
    local name="$1"
    local json_data="$2"
    local tmpfile
    tmpfile=$(mktemp)
    echo "$json_data" > "$tmpfile"

    cd "$CLI_DIR"
    if uv run python3 "$VALIDATE_SCRIPT" kloc-cli-context "$tmpfile" >/dev/null 2>&1; then
        rm -f "$tmpfile"
        return 0
    else
        local errors
        errors=$(uv run python3 "$VALIDATE_SCRIPT" kloc-cli-context "$tmpfile" 2>&1 || true)
        rm -f "$tmpfile"
        echo "$errors" >&2
        return 1
    fi
}

# --- Commands ---

cmd_generate() {
    if [[ ! -f "$SOT_FILE" ]]; then
        regenerate_sot
    fi

    echo "Generating cases from $SOT_FILE..." >&2

    cd "$CLI_DIR"
    uv run python3 "$GENERATE_SCRIPT" "$SOT_FILE" --seed 42 --count 20

    # Show summary
    local total
    total=$(python3 -c "import json; print(len(json.load(open('$CASES_FILE'))['cases']))")
    echo "" >&2
    echo "Total cases: $total" >&2
}

cmd_capture() {
    regenerate_sot

    if [[ ! -f "$CASES_FILE" ]]; then
        echo "Error: cases.json not found. Run 'generate' first." >&2
        exit 1
    fi

    local timestamp
    timestamp=$(date +%d%m%y%H%M)
    local snapshot_file="$SCRIPT_DIR/snapshot-$timestamp.json"

    local total
    total=$(python3 -c "import json; print(len(json.load(open('$CASES_FILE'))['cases']))")

    echo "Capturing detailed snapshot: $snapshot_file" >&2
    echo "SOT: $SOT_FILE" >&2
    echo "Cases: $total" >&2
    echo "" >&2

    run_all_cases > "$snapshot_file"

    echo "" >&2
    echo "Snapshot saved: $snapshot_file" >&2

    local count
    count=$(python3 -c "import json; print(len(json.load(open('$snapshot_file'))))")
    echo "Cases captured: $count / $total" >&2
}

cmd_verify() {
    regenerate_sot

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

    local total passed failed errors schema_errors
    total=$(echo "$cases" | wc -l | tr -d ' ')
    passed=0
    failed=0
    errors=0
    schema_errors=0
    current=0

    # Track failures by category
    declare -A cat_passed cat_failed cat_errors
    local failed_names=""

    while IFS= read -r case_json; do
        current=$((current + 1))
        local name symbol depth impl category
        name=$(echo "$case_json" | python3 -c "import json,sys; print(json.load(sys.stdin)['name'])")
        symbol=$(echo "$case_json" | python3 -c "import json,sys; print(json.load(sys.stdin)['symbol'])")
        depth=$(echo "$case_json" | python3 -c "import json,sys; print(json.load(sys.stdin)['depth'])")
        impl=$(echo "$case_json" | python3 -c "import json,sys; print(str(json.load(sys.stdin).get('impl', False)).lower())")
        category=$(echo "$case_json" | python3 -c "import json,sys; print(json.load(sys.stdin).get('category', 'unknown'))")

        if (( current % 50 == 1 )); then
            echo "  [$current/$total] $name..." >&2
        fi

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
            continue
        fi

        local actual
        if actual=$(run_case "$symbol" "$depth" "$impl" 2>&1); then
            local actual_sorted
            actual_sorted=$(echo "$actual" | python3 -c "import json,sys; print(json.dumps(json.load(sys.stdin), sort_keys=True))" 2>/dev/null)

            if [[ "$actual_sorted" == "$expected" ]]; then
                if validate_case_output "$name" "$actual" 2>/dev/null; then
                    passed=$((passed + 1))
                    cat_passed[$category]=$(( ${cat_passed[$category]:-0} + 1 ))
                else
                    echo "  SCHEMA FAIL: $name" >&2
                    schema_errors=$((schema_errors + 1))
                    passed=$((passed + 1))
                    cat_passed[$category]=$(( ${cat_passed[$category]:-0} + 1 ))
                fi
            else
                echo "  DIFF FAIL: $name" >&2
                failed=$((failed + 1))
                cat_failed[$category]=$(( ${cat_failed[$category]:-0} + 1 ))
                failed_names="$failed_names\n  $name"
            fi
        else
            echo "  ERROR: $name (command failed)" >&2
            errors=$((errors + 1))
            cat_errors[$category]=$(( ${cat_errors[$category]:-0} + 1 ))
        fi
    done <<< "$cases"

    echo "" >&2
    echo "=== Results ===" >&2
    echo "  Total: $total | Passed: $passed | Failed: $failed | Errors: $errors" >&2
    if [[ $schema_errors -gt 0 ]]; then
        echo "  Schema validation failures: $schema_errors" >&2
    fi

    # Per-category summary
    echo "" >&2
    echo "=== By Category ===" >&2
    for cat in class interface method property value-parameter value-local; do
        local cp=${cat_passed[$cat]:-0}
        local cf=${cat_failed[$cat]:-0}
        local ce=${cat_errors[$cat]:-0}
        local ct=$((cp + cf + ce))
        if [[ $ct -gt 0 ]]; then
            echo "  $cat: $cp/$ct passed" >&2
            if [[ $cf -gt 0 ]]; then
                echo "    $cf failed" >&2
            fi
        fi
    done

    if [[ -n "$failed_names" ]]; then
        echo "" >&2
        echo "=== Failed Cases ===" >&2
        echo -e "$failed_names" >&2
    fi

    if [[ $failed -gt 0 || $errors -gt 0 ]]; then
        exit 1
    fi
}

cmd_validate() {
    local source="live"
    local snapshot_file=""

    if [[ -n "${1:-}" ]]; then
        snapshot_file="$1"
        source="snapshot"

        if [[ ! "$snapshot_file" = /* ]]; then
            if [[ -f "$SCRIPT_DIR/$snapshot_file" ]]; then
                snapshot_file="$SCRIPT_DIR/$snapshot_file"
            fi
        fi

        if [[ ! -f "$snapshot_file" ]]; then
            echo "Error: Snapshot file not found: $snapshot_file"
            exit 1
        fi
    fi

    if [[ "$source" == "live" ]]; then
        regenerate_sot
    fi

    echo "Validating against kloc-cli-context schema (source: $source)" >&2
    echo "" >&2

    local cases
    cases=$(python3 -c "
import json
with open('$CASES_FILE') as f:
    data = json.load(f)
for c in data['cases']:
    print(json.dumps(c))
")

    local total passed failed errors
    total=$(echo "$cases" | wc -l | tr -d ' ')
    passed=0
    failed=0
    errors=0
    current=0

    local snapshot=""
    if [[ "$source" == "snapshot" ]]; then
        snapshot=$(cat "$snapshot_file")
    fi

    while IFS= read -r case_json; do
        current=$((current + 1))
        local name symbol depth impl
        name=$(echo "$case_json" | python3 -c "import json,sys; print(json.load(sys.stdin)['name'])")
        symbol=$(echo "$case_json" | python3 -c "import json,sys; print(json.load(sys.stdin)['symbol'])")
        depth=$(echo "$case_json" | python3 -c "import json,sys; print(json.load(sys.stdin)['depth'])")
        impl=$(echo "$case_json" | python3 -c "import json,sys; print(str(json.load(sys.stdin).get('impl', False)).lower())")

        if (( current % 50 == 1 )); then
            echo "  [$current/$total] $name..." >&2
        fi

        local json_output=""

        if [[ "$source" == "snapshot" ]]; then
            json_output=$(echo "$snapshot" | python3 -c "
import json, sys
snap = json.load(sys.stdin)
name = '$name'
if name in snap:
    print(json.dumps(snap[name]))
else:
    print('MISSING')
" 2>/dev/null)

            if [[ "$json_output" == "MISSING" ]]; then
                continue
            fi
        else
            if json_output=$(run_case "$symbol" "$depth" "$impl" 2>&1); then
                :
            else
                echo "  ERROR: $name (command failed)" >&2
                errors=$((errors + 1))
                continue
            fi
        fi

        if validate_case_output "$name" "$json_output" 2>/dev/null; then
            passed=$((passed + 1))
        else
            echo "  SCHEMA FAIL: $name" >&2
            failed=$((failed + 1))
        fi
    done <<< "$cases"

    echo "" >&2
    echo "Schema validation: $passed passed, $failed failed, $errors errors (out of $total)" >&2

    if [[ $failed -gt 0 || $errors -gt 0 ]]; then
        exit 1
    fi
}

cmd_summary() {
    local snapshot_file="${1:-}"

    if [[ -z "$snapshot_file" ]]; then
        # Find latest snapshot
        snapshot_file=$(ls -t "$SCRIPT_DIR"/snapshot-*.json 2>/dev/null | head -1)
        if [[ -z "$snapshot_file" ]]; then
            echo "Error: No snapshots found. Run 'capture' first."
            exit 1
        fi
    else
        if [[ ! "$snapshot_file" = /* ]]; then
            if [[ -f "$SCRIPT_DIR/$snapshot_file" ]]; then
                snapshot_file="$SCRIPT_DIR/$snapshot_file"
            fi
        fi
    fi

    echo "Snapshot: $(basename "$snapshot_file")" >&2
    echo "" >&2

    python3 -c "
import json, sys
from collections import defaultdict

with open('$CASES_FILE') as f:
    cases_data = json.load(f)

with open('$snapshot_file') as f:
    snapshot = json.load(f)

# Stats by category
by_cat = defaultdict(lambda: {'total': 0, 'captured': 0, 'depths': set(), 'symbols': set()})

for case in cases_data['cases']:
    cat = case.get('category', 'unknown')
    by_cat[cat]['total'] += 1
    by_cat[cat]['depths'].add(case['depth'])
    by_cat[cat]['symbols'].add(case['symbol'])
    if case['name'] in snapshot:
        by_cat[cat]['captured'] += 1

print(f'Total cases: {len(cases_data[\"cases\"])}')
print(f'Captured in snapshot: {len(snapshot)}')
print()
print(f'{\"Category\":<20} {\"Symbols\":<10} {\"Cases\":<10} {\"Captured\":<10} {\"Rate\":<10}')
print('-' * 60)
for cat in sorted(by_cat.keys()):
    s = by_cat[cat]
    rate = f\"{s['captured']/s['total']*100:.0f}%\" if s['total'] > 0 else 'N/A'
    print(f'{cat:<20} {len(s[\"symbols\"]):<10} {s[\"total\"]:<10} {s[\"captured\"]:<10} {rate:<10}')
"
}

cmd_diff() {
    regenerate_sot

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

    local diff_count=0

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
                diff_count=$((diff_count + 1))
            fi
        fi
    done <<< "$cases"

    if [[ $diff_count -eq 0 ]]; then
        echo "No diffs found." >&2
    else
        echo "$diff_count case(s) have differences." >&2
    fi
}

cmd_list() {
    echo "Available detailed snapshots:"
    ls -1 "$SCRIPT_DIR"/snapshot-*.json 2>/dev/null | while read -r f; do
        local count
        count=$(python3 -c "import json; print(len(json.load(open('$f'))))" 2>/dev/null || echo "?")
        echo "  $(basename "$f")  ($count cases)"
    done
}

# --- Main ---

case "${1:-}" in
    generate)
        cmd_generate
        ;;
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
    validate)
        cmd_validate "${2:-}"
        ;;
    summary)
        cmd_summary "${2:-}"
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
        echo "Usage: $0 {generate|capture|verify|validate|summary|diff|list}"
        echo ""
        echo "Commands:"
        echo "  generate              Analyze sot.json, pick random symbols, create cases.json"
        echo "  capture               Run all cases, save snapshot as snapshot-DDMMYYHHMM.json"
        echo "  verify <snapshot>     Run all cases, compare against snapshot (+ schema validation)"
        echo "  validate [snapshot]   Validate output against kloc-cli-context contract schema"
        echo "  summary [snapshot]    Show case counts and capture rates by category"
        echo "  diff <snapshot>       Show JSON diffs for failing cases"
        echo "  list                  List available snapshots"
        exit 1
        ;;
esac
