# Progress: Calls Tracking Fixes (v2)

**Feature**: calls-tracking-fixes
**Issues file**: docs/feature-issues/calls-tracing-issues-v2.md
**Started**: 2026-01-29

## Implementation Steps

### Step 1: Fix duplicate call IDs in property chains
**Status**: done
**Description**: Add `$positionNode` parameter to `buildExpressionCallRecord()` and use property name position for property accesses.
**Files**: scip-php/src/DocIndexer.php

### Step 2: Register parameter types in localVars
**Status**: done
**Description**: Register method/function parameter types when processing Param nodes so type resolution works for parameter-based expressions.
**Files**: scip-php/src/DocIndexer.php, scip-php/src/Types/Types.php

### Step 3: Build and test indexer
**Status**: done
**Description**: Build the indexer and run on test codebase to validate fixes.
**Result**: Build successful, indexed 6628 calls from /Users/michal/dev/mms/usynxissetup/app

### Step 4: Orchestrator validation with evidence
**Status**: done
**Description**: Main orchestrator produces evidence file with 60+ examples across 20+ cases showing fixes work correctly.
**Result**: Evidence file created at docs/feature-issues/calls-tracking-evidence.md with 65 examples across 22 cases.

### Step 5: Commit changes
**Status**: done
**Description**: Commit all changes with descriptive message.
**Result**: Committed in scip-php (375b8ef) and kloc (7a24675)
