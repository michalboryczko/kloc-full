# Task 10 Summary: Interface/Method Context

## What Was Implemented

Interface and Method context commands with full USED BY and USES sections. Interface context builds implementor lists with override methods, injection points with contract relevance filtering, and method signature types. Method context builds caller chains with rich arguments (source chains, value types, parameter mappings) and execution flow (local variables, direct calls, consumed call exclusion).

### Interface Context (build_interface_used_by / build_interface_uses)

**USED BY:**
- Implementors with `[implements]` refType
- Injection points (properties typed to interface) with `[property_type]` refType
- Contract relevance filtering via `check_contract_relevance()`
- Depth-2: override methods for implementors, method calls through injection points with flat args
- Injection points from both direct TYPE_HINT and concrete implementor types

**USES:**
- Parent interface with `[extends]` refType
- Method signature types with `[parameter_type]` / `[return_type]` refType
- Dedup rules: parameter_type wins over return_type
- Depth-2: recursive class-level USES expansion

### Method Context (build_method_used_by / build_method_uses)

**USED BY:**
- Callers of the method via Call -> CALLS -> Method edges
- Each caller has: signature (from documentation), member_ref (target_name, reference_type from call_kind), rich arguments
- Rich argument format: position, param_name, value_expr, value_source, value_type, param_fqn, value_ref_symbol, source_chain
- Source chain for result-type arguments: traces the producing call (constructor vs method)
- Promoted property resolution: `Estate::$id` instead of `Estate::__construct().$id`
- Call site line used for entry line (not caller method start line)
- Depth-2: callers of the caller (same format recursively)

**USES (execution flow):**
- Kind 1 (local_variable): Call produces result assigned to local variable. Entry has variable_name, variable_symbol, variable_type, source_call (nested entry with signature, member_ref, arguments)
- Kind 2 (direct call): Call with no local assignment. Entry has member_ref (with access_chain, on_kind, on_file, on_line), arguments, signature
- Consumed calls excluded from top-level (calls whose result feeds into another call as receiver or argument)
- Depth-2: execution flow of the callee method

### Injection Point Args

Added flat argument fetching to `_build_injection_point_children` in class_context.py. Injection point method_call depth-2 entries now include args (e.g., `{"EstateRepository::get().$id": "$estateId"}`).

### Snapshot Comparison Fix

Updated `_entry_sort_key` to detect method context entries (via `entry_type` field) and sort by line number instead of FQN, preserving execution flow order.

## Files Created/Modified

### New Files
- `kloc-intelligence/src/orchestration/method_context.py` -- build_method_used_by, build_method_uses with full argument resolution, receiver info, depth-2 expansion
- `docs/specs/kloc-intelligence/task-10-summary.md` -- this summary

### Modified Files
- `kloc-intelligence/src/db/queries/context_method.py`:
  - Added METHOD_CALL_ARGUMENTS query for rich argument info (value_type, source callee, value_fqn)
  - Added ARGUMENT_SOURCE_CHAIN query for deep source chain tracing
  - Added fetch_method_call_arguments() helper
  - Enhanced METHOD_CALLS query with recv.fqn, recv.file, recv.start_line for on_file/on_line
- `kloc-intelligence/src/db/queries/context_interface.py`:
  - All 7 queries and helper functions (no changes from initial creation)
- `kloc-intelligence/src/orchestration/interface_context.py`:
  - Fixed injection point children to pass prop_id instead of class_id
- `kloc-intelligence/src/orchestration/class_context.py`:
  - Added flat argument fetching to `_build_injection_point_children`
- `kloc-intelligence/tests/snapshot_compare.py`:
  - Updated `_entry_sort_key` to sort method context entries by line (preserves execution flow order)

## Test Results

- 44 passed, 6 failed (up from 42 passed, 8 failed before T10)
- All pre-existing tests pass (no regressions)

### Newly Passing Context Tests (2)
- context-interface-d1: 0 diffs
- context-method-d1: 0 diffs

### Failing Context Tests (6)
- context-class-d1: 12 diffs (known T09 limitations: on/onKind resolution, refType classification)
- context-class-d2: 25 diffs (same + depth-2 uses children classification)
- context-interface-d2: 28 diffs (injection point args values, sites ordering, depth-2 uses children)
- context-method-d2: 86 diffs (source_chain deep tracing -- reference_type "access" vs "method", on/on_kind resolution for parameter access chains)
- context-property: 9 diffs (T11 scope: property-specific context builder needed)
- context-file: 2 diffs (T11 scope: file context builder needed)

## Known Limitations

### Source Chain Deep Tracing (method-d2: 86 diffs)
kloc-cli's `trace_source_chain()` follows PRODUCES/CALLS/RECEIVER chains deeply to resolve parameter access patterns like `$input->getName()`. Our implementation only traces the immediate PRODUCES edge, producing `reference_type: "method"` with call site info instead of `reference_type: "access"` with parameter info. All 86 diffs are source_chain-related; the structural format is correct.

### Call Line Numbering Convention
Method USED BY entries use the call site line (from Call node start_line). kloc-cli outputs these as 0-based values without +1 conversion. To match, ContextEntry stores `call_line - 1` so OutputEntry's +1 produces the raw call_line value.

### Depth-2 USES Children Classification (interface-d2)
NODE_DEPS query classifies depth-2 uses children as `type_hint` instead of `parameter_type`/`return_type`/`method_call`. This is the same limitation documented in T09 summary.

## Key Design Decisions

### Rich vs Flat Argument Format
Method-level context uses rich arguments (list with source_chain, value_ref_symbol). Class-level context uses flat args (dict of param_key -> value_expr). The output model (`OutputEntry.from_entry`) handles this distinction via the `class_level` flag.

### Execution Flow Order
Method USES entries preserve line-number order (execution flow). The snapshot comparator now detects method context entries (via `entry_type` field) and sorts by `(file, line)` instead of `(fqn, refType)` for stable comparison.

### Call Site vs Method Definition Lines
For USED BY, entry `line` and `member_ref.line` use the call site (where the target is called). For USES, entry `line` uses the call site within the method. Both conventions match kloc-cli behavior.
