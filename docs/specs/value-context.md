# Feature: Value Context — Value Node Data Flow and Definition Enhancement

## Summary

The `context` command produces high-quality output for Method and Function queries (execution flow, argument tracking, receiver chains). However, querying a **Value** node (local variable, parameter, or result) returns an empty context — both USES and USED BY sections are blank, and the DEFINITION section shows only minimal metadata.

This feature adds Value-specific traversal and definition logic to the CLI so that users and AI agents can trace data flow from the perspective of individual variables: where a value comes from (USES = source chain) and where it flows to (USED BY = consumer chain).

All graph data already exists in sot.json (411 Value nodes, 6 edge types, ~1,350 connections in the reference project). The gap is purely in CLI query traversal and output rendering. No changes to scip-php, kloc-mapper, or kloc-contracts are required.

## Issues

| ID | Title | Priority | Effort | Component |
|----|-------|----------|--------|-----------|
| ISSUE-A | Value node data flow traversal (USES/USED BY) | P1 | L | kloc-cli |
| ISSUE-B | Value node definition enhancement | P2 | S | kloc-cli |

Priority: P1 (must fix), P2 (should fix). Effort: S (< 1 day), L (3+ days).

### ISSUE-A: Value Node Data Flow Traversal

Adds Value-specific branches to `_build_outgoing_tree()` and `_build_incoming_tree()`. Uses consistent USES/USED BY semantics:

- **USES** = source chain: what this value depends on (how it was created). Traces `assigned_from` -> `produces` -> Call -> arguments recursively with depth.
- **USED BY** = consumer chain: what depends on this value (property accesses, argument passes). Traces receiver/argument edges -> consuming Calls -> forward into callee body with depth.

New methods: `_build_value_source_chain()` (USES) and `_build_value_consumer_chain()` (USED BY). Requires new reverse-lookup graph API methods: `get_calls_with_receiver(value_id)`, `get_calls_with_argument(value_id)`.

### ISSUE-B: Value Node Definition Enhancement

Adds `_build_value_definition()` method to populate Value-specific metadata in DefinitionInfo: `value_kind` (local/parameter/result/literal/constant), type info (from `type_of` edges), source assignment (from `assigned_from` + `get_source_call()` chain), and containing scope (from `contains` parent).

New fields on DefinitionInfo: `value_kind`, `type_info`, `source`. Updates to tree and JSON renderers to display these fields.

## Acceptance Criteria

### ISSUE-A: Data Flow Traversal

1. GIVEN a local variable Value node, WHEN querying with `kloc context`, THEN the USES section shows the source call that produced this value at depth 1.
2. GIVEN `--depth >= 2`, WHEN querying a Value, THEN USES traces recursively: source call -> its arguments -> their source calls -> deeper.
3. GIVEN a parameter Value node, WHEN querying, THEN USES is empty (parameters receive data from callers, not from assignments within the method).
4. GIVEN a Value with receiver edges, WHEN querying, THEN USED BY shows all Calls that access properties on this value, grouped by consuming Call, sorted by line number.
5. GIVEN a receiver access at depth 1 that feeds into another Call as argument, WHEN `--depth >= 2`, THEN USED BY traces forward into the callee (e.g., promoted property, further usage).
6. GIVEN a Value used directly as argument to a Call (not via property access), WHEN querying, THEN USED BY shows that Call with the parameter mapping.
7. GIVEN `--json` flag, WHEN querying a Value node, THEN JSON output uses the same ContextEntry structure as method queries (same fields, same nesting).
8. GIVEN a Value node with no receiver or argument edges (e.g., a literal), WHEN querying, THEN USED BY section is empty.
9. GIVEN a result Value node (produced by a Call), WHEN querying, THEN USES shows the producing Call at depth 1.
10. Existing Method/Function context queries continue to work identically (no regression).
11. USES entries reuse existing argument display format (`_get_argument_info`, `_format_argument_lines`).
12. USED BY entries are sorted by source line number.

### ISSUE-B: Definition Enhancement

13. GIVEN a local variable Value node, WHEN querying, THEN DEFINITION shows "Kind: Value (local)".
14. GIVEN a parameter Value node, WHEN querying, THEN DEFINITION shows "Kind: Value (parameter)".
15. GIVEN a result Value node, WHEN querying, THEN DEFINITION shows "Kind: Value (result)".
16. GIVEN a Value with a `type_of` edge, WHEN querying, THEN DEFINITION shows "Type: {class FQN}".
17. GIVEN a Value with no `type_of` edge, WHEN querying, THEN DEFINITION omits the Type line.
18. GIVEN a local variable with `assigned_from` -> `produces` chain, WHEN querying, THEN DEFINITION shows "Source: {method_name}() result (line N)".
19. GIVEN a parameter Value, WHEN querying, THEN DEFINITION omits Source (parameters come from callers).
20. GIVEN any Value node, WHEN querying, THEN DEFINITION shows "Scope: {containing method FQN}".
21. GIVEN `--json` flag, WHEN querying a Value, THEN JSON output includes `value_kind`, `type`, and `source` fields in the definition object.
22. Existing Method/Function/Class/Property definitions continue to work identically (no regression).

## Output Examples

### Example 1: Local variable with source chain and consumers ($savedOrder)

**Before:**
```
$ kloc context 'App\Service\OrderService::createOrder().local$savedOrder@45' --depth 3

== DEFINITION ==
  App\Service\OrderService::createOrder().local$savedOrder@45
  Kind: Value
  File: src/Service/OrderService.php:45

== USES ==
  (empty)

== USED BY ==
  (empty)
```

**After:**
```
$ kloc context 'App\Service\OrderService::createOrder().local$savedOrder@45' --depth 3

== DEFINITION ==
  App\Service\OrderService::createOrder().local$savedOrder@45
  Kind: Value (local)
  Type: App\Entity\Order
  Source: save() result (line 45)
  Scope: App\Service\OrderService::createOrder()
  File: src/Service/OrderService.php:45

== USES ==
+-- [1] OrderRepositoryInterface::save() [method_call] (line 45)
          on: $this->orderRepository [param]
          args:
            ...$order (Order): `$processedOrder` ...local$processedOrder@42
    +-- [2] AbstractOrderProcessor::process() [method_call] (line 42)
              on: $this->orderProcessor [param]
              args:
                ...$order (Order): `$order` ...local$order@32
        +-- [3] Order::__construct() [instantiation] (line 32)
                  args:
                    ...$customerEmail: `$input->customerEmail`
                    ...$productId: `$input->productId`
                    ...$quantity: `$input->quantity`
                    ...$status: `'pending'` literal

== USED BY ==
+-- [1] EmailSenderInterface::send() (line 48)
          $savedOrder->customerEmail as $to
          $savedOrder->id in $subject
+-- [1] sprintf() (line 51)
          $savedOrder->id as arg #1
          $savedOrder->productId as arg #2
          $savedOrder->quantity as arg #3
+-- [1] OrderCreatedMessage::__construct() (line 58)
          $savedOrder->id as $orderId
    +-- [2] -> OrderCreatedMessage::$orderId (promoted property)
+-- [1] OrderOutput::__construct() (line 61)
          $savedOrder->id as $id
          $savedOrder->customerEmail as $customerEmail
          $savedOrder->productId as $productId
          $savedOrder->quantity as $quantity
          $savedOrder->status as $status
          $savedOrder->createdAt as $createdAt
```

### Example 2: Parameter with property access consumers ($input)

**Before:**
```
== DEFINITION ==
  App\Service\OrderService::createOrder().$input
  Kind: Value
  File: src/Service/OrderService.php:28

== USES ==
  (empty)

== USED BY ==
  (empty)
```

**After:**
```
== DEFINITION ==
  App\Service\OrderService::createOrder().$input
  Kind: Value (parameter)
  Type: App\Dto\CreateOrderInput
  Scope: App\Service\OrderService::createOrder()
  File: src/Service/OrderService.php:28

== USES ==
  (empty -- parameters receive data from callers, not from assignments)

== USED BY ==
+-- [1] InventoryCheckerInterface::checkAvailability() (line 30)
          $input->productId as $productId
          $input->quantity as $quantity
+-- [1] Order::__construct() (line 32)
          $input->customerEmail as $customerEmail
          $input->productId as $productId
          $input->quantity as $quantity
```

### Example 3: Local variable passed as argument ($processedOrder)

**Before:** USES and USED BY both empty.

**After:**
```
== DEFINITION ==
  App\Service\OrderService::createOrder().local$processedOrder@42
  Kind: Value (local)
  Source: process() result (line 42)
  Scope: App\Service\OrderService::createOrder()
  File: src/Service/OrderService.php:42

== USES ==
+-- [1] AbstractOrderProcessor::process() [method_call] (line 42)
          on: $this->orderProcessor [param]
          args:
            ...$order (Order): `$order` ...local$order@32

== USED BY ==
+-- [1] OrderRepositoryInterface::save() (line 45)
          $processedOrder as $order arg
```

### Example 4: Method query (no change -- regression check)

Existing Method/Function queries continue using `_build_execution_flow()`. No change to output format, structure, or content.

## Edge Cases

1. **Parameter Values**: USES is empty (source is callers, shown in USED BY of the containing method). USED BY shows all consumption within the method body.
2. **Literal values**: Both USES and USED BY may be empty. USES has no source chain; USED BY only shows consumers if the literal is used as argument.
3. **Result values without variable name**: `file:line:(result)` -- USES shows the producing Call, USED BY shows where the result flows.
4. **Promoted constructor parameters**: USED BY shows the promoted property assignment as depth 1, and usages of that property at depth 2+. Source shows "promotes to Property::$name".
5. **Chained property access**: `$savedOrder->createdAt->format('c')` -- at depth 2+ in USED BY, the result of `$savedOrder->createdAt` becomes receiver of `format()`.
6. **Multiple assignments to same variable**: Each assignment creates a separate Value node with a different `@line` suffix, so they are distinct queries with distinct source chains.
7. **Union types**: A Value may have multiple `type_of` edges. Show all types pipe-separated: "Type: User|null".
8. **Values with no consumers**: Variable assigned but never used -- USES shows source chain, USED BY is empty.
9. **Constant values**: value_kind is "constant". No type or source. Show "Kind: Value (constant)" with file/line only.

## Non-Goals

The following are explicitly out of scope for this iteration:

1. **No changes to scip-php, kloc-mapper, or kloc-contracts** -- this is a CLI-only feature. All graph data already exists.
2. **No new CLI commands or flags** -- uses existing `kloc context` command with existing `--depth` and `--json` flags.
3. **No cross-method data flow linking** -- tracing a parameter backwards to all callers is not in scope (this would require inverse call graph traversal).
4. **No new section names** -- USES and USED BY headers remain the same; semantics are extended to cover Value nodes.
5. **No modeling of built-in functions** -- `sprintf()`, string concatenation, and other PHP built-ins are not modeled as Call nodes in the graph.
6. **No `--impl-limit` or similar new flags** -- depth limiting uses the existing `--depth` parameter.
7. **No changes to Class, Interface, Trait, Enum, or Property context queries** -- these continue to work as-is.

## Key Code Locations

| Component | File | Function / Area |
|-----------|------|-----------------|
| Outgoing tree (USES) | `kloc-cli/src/queries/context.py` | `_build_outgoing_tree()` |
| Incoming tree (USED BY) | `kloc-cli/src/queries/context.py` | `_build_incoming_tree()` |
| Definition builder | `kloc-cli/src/queries/context.py` | `_build_definition()` |
| Execution flow builder | `kloc-cli/src/queries/context.py` | `_build_execution_flow()` |
| Source chain tracer | `kloc-cli/src/queries/context.py` | `_trace_source_chain()` |
| Argument info builder | `kloc-cli/src/queries/context.py` | `_get_argument_info()` |
| Graph API | `kloc-cli/src/graph/index.py` | `get_assigned_from()`, `get_type_of()`, `get_source_call()`, `get_receiver()`, `get_arguments()` |
| Result models | `kloc-cli/src/models/results.py` | `ContextEntry`, `DefinitionInfo`, `MemberRef`, `ArgumentInfo` |
| Tree rendering | `kloc-cli/src/output/tree.py` | `add_context_children()` |
| JSON rendering | `kloc-cli/src/output/tree.py` | `context_entry_to_dict()` |

## Related Documents

- v4 spec: `docs/specs/cli-context-fix-v4.md`
- v5 todo: `docs/todo/context-command-v5/`
- ISSUE-A details: `docs/todo/context-command-v5/issue-a-value-data-flow/`
- ISSUE-B details: `docs/todo/context-command-v5/issue-b-value-definition/`
