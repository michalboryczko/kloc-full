# Implementation Plan: Data Flow Context Improvements (v6)

**Spec:** `docs/specs/data-flow-context-improvments.md`
**Branch:** `feature/value-context`

## Work Streams

Two developers work in parallel. Stream 1 handles ISSUE-D (cross-component, foundation). Stream 2 handles ISSUE-C (parallel), then ISSUE-E (after D), then ISSUE-F (after E).

### File Ownership

| File | Developer-1 (ISSUE-D) | Developer-2 (ISSUE-C, E, F) | Notes |
|------|----------------------|----------------------------|-------|
| `kloc-contracts/sot-json.json` | OWN | -- | Schema change |
| `kloc-mapper/src/models.py` | OWN | -- | Edge.parameter field |
| `kloc-mapper/src/calls_mapper.py` | OWN | -- | Store parameter, fix FQN |
| `kloc-mapper/src/parser.py` | OWN | -- | FQN separator fix |
| `kloc-cli/src/models/edge.py` | OWN | -- | EdgeData.parameter field |
| `kloc-cli/src/graph/index.py` | OWN (get_arguments) | -- | parameter field in loading |
| `kloc-cli/src/queries/context.py` | `_get_argument_info()`, `_resolve_param_name()`, `_resolve_param_fqn()` | `_build_value_consumer_chain()`, `_build_value_source_chain()`, `_build_incoming_tree()` | Split by function |
| `kloc-cli/src/models/results.py` | -- | OWN (ContextEntry.crossed_from) | New field for E |
| `kloc-cli/src/output/tree.py` | -- | OWN (rendering changes for C, E, F) | Rendering updates |

**Critical constraint:** Both developers touch `context.py` but in non-overlapping functions. Developer-1 modifies argument resolution functions (lines 1415-1540, 1481-1538). Developer-2 modifies consumer/source chain functions (lines 1047-1325, 2020-2140) and incoming tree (lines 757-1045). No merge conflicts expected.

---

## Stream 1: Developer-1 -- ISSUE-D (Argument-Parameter Linking)

### Task D1: Add `parameter` field to contract schema

**File:** `kloc-contracts/sot-json.json`
**Location:** Line 189-196 (Edge properties)
**Change:** Add `parameter` property to the Edge schema `$defs/Edge/properties`:

```json
"parameter": {
  "type": "string",
  "description": "FQN of the formal parameter for 'argument' edges. Resolved from scip-php parameter symbol."
}
```

Also update Edge description (line 172) to mention `parameter`.

**Lines changed:** ~4 lines added at line 196.

---

### Task D2: Add `parameter` field to mapper Edge model

**File:** `kloc-mapper/src/models.py`
**Location:** Line 120-141 (Edge dataclass)
**Change:** Add `parameter: Optional[str] = None` field after `expression`:

```python
@dataclass
class Edge:
    type: EdgeType
    source: str
    target: str
    location: Optional[Location] = None
    position: Optional[int] = None
    expression: Optional[str] = None
    parameter: Optional[str] = None  # NEW: formal parameter FQN for argument edges
```

Update `to_dict()` (line 129-141) to include `parameter` when set:

```python
if self.parameter is not None:
    d["parameter"] = self.parameter
```

**Lines changed:** ~4 lines added.

---

### Task D3: Store parameter FQN on argument edges in mapper

**File:** `kloc-mapper/src/calls_mapper.py`
**Location:** Line 188-203 (`_create_call_edges()`, argument edge creation loop)
**Change:** Read `parameter` from scip-php argument data, resolve SCIP symbol to FQN, store on edge:

```python
# In the argument loop at line 190:
for arg in arguments:
    position = arg.get("position")
    value_id = arg.get("value_id")
    if value_id and position is not None:
        arg_node_id = self.value_id_to_node_id.get(value_id)
        if arg_node_id:
            value_expr = arg.get("value_expr")
            # NEW: resolve parameter SCIP symbol to FQN
            param_symbol = arg.get("parameter")
            param_fqn = self._resolve_param_fqn(param_symbol) if param_symbol else None
            self.edges.append(Edge(
                type=EdgeType.ARGUMENT,
                source=call_node_id,
                target=arg_node_id,
                position=position,
                expression=value_expr if value_expr else None,
                parameter=param_fqn,  # NEW
            ))
```

Add helper method `_resolve_param_fqn()` to `CallsMapper` class:

```python
def _resolve_param_fqn(self, param_symbol: str) -> Optional[str]:
    """Resolve a scip-php parameter SCIP symbol to its FQN.

    The parameter symbol is like:
      "scip-php composer ... App/Repository/OrderRepositoryInterface#save().($order)"
    This should resolve to:
      "App\\Repository\\OrderRepositoryInterface::save().$order"
    """
    # First try to find the node and use its FQN
    node_id = self._resolve_symbol_to_node_id(param_symbol)
    if node_id and node_id in self.nodes:
        return self.nodes[node_id].fqn
    # Fall back to manual FQN conversion using existing _symbol_to_fqn
    fqn = self._symbol_to_fqn(param_symbol)
    # Fix parameter separator: convert ::$param to .$param
    # The _symbol_to_fqn produces method()::$param, but parameters use . separator
    if "::" in fqn:
        parts = fqn.rsplit("::", 1)
        last = parts[-1]
        if last.startswith("$"):
            fqn = f"{parts[0]}.{last}"
    return fqn
```

**Lines changed:** ~25 lines added/modified in `_create_call_edges()` and new helper.

---

### Task D4: Fix Argument node FQN separator from `::` to `.`

**File:** `kloc-mapper/src/parser.py`
**Location:** Line 61 (`extract_fqn_from_descriptor()`)
**Change:** Change separator from `::` to `.`:

```python
# Before (line 61):
return f"{method_fqn}::{arg_name}"

# After:
return f"{method_fqn}.{arg_name}"
```

**Lines changed:** 1 line.

**Risk:** This changes all Argument node FQNs in sot.json. Existing tests that check Argument FQNs will need updating. The CLI's `resolve_symbol()` uses FQN matching, so queries for `method()::$param` will need to use `method().$param` instead.

---

### Task D5: Add `parameter` field to CLI EdgeData model

**File:** `kloc-cli/src/models/edge.py`
**Location:** Line 8-16 (EdgeData dataclass)
**Change:** Add `parameter: Optional[str] = None` field:

```python
@dataclass
class EdgeData:
    """Edge from SoT JSON."""
    type: str
    source: str
    target: str
    location: Optional[dict] = None
    position: Optional[int] = None
    expression: Optional[str] = None
    parameter: Optional[str] = None  # NEW: formal parameter FQN for argument edges
```

**Lines changed:** 1 line added.

---

### Task D6: Load `parameter` field in graph index

**File:** `kloc-cli/src/graph/index.py`
**Location:** Line 59-67 (EdgeData construction in `_load()`)
**Change:** Add `parameter` to EdgeData construction:

```python
edge = EdgeData(
    type=e["type"],
    source=e["source"],
    target=e["target"],
    location=e.get("location"),
    position=e.get("position"),
    expression=e.get("expression"),
    parameter=e.get("parameter"),  # NEW
)
```

Also update `get_arguments()` (line 501-509) to return the parameter FQN:

```python
def get_arguments(self, call_node_id: str) -> list[tuple[str, int, Optional[str], Optional[str]]]:
    """Get argument Value node IDs with their positions for a Call node.

    Returns:
        List of (value_node_id, position, expression, parameter) tuples sorted by position.
    """
    edges = self.outgoing[call_node_id].get("argument", [])
    args = [(e.target, e.position or 0, e.expression, e.parameter) for e in edges]
    return sorted(args, key=lambda x: x[1])
```

**Lines changed:** ~6 lines modified.

**Risk:** Changing `get_arguments()` return type from 3-tuple to 4-tuple. All callers must be updated. Callers:
- `kloc-cli/src/queries/context.py` line 1495-1497 (`_get_argument_info()`)
- `kloc-cli/src/queries/context.py` line 1795-1796 (`_build_execution_flow()` consumed detection)
- Any tests calling `get_arguments()`

---

### Task D7: Update CLI to use `parameter` field with position fallback

**File:** `kloc-cli/src/queries/context.py`
**Location:** Line 1481-1538 (`_get_argument_info()`)
**Change:** Read `parameter` from edge data, fall back to position-based matching:

```python
def _get_argument_info(self, call_node_id: str) -> list:
    arg_edges = self.index.get_arguments(call_node_id)
    arguments = []
    for arg_node_id, position, expression, parameter in arg_edges:  # NEW: 4-tuple
        arg_node = self.index.nodes.get(arg_node_id)
        if arg_node:
            # NEW: Direct read from edge, with position fallback
            if parameter:
                param_fqn = parameter
                # Extract param_name from FQN: "method().$param" -> "$param"
                param_name = param_fqn.rsplit(".", 1)[-1] if "." in param_fqn else param_fqn
            else:
                # Fallback for sot.json without parameter field
                param_name = self._resolve_param_name(call_node_id, position)
                param_fqn = self._resolve_param_fqn(call_node_id, position)
            # ... rest unchanged
```

Also update the consumed detection loop in `_build_execution_flow()` (line 1795-1796):

```python
# Update tuple unpacking to handle 4-tuple
arg_edges = self.index.get_arguments(call_id)
for arg_id, _, _, _ in arg_edges:  # was: for arg_id, _, _ in arg_edges
```

**Lines changed:** ~15 lines modified.

---

### Task D8: Prefer Value over Argument in symbol resolution

**File:** `kloc-cli/src/graph/index.py`
**Location:** Line 105-183 (`resolve_symbol()`)
**Change:** When multiple candidates are found with the same FQN, sort Value nodes before Argument nodes:

After collecting candidates in `resolve_symbol()`, before returning, add:

```python
# Prefer Value nodes over Argument nodes when both share the same FQN
if len(candidates) > 1:
    candidates.sort(key=lambda n: (0 if n.kind == "Value" else 1))
```

**Lines changed:** ~3 lines added.

---

### Task D9: Update mapper and CLI tests for ISSUE-D

**Files:**
- `kloc-mapper/tests/test_models.py` -- Edge serialization with `parameter`
- `kloc-mapper/tests/test_parser.py` -- FQN extraction with `.` separator
- `kloc-cli/tests/` -- Update any tests that check Argument FQNs or argument resolution

**Estimated lines:** ~30 lines of test updates.

---

## Stream 2: Developer-2 -- ISSUE-C, then E, then F

### Phase A: ISSUE-C (Consumer Access Chain Fix) -- parallel with D

#### Task C1: Add receiver resolution to `_build_value_consumer_chain()` Part 1

**File:** `kloc-cli/src/queries/context.py`
**Location:** Line 1166-1178 (Part 1: grouped consumer calls, MemberRef construction)
**Change:** Before the `MemberRef` construction at line 1169, add access chain resolution for `consumer_call_id`:

```python
# Resolve access chain for the consuming Call
access_chain = build_access_chain(self.index, consumer_call_id)
access_chain_symbol_val = resolve_access_chain_symbol(self.index, consumer_call_id)

# Resolve receiver variable identity
on_kind = None
on_file = None
on_line = None
recv_id = self.index.get_receiver(consumer_call_id)
if recv_id:
    recv_node = self.index.nodes.get(recv_id)
    if recv_node and recv_node.kind == "Value" and recv_node.value_kind in ("local", "parameter"):
        on_kind = "local" if recv_node.value_kind == "local" else "param"
        if recv_node.file:
            on_file = recv_node.file
        if recv_node.range and recv_node.range.get("start_line") is not None:
            on_line = recv_node.range["start_line"]

member_ref = MemberRef(
    ...
    access_chain=access_chain,              # was None
    access_chain_symbol=access_chain_symbol_val,  # was None
    on_kind=on_kind,                          # was not set
    on_file=on_file,                          # was not set
    on_line=on_line,                          # was not set
)
```

**Lines changed:** ~15 lines added before MemberRef at line 1169.

---

#### Task C2: Add receiver resolution to `_build_value_consumer_chain()` Part 2

**File:** `kloc-cli/src/queries/context.py`
**Location:** Line 1240-1249 (Part 2: standalone property accesses, MemberRef construction)
**Change:** Same pattern as Task C1, but using `access_call_id` as the Call node:

```python
# Before MemberRef at line 1240:
access_chain = build_access_chain(self.index, access_call_id)
access_chain_symbol_val = resolve_access_chain_symbol(self.index, access_call_id)
on_kind, on_file, on_line = None, None, None
recv_id = self.index.get_receiver(access_call_id)
if recv_id:
    recv_node = self.index.nodes.get(recv_id)
    if recv_node and recv_node.kind == "Value" and recv_node.value_kind in ("local", "parameter"):
        on_kind = "local" if recv_node.value_kind == "local" else "param"
        if recv_node.file:
            on_file = recv_node.file
        if recv_node.range and recv_node.range.get("start_line") is not None:
            on_line = recv_node.range["start_line"]

member_ref = MemberRef(
    ...
    access_chain=access_chain,              # was None
    access_chain_symbol=access_chain_symbol_val,  # was None
    on_kind=on_kind,
    on_file=on_file,
    on_line=on_line,
)
```

**Lines changed:** ~15 lines added before MemberRef at line 1240.

---

#### Task C3: Add receiver resolution to `_build_value_consumer_chain()` Part 3

**File:** `kloc-cli/src/queries/context.py`
**Location:** Line 1293-1305 (Part 3: direct argument edges, MemberRef construction)
**Change:** Same pattern as Tasks C1/C2, using `consumer_call_id`:

```python
# Before MemberRef at line 1296:
access_chain = build_access_chain(self.index, consumer_call_id)
access_chain_symbol_val = resolve_access_chain_symbol(self.index, consumer_call_id)
on_kind, on_file, on_line = None, None, None
recv_id = self.index.get_receiver(consumer_call_id)
if recv_id:
    recv_node = self.index.nodes.get(recv_id)
    if recv_node and recv_node.kind == "Value" and recv_node.value_kind in ("local", "parameter"):
        on_kind = "local" if recv_node.value_kind == "local" else "param"
        if recv_node.file:
            on_file = recv_node.file
        if recv_node.range and recv_node.range.get("start_line") is not None:
            on_line = recv_node.range["start_line"]

member_ref = MemberRef(
    ...
    access_chain=access_chain,
    access_chain_symbol=access_chain_symbol_val,
    on_kind=on_kind,
    on_file=on_file,
    on_line=on_line,
)
```

**Lines changed:** ~15 lines added before MemberRef at line 1296.

---

#### Task C4 (optional): Extract `_resolve_receiver_identity()` helper

**File:** `kloc-cli/src/queries/context.py`
**Location:** New method in `ContextQuery` class, after `_resolve_containing_method()` (~line 1376)
**Change:** Extract shared pattern from `_build_execution_flow()` (line 1836-1847) and the three C1/C2/C3 additions:

```python
def _resolve_receiver_identity(self, call_node_id: str) -> tuple[
    Optional[str], Optional[str], Optional[str], Optional[str], Optional[int]
]:
    """Resolve access chain and receiver identity for a Call node.

    Returns:
        (access_chain, access_chain_symbol, on_kind, on_file, on_line)
    """
    access_chain = build_access_chain(self.index, call_node_id)
    access_chain_symbol = resolve_access_chain_symbol(self.index, call_node_id)
    on_kind = None
    on_file = None
    on_line = None
    recv_id = self.index.get_receiver(call_node_id)
    if recv_id:
        recv_node = self.index.nodes.get(recv_id)
        if recv_node and recv_node.kind == "Value" and recv_node.value_kind in ("local", "parameter"):
            on_kind = "local" if recv_node.value_kind == "local" else "param"
            if recv_node.file:
                on_file = recv_node.file
            if recv_node.range and recv_node.range.get("start_line") is not None:
                on_line = recv_node.range["start_line"]
    return access_chain, access_chain_symbol, on_kind, on_file, on_line
```

Then simplify C1/C2/C3 and the existing code in `_build_execution_flow()` (line 1830-1847) and `_build_value_source_chain()` (line 2077-2089) to call this helper.

**Lines changed:** ~20 lines new helper + ~-30 lines simplified call sites = net reduction.

---

### Phase B: ISSUE-E (Cross-Method Parameter Tracing) -- after D completes

#### Task E1: Add `crossed_from` field to ContextEntry model

**File:** `kloc-cli/src/models/results.py`
**Location:** Line 134-162 (ContextEntry dataclass)
**Change:** Add field for boundary crossing indicator:

```python
# After source_call field (line 162):
crossed_from: Optional[str] = None  # FQN of the parameter crossed from (boundary indicator)
```

**Lines changed:** 1 line added.

---

#### Task E2: Add JSON serialization for `crossed_from`

**File:** `kloc-cli/src/output/tree.py`
**Location:** Line 519-529 (in `context_entry_to_dict()`, ISSUE-C fields section)
**Change:** Add `crossed_from` serialization:

```python
if entry.crossed_from:
    d["crossed_from"] = entry.crossed_from
```

**Lines changed:** 2 lines added after line 529.

---

#### Task E3: Update tree rendering for boundary crossing display

**File:** `kloc-cli/src/output/tree.py`
**Location:** Line 315-410 (`add_context_children()`)
**Change:** When an entry has `crossed_from`, display a boundary crossing indicator before the entry's children:

```python
# After the branch is added (line 393), before adding children:
if entry.crossed_from:
    branch.add(f"[dim italic]crosses into {entry.crossed_from}[/dim italic]")
```

**Lines changed:** ~3 lines added.

---

#### Task E4: Add cross-method USED BY to `_build_value_consumer_chain()`

**File:** `kloc-cli/src/queries/context.py`
**Location:** Line 1047-1325 (`_build_value_consumer_chain()`)
**Change:** Add `visited` parameter and cross-method recursion. Modify the method signature to accept a visited set:

```python
def _build_value_consumer_chain(
    self, value_id: str, depth: int, max_depth: int, limit: int,
    visited: set | None = None  # NEW: cycle detection
) -> list[ContextEntry]:
```

In Part 1 and Part 3 (where a Value is passed as argument to a Call), after building the entry, add cross-method recursion:

```python
# After entry construction, before appending:
if depth < max_depth and consumer_target_id and consumer_target:
    if consumer_target.kind in ("Method", "Function"):
        # Cross into callee: find matching parameter Value
        for arg_info in arguments:
            if arg_info.param_fqn:
                # Look up callee's Value(parameter) by FQN
                param_matches = self.index.resolve_symbol(arg_info.param_fqn)
                for pm in param_matches:
                    if pm.kind == "Value" and pm.value_kind == "parameter":
                        if pm.id not in visited:
                            # CROSS METHOD BOUNDARY
                            child_entries = self._build_value_consumer_chain(
                                pm.id, depth + 1, max_depth, limit, visited
                            )
                            for ce in child_entries:
                                ce.crossed_from = arg_info.param_fqn
                            entry.children.extend(child_entries)
                        break
```

Also handle the return value path: when the callee returns a value that's assigned to a local in the caller, trace that local's consumers:

```python
# Return value path (after cross-method recursion):
# Check if the call produces a result assigned to a local
local_value = self._find_local_value_for_call(consumer_call_id)  # existing helper
if local_value and local_value.id not in visited:
    return_entries = self._build_value_consumer_chain(
        local_value.id, depth + 1, max_depth, limit, visited
    )
    entry.children.extend(return_entries)
```

Update the call site in `_build_incoming_tree()` (line 810):

```python
# Line 810:
return self._build_value_consumer_chain(start_id, 1, max_depth, limit, visited=set())
```

**Lines changed:** ~40 lines added/modified.

**Risk:** This is the most complex change. The recursive cross-method traversal must:
1. Track visited Value IDs to prevent infinite loops
2. Increment depth for each boundary crossing
3. Handle interface methods as terminal nodes (no body, no Value children)
4. Handle promoted constructor parameters (existing depth expansion at line 1194-1218 already handles this for one level)

---

#### Task E5: Add cross-method USES to `_build_value_source_chain()`

**File:** `kloc-cli/src/queries/context.py`
**Location:** Line 2020-2140 (`_build_value_source_chain()`)
**Change:** When the Value is a parameter with no local sources, search argument edges by `parameter` field to find callers:

Add `visited` parameter:

```python
def _build_value_source_chain(
    self, value_id: str, depth: int, max_depth: int, limit: int,
    visited: set | None = None  # NEW: cycle detection
) -> list[ContextEntry]:
```

Add parameter-specific branch at the top of the method (after the `value_node` check at line 2042):

```python
if value_node.value_kind == "parameter":
    # Parameter Values have no local sources — find callers via argument edges
    return self._build_parameter_uses(value_id, value_node, depth, max_depth, limit, visited)
```

New method `_build_parameter_uses()`:

```python
def _build_parameter_uses(
    self, param_value_id: str, param_node, depth: int, max_depth: int, limit: int,
    visited: set | None = None
) -> list[ContextEntry]:
    """Find callers of a parameter Value via argument edges with matching parameter FQN.

    Searches all argument edges in the graph where the `parameter` field matches
    this Value's FQN, then traces the source of each caller's argument Value.
    """
    if depth > max_depth:
        return []
    if visited is None:
        visited = set()
    if param_value_id in visited:
        return []
    visited.add(param_value_id)

    entries = []
    param_fqn = param_node.fqn

    # Search argument edges where parameter field matches this FQN
    for edge in self.index.edges:
        if edge.type != "argument":
            continue
        if edge.parameter != param_fqn:
            continue

        # Found a caller's argument edge
        caller_call_id = edge.source  # Call node in the caller
        caller_value_id = edge.target  # Value passed by the caller

        call_node = self.index.nodes.get(caller_call_id)
        caller_value = self.index.nodes.get(caller_value_id)
        if not call_node or not caller_value:
            continue

        # Find the containing method of the caller
        scope_id = get_containing_scope(self.index, caller_call_id)
        scope_node = self.index.nodes.get(scope_id) if scope_id else None

        call_line = call_node.range.get("start_line") if call_node.range else None

        entry = ContextEntry(
            depth=depth,
            node_id=scope_id or caller_call_id,
            fqn=scope_node.fqn if scope_node else call_node.fqn,
            kind=scope_node.kind if scope_node else call_node.kind,
            file=call_node.file,
            line=call_line,
            signature=scope_node.signature if scope_node else None,
            children=[],
            crossed_from=param_fqn,
        )

        # Trace the caller's argument Value's source chain (recurse with depth+1)
        if depth < max_depth and caller_value_id not in visited:
            child_entries = self._build_value_source_chain(
                caller_value_id, depth + 1, max_depth, limit, visited
            )
            entry.children.extend(child_entries)

        entries.append(entry)
        if len(entries) >= limit:
            break

    entries.sort(key=lambda e: (e.file or "", e.line if e.line is not None else 0))
    return entries
```

Update recursive calls to `_build_value_source_chain()` at line 2135-2138 to pass visited:

```python
children = self._build_value_source_chain(
    arg_value_node.id, depth + 1, max_depth, limit, visited
)
```

Update call site in `_build_outgoing_tree()` (line 2187):

```python
return self._build_value_source_chain(start_id, 1, max_depth, limit, visited=set())
```

**Lines changed:** ~60 lines new method + ~10 lines modifications.

**Performance note:** The `_build_parameter_uses()` method iterates all edges to find matching `parameter` fields. For the reference project (~1200 edges), this is fast. For larger projects, consider building a `parameter_fqn -> list[EdgeData]` index in `SoTIndex._build_indexes()`. This is an optimization that can be deferred.

---

#### Task E6: Add `parameter` field index for efficient lookup (optional optimization)

**File:** `kloc-cli/src/graph/index.py`
**Location:** Line 69-103 (`_build_indexes()`)
**Change:** Add index for parameter FQN to argument edges:

```python
# In _build_indexes(), after edge indexes:
self.parameter_to_edges: dict[str, list[EdgeData]] = defaultdict(list)
for edge in self.edges:
    if edge.type == "argument" and edge.parameter:
        self.parameter_to_edges[edge.parameter].append(edge)
```

Add query method:

```python
def get_argument_edges_by_parameter(self, param_fqn: str) -> list[EdgeData]:
    """Get argument edges where the parameter field matches the given FQN."""
    return self.parameter_to_edges.get(param_fqn, [])
```

Then `_build_parameter_uses()` uses this instead of iterating all edges.

**Lines changed:** ~8 lines added.

---

### Phase C: ISSUE-F (Property Cross-Method Tracing) -- after E completes

#### Task F1: Add Property USES tracing

**File:** `kloc-cli/src/queries/context.py`
**Location:** Line 2142-2187 (`_build_outgoing_tree()`)
**Change:** Add Property-specific branch (similar to Value branch at line 2186):

```python
# After the Value branch at line 2187:
# For Property nodes, trace who sets the property
if start_node and start_node.kind == "Property":
    return self._build_property_uses(start_id, 1, max_depth, limit)
```

New method `_build_property_uses()`:

```python
def _build_property_uses(
    self, property_id: str, depth: int, max_depth: int, limit: int
) -> list[ContextEntry]:
    """Build USES chain for a Property node.

    For promoted constructor properties: follow assigned_from edge to
    Value(parameter), then trace callers via ISSUE-E USES.

    For other properties: follow assigned_from edges to source Values.
    """
    if depth > max_depth:
        return []

    visited = set()

    # Check for assigned_from edges (promoted property -> Value(parameter))
    assigned_edges = self.index.incoming[property_id].get("assigned_from", [])
    for edge in assigned_edges:
        source_node = self.index.nodes.get(edge.source)
        if source_node and source_node.kind == "Value" and source_node.value_kind == "parameter":
            # Promoted property: trace the parameter's callers
            return self._build_value_source_chain(
                source_node.id, depth, max_depth, limit, visited
            )

    # No assigned_from: property may be set by DI container or direct assignment
    # Return empty (terminal)
    return []
```

**Lines changed:** ~25 lines.

---

#### Task F2: Add Property USED BY tracing

**File:** `kloc-cli/src/queries/context.py`
**Location:** Line 757-810 (`_build_incoming_tree()`)
**Change:** Add Property-specific branch (similar to Value branch at line 809-810):

```python
# After the Value branch check at line 809:
if start_node and start_node.kind == "Property":
    return self._build_property_used_by(start_id, 1, max_depth, limit)
```

New method `_build_property_used_by()`:

```python
def _build_property_used_by(
    self, property_id: str, depth: int, max_depth: int, limit: int
) -> list[ContextEntry]:
    """Build USED BY chain for a Property node.

    Finds all Calls that access this Property (via 'calls' edges targeting
    the Property), gets the result Value of each access, then traces
    consumers via ISSUE-E USED BY.
    """
    if depth > max_depth:
        return []

    entries = []
    visited = set()
    count = 0

    # Find all Call nodes that target this Property (via 'calls' edges)
    call_ids = self.index.get_calls_to(property_id)

    for call_id in call_ids:
        if count >= limit:
            break
        call_node = self.index.nodes.get(call_id)
        if not call_node:
            continue

        call_line = call_node.range.get("start_line") if call_node.range else None
        reference_type = get_reference_type_from_call(self.index, call_id)
        access_chain = build_access_chain(self.index, call_id)
        access_chain_symbol = resolve_access_chain_symbol(self.index, call_id)

        # Resolve receiver identity
        on_kind, on_file, on_line = None, None, None
        recv_id = self.index.get_receiver(call_id)
        if recv_id:
            recv_node = self.index.nodes.get(recv_id)
            if recv_node and recv_node.kind == "Value" and recv_node.value_kind in ("local", "parameter"):
                on_kind = "local" if recv_node.value_kind == "local" else "param"
                if recv_node.file:
                    on_file = recv_node.file
                if recv_node.range and recv_node.range.get("start_line") is not None:
                    on_line = recv_node.range["start_line"]

        # Find the containing method for this access
        scope_id = get_containing_scope(self.index, call_id)
        scope_node = self.index.nodes.get(scope_id) if scope_id else None

        property_node = self.index.nodes.get(property_id)

        member_ref = MemberRef(
            target_name=self._member_display_name(property_node) if property_node else "?",
            target_fqn=property_node.fqn if property_node else "?",
            target_kind="Property",
            file=call_node.file,
            line=call_line,
            reference_type=reference_type,
            access_chain=access_chain,
            access_chain_symbol=access_chain_symbol,
            on_kind=on_kind,
            on_file=on_file,
            on_line=on_line,
        )

        entry = ContextEntry(
            depth=depth,
            node_id=scope_id or call_id,
            fqn=scope_node.fqn if scope_node else call_node.fqn,
            kind=scope_node.kind if scope_node else call_node.kind,
            file=call_node.file,
            line=call_line,
            signature=scope_node.signature if scope_node else None,
            children=[],
            member_ref=member_ref,
        )

        # Trace the result Value's consumers (cross-method via ISSUE-E)
        result_id = self.index.get_produces(call_id)
        if result_id and depth < max_depth:
            child_entries = self._build_value_consumer_chain(
                result_id, depth + 1, max_depth, limit, visited
            )
            entry.children.extend(child_entries)

        count += 1
        entries.append(entry)

    entries.sort(key=lambda e: (e.file or "", e.line if e.line is not None else 0))
    return entries
```

**Lines changed:** ~70 lines.

---

#### Task F3: Handle service dependency properties in USED BY

**File:** `kloc-cli/src/queries/context.py`
**Location:** Inside `_build_property_used_by()` (from Task F2)
**Change:** Service dependencies (e.g., `$emailSender`) are used as receivers for method calls. The `_build_property_used_by()` already handles this through `calls` edges and receiver resolution. No additional code needed -- the general algorithm covers this case.

**Verification:** Ensure that `get_calls_to(property_id)` returns Call nodes where the property is accessed. These Call nodes have `produces` edges to result Values, which are then used as receivers for further method calls. The consumer chain traversal in ISSUE-E will pick up the method calls on the receiver.

---

## Critical Path and Task Order

```
Stream 1 (Developer-1):              Stream 2 (Developer-2):
D1 (schema)                          C1 (Part 1 access chain)
D2 (mapper Edge)                     C2 (Part 2 access chain)
D3 (mapper store param)              C3 (Part 3 access chain)
D4 (FQN fix)                         C4 (optional helper)
D5 (CLI EdgeData)                    |
D6 (graph index load)                | (wait for D to complete)
D7 (CLI use param)                   |
D8 (Value preference)                E1 (crossed_from field)
D9 (tests)                           E2 (JSON serialization)
                                     E3 (tree rendering)
                                     E4 (USED BY cross-method)
                                     E5 (USES cross-method)
                                     E6 (optional: param index)
                                     F1 (Property USES)
                                     F2 (Property USED BY)
                                     F3 (verify service deps)
```

**Estimated task sizes:**
- D1-D9: 9 tasks, ~80 lines changed, ~1.5 days
- C1-C4: 4 tasks, ~65 lines changed, ~0.5 days
- E1-E6: 6 tasks, ~120 lines changed, ~2 days
- F1-F3: 3 tasks, ~95 lines changed, ~1.5 days

**Total:** ~360 lines of production code changes.

## Risk Areas

### 1. `context.py` merge conflicts (MEDIUM)

Both developers touch `context.py`. Ownership is split by function but adjacent lines could conflict. Mitigation: Developer-2 finishes C1-C4 before D completes, so there's a clean merge point before E starts.

### 2. `get_arguments()` return type change (LOW)

Task D6 changes the return type from 3-tuple to 4-tuple. All callers must be updated atomically. There are 2 call sites in `context.py` and potentially in tests.

### 3. Argument FQN change breaks tests (LOW)

Task D4 changes all Argument FQNs from `method()::$param` to `method().$param`. Any snapshot tests or assertions on Argument FQNs must be updated. The mapper integration test (`test_mapper.py`) uses real artifacts and should be re-run.

### 4. Cross-method recursion performance (LOW)

ISSUE-E's `_build_parameter_uses()` iterates all edges to find matching `parameter` fields. For the reference project (~1200 edges), this is fast. For large codebases, Task E6 (parameter index) mitigates this.

### 5. Cycle detection correctness (MEDIUM)

ISSUE-E introduces recursive cross-method traversal. The visited set must track Value IDs, not method IDs, because the same method can be entered for different parameters. The depth limit (default 5) provides a hard backstop.

### 6. sot.json regeneration required (LOW)

After ISSUE-D mapper changes, sot.json must be regenerated from the reference project for ISSUE-E/F development and testing. The regeneration command: `cd kloc-mapper && uv run kloc-mapper map ../kloc-reference-project-php/artifacts/index.json -o ../kloc-reference-project-php/artifacts/sot.json --pretty`

## Verification Checkpoints

### After Stream 1 (ISSUE-D):
- [ ] Regenerated sot.json has `parameter` field on argument edges
- [ ] Argument FQNs use `.` separator
- [ ] CLI displays correct param names for named arguments
- [ ] Position fallback works for old sot.json
- [ ] Querying `createOrder().$input` finds Value node (not Argument)
- [ ] All existing tests pass

### After ISSUE-C:
- [ ] `send()` in Value USED BY shows `on: $this->emailSender [param]`
- [ ] Constructors do NOT show `on:` line
- [ ] JSON output includes access_chain/on_kind fields
- [ ] Method-level context output unchanged

### After ISSUE-E:
- [ ] Parameter USES shows callers at depth 1
- [ ] Value USED BY crosses into callee at depth+1
- [ ] Depth 5 shows 2-3 method boundary crossings
- [ ] Interface methods are terminal nodes (no crash)
- [ ] No infinite loops on recursive patterns
- [ ] Cycle detection works

### After ISSUE-F:
- [ ] `Order::$customerEmail` USES traces back to constructor callers
- [ ] `Order::$customerEmail` USED BY traces through OrderOutput to OrderResponse
- [ ] Service dependency USED BY shows method calls on receiver
- [ ] Property with no assigned_from shows empty USES
- [ ] All existing tests pass
