# Feature: Context Command USES Redesign

## Summary

The `context` command's USES section has 6 issues where the CLI underutilizes the Call/Value graph that the data pipeline already produces. Constructor calls show as generic `[type_hint]` instead of `[instantiation]`, parameter and return types are not distinguished, argument-to-parameter mappings are absent, local variables are invisible, value flow is hidden, and deeper levels show irrelevant structural dependencies. All 6 issues are kloc-cli presentation problems -- the data pipeline (scip-php + kloc-mapper) already produces all necessary data in sot.json v2.0. This feature fixes these issues across 4 phases, transforming USES from a structural reference list into an execution flow view.

## Issues

| # | Issue | Symptom | Root Cause |
|---|-------|---------|------------|
| 1 | Constructor calls shown as `[type_hint]` | `new Order(...)` displays as `App\Entity\Order [type_hint]` | `find_call_for_usage()` fails to match constructor Call nodes because `uses` edge targets the Class but Call's `calls` edge targets `__construct()` |
| 2 | No local variable references | `$order`, `$processedOrder`, `$savedOrder` never shown | Context query only traverses `uses` edges, never shows Value nodes |
| 3 | No param vs return type distinction | `CreateOrderInput` and `OrderOutput` both show as `[type_hint]` | `_infer_reference_type()` does not check whether the type_hint source is an Argument or Method node |
| 4 | No argument tracking displayed | `checkAvailability($input->productId, $input->quantity)` shows the call but not argument values | `get_arguments()` exists in index.py but is never called by the context query |
| 5 | Redundant deep nesting | Depth 2 shows `Order`'s internal properties (`$customerEmail`, `$status`) which are not relevant to the calling method | `_build_outgoing_tree()` recursively expands ALL deps, not just call-chain-relevant ones |
| 6 | Missing value flow context | No visibility into which variable holds what, what is passed where, how results chain | Context command uses the Call/Value graph only for access chains, not for value flow display |

## Phases

### Phase 1: Fix Reference Types (Issues 1, 3)

**Goal:** Constructor calls show as `[instantiation]`, parameter types show as `[parameter_type]`, return types show as `[return_type]`.

**Changes:**

1. **Fix constructor reference type (Issue 1):** In `find_call_for_usage()`, when matching a `uses` edge targeting a Class node, also search for Call nodes with `call_kind='constructor'` contained by the source method whose `calls` edge targets `ClassName::__construct()`. The `_call_matches_target()` helper already handles constructor-to-class resolution; the problem is that the Call node is not found in the first place.

2. **Distinguish param/return types (Issue 3):** In `_infer_reference_type()` or `_build_outgoing_tree()`, when the reference type is `type_hint`, check the source node kind of the type_hint edge:
   - Source is `Argument` node -> `"parameter_type"`
   - Source is `Method`/`Function` node -> `"return_type"`
   - Source is `Property` node -> `"property_type"`
   - Fallback -> `"type_hint"`

**Files modified:**
- `kloc-cli/src/queries/context.py` -- `find_call_for_usage()`, `_infer_reference_type()`
- `kloc-cli/src/output/tree.py` -- display new reference type labels (no structural changes, labels render as-is)

**Expected output after Phase 1:**
```
== USES ==
App\Service\OrderService::createOrder(CreateOrderInput $input): OrderOutput
+-- [1] App\Dto\CreateOrderInput [parameter_type] (src/Service/OrderService.php:28)
+-- [1] App\Dto\OrderOutput [return_type] (src/Service/OrderService.php:28)
+-- [1] App\Entity\Order [instantiation] (src/Service/OrderService.php:32)
+-- [1] App\Component\InventoryCheckerInterface::checkAvailability(...) [method_call]
|       on: $this->inventoryChecker
...
```

### Phase 2: Argument Tracking (Issue 4)

**Goal:** Each method call and constructor in USES shows its argument-to-parameter mappings and, when available, the local variable that receives the call's result.

**Changes:**

1. **New `ArgumentInfo` dataclass** in `results.py`:
   - `position: int` -- argument position (0-indexed)
   - `param_name: Optional[str]` -- formal parameter name from callee
   - `value_expr: Optional[str]` -- source expression text
   - `value_source: Optional[str]` -- "parameter", "local", "literal", "call_result"

2. **New fields on `ContextEntry`:**
   - `arguments: list[ArgumentInfo]` -- argument mappings for calls
   - `result_var: Optional[str]` -- local variable name receiving the call result

3. **Query logic in `context.py`:** After finding a Call node for a usage, call `index.get_arguments()` to retrieve `(value_node_id, position)` tuples. Resolve each value node to its expression and the formal parameter name from the callee's Argument children.

4. **Result variable resolution:** For each Call node, check if any Value node with `value_kind='local'` has an `assigned_from` edge pointing to the call's result (via `produces`).

**Files modified:**
- `kloc-cli/src/models/results.py` -- add `ArgumentInfo`, update `ContextEntry`
- `kloc-cli/src/queries/context.py` -- argument/result querying logic
- `kloc-cli/src/output/tree.py` -- render argument block and result variable
- `kloc-cli/src/server/mcp.py` -- serialize `arguments` and `result_var` in JSON

**Expected output after Phase 2:**
```
== USES ==
App\Service\OrderService::createOrder(CreateOrderInput $input): OrderOutput
+-- [1] App\Component\InventoryCheckerInterface::checkAvailability(...) [method_call]
|       on: $this->inventoryChecker
|       arguments:
|         $productId <- $input->productId
|         $quantity <- $input->quantity
+-- [1] App\Entity\Order [instantiation] (src/Service/OrderService.php:32)
|       arguments:
|         customerEmail <- $input->customerEmail
|         productId <- $input->productId
|         quantity <- $input->quantity
|         status <- 'pending'
|         createdAt <- new DateTimeImmutable()
|       result: $order
+-- [1] App\Component\AbstractOrderProcessor::process(...) [method_call]
|       on: $this->orderProcessor
|       arguments:
|         $order <- $order
|       result: $processedOrder
+-- [1] App\Repository\OrderRepositoryInterface::save(...) [method_call]
        on: $this->orderRepository
        arguments:
          $order <- $processedOrder
        result: $savedOrder
```

**JSON output additions (additive, backward-compatible):**
```json
{
  "arguments": [
    {"position": 0, "param_name": "$productId", "value_expr": "$input->productId", "value_source": "parameter"},
    {"position": 1, "param_name": "$quantity", "value_expr": "$input->quantity", "value_source": "parameter"}
  ],
  "result_var": "$order"
}
```

### Phase 3: Execution Flow (Issues 2, 6)

**Goal:** The USES section shows local variable assignments and value flow as part of execution-order display. Method calls are ordered by source line number, and local variables appear when they participate in data flow (passed as arguments, receive call results, or serve as receivers).

**Changes:**

1. **New traversal method `_build_execution_flow()`** in `context.py`:
   - Gets all Call nodes contained by the method via `contains` edges
   - Gets all Value nodes with `value_kind='local'` contained by the method
   - Orders entries by source line number for execution flow
   - For each Call node: resolves callee, arguments, result assignment
   - Shows local variables only when they participate in inter-symbol data flow

2. **Fallback strategy:**
   - For Method/Function nodes: use the new execution flow traversal
   - For Class/Interface/Trait nodes: keep existing structural `_build_outgoing_tree()`

3. **Value flow display:** Shows how values chain between calls:
   - `$order = new Order(...)` -- call result assignment
   - `process($order)` -- local variable passed as argument
   - `$processedOrder = process(...)` -- result of one call feeds into next

**Files modified:**
- `kloc-cli/src/queries/context.py` -- new `_build_execution_flow()`, conditional dispatch
- `kloc-cli/src/models/results.py` -- minor model updates if needed
- `kloc-cli/src/output/tree.py` -- line-ordered rendering
- `kloc-cli/src/server/mcp.py` -- updated JSON structure

### Phase 4: Smart Depth (Issue 5)

**Goal:** At depth > 1 in USES, only expand into the callee method's execution flow (Call nodes), not its structural dependencies (type hints, extends, property definitions).

**Changes:**

1. **Call-chain-aware depth expansion:** When expanding a call at depth N to depth N+1, recursively call `_build_execution_flow()` on the callee method. Since execution flow only shows Call nodes, structural noise is naturally filtered.

2. **Depth semantics:**
   - Depth 1: Execution flow of the queried method (calls + arguments + results)
   - Depth 2: For each callee at depth 1, show its internal execution flow
   - Depth N: Continue recursively into the call stack
   - USED BY direction: unchanged (structural depth model)

3. **Cycle prevention:** Track visited method IDs to prevent infinite recursion on recursive calls.

**Files modified:**
- `kloc-cli/src/queries/context.py` -- depth expansion logic in `_build_execution_flow()`

## Acceptance Criteria

### Phase 1

1. GIVEN a method that calls `new Order(...)` WHEN running `context` on that method THEN the Order entry shows `[instantiation]` not `[type_hint]`
2. GIVEN a method with parameter type `CreateOrderInput` WHEN running `context` THEN the type shows `[parameter_type]`
3. GIVEN a method with return type `OrderOutput` WHEN running `context` THEN the type shows `[return_type]`
4. GIVEN a property with type hint `LoggerInterface` WHEN running `context` on the containing class THEN the type shows `[property_type]`
5. GIVEN the existing CLI tree output format WHEN Phase 1 is applied THEN the reference type labels change but the tree structure is unchanged
6. GIVEN `--json` output WHEN Phase 1 is applied THEN `reference_type` field values include the new types (`parameter_type`, `return_type`, `instantiation`)

### Phase 2

7. GIVEN a method call `checkAvailability($input->productId, $input->quantity)` WHEN running `context` THEN the call entry shows argument mappings with parameter names and value expressions
8. GIVEN a constructor `new Order(customerEmail: $input->customerEmail, ...)` WHEN running `context` THEN the instantiation entry shows argument mappings
9. GIVEN a call whose result is assigned to `$order` WHEN running `context` THEN the call entry shows `result: $order`
10. GIVEN a call with no arguments WHEN running `context` THEN no `arguments:` block is shown
11. GIVEN `--json` output WHEN Phase 2 is applied THEN each call entry has an `arguments` array (may be empty) and optional `result_var` field
12. GIVEN an existing MCP consumer that ignores unknown fields WHEN Phase 2 JSON is returned THEN the consumer does not break (additive fields only)

### Phase 3

13. GIVEN a method body with calls on lines 30, 32, 42, 45 WHEN running `context` THEN the USES entries appear in line-number order (30, 32, 42, 45)
14. GIVEN a local variable `$order` that receives a constructor result AND is later passed as argument to `process()` WHEN running `context` THEN both the result assignment and argument usage are visible
15. GIVEN a local variable that is never passed to another call WHEN running `context` THEN the variable is NOT shown (only data-flow-relevant variables appear)
16. GIVEN a class-level context query (not a method) WHEN running `context` THEN the existing structural USES view is preserved
17. GIVEN `--json` output WHEN Phase 3 is applied THEN the JSON structure includes execution flow with line-ordered entries

### Phase 4

18. GIVEN `context "OrderService::createOrder" --depth 2` WHEN the method uses `Order` via `new Order()` THEN depth 2 shows what `Order::__construct()` does internally, NOT `Order`'s property type hints
19. GIVEN `context --depth 2` WHEN a callee method itself calls other methods THEN those inner calls appear at depth 2 with their arguments
20. GIVEN a recursive method that calls itself WHEN expanding with `--depth 2` THEN the recursive call is shown but not expanded further (cycle prevention)
21. GIVEN USED BY direction with `--depth 2` WHEN running `context` THEN USED BY depth semantics are unchanged (structural expansion)

## Files Modified

All changes are confined to `kloc-cli/`:

| File | Phase(s) | What Changes |
|------|----------|-------------|
| `src/queries/context.py` | 1, 2, 3, 4 | Reference type fixes, argument querying, execution flow traversal, smart depth |
| `src/models/results.py` | 2 | Add `ArgumentInfo` dataclass, add `arguments`/`result_var` fields to `ContextEntry` |
| `src/output/tree.py` | 1, 2, 3 | Render new reference type labels, argument blocks, line-ordered entries |
| `src/server/mcp.py` | 2, 3 | Serialize `arguments`, `result_var`, execution flow in JSON |

## Backward Compatibility

**JSON output changes are additive:**
- New fields: `arguments` (array), `result_var` (string) on USES entries -- absent when not applicable
- Existing consumers that ignore unknown fields will not break

**Breaking change (documented):**
- `reference_type` value `"type_hint"` splits into `"parameter_type"`, `"return_type"`, `"property_type"` (Phase 1)
- Consumers matching on `reference_type == "type_hint"` for method signatures will need updating
- `"type_hint"` remains valid for cases that are not param/return/property types

**Recommended consumer migration:**
- Treat `parameter_type`, `return_type`, `property_type` as subtypes of the former `type_hint`
- Match on the set `{"type_hint", "parameter_type", "return_type", "property_type"}` for backward compatibility

## Out of Scope

- No changes to `scip-php` (PHP indexer)
- No changes to `kloc-mapper` (graph builder)
- No changes to `kloc-contracts/` (JSON schemas -- schemas may be updated separately after feature ships)
- No new CLI commands or flags (enhances existing `context` command)
- No conditional branch analysis (if/else affecting which calls are made)
- No loop context display (call inside foreach vs one-time call)
- No exception handling context (try/catch around calls)
