# Feature: Data Flow Context Improvements (v6)

## Summary

The `cli-context-fix` v5 feature shipped Value node data flow traversal (USES/USED BY) and definition enhancement for the `kloc context` command. Users can now query individual variables and trace source chains (USES) and consumer chains (USED BY) within a single method. However, four gaps remain: consuming Calls in USED BY omit the `on:` receiver access chain, argument-to-parameter matching uses fragile position-based reconstruction instead of the direct link scip-php provides, data flow tracing stops at method boundaries, and Property nodes carry no cross-method data flow. This v6 batch addresses all four issues to deliver full cross-method data lineage from HTTP input to output.

## Background

The v5 iteration (see `docs/specs/value-context.md`) added Value-specific branches to `_build_outgoing_tree()` (USES) and `_build_incoming_tree()` (USED BY), along with Value definition enhancement. It introduced `_build_value_source_chain()` and `_build_value_consumer_chain()` methods, new reverse-lookup graph API methods, and Value-specific definition fields. All graph data was already present in sot.json -- the v5 work was CLI-only.

The v6 batch builds on this foundation. Issue D adds a `parameter` field to argument edges (the only mapper/schema change), enabling Issues E and F to implement cross-method traversal entirely in the CLI. Issue C is a small CLI-only rendering fix that can be done in parallel.

## Issues

| ID | Title | Priority | Effort | Components | Depends On |
|----|-------|----------|--------|------------|------------|
| ISSUE-C | Missing receiver access chain on USED BY consuming Calls | P2 | S | kloc-cli | -- |
| ISSUE-D | Argument-parameter linking (preserve scip-php parameter symbol + FQN fix + Argument-to-Value resolution) | P1 | M | kloc-contracts + kloc-mapper + kloc-cli | -- |
| ISSUE-E | Cross-method parameter tracing (USES + USED BY across method boundaries) | P1 | M | kloc-cli | ISSUE-D |
| ISSUE-F | Property cross-method tracing (who sets / who reads across methods) | P1 | M | kloc-cli | ISSUE-E |

Priority: P1 (must fix), P2 (should fix). Effort: S (< 1 day), M (1-3 days).

## Issue Details

### ISSUE-C: Missing Receiver Access Chain on USED BY Consuming Calls

**What's wrong:**

When querying a Value node, the USED BY section shows consuming Calls but omits the `on:` receiver access chain annotation. Method-level execution flow entries correctly render `on: $this->emailSender [param]` for `send()`, but Value USED BY rendering builds `MemberRef` objects with `access_chain=None` and `access_chain_symbol=None`, and never populates `on_kind`, `on_file`, or `on_line`.

The root cause is in `_build_value_consumer_chain()` which constructs `MemberRef` objects without calling `build_access_chain()`, `resolve_access_chain_symbol()`, or resolving receiver identity -- code that `_build_execution_flow()` already uses correctly.

**What changes:**

- In `_build_value_consumer_chain()` (all three parts: grouped consumers, standalone accesses, direct arguments), add receiver chain resolution before `MemberRef` construction
- Call `build_access_chain()` and `resolve_access_chain_symbol()` on each consuming Call
- Resolve receiver identity (`on_kind`, `on_file`, `on_line`) using `get_receiver()` pattern from `_build_execution_flow()`
- Optional: extract `_resolve_receiver_identity()` helper to avoid code duplication

**Acceptance criteria:**

1. GIVEN a Value USED BY entry that is a method_call (e.g., `send()`), WHEN the consuming Call has a receiver, THEN the entry's `member_ref` contains `access_chain`, `access_chain_symbol`, `on_kind`, `on_file`, and `on_line` matching the receiver.
2. GIVEN a Value USED BY entry that is an instantiation (e.g., `OrderCreatedMessage::__construct()`), WHEN the consuming Call has no receiver, THEN the entry's `member_ref` has `access_chain=None` and no `on:` line is rendered.
3. GIVEN a Value USED BY entry that is a static_call, WHEN the consuming Call has no receiver, THEN the entry's `member_ref` has `access_chain=None` and no `on:` line is rendered.
4. GIVEN `--json` output for a Value USED BY, WHEN the consuming Call has a receiver, THEN the JSON `member_ref` object includes `access_chain`, `access_chain_symbol`, `on_kind`, `on_file`, and `on_line` fields.
5. GIVEN the same Call appearing in both method-level USES and Value-level USED BY, THEN the `on:` annotation is identical in both views (same access chain text, same symbol, same location).
6. GIVEN the fix is applied, WHEN querying a Method/Function node, THEN the output is unchanged (no regression).
7. All three parts of `_build_value_consumer_chain()` (grouped consumers, standalone accesses, direct arguments) populate access chain fields.

### ISSUE-D: Argument-Parameter Linking

**What's wrong:**

scip-php emits a `parameter` field on each call argument in index.json, providing a direct SCIP symbol link to the formal parameter. The mapper discards this field and only stores `position` + `expression` on argument edges. The CLI then reconstructs the link by position-based matching (`_resolve_param_name()`), which breaks with PHP named arguments, requires complex fallback logic for promoted constructors, and prevents cross-method parameter tracing.

Additionally, Argument node FQNs use `::` separator (`save()::$order`) while Value nodes use `.` separator (`save().$order`). After the FQN fix, both node types share the same FQN, allowing the CLI to prefer Value nodes (which carry data flow edges) over Argument nodes (which only carry structural edges) when searching by symbol.

**What changes:**

- **kloc-contracts (sot-json.json):** Add `parameter` field to edge schema -- FQN of the formal parameter for argument edges
- **kloc-mapper (models.py):** Add `parameter: Optional[str]` field to Edge dataclass
- **kloc-mapper (calls_mapper.py):** Read `parameter` from scip-php index.json, resolve SCIP symbol to FQN, store on argument edges
- **kloc-mapper (calls_mapper.py):** Fix Argument node FQN separator from `::` to `.`
- **kloc-cli (context.py):** Read `parameter` from edge with position-based fallback for old sot.json files
- **kloc-cli:** When symbol search finds both Argument and Value(parameter) with same FQN, prefer Value for data flow display

**Acceptance criteria:**

1. GIVEN a call with arguments in index.json WHEN the mapper creates argument edges THEN the `parameter` field is populated with the formal parameter's FQN.
2. GIVEN a named argument `send(subject: $x)` WHEN the mapper processes it THEN the `parameter` field contains `send().$subject` regardless of position.
3. GIVEN the sot-json.json schema WHEN validated THEN the `parameter` field is accepted on edge objects.
4. GIVEN a sot.json with `parameter` fields WHEN the CLI resolves argument info THEN it reads from the `parameter` field directly instead of position matching.
5. GIVEN a sot.json WITHOUT `parameter` fields (old format) WHEN the CLI resolves argument info THEN it falls back to position-based matching (backward compatible).
6. GIVEN an Argument node WHEN the mapper builds its FQN THEN it uses `.` separator: `method().$param` not `method()::$param`.
7. GIVEN existing tests WHEN running after the change THEN no regressions in Method/Function context queries.
8. GIVEN a promoted constructor `new Order(...)` WHEN the CLI resolves arguments THEN the `parameter` field points to the promoted Property FQN (same resolution, no special fallback needed).
9. GIVEN `--json` output WHEN viewing argument edges THEN the `parameter` field appears in the JSON.
10. GIVEN the Edge model WHEN serializing/deserializing THEN the `parameter` field round-trips correctly.
11. GIVEN a user queries `createOrder().$input` WHEN both Argument and Value(parameter) nodes exist with that FQN THEN the CLI prefers the Value node for data flow display.
12. GIVEN a user queries a parameter FQN WHEN only an Argument node exists (no Value) THEN the CLI falls back to Argument-only info (type_hint, containment).

### ISSUE-E: Cross-Method Parameter Tracing

**What's wrong:**

The context command stops at method boundaries. When a Value is passed as an argument to a method call, USED BY shows the Call but doesn't trace into the callee to show what happens to the parameter. When a parameter Value has no local sources, USES shows "None" instead of tracing back to callers. The graph data supports full cross-method traversal -- this is the main goal of the v6 batch.

Two directions are needed: USES (backward -- find callers of a parameter via argument edges with `parameter` FQN from ISSUE-D) and USED BY (forward -- cross into the callee, find the matching parameter Value, trace its consumers recursively). Boundary crossing costs 1 depth unit. Cycle detection via visited set prevents infinite loops.

**What changes:**

- Add FQN-to-node-ID lookup for Value nodes (verify graph index supports it)
- **USED BY direction:** When a Value is an argument target, read `parameter` field from argument edge, look up callee Value by FQN, recurse with depth+1
- **USES direction:** For parameter Values, search argument edges by `parameter` field, trace caller Values with depth+1
- **Return value path:** Follow `produces` edges to result Values, then `assigned_from` to caller locals, continue tracing
- Add cycle detection via visited set and depth limit enforcement (boundary crossing = +1 depth)
- Add `crossed_from` field to ContextEntry model for boundary crossing indicator
- Update tree rendering for boundary crossing display

**Acceptance criteria:**

1. GIVEN a parameter Value with no local sources WHEN callers exist with argument edges (with `parameter` field) THEN USES shows caller-provided values at depth 1 (boundary crossing).
2. GIVEN a Value passed as argument to a Call WHEN the callee has a method body THEN USED BY crosses into the callee and shows the parameter's consumers at depth +1.
3. GIVEN depth=5 and a chain crossing 3 method boundaries THEN the tree shows all 3 crossings with correct depth numbering.
4. GIVEN an interface method with no body WHEN USED BY tries to cross into it THEN it shows the method as a terminal node (no crash, no empty children).
5. GIVEN a recursive method calling itself WHEN depth limit is reached THEN the tree stops (no infinite loop).
6. GIVEN a promoted constructor parameter WHEN crossing into the constructor THEN the promoted property's consumers are included in the USED BY tree.
7. GIVEN multiple callers to the same method WHEN tracing parameter USES THEN all callers appear as separate branches at depth 1.
8. GIVEN a return value flowing back to the caller WHEN tracing USED BY THEN the return path is followed: callee return, result Value, assigned_to in caller, caller's consumers.
9. GIVEN `--json` output WHEN cross-method entries are included THEN each entry includes `crossed_from` field indicating the boundary crossing.
10. GIVEN an old sot.json without `parameter` field WHEN cross-method tracing is attempted THEN position-based fallback is used (may be inaccurate but doesn't crash).
11. GIVEN cycle detection WHEN a Value is visited twice in the same trace path THEN the second visit is skipped.
12. GIVEN named arguments with reordered positions WHEN crossing method boundary THEN `parameter` FQN matches correctly (not position).

### ISSUE-F: Property Cross-Method Tracing

**What's wrong:**

Property nodes (69 total in the reference project) carry no cross-method data flow. Querying `Order::$customerEmail` doesn't show who sets it (constructor callers via argument edges) or who reads it across methods (via `$obj->prop` accesses and their downstream consumers). Properties -- especially promoted constructor properties -- are central data carriers and should trace the full flow from HTTP input to output.

USES traces backward through constructor arguments: Property -> `assigned_from` edge -> Value(parameter) -> use ISSUE-E's caller tracing to find all constructor call sites and what they pass. USED BY traces forward through property accesses: find `calls` edges targeting the Property (166 exist), get the `produces` result Value of each access, then use ISSUE-E's consumer tracing recursively. Service dependency properties show method calls on the receiver.

**What changes:**

- Add Property-to-Value(parameter) resolution for promoted constructor properties via `assigned_from` edges (63 exist)
- Add property USES tracing: follow `assigned_from` edge to Value(parameter), then use ISSUE-E USES tracing to find callers
- Add property USED BY tracing: find `calls` edges targeting the Property (166 exist), get `produces` Value (result of access), use ISSUE-E USED BY tracing on that Value
- Handle service dependency properties: USED BY shows method calls on the receiver
- Handle properties with no `assigned_from` (graceful empty)

**Acceptance criteria:**

1. GIVEN a promoted constructor property WHEN querying USES THEN trace backward through constructor arguments to all call sites.
2. GIVEN a promoted property accessed as `$obj->prop` WHEN querying USED BY THEN show all access sites and trace each downstream.
3. GIVEN a property set by DI container WHEN querying USES THEN show constructor parameter as terminal (no crash).
4. GIVEN a property read in multiple methods WHEN querying USED BY THEN all access sites appear as separate branches.
5. GIVEN a mutable property with direct assignment WHEN querying USES THEN show assignment source values.
6. GIVEN `Order::$customerEmail` WHEN querying USED BY depth 5 THEN trace through OrderOutput, OrderController, OrderResponse (3 boundary crossings).
7. GIVEN a service dependency property WHEN querying USED BY THEN show method calls made on the service receiver.
8. GIVEN `--json` output WHEN property trace includes cross-method data THEN entries include `property_source` field.
9. GIVEN depth limit reached WHEN property has deep chains THEN tree stops cleanly.
10. GIVEN a property never read (dead property) WHEN querying USED BY THEN show empty tree (no crash).

## Cross-Component Impact

| Issue | scip-php | kloc-mapper | kloc-contracts | kloc-cli |
|-------|----------|-------------|----------------|----------|
| C | -- | -- | -- | CHANGE |
| D | -- | CHANGE | CHANGE | CHANGE |
| E | -- | -- | -- | CHANGE |
| F | -- | -- | -- | CHANGE |

Issue C is a CLI-only rendering fix. Issue D spans kloc-contracts (schema), kloc-mapper (store parameter FQN on argument edges, fix Argument FQN separator), and kloc-cli (read from edge, prefer Value over Argument in search). Issues E and F are CLI-only features. E depends on D's `parameter` field for cross-method matching. F depends on E's cross-method infrastructure and uses existing `assigned_from` (63 edges) and `calls` (166 edges) to trace property data flow.

## Implementation Phases

### Phase 1 -- Argument Linking Foundation (ISSUE-D) [M, P1]

Components: kloc-contracts + kloc-mapper + kloc-cli

1. kloc-contracts: add `parameter` field to sot-json.json edge schema
2. kloc-mapper/models.py: add `parameter` field to Edge dataclass
3. kloc-mapper/calls_mapper.py: store parameter FQN on argument edges (resolve SCIP symbol to FQN)
4. kloc-mapper/calls_mapper.py: fix Argument node FQN separator `::` to `.`
5. kloc-cli: read `parameter` from edge, position fallback for old sot.json
6. kloc-cli: prefer Value over Argument when both share FQN

**Enables:** Phase 3 (ISSUE-E), Phase 4 (ISSUE-F indirectly via Phase 3).

### Phase 2 -- Consumer Chain Fix (ISSUE-C) [S, P2]

Components: kloc-cli only. Can run in parallel with Phase 1 (no dependency).

1. Wire `build_access_chain()` into `_build_value_consumer_chain()` Parts 1/2/3
2. Add receiver identity resolution (`on_kind`, `on_file`, `on_line`)
3. Optional: extract `_resolve_receiver_identity()` helper for reuse

**Shared benefit:** The receiver resolution pattern will be reused by E and F when displaying consumers at deeper depths.

### Phase 3 -- Cross-Method Tracing (ISSUE-E) [M, P1]

Components: kloc-cli only. **Hard dependency: Phase 1 (ISSUE-D) must be complete.**

1. Add FQN-to-node-ID lookup for Value nodes (verify index supports it)
2. USED BY direction: when Value is argument target, read `parameter` field, look up callee Value, recurse with depth+1
3. USES direction: for parameter Values, search argument edges by `parameter` field, trace caller Values with depth+1
4. Return value path: `produces` -> `assigned_from` -> continue tracing
5. Cycle detection via visited set
6. Depth limit enforcement (boundary crossing = +1 depth)

**This is the main goal of v6.**

### Phase 4 -- Property Flow (ISSUE-F) [M, P1]

Components: kloc-cli only. **Hard dependency: Phase 3 (ISSUE-E) must be complete.**

1. Property USES: follow `assigned_from` edge (Property -> Value(parameter)) -> use ISSUE-E USES tracing on that Value
2. Property USED BY: find `calls` edges targeting Property (166 exist) -> get `produces` Value (result of access) -> use ISSUE-E USED BY tracing
3. Service dependency properties: USED BY shows method calls on receiver
4. Handle properties with no `assigned_from` (graceful empty)

### Dependency Graph

```
ISSUE-C --- independent (can start anytime) ---
                                               |
ISSUE-D --> ISSUE-E --> ISSUE-F                |
  |           |           |                    |
  |           |           +-- uses E's cross-  |
  |           |               method traversal |
  |           +-- uses D's `parameter` field   |
  |               on argument edges            |
  +-- foundation: adds `parameter` field,      |
      fixes Argument FQN                       |
                                               |
  Critical path: D -> E -> F                   |
  Parallel track: C (can start anytime)        |
```

## Key Code Locations

| Component | File | Function / Area | Line |
|-----------|------|-----------------|------|
| Value consumer chain builder | `kloc-cli/src/queries/context.py` | `_build_value_consumer_chain()` | ~1047 |
| Value source chain builder | `kloc-cli/src/queries/context.py` | `_build_value_source_chain()` | ~945 |
| Argument info builder | `kloc-cli/src/queries/context.py` | `_get_argument_info()` | ~1481 |
| Param name resolver | `kloc-cli/src/queries/context.py` | `_resolve_param_name()` | ~1415 |
| Param FQN resolver | `kloc-cli/src/queries/context.py` | `_resolve_param_fqn()` | ~1154 |
| Incoming tree (USED BY) | `kloc-cli/src/queries/context.py` | `_build_incoming_tree()` | ~658 |
| Execution flow builder | `kloc-cli/src/queries/context.py` | `_build_execution_flow()` | ~1298 |
| Access chain builder | `kloc-cli/src/queries/context.py` | `build_access_chain()` | ~28 |
| Chain symbol resolver | `kloc-cli/src/queries/context.py` | `resolve_access_chain_symbol()` | ~307 |
| Graph API: receiver | `kloc-cli/src/graph/index.py` | `get_receiver()` | ~408 |
| Graph API: arguments | `kloc-cli/src/graph/index.py` | `get_arguments()` | ~501 |
| Graph API: contains children | `kloc-cli/src/graph/index.py` | `get_contains_children()` | -- |
| Call target resolver | `kloc-cli/src/graph/index.py` | `get_call_target()` | -- |
| Argument rendering | `kloc-cli/src/output/tree.py` | `_format_argument_lines()` | ~147 |
| Tree rendering | `kloc-cli/src/output/tree.py` | `add_context_children()` | ~285 |
| Result models | `kloc-cli/src/models/results.py` | `ContextEntry`, `ArgumentInfo`, `MemberRef` | -- |
| Edge model | `kloc-mapper/src/models.py` | `Edge` dataclass | ~120 |
| Argument edge creation | `kloc-mapper/src/calls_mapper.py` | `_create_call_edges()` | ~188 |
| Argument FQN builder | `kloc-mapper/src/calls_mapper.py` | Argument node FQN construction | -- |
| Edge schema | `kloc-contracts/sot-json.json` | Edge `position`/`expression`/`parameter` properties | ~189 |

## Non-Goals

The following are explicitly out of scope for this iteration:

1. **No changes to scip-php** -- scip-php already emits all needed data (`parameter` symbols on arguments). No indexer changes required.
2. **No new CLI commands or flags** -- uses existing `kloc context` command with existing `--depth` and `--json` flags.
3. **No new section names** -- USES and USED BY headers remain the same; semantics are extended to cover cross-method traversal.
4. **No modeling of built-in functions** -- `sprintf()`, string concatenation, and other PHP built-ins are not modeled as Call nodes in the graph.
5. **No cross-method tracing without graph data** -- the CLI must NOT invent virtual relations. All cross-method traversal must be grounded in actual graph edges and the `parameter` field. If an edge or field does not exist in sot.json, the CLI must not synthetically create that relationship.
6. **No changes to Class, Interface, Trait, or Enum context queries** -- these continue to work as-is.
7. **No `--impl-limit` or similar new flags** -- depth limiting uses the existing `--depth` parameter.
