# Task 12 Summary: Context Orchestrator & Wiring

## What Was Implemented

Central `ContextOrchestrator` class that provides kind-based dispatch to all specialized context builders, replacing the inline dispatch logic in test_snapshot.py. The orchestrator routes context queries to the appropriate handler based on target node kind.

### ContextOrchestrator (S01 - Context Dispatch)

- `execute(node_id, depth, limit, include_impl)` -> ContextResult
- `execute_symbol(symbol, depth, limit, include_impl)` -> ContextResult
- `_dispatch_used_by(target, depth, limit, include_impl)` - kind-based routing for USED BY
- `_dispatch_uses(target, depth, limit, include_impl)` - kind-based routing for USES

**Dispatch routing:**
- Class -> `build_class_used_by` / `build_class_uses`
- Interface -> `build_interface_used_by` / `build_interface_uses`
- Method -> `build_method_used_by` / `build_method_uses` (with `__construct` redirect to Class)
- Property -> `build_property_used_by` / `build_property_uses`
- File -> `build_file_used_by` / `build_file_uses`
- Enum, Trait, Value, Constant, Function -> `build_generic_used_by` / `build_generic_uses`

**Constructor redirect (ISSUE-A):** When a `__construct` method is queried, its USED BY is redirected to the containing Class's USED BY handler, matching kloc-cli behavior.

### Test Wiring

The `test_snapshot.py` context command handler was simplified from ~40 lines of inline kind-switching to a clean orchestrator call:

```python
orchestrator = ContextOrchestrator(runner)
result = orchestrator.execute_symbol(symbol, depth=depth, limit=limit, include_impl=include_impl)
output = ContextOutput.from_result(result)
return output.to_dict()
```

### S02-S03: Generic USED BY / USES

Already implemented in previous tasks as `generic_context.py`. No changes needed.

### S04: Deferred Callbacks

Not needed for the current test suite. The circular dependency between `build_execution_flow` and `get_implementations_for_node` only manifests with `include_impl=True` for polymorphic analysis. The existing method_context module handles depth-2 expansion without circular imports. Deferred callbacks can be wired when polymorphic `--impl` support is added.

### S05: Output Pipeline

Already implemented in `models/output.py` as `ContextOutput.from_result().to_dict()`. No changes needed.

### S06: CLI Command

Deferred to T13 (CLI Interface). The orchestrator is ready to be wired into a CLI command.

### S07: Full Snapshot Suite

The existing snapshot suite covers 14 context test cases across 8 node kinds (Class, Interface, Method, Property, Enum, Const, Trait, File). Additional test cases can be added as coverage expands.

## Files Created/Modified

### New Files
- `kloc-intelligence/src/orchestration/context.py` -- ContextOrchestrator with kind-based dispatch
- `docs/specs/kloc-intelligence/task-12-summary.md` -- this summary

### Modified Files
- `kloc-intelligence/src/orchestration/__init__.py` -- exports ContextOrchestrator
- `kloc-intelligence/tests/test_snapshot.py` -- replaced inline dispatch with ContextOrchestrator call

## Test Results

- 45 passed, 5 failed (no regression from T11)

### Passing Context Tests (9)
- context-interface-d1, context-method-d1, context-property, context-enum, context-const, context-trait, context-small-class, context-factory, context-event

### Failing Context Tests (5, carried from T9-T11)
- context-class-d1: 12 diffs (on/onKind resolution, refType classification)
- context-class-d2: 25 diffs (same + depth-2 uses children classification)
- context-interface-d2: 28 diffs (depth-2 children issues)
- context-method-d2: 86 diffs (source_chain deep tracing)
- context-file: 863 diffs (count mismatch + ordering)

## Key Design Decisions

### NodeData Instead of Custom ResolvedNode
The orchestrator uses `NodeData` (the existing node model) directly from `resolve_symbol()` and `records_to_nodes()`, avoiding the creation of a redundant `ResolvedNode` type. This keeps the type hierarchy clean.

### Lazy Imports for Specialized Handlers
Specialized context handlers are imported inside dispatch methods (`if kind == "Class": from .class_context import ...`) to avoid circular import issues and to keep module loading lightweight.

### Orchestrator as Single Entry Point
All context queries now flow through `ContextOrchestrator`, making it the single point for future enhancements (deferred callbacks, caching, logging).
