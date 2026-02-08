# Implementation Plan: Context Command Fix (cli-context-fix)

**Date:** 2026-02-08
**Branch:** feature/scip-php-indexer-issues
**Scope:** kloc-cli only (no changes to scip-php, kloc-mapper, or kloc-contracts)

## Overview

### Problem Statement

The `kloc-cli context` command has 6 issues in its USES output direction. The data pipeline (scip-php -> kloc-mapper -> sot.json) already produces all required data (Value nodes, Call nodes, argument edges, produces edges, etc.), but the CLI does not use it. All fixes are in `kloc-cli/`.

### Issues Being Fixed

| # | Issue | Root Cause | Phase |
|---|-------|-----------|-------|
| 1 | Constructor calls shown as `[type_hint]` | `find_call_for_usage()` fails to find constructor Call nodes | 1 |
| 3 | No distinction between parameter types and return types | `_infer_reference_type()` returns `type_hint` for all | 1 |
| 4 | No argument tracking displayed | `get_arguments()` exists but never called by context query | 2 |
| 2 | No local variable references | Context query only traverses `uses` edges, ignores Value nodes | 3 |
| 6 | Missing value flow context | Call/Value graph traversal not used for execution flow | 3 |
| 5 | Redundant information in deep levels | Depth 2+ shows structural deps instead of call-chain-relevant data | 4 |

### Constraints

- Backward compatibility: existing JSON output structure must not break. New fields are additive.
- MCP server must pass through all new data (currently lossy -- drops member_ref entirely).
- USED BY direction is not changed (already working well with R1-R8 rules).
- Existing tests must continue to pass.

---

## Codebase Summary

### Architecture (5 layers)

```
CLI entry (src/cli.py)
  -> ContextQuery (src/queries/context.py)
    -> SoTIndex (src/graph/index.py)
  -> Output formatters (src/output/tree.py)
  -> MCP server (src/server/mcp.py)
```

### Key Patterns

- **Query pattern**: All queries extend `Query[T]` base class, take a `SoTIndex`, return typed result dataclass.
- **Tree building**: BFS with depth tracking, visited sets for dedup, per-parent vs global dedup depending on direction.
- **Reference type resolution**: Two-tier: first try Call node lookup (`find_call_for_usage` -> `get_reference_type_from_call`), then fall back to `_infer_reference_type()` from edge/node kinds.
- **Access chain building**: Traverse receiver edges from Call -> Value nodes, building "$this->property->method()" chains.
- **Output duality**: Every result has both Rich console formatter (`print_*`) and JSON serializer (`*_to_dict`). MCP server has its own inline serializer.

### Existing Graph API (available but unused by context query)

From `src/graph/index.py`:
- `get_arguments(call_node_id)` -> `list[tuple[str, int]]` (value_node_id, position)
- `get_produces(call_node_id)` -> `Optional[str]` (result Value node ID)
- `get_assigned_from(value_node_id)` -> `Optional[str]` (source Value node ID)
- `get_type_of(value_node_id)` -> `Optional[str]` (type Class/Interface node ID)

---

## Technical Approach

### Phase 1: Fix Reference Types (Issues 1 + 3)

**Approach:** Targeted fixes to existing functions. No new dataclasses. No structural changes.

#### 1a. Fix constructor reference type (Issue 1)

**Problem:** `find_call_for_usage()` searches `get_calls_to(target_id)` for Call nodes targeting the Class, but constructor Calls target `__construct()`, not the Class. The existing `_call_matches_target()` helper handles constructor-to-class mapping, but the Call node isn't found in the search path.

**Fix in `find_call_for_usage()` (context.py:177-222):**

In the location-based matching loop (lines 198-209), the function already checks `call_children` (Call nodes contained by the source method). The issue is that `get_calls_to(target_id)` for a Class ID returns nothing because constructor Calls target `__construct()`.

The fix: After the existing location-based loop fails to find a match, add a fallback that searches `call_children` for constructor Call nodes whose containing class matches `target_id`:

```python
# After the location-based matching loop (line 209):
# Constructor fallback: search call_children for constructor calls
# whose callee's containing class matches the target_id
if file and line is not None:
    for call_id in call_children:
        call_node = index.nodes.get(call_id)
        if call_node and call_node.file == file:
            if call_node.range:
                call_line = call_node.range.get("start_line", -1)
                if call_line == line:
                    if _call_matches_target(index, call_id, target_id):
                        return call_id
```

Note: The existing code already iterates `calls + call_children` (line 199), but the `_call_matches_target` check is applied. The real issue is that `call_children` already contains the constructor Call, but the location check may fail due to how PHP line numbers map. Verify with test fixtures.

#### 1b. Distinguish parameter_type vs return_type (Issue 3)

**Problem:** `_infer_reference_type()` returns `type_hint` for all Class/Interface targets. Parameter types and return types are indistinguishable.

**Fix in `_infer_reference_type()` (context.py:333-389):**

The `type_hint` edges in sot.json have different sources:
- Argument node -> Class = parameter type
- Method node -> Class = return type
- Property node -> Class = property type

The edge data includes `source` which tells us the node kind. Add source node lookup:

```python
# In _infer_reference_type(), replace the Class/Interface/Trait/Enum case:
if kind in ("Class", "Interface", "Trait", "Enum"):
    # Check if we can distinguish the type hint kind from edge metadata
    source_node = index.nodes.get(edge.source) if hasattr(edge, 'source') else None
    if source_node:
        if source_node.kind == "Argument":
            return "parameter_type"
        if source_node.kind in ("Method", "Function"):
            return "return_type"
        if source_node.kind == "Property":
            return "property_type"
    return "type_hint"
```

**Important:** `_infer_reference_type()` currently takes `(edge, target_node)`. We need to also pass the `index` to look up the source node. This changes the function signature from:
```python
def _infer_reference_type(edge: EdgeData, target_node: Optional[NodeData]) -> str:
```
to:
```python
def _infer_reference_type(edge: EdgeData, target_node: Optional[NodeData], index: Optional["SoTIndex"] = None) -> str:
```

All callers in context.py already have `self.index` available, so passing it is straightforward. The `Optional` default preserves backward compatibility.

### Phase 2: Argument Tracking (Issue 4)

**Approach:** Add `ArgumentInfo` dataclass, wire up `get_arguments()` in the outgoing tree builder.

#### 2a. New dataclass in results.py

```python
@dataclass
class ArgumentInfo:
    """Argument-to-parameter mapping at a call site."""
    position: int                    # 0-based argument position
    param_name: Optional[str]        # Formal parameter name from callee (e.g., "$productId")
    value_expr: Optional[str]        # Source expression (Value node name, e.g., "$input->productId")
    value_source: Optional[str]      # Value kind: "parameter", "local", "literal", "result"
```

#### 2b. Update ContextEntry

Add two new fields:
```python
@dataclass
class ContextEntry:
    # ... existing fields ...
    arguments: list["ArgumentInfo"] = field(default_factory=list)
    result_var: Optional[str] = None  # Name of variable receiving this call's result
```

#### 2c. Wire up in context.py `_build_outgoing_tree()`

After finding a Call node via `find_call_for_usage()`, call `index.get_arguments()` and resolve each to `ArgumentInfo`:

```python
# After call_node_id is found:
arguments = []
if call_node_id:
    arg_edges = self.index.get_arguments(call_node_id)
    for arg_node_id, position in arg_edges:
        arg_node = self.index.nodes.get(arg_node_id)
        if arg_node:
            param_name = self._resolve_param_name(call_node_id, position)
            arguments.append(ArgumentInfo(
                position=position,
                param_name=param_name,
                value_expr=arg_node.name,
                value_source=arg_node.value_kind,
            ))
    result_var = self._find_result_var(call_node_id)
```

New helper methods in `ContextQuery`:

```python
def _resolve_param_name(self, call_node_id: str, position: int) -> Optional[str]:
    """Get formal parameter name at position from the callee."""
    target_id = self.index.get_call_target(call_node_id)
    if not target_id:
        return None
    children = self.index.get_contains_children(target_id)
    arg_nodes = []
    for child_id in children:
        child = self.index.nodes.get(child_id)
        if child and child.kind == "Argument":
            arg_nodes.append(child)
    if position < len(arg_nodes):
        return arg_nodes[position].name
    return None

def _find_result_var(self, call_node_id: str) -> Optional[str]:
    """Find local variable name that receives this call's result."""
    result_id = self.index.get_produces(call_node_id)
    if not result_id:
        return None
    # Check incoming assigned_from edges on the result value
    for edge in self.index.incoming[result_id].get("assigned_from", []):
        source_node = self.index.nodes.get(edge.source)
        if source_node and source_node.kind == "Value" and source_node.value_kind == "local":
            return source_node.name
    return None
```

**Note on `_find_result_var` edge direction:** The `assigned_from` edge goes `local_value -> result_value`. So we need `incoming["assigned_from"]` on the result value. Check that `index.incoming` stores these correctly. If the edge direction is inverted (source=local, target=result), then we need `incoming[result_id]["assigned_from"]` which gives edges where target=result_id, source=local. This should work.

#### 2d. Update output formatters

In `tree.py` `add_context_children()`, after the main label, add argument display:
```python
if entry.arguments:
    label += "\n        [dim]args:[/dim]"
    for arg in entry.arguments:
        param = arg.param_name or f"arg[{arg.position}]"
        label += f"\n          {param} <- {arg.value_expr}"
if entry.result_var:
    label += f"\n        [dim]result ->[/dim] {entry.result_var}"
```

In `context_tree_to_dict()`, serialize arguments:
```python
if entry.arguments:
    d["arguments"] = [
        {"position": a.position, "param_name": a.param_name,
         "value_expr": a.value_expr, "value_source": a.value_source}
        for a in entry.arguments
    ]
if entry.result_var:
    d["result_var"] = entry.result_var
```

### Phase 3: Execution Flow USES (Issues 2 + 6)

**Approach:** Add new `_build_execution_flow()` method to `ContextQuery`. Use for Method/Function targets in USES. Keep existing `_build_outgoing_tree()` for non-method targets (classes, interfaces).

This is the largest change. The key insight: instead of following `uses` edges (structural), iterate the method's Call children in line-number order (behavioral).

```python
def _build_execution_flow(self, method_id: str, depth: int, max_depth: int,
                          limit: int, cycle_guard: set, count: list) -> list[ContextEntry]:
    """Build execution flow for a method by iterating its Call children."""
    if depth > max_depth or count[0] >= limit:
        return []

    children = self.index.get_contains_children(method_id)
    entries = []

    for child_id in children:
        child = self.index.nodes.get(child_id)
        if not child or child.kind != "Call":
            continue
        if count[0] >= limit:
            break

        target_id = self.index.get_call_target(child_id)
        if not target_id or target_id in cycle_guard:
            continue
        target_node = self.index.nodes.get(target_id)
        if not target_node:
            continue

        count[0] += 1
        reference_type = get_reference_type_from_call(self.index, child_id)
        access_chain = build_access_chain(self.index, child_id)
        access_chain_symbol = resolve_access_chain_symbol(self.index, child_id)
        arguments = self._get_argument_info(child_id)
        result_var = self._find_result_var(child_id)

        call_line = child.range.get("start_line") if child.range else None

        entry = ContextEntry(
            depth=depth,
            node_id=target_id,
            fqn=target_node.fqn,
            kind=target_node.kind,
            file=child.file,
            line=call_line,
            signature=target_node.signature,
            children=[],
            member_ref=MemberRef(
                target_name="",
                target_fqn=target_node.fqn,
                target_kind=target_node.kind,
                file=child.file,
                line=call_line,
                reference_type=reference_type,
                access_chain=access_chain,
                access_chain_symbol=access_chain_symbol,
            ),
            arguments=arguments,
            result_var=result_var,
        )

        # Depth expansion: recurse into callee's execution flow
        if depth < max_depth:
            # For methods, show their execution flow at depth+1
            if target_node.kind == "Method":
                entry.children = self._build_execution_flow(
                    target_id, depth + 1, max_depth, limit,
                    cycle_guard | {target_id}, count
                )

        entries.append(entry)

    # Sort by line number for execution order
    entries.sort(key=lambda e: (e.file or "", e.line if e.line is not None else 0))
    return entries
```

**Integration point:** In `_build_outgoing_tree()`, check if the start node is a Method/Function. If yes, also call `_build_execution_flow()` and merge/replace the structural results. Keep structural results for non-method targets.

**Decision point:** Whether execution flow replaces or supplements the existing structural output. The BA recommends opt-in via `--execution-flow` flag initially. This preserves backward compatibility.

### Phase 4: Smart Depth Filtering (Issue 5)

**Deferred.** Once Phase 3's execution flow is working, depth 2+ naturally shows only call-chain-relevant data (since `_build_execution_flow` only iterates Call nodes, not structural deps). This phase may become unnecessary or minimal.

---

## Phased Implementation

### Phase 1: Reference Type Fixes (Issues 1 + 3)
- [x] Understand `find_call_for_usage()` call paths
- [ ] Fix constructor Call node matching in `find_call_for_usage()`
- [ ] Add index parameter to `_infer_reference_type()`
- [ ] Add `parameter_type` / `return_type` / `property_type` distinction
- [ ] Update all callers of `_infer_reference_type()` to pass index
- [ ] Add/update unit tests in `test_reference_type.py`
- [ ] Add integration tests in `test_usage_flow.py` for constructor and type distinction

### Phase 2: Argument Tracking (Issue 4)
- [ ] Add `ArgumentInfo` dataclass to `results.py`
- [ ] Add `arguments` and `result_var` fields to `ContextEntry`
- [ ] Add `_resolve_param_name()` helper to `ContextQuery`
- [ ] Add `_find_result_var()` helper to `ContextQuery`
- [ ] Wire up argument tracking in `_build_outgoing_tree()`
- [ ] Update `tree.py` Rich console formatter for arguments
- [ ] Update `tree.py` JSON serializer for arguments
- [ ] Update `mcp.py` to include member_ref, arguments in context response
- [ ] Add integration tests for argument display
- [ ] Export `ArgumentInfo` from `models/__init__.py`

### Phase 3: Execution Flow (Issues 2 + 6)
- [ ] Add `_build_execution_flow()` method to `ContextQuery`
- [ ] Integrate with `_build_outgoing_tree()` for method targets
- [ ] Update output formatters for execution flow entries
- [ ] Add tests for execution flow ordering and content
- [ ] Add backward compatibility tests (class-level queries unchanged)

### Phase 4: Smart Depth (Issue 5)
- [ ] Evaluate if execution flow already solves the problem
- [ ] If needed, add call-chain-aware filtering at depth 2+
- [ ] Add depth expansion tests

---

## File Manifest

| Action | File Path | Description |
|--------|-----------|-------------|
| MODIFY | `src/queries/context.py` | Fix `find_call_for_usage()`, update `_infer_reference_type()` signature, add argument helpers, add `_build_execution_flow()` |
| MODIFY | `src/models/results.py` | Add `ArgumentInfo` dataclass, add `arguments`/`result_var` to `ContextEntry` |
| MODIFY | `src/models/__init__.py` | Export `ArgumentInfo` |
| MODIFY | `src/output/tree.py` | Update `print_context_tree()` and `context_tree_to_dict()` for arguments and execution flow |
| MODIFY | `src/server/mcp.py` | Fix lossy `_handle_context()` to include member_ref, reference_type, access_chain, arguments |
| MODIFY | `tests/test_reference_type.py` | Add tests for `parameter_type`, `return_type`, `property_type` inference |
| MODIFY | `tests/test_usage_flow.py` | Add tests for constructor detection, argument display, execution flow |

---

## File Ownership Suggestion

| Developer | Files | Rationale |
|-----------|-------|-----------|
| **developer-1** | `src/queries/context.py` | All core query logic changes: `find_call_for_usage()`, `_infer_reference_type()`, argument helpers, `_build_execution_flow()`. Single-file ownership prevents merge conflicts. This is the "engine" of the feature. |
| **developer-2** | `src/models/results.py`, `src/models/__init__.py`, `src/output/tree.py`, `src/server/mcp.py`, `tests/test_reference_type.py`, `tests/test_usage_flow.py` | Models, output formatting, MCP serialization, and tests. These files depend on context.py's output but don't modify its logic. This is the "surface" of the feature. |

### Why This Split Works

- **Zero file overlap**: Each developer owns distinct files.
- **Clear dependency direction**: developer-2's work consumes developer-1's output (ContextEntry instances with new fields). developer-1 defines the data; developer-2 formats and tests it.
- **Parallelizable phases**: In Phase 1, developer-1 fixes context.py while developer-2 adds ArgumentInfo to results.py and prepares test scaffolding. In Phase 2, developer-1 adds argument wiring while developer-2 updates formatters.

---

## Interface Contracts

### Contract 1: ContextEntry (context.py -> tree.py, mcp.py)

developer-1 produces `ContextEntry` instances. developer-2 consumes them in formatters. The contract:

```python
# Existing fields (unchanged):
ContextEntry.depth: int
ContextEntry.fqn: str
ContextEntry.kind: Optional[str]
ContextEntry.member_ref: Optional[MemberRef]  # with .reference_type, .access_chain

# New fields (Phase 2, added by developer-2 in results.py, populated by developer-1 in context.py):
ContextEntry.arguments: list[ArgumentInfo]     # default []
ContextEntry.result_var: Optional[str]         # default None
```

### Contract 2: ArgumentInfo (results.py -> context.py, tree.py, mcp.py)

```python
ArgumentInfo.position: int          # 0-based
ArgumentInfo.param_name: Optional[str]  # "$productId" or None
ArgumentInfo.value_expr: Optional[str]  # "$input->productId" or "0" or "'pending'"
ArgumentInfo.value_source: Optional[str]  # "parameter", "local", "literal", "result"
```

### Contract 3: _infer_reference_type signature change

Old: `_infer_reference_type(edge, target_node) -> str`
New: `_infer_reference_type(edge, target_node, index=None) -> str`

developer-1 makes this change. developer-2's test_reference_type.py tests can pass `index=None` for existing tests and a mock index for new tests.

### Contract 4: New reference type values

Phase 1 introduces three new reference_type values:
- `"parameter_type"` (was `"type_hint"`)
- `"return_type"` (was `"type_hint"`)
- `"property_type"` (was `"type_hint"`)

Existing `"type_hint"` remains as fallback when source node kind can't be determined.

### Contract 5: MCP response structure

developer-2 updates `_handle_context()` in mcp.py. The entry_to_dict must include:
```json
{
  "depth": 1,
  "fqn": "...",
  "kind": "Method",
  "file": "...",
  "line": 30,
  "signature": "...",
  "children": [...],
  "member_ref": {
    "target_name": "",
    "target_fqn": "...",
    "target_kind": "Method",
    "reference_type": "method_call",
    "access_chain": "$this->orderRepository",
    "access_chain_symbol": "App\\Service\\OrderService::$orderRepository"
  },
  "arguments": [
    {"position": 0, "param_name": "$order", "value_expr": "$processedOrder", "value_source": "local"}
  ],
  "result_var": "$savedOrder"
}
```

---

## Test Cases

Cross-referenced with acceptance criteria (AC) from the PM spec at `docs/specs/cli-context-fix.md`.

### Phase 1 Tests

| ID | Test | File | Validates | AC |
|----|------|------|-----------|-----|
| T1.1 | `new Order(...)` shows `[instantiation]` in USES | test_usage_flow.py | Issue 1 fix | AC1 |
| T1.2 | Constructor in USED BY shows `[instantiation]` | test_usage_flow.py | Issue 1 fix | AC1 |
| T1.3 | `CreateOrderInput` param shows `[parameter_type]` | test_usage_flow.py | Issue 3 fix | AC2 |
| T1.4 | `OrderOutput` return shows `[return_type]` | test_usage_flow.py | Issue 3 fix | AC3 |
| T1.5 | Property type hint shows `[property_type]` | test_usage_flow.py | Issue 3 fix | AC4 |
| T1.6 | Unit test: _infer_reference_type with Argument source | test_reference_type.py | Issue 3 logic | AC2 |
| T1.7 | Unit test: _infer_reference_type with Method source | test_reference_type.py | Issue 3 logic | AC3 |
| T1.8 | Unit test: _infer_reference_type without index (backward compat) | test_reference_type.py | No regression | AC5 |
| T1.9 | Existing tests still pass (no regression) | all test files | Backward compat | AC5 |
| T1.10 | JSON output includes new reference_type values | test_usage_flow.py | JSON format | AC6 |

### Phase 2 Tests

| ID | Test | File | Validates | AC |
|----|------|------|-----------|-----|
| T2.1 | `checkAvailability()` shows 2 arguments with param names | test_usage_flow.py | Issue 4 | AC7 |
| T2.2 | `new Order()` shows constructor arguments | test_usage_flow.py | Issue 4 | AC8 |
| T2.3 | `save()` shows argument from local variable | test_usage_flow.py | Issue 4 | AC7 |
| T2.4 | Call result assigned to `$order` shows `result: $order` | test_usage_flow.py | Result tracking | AC9 |
| T2.5 | Call with no arguments shows no `arguments:` block | test_usage_flow.py | Edge case | AC10 |
| T2.6 | JSON output includes `arguments` array and `result_var` | test_usage_flow.py | JSON format | AC11 |
| T2.7 | MCP response includes member_ref with reference_type | test_usage_flow.py or new | MCP fix | AC12 |
| T2.8 | Entry with no Call node has empty arguments list | test_usage_flow.py | Edge case | AC10 |

### Phase 3 Tests

| ID | Test | File | Validates | AC |
|----|------|------|-----------|-----|
| T3.1 | Method USES shows Call nodes in line-number order (30, 32, 42, 45) | test_usage_flow.py | Execution flow | AC13 |
| T3.2 | `$order` visible as constructor result AND as argument to process() | test_usage_flow.py | Value flow | AC14 |
| T3.3 | Local variable not passed to any call is NOT shown | test_usage_flow.py | Data-flow filter | AC15 |
| T3.4 | Class-level context query still uses structural approach | test_usage_flow.py | Backward compat | AC16 |
| T3.5 | JSON includes execution flow with line-ordered entries | test_usage_flow.py | JSON format | AC17 |

### Phase 4 Tests

| ID | Test | File | Validates | AC |
|----|------|------|-----------|-----|
| T4.1 | Depth 2 shows `__construct()` execution, not Order property types | test_usage_flow.py | Smart depth | AC18 |
| T4.2 | Depth 2 shows callee's internal calls with arguments | test_usage_flow.py | Smart depth | AC19 |
| T4.3 | Recursive method call shown but not expanded at depth 2 | test_usage_flow.py | Cycle prevention | AC20 |
| T4.4 | USED BY depth semantics unchanged | test_usage_flow.py | No regression | AC21 |

---

## Risks & Mitigations

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| Constructor Call node not found due to line number mismatch | Phase 1 incomplete | Medium | Verify with reference-project fixtures. `_call_matches_target()` already handles the class->__construct mapping; the issue is finding the Call node in the first place. Add debug logging in tests. |
| `_infer_reference_type()` signature change breaks callers | Regression | Low | Use `index=None` default. All 6 callers are in context.py and controlled by developer-1. |
| `assigned_from` edge direction confusion | Phase 2 argument tracking wrong | Medium | Verify edge direction in sot.json fixtures. The edge `source --assigned_from--> target` means source is assigned FROM target. Check `index.incoming[result_id]["assigned_from"]`. |
| Argument position resolution off-by-one | Wrong parameter names | Medium | Validate with reference project PHP fixtures. Argument nodes in contains order should match position order. |
| MCP backward compatibility break | AI agent integration breaks | High if unmanaged | New fields are additive (arguments, result_var). reference_type value changes (type_hint -> parameter_type) could break string-matching consumers. Document as known change. |
| Execution flow (Phase 3) too verbose | Output overwhelming | Medium | Apply BA recommendation: only show local variables when they participate in data flow (passed as arguments, receive call results, used as receivers). |
| Performance: iterating all contained Call nodes | Slow for large methods | Low | Already in-memory, O(n) in method body size. Methods rarely have 100+ calls. |

---

## Phase Boundaries

```
Phase 1 (Issues 1+3) -----> Phase 2 (Issue 4) -----> Phase 3 (Issues 2+6) -----> Phase 4 (Issue 5)
   |                            |                         |
   | No new dataclasses         | New ArgumentInfo        | New _build_execution_flow()
   | Fix existing functions     | Wire up get_arguments() | Behavioral USES view
   | Small, targeted            | Medium complexity        | Large refactor
   |                            |                         |
   v                            v                         v
  All existing tests pass      Arguments in output       Execution flow for methods
  New reference type values    MCP includes member_ref   Depth = call stack depth
```

**What must be sequential:**
1. Phase 1 must complete before Phase 2 (Phase 2 relies on correct Call node matching from Phase 1)
2. Phase 2 must complete before Phase 3 (Phase 3 uses argument helpers from Phase 2)

**What can be parallel within each phase:**
- developer-1 and developer-2 can work in parallel within any phase (no file overlap)
- Within Phase 1: developer-1 fixes context.py while developer-2 prepares test scaffolding
- Within Phase 2: developer-1 adds argument wiring while developer-2 updates formatters

---

## Implementation Order for Developers

### developer-1 (context.py)

1. **Phase 1a**: Fix `find_call_for_usage()` constructor matching
2. **Phase 1b**: Update `_infer_reference_type()` signature and add parameter_type/return_type/property_type logic
3. **Phase 1c**: Update all callers of `_infer_reference_type()` to pass `self.index`
4. **Phase 2a**: Add `_resolve_param_name()` and `_find_result_var()` helpers
5. **Phase 2b**: Wire up argument tracking in `_build_outgoing_tree()` and `_build_deps_subtree()`
6. **Phase 3a**: Add `_build_execution_flow()` method
7. **Phase 3b**: Integrate execution flow into `_build_outgoing_tree()` for method targets

### developer-2 (models + output + tests + MCP)

1. **Phase 1a**: Add new reference type unit tests to `test_reference_type.py`
2. **Phase 1b**: Add integration tests for constructor and type distinction to `test_usage_flow.py`
3. **Phase 2a**: Add `ArgumentInfo` dataclass to `results.py`, add fields to `ContextEntry`, update `models/__init__.py`
4. **Phase 2b**: Update `tree.py` Rich formatter for arguments display
5. **Phase 2c**: Update `tree.py` JSON serializer for arguments
6. **Phase 2d**: Fix `mcp.py` `_handle_context()` to include full member_ref, arguments
7. **Phase 2e**: Add integration tests for argument display
8. **Phase 3a**: Update formatters for execution flow output
9. **Phase 3b**: Add execution flow integration tests
