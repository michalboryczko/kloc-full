# Progress: Calls Tracking v4 Implementation

**Feature**: calls-tracking-v4
**Issues file**: docs/feature-issues/calls-tracking-issues-v4.md
**Reference**: docs/reference/kloc-scip/calls-and-data-flow.md
**Started**: 2026-01-30
**Completed**: 2026-01-30

## Overview

This iteration implements the v4 fixes:
1. Fix promoted constructor property type resolution
2. Create result values for all calls (intermediate results)

## Implementation Steps

### Step 1: Fix promoted constructor property types in Types.php
**Status**: done
**Description**: In Types.php around line 651-660, the promoted constructor property type collection creates a synthetic PropertyItem but doesn't properly set its parent to the ClassLike. Use Option B from issues doc - directly use nameProp with the class symbol.
**Files**: scip-php/src/Types/Types.php

### Step 1b: Fix nullable builtin types in TypeParser
**Status**: done
**Description**: TypeParser.parse() was returning null for nullable builtin types like ?string. Added logic to convert builtin Identifier to NamedType when inside NullableType.
**Files**: scip-php/src/Types/Internal/TypeParser.php

### Step 2: Add 'result' kind to ValueRecord
**Status**: done
**Description**: Add 'result' to the valid kinds in ValueRecord class. Result values represent the output of a call.
**Files**: scip-php/src/Calls/ValueRecord.php

### Step 3: Create result values for each call in DocIndexer
**Status**: done
**Description**: After creating each CallRecord, also create a corresponding ValueRecord with kind='result', same ID as the call, type from return_type, and source_call_id pointing to itself. This applies to method calls, property access, constructors, functions, and operators.
**Files**: scip-php/src/DocIndexer.php

### Step 4: Update tests for result values
**Status**: skipped (per user instructions)
**Description**: Update existing CallsWriter tests and add new tests to verify result values are created for calls.
**Files**: scip-php/tests/Calls/*.php

### Step 5: Build and validate indexer
**Status**: done
**Description**: Build the indexer and run on test codebase to validate changes work correctly. Check that promoted constructor property access has return_type and that result values are created.
**Result**: Build successful. All validation checks pass:
- Total values: 7490
- Total calls: 3489
- Result values: 3489 (100% match)
- All receiver_value_id references valid
- All promoted properties have return_type

### Step 6: Generate evidence file
**Status**: done
**Description**: Create evidence file with 60+ real examples across 20+ cases showing all requirements are met.
**Files**: docs/feature-issues/calls-tracking-v4-evidence.md
**Result**: 92 examples across 24 cases, 4076 lines

## Commits

| Repo | Commit | Description |
|------|--------|-------------|
| scip-php | d2e58e8 | Checkpoint before v4 implementation |
| scip-php | 796f1b7 | Fix nullable builtin types in TypeParser |

## Final Statistics

| Metric | Count |
|--------|-------|
| Total values | 7490 |
| Total calls | 3489 |
| Result values | 3489 |
| Local values | 1663 |
| Parameters | 581 |
| Literals | 1447 |
| Constants | 310 |
| Method calls | 1801 |
| Property access | 951 |
| Constructors | 388 |
| Functions | 221 |
