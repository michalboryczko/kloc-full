# Progress: Calls Tracking v3 Implementation

**Feature**: calls-tracking-v3
**Issues file**: docs/feature-issues/calls-tracking-issues-v3.md
**Reference**: docs/reference/kloc-scip/calls-and-data-flow.md
**Started**: 2026-01-29

## Overview

This iteration implements the v3 schema changes:
1. Split into separate `values` and `calls` arrays
2. Add `kind_type` to calls (invocation, access, operator)
3. Local variable unique symbols with `@line`
4. Assignment tracking via `source_call_id`/`source_value_id`

## Implementation Steps

### Step 1: Create ValueRecord class
**Status**: done
**Description**: Create new ValueRecord class for values (local, parameter, literal, constant) with id, kind, symbol, type, location, source_call_id, source_value_id fields.
**Files**: scip-php/src/Calls/ValueRecord.php

### Step 2: Update CallRecord class
**Status**: done
**Description**: Add kind_type field (invocation, access, operator). Rename receiver_id to receiver_value_id. Remove value kinds (variable, literal, constant) - these go to ValueRecord. Update property kinds to access, access_static, access_nullsafe.
**Files**: scip-php/src/Calls/CallRecord.php

### Step 3: Update ArgumentRecord class
**Status**: done
**Description**: Rename value_call_id to value_id. The field can reference either a value ID or a call ID (unique across both arrays).
**Files**: scip-php/src/Calls/ArgumentRecord.php

### Step 4: Update CallsWriter class
**Status**: done
**Description**: Update to version 3.0. Output both `values` and `calls` arrays. Accept both value and call records.
**Files**: scip-php/src/Calls/CallsWriter.php

### Step 5: Update Indexer class
**Status**: done
**Description**: Collect both values and calls separately. Pass both to CallsWriter.
**Files**: scip-php/src/Indexer.php

### Step 6: Update DocIndexer - separate value/call tracking
**Status**: done
**Description**: Track values and calls separately. Create value records for parameters, literals, constants, local variables. Create call records for method calls, property access, operators. Property access becomes a call (kind: access), not a value.
**Files**: scip-php/src/DocIndexer.php

### Step 7: Update DocIndexer - unique local symbols
**Status**: done
**Description**: Generate unique local variable symbols with scope and line: `{scope}.local${name}@{line}`. Each re-assignment gets a new symbol with different line number.
**Files**: scip-php/src/DocIndexer.php, scip-php/src/SymbolNamer.php

### Step 8: Update DocIndexer - assignment tracking
**Status**: done
**Description**: For local variable assignments, add source_call_id (assigned from call) or source_value_id (assigned from value). Track usages to reference the correct symbol based on definition line.
**Files**: scip-php/src/DocIndexer.php

### Step 9: Update tests
**Status**: done
**Description**: Update existing tests to match new schema. Add tests for new functionality (value/call separation, unique local symbols, assignment tracking).
**Files**: scip-php/tests/Calls/*.php, scip-php/tests/Indexer/CallsTrackingTest.php

### Step 10: Build and run indexer
**Status**: done
**Description**: Build the indexer and run on test codebase to validate changes work correctly.
**Result**: Build successful. Indexed /Users/michal/dev/mms/usynxissetup/app with 4001 values and 3489 calls.

### Step 11: Generate evidence file
**Status**: done
**Description**: Create evidence file with 60+ real examples across 20+ cases showing all requirements are met.
**Files**: docs/feature-issues/calls-tracking-v3-evidence.md
**Result**: 67 cases with 180+ individual entries demonstrating all requirements.

### Step 12: Commit changes
**Status**: done
**Description**: Commit all changes with descriptive message.
**Result**: scip-php commits (7eda448, 0c49463) and kloc commit (bed4309)

## Statistics

| Metric | Count |
|--------|-------|
| Total values | 4001 |
| Total calls | 3489 |
| Local values | 1663 |
| Parameters | 581 |
| Literals | 1447 |
| Constants | 310 |
| Method calls | 1801 |
| Property access | 951 |
| Constructors | 388 |
| Functions | 221 |
| source_call_id | 476 |
| source_value_id | 147 |
