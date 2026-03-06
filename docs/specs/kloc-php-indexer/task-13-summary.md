# Task 13 Summary: Expression Tracking & Call Records

**Status:** COMPLETE
**Date:** 2026-03-06

## What Was Implemented

### src/indexing/calls.rs (NEW, ~300 lines incl. tests)

- **CallKind** enum (5 variants): MethodCall, NullsafeMethodCall, StaticCall, FuncCall, New — serde snake_case
- **ValueKind** enum (7 variants): PropertyRead, PropertyWrite, StaticPropertyRead, StaticPropertyWrite, ClassConstRead, ArrayDimRead, ArrayDimWrite
- **ArgumentValue** tagged union (15 variants): StringLiteral, IntLiteral, FloatLiteral, BoolLiteral, NullLiteral, Variable, ClassConst, StaticPropertyFetch, PropertyFetch, MethodCall, StaticCall, FuncCall, New, ArrayLiteral (recursive), Unknown
- **CallRecord**: caller, callee, kind: CallKind, file, line: u32, arguments: Vec<ArgumentValue> (skip_serializing_if empty)
- **ValueRecord**: source, target, kind: ValueKind, file, line: u32
- **CallsOutput**: top-level calls.json structure with calls + values vectors
- 14 unit tests for serialization roundtrips

### src/indexing/expression_tracker.rs (NEW, ~1570 lines incl. tests)

- **ExpressionTracker** struct with call_records and value_records accumulators
- **track_method_call()** — member_call_expression + nullsafe → CallRecord with MethodCall/NullsafeMethodCall
- **track_static_call()** — scoped_call_expression → CallRecord with StaticCall, handles self/static/parent
- **track_func_call()** — function_call_expression → CallRecord with FuncCall, skips variable calls
- **track_new_call()** — object_creation_expression → CallRecord with New, callee is __construct
- **track_property_access()** — member_access_expression → ValueRecord with PropertyRead/PropertyWrite
- **track_static_property_access()** — scoped_property_access_expression → ValueRecord with StaticPropertyRead/Write
- **track_class_const_access()** — class_constant_access_expression → ValueRecord with ClassConstRead, skips ::class
- **extract_arguments()** — iterates argument list, unwraps named_argument, skips spread
- **track_value()** — 14-case dispatch matching PHP ExpressionTracker::trackValue()
- **is_write_context()** — parent node assignment_expression LHS check
- **strip_string_quotes()** — strips surrounding single/double quotes
- **parse_php_int()** — handles hex, octal, binary, numeric separators
- Methods take individual references to avoid borrow checker conflicts with &mut IndexingContext
- 38 unit tests total (15 from Dev-1 + 23 from Dev-2)

### src/indexing/context.rs (updated)

- Changed import from `output::calls` to `indexing::calls` for new CallRecord/ValueRecord
- Added `pub expression_tracker: ExpressionTracker` field to IndexingContext
- Initialized in `new()` with `ExpressionTracker::new()`

### src/indexing/mod.rs (updated)

- Added `pub mod calls;` and `pub mod expression_tracker;`
- Wired ExpressionTracker calls into enter_node for: MethodCall, StaticCall, FuncCall, New, PropertyFetch, StaticPropertyFetch, ClassConstFetch
- All dispatch passes individual fields (scope, var_types, type_db, resolver, namer, source) to avoid borrow checker issues

## Test Results
- 450 unit tests passed (398 from Tasks 01-12 + 52 new)
- 3 integration tests passed, 1 ignored
- Build: zero warnings, clean compile (debug + release)

## Key Design Decisions
- ExpressionTracker methods take individual references (not &IndexingContext) to avoid borrow checker conflicts
- Caller symbols built from scope's current_callable_fqn() — methods get SCIP method symbols, functions get SCIP function symbols
- Unknown receivers produce "unknown#methodName()." sentinel callee strings
- Dynamic/variable calls ($obj->$method(), $func()) silently skipped
- Lines are 1-indexed (tree-sitter row + 1)
- ::class pseudo-constant skipped (no ValueRecord)
- Write context detected by parent assignment_expression LHS check
- Arguments extraction uses track_value() for 14-case dispatch matching PHP
- Old src/output/calls.rs kept but imports redirected to new src/indexing/calls.rs
- Subtask 13.6 (snapshot parity check) skipped — no PHP binary available

## Parity Improvements
- Call tracking now covers: method calls, nullsafe calls, static calls, function calls, constructor (new) calls
- Value tracking covers: property read/write, static property read/write, class constant read
- Argument extraction covers all 14 PHP expression types
- Chained calls ($a->b()->c()) produce independent CallRecords per call site
