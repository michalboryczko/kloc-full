# Implementation Plan: Value Context — Value Node Data Flow and Definition Enhancement

**Date:** 2026-02-11
**Branch:** feature/cli-context-fix
**Spec:** docs/specs/value-context.md (22 ACs)
**Todo docs:** docs/todo/context-command-v5/

## Overview

### Problem Statement

Querying a Value node (local variable, parameter, or result) with `kloc context` returns empty USES and USED BY sections, and the DEFINITION section shows only minimal metadata. The graph data already exists — the gap is in CLI query traversal and output rendering.

Two issues:

| Issue | Title | Priority | Effort | ACs |
|-------|-------|----------|--------|-----|
| ISSUE-B | Value node definition enhancement | P2 | S | 13-22 (10 ACs) |
| ISSUE-A | Value node data flow traversal (USES/USED BY) | P1 | L | 1-12 (12 ACs) |

Total: 22 acceptance criteria across 2 issues. All changes are CLI-only.

### Constraints

- No changes to scip-php, kloc-mapper, or kloc-contracts.
- No new CLI commands or flags — uses existing `kloc context` with `--depth` and `--json`.
- No new section names — USES and USED BY headers remain the same.
- Existing Method/Function/Class/Property context queries must continue to work identically.
- All graph data already exists in sot.json (411 Value nodes, 6 edge types, ~1,350 connections).

---

## Codebase Summary

### Architecture (current state)

```
ContextQuery.execute()
  _build_definition()        → DEFINITION section
    _build_method_definition()    for Method/Function
    _build_class_definition()     for Class/Interface/Trait/Enum
    _build_property_definition()  for Property
    _build_argument_definition()  for Argument
    (no handler)                  for Value  ← GAP (ISSUE-B)

  _build_incoming_tree()     → USED BY section
    get_usages_grouped()     → incoming 'uses' edges
    Value nodes have no 'uses' edges  ← GAP (ISSUE-A)

  _build_outgoing_tree()     → USES section
    Method/Function → _build_execution_flow() + _get_type_references()
    Class/Interface → build_tree() → get_deps()
    Value → build_tree() → get_deps() → empty  ← GAP (ISSUE-A)
```

### Value Node Edge Patterns (from graph)

Value nodes connect through 6 edge types. They are never sources of `receiver`, `argument`, `produces`, or `contains`.

```
Edge Type        Value→Source    Value→Target    Traversal Direction
────────────────────────────────────────────────────────────────────
assigned_from    261             324             outgoing (value→value)
type_of          101               0             outgoing (value→class)
receiver           0             149             incoming (call→value)
argument           0             122             incoming (call→value)
produces           0             226             incoming (call→value)
contains           0             170             incoming (method→value)
```

Key insight: USES direction follows **outgoing** edges (assigned_from → produces → Call), while USED BY direction follows **incoming** edges (receiver, argument from Calls).

### Key Code Locations

| Component | File | Function / Area |
|-----------|------|-----------------|
| Definition builder | `kloc-cli/src/queries/context.py` | `_build_definition()` ~L525 |
| Incoming tree (USED BY) | `kloc-cli/src/queries/context.py` | `_build_incoming_tree()` ~L658 |
| Outgoing tree (USES) | `kloc-cli/src/queries/context.py` | `_build_outgoing_tree()` ~L1634 |
| Execution flow builder | `kloc-cli/src/queries/context.py` | `_build_execution_flow()` ~L1353 |
| Source chain tracer | `kloc-cli/src/queries/context.py` | `_trace_source_chain()` ~L1216 |
| Argument info builder | `kloc-cli/src/queries/context.py` | `_get_argument_info()` ~L1095 |
| Graph API | `kloc-cli/src/graph/index.py` | `get_assigned_from()`, `get_type_of()`, `get_source_call()`, etc. |
| Result models | `kloc-cli/src/models/results.py` | `DefinitionInfo`, `ContextEntry`, `MemberRef`, `ArgumentInfo` |
| Tree rendering | `kloc-cli/src/output/tree.py` | `print_definition_section()` ~L184, `add_context_children()` ~L290 |
| JSON rendering | `kloc-cli/src/output/tree.py` | `context_tree_to_dict()` ~L420 |

---

## Work Streams

### Stream A: Developer-1 — ISSUE-B (Definition) + ISSUE-A USES Direction

**File ownership:** `results.py`, `tree.py`, `context.py` bottom half (lines 525-657 and 1216-1786)

**Tasks (in order):**

#### Task A1: Add DefinitionInfo fields (results.py)
**File:** `kloc-cli/src/models/results.py`
**Change:** Add 3 new optional fields to DefinitionInfo dataclass:

```python
# After existing fields (line ~269):
value_kind: Optional[str] = None      # "local", "parameter", "result", "literal", "constant"
type_info: Optional[dict] = None      # {"fqn": ..., "name": ...}
source: Optional[dict] = None         # {"call_fqn": ..., "method_fqn": ..., "method_name": ..., "file": ..., "line": ...}
```

**ACs:** Foundation for 13-22

#### Task A2: Add _build_value_definition() method (context.py)
**File:** `kloc-cli/src/queries/context.py`
**Change:** Add new method after `_build_argument_definition()` (~L656):

```python
def _build_value_definition(self, node_id: str, node: NodeData, info: DefinitionInfo):
    """Populate definition for Value nodes with data flow metadata."""
    # value_kind from node.value_kind
    # type_info from get_type_of() / get_type_of_all()
    # source from get_assigned_from() → get_source_call() → get_call_target()
    # For result values without assigned_from: use get_source_call() directly
    # For promoted params: follow assigned_from to Property
```

**Also change:** `_build_definition()` at ~L569, add:
```python
elif node.kind == "Value":
    self._build_value_definition(node_id, node, info)
```

**ACs:** 13-22

#### Task A3: Update definition rendering (tree.py)
**File:** `kloc-cli/src/output/tree.py`
**Change:** Update `print_definition_section()` (~L184) to render:
- `Kind: Value (local)` instead of `Kind: Value`
- `Type: App\Entity\Order` (from type_info)
- `Source: save() result (line 45)` (from source)
- `Scope: App\Service\OrderService::createOrder()` (from declared_in)

**Also change:** `context_tree_to_dict()` (~L523) to serialize new DefinitionInfo fields to JSON:
- `value_kind`, `type` (from type_info), `source`

**ACs:** 13-22

#### Task A4: Add _build_value_source_chain() method (context.py)
**File:** `kloc-cli/src/queries/context.py`
**Change:** Add new method (insert near `_build_execution_flow`, ~L1580):

```python
def _build_value_source_chain(self, value_id: str, depth: int,
                               max_depth: int, limit: int) -> list[ContextEntry]:
    """Build source chain for a Value node (USES section).

    For local variables: follows assigned_from → produces → Call, then
    recursively traces each argument's source Value at depth+1.
    For parameters: returns empty (parameters receive data from callers).
    For result values: follows produces → Call at depth 1.
    """
```

**Algorithm:**
1. Check value_kind: if "parameter", return [] (AC 3)
2. Follow assigned_from to get source Value
3. If source is "result" value, follow get_source_call() to get producing Call
4. Build ContextEntry for the Call using existing patterns:
   - Reuse `get_reference_type_from_call()` for reference_type
   - Reuse `build_access_chain()` for on: display
   - Reuse `_get_argument_info()` for args display
   - Reuse `resolve_access_chain_symbol()` for access chain symbol
5. For depth+1: trace each argument's source Value recursively
   - If arg.value_kind == "local": find that Value node, call `_build_value_source_chain()` recursively
   - If arg.value_kind == "result": find producing Call, build entry

**Also change:** `_build_outgoing_tree()` at ~L1664, add:
```python
elif start_node and start_node.kind == "Value":
    return self._build_value_source_chain(start_id, 1, max_depth, limit)
```

**ACs:** 1, 2, 3, 9, 10, 11

---

### Stream B: Developer-2 — ISSUE-A USED BY Direction (Consumer Chain)

**File ownership:** `context.py` top half (lines 658-830)

**Tasks (in order):**

#### Task B1: Add _build_value_consumer_chain() method (context.py)
**File:** `kloc-cli/src/queries/context.py`
**Change:** Add new method (insert after `_build_incoming_tree`, ~L940):

```python
def _build_value_consumer_chain(self, value_id: str, depth: int,
                                 max_depth: int, limit: int) -> list[ContextEntry]:
    """Build consumer chain for a Value node (USED BY section).

    Finds all Calls using this Value as receiver (property accesses) or
    as argument (direct passes). Groups by consuming Call, sorted by line.
    At depth 2+: follows forward into callee body.
    """
```

**Algorithm:**
1. Collect consuming Calls from two sources:
   a. `incoming[value_id]["receiver"]` — Calls using this Value as receiver (property accesses like `$savedOrder->id`)
   b. `incoming[value_id]["argument"]` — Calls using this Value directly as argument (like `save($processedOrder)`)
2. For receiver edges: the Call accesses a property/method on this Value. Follow Call → `get_call_target()` to find what's accessed (e.g., Order::$id). Then find what *consumes* the result — follow Call → `get_produces()` → result Value → check if it's an argument to another Call.
3. Group entries by consuming Call: e.g., `EmailSenderInterface::send()` at line 48 with sub-entries showing which properties are accessed as which parameters.
4. For argument edges: the Value is passed directly as argument to a Call. Show the Call with parameter mapping.
5. Sort all entries by line number (AC 12).
6. Depth expansion (depth 2+): for consuming Calls that are constructors with promoted parameters, follow the promoted property → find usages of that property (AC 5).

**Display pattern for receiver-based entries:**
```
[1] EmailSenderInterface::send() (line 48)
      $savedOrder->customerEmail as $to
      $savedOrder->id in $subject
```

**Display pattern for argument-based entries:**
```
[1] OrderRepositoryInterface::save() (line 45)
      $processedOrder as $order arg
```

**ACs:** 4, 5, 6, 8, 12

#### Task B2: Add Value branch in _build_incoming_tree() (context.py)
**File:** `kloc-cli/src/queries/context.py`
**Change:** At the top of `_build_incoming_tree()` (~L704), add early return for Value nodes:

```python
# At the start of _build_incoming_tree(), before existing logic:
start_node = self.index.nodes.get(start_id)
if start_node and start_node.kind == "Value":
    return self._build_value_consumer_chain(start_id, 1, max_depth, limit)
```

This avoids entering the existing uses-edge-based logic which returns empty for Values.

**ACs:** 4, 5, 6, 8, 10, 12

#### Task B3: Verify tree.py rendering handles consumer entries (tree.py)
**File:** `kloc-cli/src/output/tree.py`
**Change:** Likely NO changes needed. The existing `add_context_children()` already handles all ContextEntry fields (member_ref, arguments, result_var, entry_type, etc.). Consumer chain entries use the same ContextEntry dataclass.

However, if the consumer chain needs a new display pattern for "property access as parameter" lines (e.g., `$savedOrder->id as $orderId`), a small rendering addition may be needed. This should be evaluated during implementation.

**Also verify:** `context_entry_to_dict()` serializes consumer entries correctly to JSON (AC 7).

**ACs:** 7

---

## Execution Order and Dependencies

```
Phase 1 (parallel start):
  Dev-1: Task A1 (results.py model changes)     [15 min, no dependencies]
  Dev-2: Task B1 (consumer chain method)         [can start immediately]

Phase 2 (after A1):
  Dev-1: Task A2 (definition builder)            [depends on A1]
  Dev-2: Task B2 (incoming tree branch)          [depends on B1]

Phase 3 (parallel):
  Dev-1: Task A3 (tree.py rendering)             [depends on A2]
  Dev-1: Task A4 (source chain method + branch)  [depends on A1, A2]
  Dev-2: Task B3 (verify tree rendering)         [depends on B1, B2]

Phase 4 (integration):
  Both: Run existing test suite to verify no regressions
  Both: Run manual tests against reference project sot.json
```

### File Conflict Analysis

| File | Dev-1 Edits | Dev-2 Edits | Conflict Risk |
|------|-------------|-------------|---------------|
| `results.py` | Add 3 fields to DefinitionInfo | None | None |
| `context.py` | Lines 525-657 (definition), 1580-1786 (outgoing tree/source chain) | Lines 658-940 (incoming tree/consumer chain) | **Low** — different methods, 500+ lines apart |
| `tree.py` | Lines 184-275 (definition section), 523-549 (JSON definition) | Lines 290-386 (tree rendering verification) | **Low** — different functions |
| `index.py` | None | None | None |

The only shared file is `context.py`, but edits target different methods with significant separation. Merge conflicts should be minimal.

---

## Acceptance Criteria Mapping

### Dev-1 (Stream A): ISSUE-B + ISSUE-A USES

| AC | Description | Task |
|----|-------------|------|
| 1 | Local var USES shows source call at depth 1 | A4 |
| 2 | USES traces recursively at depth >= 2 | A4 |
| 3 | Parameter USES is empty | A4 |
| 9 | Result value USES shows producing Call | A4 |
| 10 | No regression on Method/Function queries | A4 (verify) |
| 11 | USES reuses existing argument display format | A4 |
| 13 | Local var definition shows "Kind: Value (local)" | A2, A3 |
| 14 | Parameter definition shows "Kind: Value (parameter)" | A2, A3 |
| 15 | Result definition shows "Kind: Value (result)" | A2, A3 |
| 16 | Value with type_of shows Type line | A2, A3 |
| 17 | Value without type_of omits Type line | A2, A3 |
| 18 | Local var with assigned_from chain shows Source line | A2, A3 |
| 19 | Parameter omits Source line | A2, A3 |
| 20 | All Values show Scope line | A2, A3 |
| 21 | JSON includes value_kind, type, source | A3 |
| 22 | No regression on existing definitions | A2 (verify) |

### Dev-2 (Stream B): ISSUE-A USED BY

| AC | Description | Task |
|----|-------------|------|
| 4 | Value with receivers shows property access Calls in USED BY | B1, B2 |
| 5 | Depth >= 2 traces forward into callee | B1 |
| 6 | Value as direct argument shows Call with param mapping | B1, B2 |
| 7 | JSON output uses same ContextEntry structure | B3 |
| 8 | Value with no receiver/argument edges shows empty USED BY | B1, B2 |
| 10 | No regression on Method/Function queries | B2 (verify) |
| 12 | USED BY entries sorted by line number | B1 |

---

## Test Strategy

### Regression Tests (both devs)
- Run existing test suite: `uv run pytest tests/ -v`
- Verify Method/Function context queries produce identical output

### Manual Verification (reference project)
- `kloc context 'App\Service\OrderService::createOrder().local$savedOrder@45' --depth 3`
  - DEFINITION: Kind: Value (local), Type: Order, Source: save() result, Scope
  - USES: save() → process() → Order::__construct()
  - USED BY: EmailSenderInterface::send(), sprintf(), OrderCreatedMessage::__construct(), OrderOutput::__construct()

- `kloc context 'App\Service\OrderService::createOrder().$input' --depth 2`
  - DEFINITION: Kind: Value (parameter), Type: CreateOrderInput, no Source
  - USES: empty
  - USED BY: checkAvailability(), Order::__construct()

- `kloc context 'App\Service\OrderService::createOrder().local$processedOrder@42'`
  - DEFINITION: Kind: Value (local), Source: process() result
  - USES: process() call
  - USED BY: save() call

- `kloc context 'App\Service\OrderService::createOrder()' --depth 2`
  - REGRESSION CHECK: output identical to before
