# Feature: Context Command v2 — Remaining Issues

## Goal

The `context` command shipped its first round of USES improvements (execution-flow ordering, reference type inference, access chains, argument-to-parameter mapping, result variable tracking). Five gaps remain between the current output and the user's full vision for flow tracking. This spec covers those 5 issues, bringing the `context` command to its target output quality for all three user personas: AI coding agents (structured, precise data), human developers (quick code comprehension), and tech leads (system-level flow insight).

## Issues Summary

| ID | Title | Priority | Effort | Components | Depends On |
|----|-------|----------|--------|------------|------------|
| ISSUE-A | Preserve `value_expr` in sot.json pipeline | P1 | S | kloc-mapper + kloc-contracts + kloc-cli | -- |
| ISSUE-B | Display resolved types for argument values | P2 | S | kloc-cli only | -- |
| ISSUE-C | Variable-centric execution flow + chain dedup | P1 | L | kloc-cli only | -- (better with A) |
| ISSUE-D | Rich argument display with source chains | P3 | L | kloc-cli only | A, B, C |
| ISSUE-E | Definition section in context output | P2 | S | kloc-cli only | -- |

## Implementation Phases

### Phase 1: Foundation (ISSUE-A)

**Goal:** Preserve the `value_expr` field from scip-php's `ArgumentRecord` through the pipeline so kloc-cli can display the original source expression for each argument.

**Problem:** kloc-mapper silently drops `value_expr` when creating `argument` edges. The CLI falls back to Value node names, which show `(result)` for property accesses and `(literal)` for literal values.

**Changes:**
1. **kloc-contracts**: Add `expression` (optional string) to the `argument` edge definition in `sot-json.json`
2. **kloc-mapper**: In `calls_mapper.py` `_create_call_edges()`, read `value_expr` from each `ArgumentRecord` and store it as `expression` on the argument edge
3. **kloc-cli**: In `context.py` `_get_argument_info()`, prefer the edge's `expression` field over the Value node's `name`; update `get_arguments()` to return edge objects (not just tuples) so `expression` is accessible

**Files modified:**
- `kloc-contracts/sot-json.json` -- add `expression` to argument edge definition
- `kloc-mapper/src/calls_mapper.py` -- read `value_expr`, store on edge
- `kloc-mapper/src/models.py` -- add `expression` field to Edge model (if needed)
- `kloc-cli/src/queries/context.py` -- prefer edge expression over Value node name
- `kloc-cli/src/graph/index.py` -- update `get_arguments()` return type

### Phase 2: Core Visibility (ISSUE-C + ISSUE-E)

#### ISSUE-C: Variable-centric execution flow + chain dedup

**Goal:** Transform the execution flow from a call-centric model to a variable-centric model. Each step in the flow is either a **variable entry** (Kind 1: call result assigned to a local) or a **call entry** (Kind 2: result discarded). Chains are deduplicated so intermediate calls consumed as receivers or arguments appear nested inside their consumer, not as separate top-level entries.

**Problem:** Local variables (`$order`, `$processedOrder`, `$savedOrder`) are invisible. They only appear indirectly as `result ->` annotations or argument expressions. Users cannot trace data flow between calls.

**Entry types:**
- **Kind 1 (Variable entry):** When a call result is assigned to a local, the variable is the primary entry with the call nested as `source:`. Example: `[2] local#32$order (Order) [variable]`
- **Kind 2 (Call entry):** When a call result is discarded or void, the call is the primary entry (same shape as today). Example: `[1] InventoryCheckerInterface::checkAvailability() [method_call]`

**Chain deduplication rules:**
- Receiver chains: property_access Calls nested inside `on:`, not separate `[N]` entries
- Argument value chains: property_access Calls nested inside arg `source:`, not separate `[N]` entries
- One logical operation = one `[N]` entry

**Cross-referencing:** Arguments reference earlier variable entries by their graph symbol (e.g., `local#32$order`), enabling traceable data flow.

**Files modified:**
- `kloc-cli/src/queries/context.py` -- rewrite `_build_execution_flow()` for variable-centric model
- `kloc-cli/src/models/results.py` -- add entry type distinction (variable vs call), update model fields
- `kloc-cli/src/output/tree.py` -- render Kind 1 (variable) and Kind 2 (call) entries with nested source/on/args
- `kloc-cli/src/server/mcp.py` -- serialize `type: "local_variable"` vs `type: "call"` in JSON

#### ISSUE-E: Definition section

**Goal:** Add a DEFINITION section before USED BY that answers "What IS this symbol?" for every symbol type. Shows structure, typed members, and container.

**Problem:** The output jumps directly to USED BY and USES. No header shows the method's signature, parameter types, return type, or class structure. Users need separate queries for basic symbol metadata.

**Per symbol type:**
- **Class/Interface/Trait/Enum**: properties (name + type), methods (signature), inheritance
- **Method/Function**: signature, typed arguments, return type, containing class
- **Property**: name, type, visibility, containing class
- **Argument**: name, type, containing method, position

**Files modified:**
- `kloc-cli/src/queries/context.py` -- add `_build_definition()` method
- `kloc-cli/src/models/results.py` -- add `DefinitionInfo` model
- `kloc-cli/src/output/tree.py` -- render DEFINITION section
- `kloc-cli/src/server/mcp.py` -- serialize `definition` object in JSON

### Phase 3: Enrichment (ISSUE-B)

**Goal:** Add a `value_type` field to `ArgumentInfo` showing the resolved type of each argument value. Types are resolved from the Value node's `type_of` edge(s).

**Problem:** Argument display has no type information. Users cannot see that `$productId` is `string` or that `$order` is `Order` without separate queries.

**Changes:**
1. Add `value_type: Optional[str]` to `ArgumentInfo`
2. In `_get_argument_info()`, look up `type_of` edge(s) on the Value node
3. Union types (multiple `type_of` edges) display as `Type1|Type2`
4. Types use short names for classes (e.g., `Order` not `App\Entity\Order`)
5. When no `type_of` edge exists, the type is omitted (no placeholder)

**Files modified:**
- `kloc-cli/src/models/results.py` -- add `value_type` to `ArgumentInfo`
- `kloc-cli/src/queries/context.py` -- resolve types via `get_type_of()`
- `kloc-cli/src/output/tree.py` -- display type in parentheses: `$productId (string) <- $input->productId`
- `kloc-cli/src/server/mcp.py` -- serialize `value_type` in JSON

### Phase 4: Gold Standard (ISSUE-D)

**Goal:** Full rich argument display with formal parameter FQN, source expression, resolved value reference, and source access chains. This is the user's exact vision from the original requirements.

**Problem:** Arguments show only 2 parts (param name + value expression). The user's vision calls for a 4-part display: formal FQN, definition (expression), value (resolved symbol), source chain.

**Display depth rule:**
- Value is an earlier local variable (already a `[N]` entry) --> one-line reference by graph symbol
- Value is a method parameter --> one-line reference by symbol
- Value is a literal --> one-line inline with `literal` tag
- Value is a property access or expression (no entry of its own) --> expand source chain

**Model changes:** Extend `ArgumentInfo` with:
- `param_fqn: Optional[str]` -- full FQN from callee's Argument node
- `value_ref_symbol: Optional[str]` -- graph symbol the value resolves to
- `source_chain: Optional[list]` -- access chain steps when value has no top-level entry

**Files modified:**
- `kloc-cli/src/models/results.py` -- extend `ArgumentInfo` with new fields
- `kloc-cli/src/queries/context.py` -- resolve formal FQN, trace value chains
- `kloc-cli/src/output/tree.py` -- render rich argument display with nested source chains
- `kloc-cli/src/server/mcp.py` -- serialize `param_fqn`, `value_ref_symbol`, `source_chain`

## Cross-Component Impact Matrix

| Issue | scip-php | kloc-mapper | kloc-contracts | kloc-cli |
|-------|----------|-------------|----------------|----------|
| ISSUE-A | -- | CHANGE | CHANGE | CHANGE |
| ISSUE-B | -- | -- | -- | CHANGE |
| ISSUE-C | -- | -- | -- | CHANGE |
| ISSUE-D | -- | -- | -- | CHANGE |
| ISSUE-E | -- | -- | -- | CHANGE |

Only ISSUE-A touches the pipeline (mapper + contracts). All other issues are CLI-only. No scip-php changes are required for any issue.

## Acceptance Criteria

### Phase 1: ISSUE-A — Preserve value_expr

1. GIVEN scip-php produces an `ArgumentRecord` with `value_expr: "$input->productId"` WHEN kloc-mapper creates the argument edge THEN the sot.json edge includes `"expression": "$input->productId"`
2. GIVEN an argument edge with `expression` field WHEN `kloc-cli context` displays arguments THEN the output shows `$productId <- $input->productId` instead of `$productId <- (result)`
3. GIVEN a literal argument `status: 'pending'` WHEN kloc-cli displays arguments THEN the output shows `$status <- 'pending'` instead of `$status <- (literal)`
4. GIVEN a constructor call `new DateTimeImmutable()` as argument WHEN the output displays THEN it shows `$createdAt <- new DateTimeImmutable()` instead of `$createdAt <- (result)`
5. GIVEN a complex expression `'Order Confirmation #' . $savedOrder->id` WHEN the output displays THEN it shows the full expression text instead of `(result)`
6. GIVEN an argument edge WITHOUT `expression` field (older sot.json) WHEN kloc-cli displays arguments THEN it falls back to the Value node `name` (backward compatible)
7. GIVEN the `kloc-contracts/sot-json.json` schema WHEN validating a sot.json with `expression` on argument edges THEN validation passes
8. GIVEN `--json` output WHEN argument edges have `expression` THEN the JSON argument objects include `value_expr` with the expression text

### Phase 2a: ISSUE-C — Variable-centric flow + chain dedup

9. GIVEN `$order = new Order(...)` (call result assigned to local) WHEN running `context` THEN the entry is Kind 1: `[2] local#32$order (Order) [variable]` with the constructor nested as `source:`
10. GIVEN `$this->inventoryChecker->checkAvailability(...)` (result discarded) WHEN running `context` THEN the entry is Kind 2: `[1] InventoryCheckerInterface::checkAvailability() [method_call]` (call as primary entry)
11. GIVEN a receiver chain `$this->inventoryChecker->checkAvailability()` WHEN building entries THEN the property_access `$this->inventoryChecker` is nested inside `on:`, NOT a separate `[N]` entry
12. GIVEN an argument property_access `$input->productId` WHEN building entries THEN the property_access is nested inside the argument's source chain, NOT a separate `[N]` entry
13. GIVEN `$processedOrder = $this->orderProcessor->process($order)` where `$order` is entry [2] WHEN displaying the argument THEN the argument cross-references by graph symbol: `$order` `local#32$order`
14. GIVEN execution flow entries WHEN ordering THEN entries appear in source line number order
15. GIVEN a class-level context query (not a method) WHEN running `context` THEN the existing structural USES view is preserved (no variable-centric model for classes)
16. GIVEN `--json` output WHEN entries are Kind 1 THEN the JSON uses `"type": "local_variable"` with `source` object; when Kind 2, uses `"type": "call"`

### Phase 2b: ISSUE-E — Definition section

17. GIVEN a Method node `OrderService::createOrder()` WHEN running `context` THEN a DEFINITION section appears before USED BY showing: signature, typed arguments (`$input: CreateOrderInput`), return type (`OrderOutput`), defined-in class and file:line
18. GIVEN a Class node `OrderService` WHEN running `context` THEN the DEFINITION section shows: class name with modifiers, properties (name + type), methods (signatures), extends/implements
19. GIVEN a Property node `OrderService::$orderRepository` WHEN running `context` THEN the DEFINITION section shows: property name, type, visibility, containing class
20. GIVEN an Argument node `createOrder().$input` WHEN running `context` THEN the DEFINITION section shows: name, type, containing method, position
21. GIVEN a constructor method `Order::__construct()` WHEN running `context` THEN the DEFINITION section shows arguments and notes `(constructor)` as return type
22. GIVEN a symbol with no signature or type_hint edges WHEN running `context` THEN the DEFINITION section shows a minimal view: kind + FQN + file + line
23. GIVEN `--json` output WHEN running `context` THEN the JSON includes a `definition` object with `fqn`, `kind`, `signature`, `arguments`, `return_type`, `declared_in`, `file`, `line`

### Phase 3: ISSUE-B — Value type resolution

24. GIVEN an argument whose Value node has a `type_of` edge to `string` WHEN displaying arguments THEN the type appears in parentheses: `$productId (string) <- $input->productId`
25. GIVEN an argument whose Value node has a `type_of` edge to `App\Entity\Order` WHEN displaying arguments THEN the short name is used: `$order (Order) <- $order`
26. GIVEN an argument whose Value node has multiple `type_of` edges (union type) WHEN displaying THEN types are joined: `$value (string|int) <- $input->value`
27. GIVEN an argument whose Value node has NO `type_of` edge WHEN displaying THEN no type is shown (no placeholder): `$param <- $value`
28. GIVEN `--json` output WHEN argument values have types THEN each argument object includes `"value_type": "string"` (or null when absent)

### Phase 4: ISSUE-D — Rich argument display

29. GIVEN an argument WHEN displaying THEN the formal parameter FQN from the callee's Argument node is shown: `process().$order (Order): ...`
30. GIVEN an argument value that is an earlier local variable entry WHEN displaying THEN it is a one-line reference with graph symbol: `process().$order (Order): \`$order\` local#32$order`
31. GIVEN an argument value that is a method parameter WHEN displaying THEN it is a one-line reference: `validate().$data (CreateOrderInput): \`$input\` OrderService::createOrder().$input`
32. GIVEN a literal argument WHEN displaying THEN it is one-line inline: `Order::__construct().$id (int): \`0\` literal`
33. GIVEN an argument value from a property access with no top-level entry WHEN displaying THEN the source chain is expanded showing the property and the object it is accessed on
34. GIVEN a nested constructor as argument value WHEN displaying THEN the constructor and its arguments are expanded in the source chain
35. GIVEN `--json` output WHEN arguments have source chains THEN `value_ref_symbol` is set (points to entry) OR `source_chain` is set (access steps), never both
36. GIVEN incomplete chain tracing data WHEN displaying THEN the display shows what it can (FQN, expression, type) and omits what it cannot resolve (no errors, no `null` display)

## Output Examples

### Before (current output for createOrder)

```
App\Service\OrderService::createOrder()
  File: src/Service/OrderService.php
  Line: 28

== USED BY ==
  [1] OrderController::create() [method_call] (src/Ui/Rest/Controller/OrderController.php:25)

== USES (execution flow) ==
  [1] InventoryCheckerInterface::checkAvailability() [method_call]
      on: $this->inventoryChecker
      args:
        $productId <- (result)
        $quantity <- (result)
  [2] Order::__construct() [instantiation]
      result -> $order
      args:
        arg[0] <- (literal)
        arg[3] <- (literal)
  [3] AbstractOrderProcessor::process() [method_call]
      on: $this->orderProcessor
      result -> $processedOrder
      args:
        $order <- $order
  [4] AbstractOrderProcessor::getName() [method_call]
      on: $this->orderProcessor
      result -> $processorName
  [5] OrderRepositoryInterface::save() [method_call]
      on: $this->orderRepository
      result -> $savedOrder
      args:
        $order <- $processedOrder
  [6] EmailSenderInterface::send() [method_call]
      on: $this->emailSender
      args:
        $to <- (result)
        $subject <- (result)
        $body <- (result)
  [7] MessageBusInterface::dispatch() [method_call]
      on: $this->messageBus
      args:
        arg[0] <- (result)
  [8] OrderOutput::__construct() [instantiation]
      args:
        arg[0] <- (result)
        arg[1] <- (result)
        ...
```

### After Phase 1 (ISSUE-A: expression text preserved)

```
  [1] InventoryCheckerInterface::checkAvailability() [method_call]
      on: $this->inventoryChecker
      args:
        $productId <- $input->productId        (was: "(result)")
        $quantity <- $input->quantity           (was: "(result)")
  [2] Order::__construct() [instantiation]
      result -> $order
      args:
        $id <- 0                               (was: "(literal)")
        $status <- 'pending'                   (was: "(literal)")
        $createdAt <- new DateTimeImmutable()   (was: "(result)")
  [6] EmailSenderInterface::send() [method_call]
      args:
        $to <- $savedOrder->customerEmail       (was: "(result)")
        $subject <- 'Order Confirmation #' . $savedOrder->id  (was: "(result)")
        $body <- sprintf(...)                  (was: "(result)")
```

### After Phase 2 (ISSUE-C + ISSUE-E: variable-centric flow + definition)

```
App\Service\OrderService::createOrder()

== DEFINITION ==
createOrder(CreateOrderInput $input): OrderOutput
  Arguments:
    $input: CreateOrderInput
  Return type: OrderOutput
  Defined in: OrderService (src/Service/OrderService.php:28)

== USED BY ==
  [1] OrderController::create() [method_call] (src/Ui/Rest/Controller/OrderController.php:25)

== USES (execution flow) ==
[1] InventoryCheckerInterface::checkAvailability() [method_call]
    on: $this->inventoryChecker (OrderService::$inventoryChecker)
    args:
        checkAvailability().$productId: `$input->productId` ...
        checkAvailability().$quantity: `$input->quantity` ...

[2] local#32$order (Order) [variable]
    source: Order::__construct() [instantiation]
        args:
            Order::__construct().$id: `0` literal
            Order::__construct().$customerEmail: `$input->customerEmail` ...
            ...

[3] local#42$processedOrder (Order) [variable]
    source: AbstractOrderProcessor::process() [method_call]
        on: $this->orderProcessor (OrderService::$orderProcessor)
        args:
            process().$order: `$order` local#32$order

[4] local#43$processorName (string) [variable]
    source: AbstractOrderProcessor::getName() [method_call]
        on: $this->orderProcessor (OrderService::$orderProcessor)

[5] local#45$savedOrder (Order) [variable]
    source: OrderRepositoryInterface::save() [method_call]
        on: $this->orderRepository (OrderService::$orderRepository)
        args:
            save().$order: `$processedOrder` local#42$processedOrder

[6] EmailSenderInterface::send() [method_call]
    on: $this->emailSender (OrderService::$emailSender)
    args:
        send().$to: `$savedOrder->customerEmail` ...
        send().$subject: `'Order Confirmation #' . $savedOrder->id` ...
        send().$body: `sprintf(...)` ...

[7] MessageBusInterface::dispatch() [method_call]
    on: $this->messageBus (OrderService::$messageBus)
    args:
        dispatch().$message: `new OrderCreatedMessage($savedOrder->id)` ...

[8] OrderOutput::__construct() [instantiation]
    args:
        OrderOutput::__construct().$id: `$savedOrder->id` ...
        ...
```

### After Phase 3 (ISSUE-B: types added)

```
args:
  checkAvailability().$productId (string): `$input->productId` ...
  checkAvailability().$quantity (int): `$input->quantity` ...

[3] local#42$processedOrder (Order) [variable]
    source: AbstractOrderProcessor::process() [method_call]
        args:
            process().$order (Order): `$order` local#32$order
```

### After Phase 4 (ISSUE-D: full rich argument display — gold standard)

```
== USES (execution flow) ==
[1] InventoryCheckerInterface::checkAvailability() [method_call]
    on: $this->inventoryChecker (OrderService::$inventoryChecker)
    args:
        checkAvailability().$productId (string): `$input->productId`
            source: CreateOrderInput::$productId [property_access]
                on: OrderService::createOrder().$input
        checkAvailability().$quantity (int): `$input->quantity`
            source: CreateOrderInput::$quantity [property_access]
                on: OrderService::createOrder().$input

[2] local#32$order (Order) [variable]
    source: Order::__construct() [instantiation]
        args:
            Order::__construct().$id (int): `0` literal
            Order::__construct().$customerEmail (string): `$input->customerEmail`
                source: CreateOrderInput::$customerEmail [property_access]
                    on: OrderService::createOrder().$input
            Order::__construct().$productId (string): `$input->productId`
                source: CreateOrderInput::$productId [property_access]
                    on: OrderService::createOrder().$input
            Order::__construct().$quantity (int): `$input->quantity`
                source: CreateOrderInput::$quantity [property_access]
                    on: OrderService::createOrder().$input
            Order::__construct().$status (string): `'pending'` literal
            Order::__construct().$createdAt (DateTimeImmutable): `new DateTimeImmutable()`
                source: DateTimeImmutable::__construct() [instantiation]

[3] local#42$processedOrder (Order) [variable]
    source: AbstractOrderProcessor::process() [method_call]
        on: $this->orderProcessor (OrderService::$orderProcessor)
        args:
            process().$order (Order): `$order` local#32$order

[4] local#43$processorName (string) [variable]
    source: AbstractOrderProcessor::getName() [method_call]
        on: $this->orderProcessor (OrderService::$orderProcessor)

[5] local#45$savedOrder (Order) [variable]
    source: OrderRepositoryInterface::save() [method_call]
        on: $this->orderRepository (OrderService::$orderRepository)
        args:
            save().$order (Order): `$processedOrder` local#42$processedOrder

[6] EmailSenderInterface::send() [method_call]
    on: $this->emailSender (OrderService::$emailSender)
    args:
        send().$to (string): `$savedOrder->customerEmail`
            source: Order::$customerEmail [property_access]
                on: local#45$savedOrder
        send().$subject (string): `'Order Confirmation #' . $savedOrder->id`
            source: string concatenation
                Order::$id [property_access]
                    on: local#45$savedOrder
        send().$body (string): `sprintf(...)`

[7] MessageBusInterface::dispatch() [method_call]
    on: $this->messageBus (OrderService::$messageBus)
    args:
        dispatch().$message (object): `new OrderCreatedMessage($savedOrder->id)`
            source: OrderCreatedMessage::__construct() [instantiation]
                args:
                    OrderCreatedMessage::__construct().$orderId (int): `$savedOrder->id`
                        source: Order::$id [property_access]
                            on: local#45$savedOrder

[8] OrderOutput::__construct() [instantiation]
    args:
        OrderOutput::__construct().$id (int): `$savedOrder->id`
            source: Order::$id [property_access]
                on: local#45$savedOrder
        OrderOutput::__construct().$customerEmail (string): `$savedOrder->customerEmail`
            source: Order::$customerEmail [property_access]
                on: local#45$savedOrder
        OrderOutput::__construct().$productId (string): `$savedOrder->productId`
            source: Order::$productId [property_access]
                on: local#45$savedOrder
        OrderOutput::__construct().$quantity (int): `$savedOrder->quantity`
            source: Order::$quantity [property_access]
                on: local#45$savedOrder
        OrderOutput::__construct().$status (string): `$savedOrder->status`
            source: Order::$status [property_access]
                on: local#45$savedOrder
        OrderOutput::__construct().$createdAt (DateTimeImmutable): `$savedOrder->createdAt`
            source: Order::$createdAt [property_access]
                on: local#45$savedOrder
```

## Backward Compatibility

### Additive changes (non-breaking)

- ISSUE-A: New `expression` field on argument edges -- absent when not available, consumers that ignore unknown fields are unaffected
- ISSUE-B: New `value_type` field on `ArgumentInfo` -- optional, default `None`
- ISSUE-C: New entry type `"local_variable"` in JSON output alongside existing `"call"` entries
- ISSUE-D: New fields `param_fqn`, `value_ref_symbol`, `source_chain` on argument objects -- optional, default `None`/null
- ISSUE-E: New `definition` object in JSON output -- absent in older versions

### Structural changes (documented breaking)

- ISSUE-C: The USES section for Method/Function targets changes from call-centric to variable-centric ordering. Variable entries (Kind 1) replace the previous call-only entries with `result ->` annotations. Class-level queries are NOT affected (structural USES preserved).
- ISSUE-E: The output structure gains a new DEFINITION section before USED BY. Existing consumers parsing the output positionally may need updating.

### Consumer migration guidance

- JSON consumers should check for `"type"` field on USES entries: `"local_variable"` or `"call"`
- JSON consumers should check for `"definition"` key at top level
- Text-parsing consumers should handle the new `== DEFINITION ==` section header
- MCP consumers: all new data is in additive fields; existing field semantics are unchanged

## Files Modified Per Phase

| Phase | File | Component | What Changes |
|-------|------|-----------|-------------|
| 1 | `kloc-contracts/sot-json.json` | contracts | Add `expression` to argument edge schema |
| 1 | `kloc-mapper/src/calls_mapper.py` | mapper | Read `value_expr`, store as `expression` on edge |
| 1 | `kloc-mapper/src/models.py` | mapper | Add `expression` to Edge model (if needed) |
| 1 | `kloc-cli/src/queries/context.py` | CLI | Prefer edge `expression` over Value node name |
| 1 | `kloc-cli/src/graph/index.py` | CLI | Update `get_arguments()` to return edge data |
| 2 | `kloc-cli/src/queries/context.py` | CLI | Rewrite `_build_execution_flow()`, add `_build_definition()` |
| 2 | `kloc-cli/src/models/results.py` | CLI | Add entry type distinction, `DefinitionInfo` model |
| 2 | `kloc-cli/src/output/tree.py` | CLI | Render variable entries, definition section |
| 2 | `kloc-cli/src/server/mcp.py` | CLI | Serialize new JSON structures |
| 3 | `kloc-cli/src/models/results.py` | CLI | Add `value_type` to `ArgumentInfo` |
| 3 | `kloc-cli/src/queries/context.py` | CLI | Resolve types via `get_type_of()` |
| 3 | `kloc-cli/src/output/tree.py` | CLI | Display types in parentheses |
| 3 | `kloc-cli/src/server/mcp.py` | CLI | Serialize `value_type` |
| 4 | `kloc-cli/src/models/results.py` | CLI | Extend `ArgumentInfo` with `param_fqn`, `value_ref_symbol`, `source_chain` |
| 4 | `kloc-cli/src/queries/context.py` | CLI | Resolve formal FQNs, trace value chains |
| 4 | `kloc-cli/src/output/tree.py` | CLI | Render rich argument display with nested source chains |
| 4 | `kloc-cli/src/server/mcp.py` | CLI | Serialize rich argument fields |

## Out of Scope

- No changes to scip-php (PHP indexer) -- it already produces all required data
- No `--depth` filtration for nested chain display inside entries (future feature)
- No conditional branch analysis (if/else affecting which calls are made)
- No loop context display (call inside foreach vs one-time call)
- No exception handling context (try/catch around calls)
- No new CLI commands or flags (enhances existing `context` command)
- USED BY direction is unchanged (structural expansion model preserved)
- The existing v1 spec's Phase 4 (Smart Depth) is superseded by ISSUE-C's variable-centric model, which naturally scopes depth expansion to execution flow
