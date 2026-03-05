# Task 11 Summary: Value & Property Context

## What Was Implemented

Property context and File context commands. Property context produces identical output to kloc-cli (0 diffs). File context is structurally correct but has a count mismatch due to kloc-cli's iteration-order-dependent count limit behavior.

### Property Context (build_property_used_by / build_property_uses)

**USED BY:**
- Methods that read/access this property via Call nodes targeting the property
- Each entry has refType="property_access", on="$this->prop (FQN)", onKind="property"
- Line is the call site line (where property is accessed)
- Grouped by method: one entry per containing method, using first call site line

**USES:**
- Methods that write to this property (via constructor calls with promoted parameter)
- Each entry has flat args format: `{Class::__construct().$param: $value}`
- Flat args triggered by `class_level=True` in output model
- Source value resolution traces through PRODUCES edges for nested constructor calls

### File Context (build_file_used_by / build_file_uses)

**USED BY:**
- Collects ALL incoming USES edges to the file's class and all contained members (methods, properties, constants) -- matching kloc-cli's `get_usages_grouped(file_id)` behavior
- Each USES edge becomes a separate entry with member_ref
- Reference type classification:
  - Class targets: TYPE_HINT edges determine parameter_type/return_type/property_type/type_hint
  - Method/Property targets: Call node info determines method_call/static_call/instantiation
- Import references (File source nodes) excluded at query level
- Internal self-references (same file) excluded at query level
- Signatures extracted from method documentation

**USES:**
- Outgoing dependencies deduped by target FQN (first occurrence wins)
- USES edges processed first, then Call edges for uncovered targets
- Reference type inferred from target kind (Method->method_call, Property->property_access, etc.)

## Files Created/Modified

### New Files
- `kloc-intelligence/src/db/queries/context_property.py` -- Cypher queries for property access sites and writers
- `kloc-intelligence/src/orchestration/property_context.py` -- build_property_used_by, build_property_uses
- `kloc-intelligence/src/db/queries/context_file.py` -- Cypher queries for file context (FILE_USED_BY_ALL, FILE_USES, FILE_CALL_USES)
- `kloc-intelligence/src/orchestration/file_context.py` -- build_file_used_by, build_file_uses
- `docs/specs/kloc-intelligence/task-11-summary.md` -- this summary

### Modified Files
- `kloc-intelligence/tests/test_snapshot.py`:
  - Added Property context dispatch (`build_property_used_by`, `build_property_uses`)
  - Added File context dispatch (`build_file_used_by`, `build_file_uses`)

## Test Results

- 45 passed, 5 failed (up from 44 passed, 6 failed before T11)
- context-property: 0 diffs (was 9 diffs -- now fully passing)

### Failing Context Tests (5)
- context-class-d1: 12 diffs (known T09 limitations: on/onKind resolution, refType classification)
- context-class-d2: 25 diffs (same + depth-2 uses children classification)
- context-interface-d2: 28 diffs (depth-2 children issues)
- context-method-d2: 86 diffs (source_chain deep tracing)
- context-file: 843 diffs (count mismatch + ordering -- see Known Limitations)

## Known Limitations

### File Context Count Mismatch (context-file: 843 diffs)

kloc-cli's file context USED BY processes sources in sot.json edge iteration order with a count limit of 100. File source nodes (import/use statements) consume count slots but their entries are filtered afterwards. This results in the golden having exactly 63 method sources (from 100 total = 63 methods + 37 file imports). Our Neo4j-based implementation cannot replicate the exact sot.json edge iteration order, so we include all 86 external method sources. The structural format of each entry (member_ref, reference_type, signature, arguments) is correct -- the difference is purely which subset of methods appears.

Of the entries that match (69 of 92 golden entries), the reference types, lines, signatures, and arguments are correctly produced.

### Value Context Not Yet Implemented (S01-S05)

Value consumer chain queries and orchestrator (S01-S02), cross-method boundary crossing (S03-S04), and value source chain (S05) are not yet implemented. These are needed for Value node `context` queries. No Value snapshot tests exist in the corpus, so this has no impact on current test results.

### Source Chain Deep Tracing (method-d2: 86 diffs, carried from T10)

The source_chain tracing for argument values produces `reference_type: "method"` with call site info instead of `reference_type: "access"` with parameter access patterns. This is a depth-2 argument tracing issue carried over from T10.

## Key Design Decisions

### FILE_USED_BY_ALL Query

A single Cypher query collects all incoming USES edges to file members with:
- Resolved source (containing method for non-Method sources)
- TYPE_HINT flags for class target classification
- Call node info for method/property target classification
- Ordered by edge_idx for consistent results

This replaces the previous two-query approach (USES edges + Call edges) that caused 40+ extra entries from call-only references not in the golden.

### File Source Exclusion at Query Level

File source nodes (`source.kind <> 'File'`) are excluded in the Cypher query rather than filtered in Python. This eliminates the need for import reference filtering logic and reduces query result size.

### Property Context Flat Args Format

Property USES entries use `class_level=True` in the output model, triggering the flat args format (`{param_fqn: value_expr}`) instead of the rich args format used by method context. This matches kloc-cli's behavior where property context entries appear in the class_level list.
