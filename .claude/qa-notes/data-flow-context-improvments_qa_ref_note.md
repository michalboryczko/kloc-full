# QA Reference Note: Data Flow Context Improvements v6 (Issues C-F)

**Date:** 2026-02-12
**Feature Branch:** feature/value-context
**Scope:** kloc-contracts + kloc-mapper + kloc-cli (Issue D spans all three; C/E/F are CLI-only)
**Todo:** `docs/todo/context-command-v6/` (detailed requirements)
**Issues:** C (receiver access chain), D (argument-parameter linking), E (cross-method parameter tracing), F (property cross-method tracing)
**Depends on:** v5 feature (shipped: Value data flow traversal, definition enhancement)
**Dependency chain:** D -> E -> F (hard); C is independent

---

## 0. Acceptance Criteria Traceability Matrix

### ISSUE-C: Missing Receiver Access Chain on USED BY (7 ACs)

| Spec AC | Description | Test Scenarios | Test Type |
|---------|-------------|----------------|-----------|
| AC-C1 | method_call USED BY entry has access_chain when Call has receiver | v6-C-INT-01, v6-C-INT-03 | Integration |
| AC-C2 | instantiation USED BY entry has access_chain=None (no `on:` line) | v6-C-INT-02 | Integration |
| AC-C3 | static_call USED BY entry has access_chain=None (no `on:` line) | v6-C-UNIT-01 | Unit |
| AC-C4 | JSON member_ref includes access_chain, access_chain_symbol, on_kind, on_file, on_line | v6-C-INT-04 | Integration |
| AC-C5 | Same Call shows identical `on:` in method-level USES and Value-level USED BY | v6-C-INT-05 | Integration |
| AC-C6 | Method/Function context queries unchanged (regression) | v6-REG-01 | Regression |
| AC-C7 | All three parts of _build_value_consumer_chain() populate access chain fields | v6-C-INT-01, v6-C-INT-03, v6-C-INT-06 | Integration |

### ISSUE-D: Argument-Parameter Linking (12 ACs)

| Spec AC | Description | Test Scenarios | Test Type |
|---------|-------------|----------------|-----------|
| AC-D1 | Mapper stores parameter FQN on argument edges | v6-D-INT-01, v6-D-INT-02 | Integration |
| AC-D2 | Named argument send() has correct parameter FQN regardless of position | v6-D-INT-03 | Integration |
| AC-D3 | sot-json.json schema accepts parameter field on edges | v6-D-CONTRACT-01 | Contract |
| AC-D4 | CLI reads parameter from edge for arg resolution | v6-D-INT-04 | Integration |
| AC-D5 | CLI falls back to position matching for old sot.json | v6-D-INT-05 | Integration |
| AC-D6 | Argument node FQN uses `.` separator (not `::`) | v6-D-INT-06 | Integration |
| AC-D7 | Existing CLI tests pass (no regression) | v6-REG-02 | Regression |
| AC-D8 | Promoted constructor parameter resolves via parameter field | v6-D-INT-07 | Integration |
| AC-D9 | JSON output shows parameter field on argument edges | v6-D-INT-08 | Integration |
| AC-D10 | Edge model round-trips parameter field | v6-D-UNIT-01 | Unit |
| AC-D11 | Querying createOrder().$input finds Value node (not Argument) | v6-D-INT-09 | Integration |
| AC-D12 | Querying parameter FQN with only Argument node falls back to Argument info | v6-D-UNIT-02 | Unit |

### ISSUE-E: Cross-Method Parameter Tracing (12 ACs)

| Spec AC | Description | Test Scenarios | Test Type |
|---------|-------------|----------------|-----------|
| AC-E1 | Parameter USES shows caller-provided values at depth 1 (boundary crossing) | v6-E-INT-01 | Integration |
| AC-E2 | USED BY crosses into callee and shows parameter consumers at depth+1 | v6-E-INT-02, v6-E-INT-03 | Integration |
| AC-E3 | depth=5 with 3 boundary crossings shows correct depth numbering | v6-E-INT-04 | Integration |
| AC-E4 | Interface method with no body: terminal node, no crash | v6-E-INT-05 | Integration |
| AC-E5 | Recursive method: depth limit prevents infinite loop | v6-E-UNIT-01 | Unit |
| AC-E6 | Promoted constructor parameter: consumers included in USED BY | v6-E-INT-06 | Integration |
| AC-E7 | Multiple callers to same method: all appear as separate branches | v6-E-INT-07 | Integration |
| AC-E8 | Return value path: callee return -> result Value -> assigned_to in caller | v6-E-INT-08 | Integration |
| AC-E9 | JSON output includes boundary crossing indicator | v6-E-INT-09 | Integration |
| AC-E10 | Old sot.json without parameter field: position fallback, no crash | v6-E-INT-10 | Integration |
| AC-E11 | Cycle detection: same Value visited twice -> skipped | v6-E-UNIT-02 | Unit |
| AC-E12 | Named arguments: parameter FQN matches correctly (not position) | v6-E-INT-11 | Integration |

### ISSUE-F: Property Cross-Method Tracing (10 ACs)

| Spec AC | Description | Test Scenarios | Test Type |
|---------|-------------|----------------|-----------|
| AC-F1 | Promoted property USES traces backward through constructor arguments to call sites | v6-F-INT-01 | Integration |
| AC-F2 | Property USED BY shows all access sites and traces downstream | v6-F-INT-02 | Integration |
| AC-F3 | DI-injected service property USES shows constructor parameter as terminal | v6-F-INT-03 | Integration |
| AC-F4 | Property read in multiple methods: all access sites appear | v6-F-INT-04 | Integration |
| AC-F5 | Mutable property with direct assignment: USES shows assignment source | v6-F-UNIT-01 | Unit |
| AC-F6 | Order::$customerEmail USED BY depth 5 traces through OrderOutput -> OrderController -> OrderResponse | v6-F-INT-05 | Integration |
| AC-F7 | Service dependency property USED BY shows method calls on receiver | v6-F-INT-06 | Integration |
| AC-F8 | JSON output includes property_source field for cross-method data | v6-F-INT-07 | Integration |
| AC-F9 | Depth limit reached: tree stops cleanly | v6-F-UNIT-02 | Unit |
| AC-F10 | Dead property (never read) USED BY: empty tree, no crash | v6-F-UNIT-03 | Unit |

---

## 1. Test Scenarios: ISSUE-C -- Receiver Access Chain on USED BY

### 1.1 Pre-conditions

- v5 features shipped (Value USES/USED BY working for intra-method)
- Reference project sot.json available
- No mapper or schema changes needed (CLI-only)

### 1.2 Integration Tests

| ID | Scenario | GIVEN | WHEN | THEN | AC |
|----|----------|-------|------|------|----|
| v6-C-INT-01 | send() in Value USED BY shows `on: $this->emailSender` | Query `$savedOrder` USED BY, send() entry at line 48 | `context "...local$savedOrder@45" --depth 1` | send() entry has access_chain="$this->emailSender", access_chain_symbol contains "OrderService::$emailSender", on_kind="param" | AC-C1, C7 |
| v6-C-INT-02 | Constructors do NOT show `on:` line | Query `$savedOrder` USED BY, OrderCreatedMessage::__construct() at line 59 | `context "...local$savedOrder@45" --depth 1` | __construct() entry has access_chain=None, on_kind=None | AC-C2 |
| v6-C-INT-03 | dispatch() shows `on: $this->messageBus` | Query `$savedOrder` USED BY (if dispatch appears) | `context "...local$savedOrder@45" --depth 2` | dispatch() entry (if present) has access_chain="$this->messageBus" | AC-C1, C7 |
| v6-C-INT-04 | JSON output includes access_chain fields | Query `$savedOrder` with --json | `context "...local$savedOrder@45" --json` | JSON member_ref for send() includes access_chain, access_chain_symbol, on_kind, on_file, on_line | AC-C4 |
| v6-C-INT-05 | Consistency: same `on:` in method-level and value-level | Compare send() entry from method query vs value query | Method: `context "createOrder()" --depth 2`, Value: `context "...local$savedOrder@45" --depth 1` | send() `on:` annotation identical in both views | AC-C5 |
| v6-C-INT-06 | checkAvailability() shows `on: $this->inventoryChecker` | Query `$input` USED BY | `context "...createOrder().$input" --depth 1` | checkAvailability() entry has access_chain="$this->inventoryChecker", on_kind="param" | AC-C1, C7 |

### 1.3 Unit Tests

| ID | Scenario | GIVEN | WHEN | THEN | AC |
|----|----------|-------|------|------|----|
| v6-C-UNIT-01 | Static call has no receiver chain | Call node with call_kind="method_static", no receiver edge | Build consumer chain | MemberRef has access_chain=None | AC-C3 |

### 1.4 Regression Tests

| ID | Scenario | GIVEN | WHEN | THEN | AC |
|----|----------|-------|------|------|----|
| v6-REG-01 | Method-level context output unchanged | `OrderService::createOrder()` | `context "OrderService::createOrder()" --depth 2` | Output identical to v5 (execution flow, impl blocks, on: lines) | AC-C6 |

### 1.5 Test Commands

```bash
cd /Users/michal/dev/ai/kloc/kloc-cli
SOT=../kloc-reference-project-php/contract-tests/output/sot.json

# v6-C-INT-01: send() shows on: $this->emailSender
uv run kloc-cli context 'App\Service\OrderService::createOrder().local$savedOrder@45' --depth 1 --sot $SOT
# CHECK: send() [method_call] has "on: $this->emailSender" line

# v6-C-INT-02: Constructor does NOT show on:
# CHECK: OrderCreatedMessage::__construct() [instantiation] has NO "on:" line

# v6-C-INT-04: JSON output
uv run kloc-cli context 'App\Service\OrderService::createOrder().local$savedOrder@45' --depth 1 --json --sot $SOT | python3 -m json.tool
# CHECK: member_ref for send() includes "access_chain", "on_kind", "on_file", "on_line"

# v6-C-INT-05: Consistency with method-level output
uv run kloc-cli context 'App\Service\OrderService::createOrder()' --depth 2 --sot $SOT
# CHECK: send() on: line matches value-level output

# v6-C-INT-06: checkAvailability() on $input
uv run kloc-cli context 'App\Service\OrderService::createOrder().$input' --depth 1 --sot $SOT
# CHECK: checkAvailability() has "on: $this->inventoryChecker"
# CHECK: Order::__construct() [instantiation] has NO "on:" line
```

### 1.6 Verification Checklist (from MUST CHECK 2)

```
[ ] send() in Value USED BY shows on: $this->emailSender [param]
[ ] dispatch() shows on: $this->messageBus [param]
[ ] Constructors do NOT show on: line
[ ] Method-level context output unchanged (no regression)
[ ] JSON output includes access_chain, on_kind, on_file, on_line
[ ] Existing CLI tests pass
```

---

## 2. Test Scenarios: ISSUE-D -- Argument-Parameter Linking

### 2.1 Pre-conditions

- kloc-contracts sot-json.json schema updated with `parameter` field
- kloc-mapper Edge model has `parameter` field
- kloc-mapper calls_mapper.py stores parameter FQN on argument edges
- kloc-mapper fixes Argument node FQN separator from `::` to `.`
- Reference project sot.json REGENERATED with updated mapper

### 2.2 Integration Tests

| ID | Scenario | GIVEN | WHEN | THEN | AC |
|----|----------|-------|------|------|----|
| v6-D-INT-01 | Argument edges have parameter field in regenerated sot.json | Regenerated sot.json | Inspect argument edges | `parameter` field populated with FQN like "...save().$order" | AC-D1 |
| v6-D-INT-02 | Multiple argument edges on checkAvailability() | Regenerated sot.json | Inspect argument edges for checkAvailability() call | Both argument edges have parameter field: one for $productId, one for $quantity | AC-D1 |
| v6-D-INT-03 | Named argument send() shows correct parameter names | Regenerated sot.json, send() uses named args | `context "...local$savedOrder@45" --depth 1` | args display `send().$subject` and `send().$to` with correct value mapping (not swapped) | AC-D2 |
| v6-D-INT-04 | CLI uses parameter field for resolution | Regenerated sot.json with parameter field | `context "...local$savedOrder@45" --depth 1` | Args show FQN with `.` separator: `send().$to` not `send()::$to` | AC-D4 |
| v6-D-INT-05 | CLI backward compat: old sot.json without parameter field | Use pre-v6 sot.json | `context "...local$savedOrder@45" --depth 1` | Args still display (position fallback), no crash | AC-D5 |
| v6-D-INT-06 | Argument FQNs use `.` separator | Regenerated sot.json | Inspect Argument nodes | FQNs like `createOrder().$input` not `createOrder()::$input` | AC-D6 |
| v6-D-INT-07 | Promoted constructor: OrderOutput::__construct() args | Regenerated sot.json | `context "...local$savedOrder@45" --depth 2` | OrderOutput::__construct() args show correct parameter FQNs (e.g., __construct().$id, __construct().$customerEmail) without needing promoted param fallback | AC-D8 |
| v6-D-INT-08 | JSON output shows parameter field | Regenerated sot.json | `context "...local$savedOrder@45" --json --depth 1` | JSON argument edges include `parameter` field | AC-D9 |
| v6-D-INT-09 | Querying createOrder().$input finds Value node | Regenerated sot.json (both Argument and Value with same FQN) | `context "...createOrder().$input"` | Result shows Value (parameter) with data flow USES/USED BY, NOT Argument-only structural view | AC-D11 |

### 2.3 Contract Tests

| ID | Scenario | GIVEN | WHEN | THEN | AC |
|----|----------|-------|------|------|----|
| v6-D-CONTRACT-01 | Schema validates parameter field | Updated sot-json.json schema | Validate regenerated sot.json | Validation passes with parameter field present | AC-D3 |

### 2.4 Unit Tests

| ID | Scenario | GIVEN | WHEN | THEN | AC |
|----|----------|-------|------|------|----|
| v6-D-UNIT-01 | Edge model round-trips parameter field | Edge with parameter="save().$order" | Serialize to dict, deserialize | parameter field preserved | AC-D10 |
| v6-D-UNIT-02 | Argument-only fallback when no Value exists | Search for FQN that has Argument node but no Value node | CLI context query | Shows Argument info (type_hint, containment) | AC-D12 |

### 2.5 Regression Tests

| ID | Scenario | GIVEN | WHEN | THEN | AC |
|----|----------|-------|------|------|----|
| v6-REG-02 | Existing CLI tests pass after D changes | Regenerated sot.json + CLI changes | Run full test suite | All existing tests green | AC-D7 |

### 2.6 Test Commands

```bash
cd /Users/michal/dev/ai/kloc

# Step 1: Regenerate sot.json with updated mapper
cd kloc-mapper
uv run kloc-mapper map ../kloc-reference-project-php/contract-tests/output/index.json -o ../kloc-reference-project-php/contract-tests/output/sot.json --pretty

# Step 2: Validate schema
cd ../kloc-contracts
python3 validate.py ../kloc-reference-project-php/contract-tests/output/sot.json

# Step 3: Inspect argument edges in regenerated sot.json
python3 -c "
import json
sot = json.load(open('../kloc-reference-project-php/contract-tests/output/sot.json'))
arg_edges = [e for e in sot['edges'] if e['type'] == 'argument']
for e in arg_edges[:10]:
    print(json.dumps(e, indent=2))
print(f'Total argument edges: {len(arg_edges)}')
with_param = [e for e in arg_edges if 'parameter' in e and e['parameter']]
print(f'With parameter field: {len(with_param)}')
"

# Step 4: Verify Argument node FQNs use . separator
python3 -c "
import json
sot = json.load(open('../kloc-reference-project-php/contract-tests/output/sot.json'))
arg_nodes = [n for n in sot['nodes'] if n['kind'] == 'Argument']
bad = [n for n in arg_nodes if '()::' in n.get('fqn', '')]
good = [n for n in arg_nodes if '().' in n.get('fqn', '')]
print(f'Argument nodes: {len(arg_nodes)}')
print(f'Using :: separator (BAD): {len(bad)}')
print(f'Using . separator (GOOD): {len(good)}')
if bad:
    print('EXAMPLES of bad FQNs:')
    for n in bad[:3]:
        print(f'  {n[\"fqn\"]}')
"

# Step 5: CLI output verification
cd kloc-cli
SOT=../kloc-reference-project-php/contract-tests/output/sot.json

uv run kloc-cli context 'App\Service\OrderService::createOrder().local$savedOrder@45' --depth 1 --sot $SOT
# CHECK: args use . separator (send().$to not send()::$to)
# CHECK: Named args for send() are correctly mapped ($to -> customerEmail, $subject -> literal)

uv run kloc-cli context 'App\Service\OrderService::createOrder().$input' --sot $SOT
# CHECK: Shows Value (parameter) definition, not Argument definition
# CHECK: Has USES and USED BY sections with data flow

# Step 6: Run all tests
cd ../kloc-mapper && uv run pytest tests/ -v
cd ../kloc-cli && uv run pytest tests/ -v
```

### 2.7 Verification Checklist (from MUST CHECK 1)

```
[ ] Regenerate sot.json from reference project with updated mapper
[ ] argument edges have `parameter` field populated (not null) for known methods
[ ] Argument node FQNs use . not :: (e.g., createOrder().$input)
[ ] Named argument example (send()) shows correct parameter names
[ ] Position fallback works for old sot.json without parameter field
[ ] Querying createOrder().$input finds Value node (not Argument)
[ ] Contract schema validates new sot.json
[ ] Existing CLI tests pass (no regressions)
```

---

## 3. Test Scenarios: ISSUE-E -- Cross-Method Parameter Tracing

### 3.1 Pre-conditions

- ISSUE-D complete (parameter field on argument edges, FQN fix)
- Regenerated sot.json with parameter field
- All D tests passing

### 3.2 Integration Tests

| ID | Scenario | GIVEN | WHEN | THEN | AC |
|----|----------|-------|------|------|----|
| v6-E-INT-01 | Parameter $input USES shows OrderController::create() as caller | `createOrder().$input` parameter with callers | `context "...createOrder().$input" --depth 3` | USES has >= 1 entry at depth 1: OrderController::create() passes $input, at depth 2: CreateOrderInput::__construct(), at depth 3: $request->customerEmail etc. | AC-E1 |
| v6-E-INT-02 | $order USED BY crosses into process() | `createOrder().local$order@32` local | `context "...local$order@32" --depth 3` | USED BY has process() at depth 1, preProcess()/doProcess() at depth 2, Order::$status accesses at depth 3 | AC-E2 |
| v6-E-INT-03 | $savedOrder USED BY crosses into OrderCreatedMessage::__construct() | `$savedOrder` local passed to __construct() | `context "...local$savedOrder@45" --depth 3` | USED BY has __construct() at depth 1, crosses into $orderId, handler reads $message->orderId at depth 2, NotificationService at depth 3 | AC-E2 |
| v6-E-INT-04 | process().$order USES at depth 5 shows 3 boundary crossings | process().$order parameter | `context "AbstractOrderProcessor::process().$order" --depth 5` | USES shows createOrder() at depth 1, Order::__construct() at depth 2, $input->customerEmail at depth 3, controller at depth 4, CreateOrderInput at depth 5 | AC-E3 |
| v6-E-INT-05 | Interface method send() is terminal | $savedOrder USED BY with send() call | `context "...local$savedOrder@45" --depth 3` | send() args traced but no children inside send() method body (interface, terminal) | AC-E4 |
| v6-E-INT-06 | Promoted constructor crossing: OrderCreatedMessage.$orderId | $savedOrder->id passed to OrderCreatedMessage::__construct() | `context "...local$savedOrder@45" --depth 3` | After crossing into __construct().$orderId, promoted property consumers shown (handler reads $message->orderId) | AC-E6 |
| v6-E-INT-07 | Multiple callers (if applicable) | Parameter with multiple call sites | Check if any parameter has multiple callers in reference project | All callers appear as separate branches | AC-E7 |
| v6-E-INT-08 | Return value path: process() return -> $processedOrder -> save() | $order USED BY with depth 5 | `context "...local$order@32" --depth 5` | After process() call, return $order -> produces -> $processedOrder -> save() -> $savedOrder | AC-E8 |
| v6-E-INT-09 | JSON output includes boundary crossing indicator | Cross-method query with --json | `context "...createOrder().$input" --depth 3 --json` | JSON entries include `crossed_from` or boundary indicator field | AC-E9 |
| v6-E-INT-10 | Old sot.json: position fallback | Use pre-v6 sot.json without parameter field | `context "...createOrder().$input" --depth 3` | Either graceful empty or position-based attempt, no crash | AC-E10 |
| v6-E-INT-11 | Named arguments: correct parameter matching across boundary | send() with named args, cross-method | `context "...local$savedOrder@45" --depth 3` | send().$to correctly linked to $savedOrder->customerEmail (not swapped) | AC-E12 |

### 3.3 Unit Tests

| ID | Scenario | GIVEN | WHEN | THEN | AC |
|----|----------|-------|------|------|----|
| v6-E-UNIT-01 | Depth limit prevents infinite recursion | Recursive call pattern in minimal fixture | Trace USED BY with depth 3 | Tree stops at depth 3, no infinite loop | AC-E5 |
| v6-E-UNIT-02 | Cycle detection: same Value visited twice | Value A -> Call -> Value B -> Call -> Value A (cycle) | Trace USED BY | Second visit to Value A skipped, no error | AC-E11 |

### 3.4 Test Commands

```bash
cd /Users/michal/dev/ai/kloc/kloc-cli
SOT=../kloc-reference-project-php/contract-tests/output/sot.json

# v6-E-INT-01: Parameter $input USES shows callers
uv run kloc-cli context 'App\Service\OrderService::createOrder().$input' --depth 3 --sot $SOT
# CHECK: USES section NOT empty (was "None" in v5)
# CHECK: [1] OrderController::create() passes $input (boundary crossing)
# CHECK: [2] CreateOrderInput::__construct() source
# CHECK: [3] $request->customerEmail, $request->productId, $request->quantity

# v6-E-INT-02: $order USED BY crosses into process()
uv run kloc-cli context 'App\Service\OrderService::createOrder().local$order@32' --depth 3 --sot $SOT
# CHECK: [1] AbstractOrderProcessor::process() with args
# CHECK: [2] preProcess($order), doProcess($order), postProcess($order)
# CHECK: [3] Order::$status property accesses

# v6-E-INT-04: Deep crossing (5 levels)
uv run kloc-cli context 'App\Service\AbstractOrderProcessor::process().$order' --depth 5 --sot $SOT
# CHECK: USES has [1] createOrder() -> [2] Order::__construct() -> [3] $input accesses -> [4] controller -> [5] CreateOrderInput

# v6-E-INT-05: Interface terminal
uv run kloc-cli context 'App\Service\OrderService::createOrder().local$savedOrder@45' --depth 3 --sot $SOT
# CHECK: send() entry has args but no children inside send() (interface)

# v6-E-INT-08: Return value path
uv run kloc-cli context 'App\Service\OrderService::createOrder().local$order@32' --depth 5 --sot $SOT
# CHECK: process() return -> $processedOrder -> save() -> $savedOrder -> property accesses

# JSON verification
uv run kloc-cli context 'App\Service\OrderService::createOrder().$input' --depth 3 --json --sot $SOT | python3 -m json.tool
# CHECK: entries with boundary crossing indicators
```

### 3.5 Verification Checklist (from MUST CHECK 3)

```
[ ] Query createOrder().$input USES shows OrderController::create() as caller at depth 1 (boundary crossing)
[ ] Query $order local USED BY shows process().$order consumers at depth 2 (preProcess -> Order::$status at depth 3)
[ ] Depth counting correct: boundary = +1 depth
[ ] depth=5 shows 2-3 method hops for $order
[ ] Interface methods (send()): terminal node, no crash
[ ] Return value path: process() return -> $processedOrder -> save()
[ ] No infinite loops on recursive patterns
[ ] Cycle detection: same Value visited twice -> skipped
[ ] JSON output includes boundary crossing indicator
[ ] Existing tests pass (no regressions)
[ ] VERIFY: all data comes from graph edges + parameter field, no virtual relations invented
```

---

## 4. Test Scenarios: ISSUE-F -- Property Cross-Method Tracing

### 4.1 Pre-conditions

- ISSUE-E complete (cross-method traversal infrastructure)
- ISSUE-D complete (parameter field on argument edges)
- Regenerated sot.json with all D changes
- All E tests passing

### 4.2 Integration Tests

| ID | Scenario | GIVEN | WHEN | THEN | AC |
|----|----------|-------|------|------|----|
| v6-F-INT-01 | Order::$customerEmail USES shows createOrder() passes $input->customerEmail | Promoted property with assigned_from edge | `context "App\Entity\Order::$customerEmail" --depth 3` | USES shows [1] createOrder() instantiates Order with arg __construct().$customerEmail <- $input->customerEmail, [2] $input->customerEmail accesses CreateOrderInput::$customerEmail, [3] controller source | AC-F1 |
| v6-F-INT-02 | Order::$customerEmail USED BY shows $savedOrder->customerEmail accesses | Property with calls edges (166 exist) | `context "App\Entity\Order::$customerEmail" --depth 3` | USED BY shows [1] $savedOrder->customerEmail accesses -> [2] send().$to and OrderOutput::__construct().$customerEmail -> [3] downstream | AC-F2 |
| v6-F-INT-03 | OrderService::$emailSender USES shows constructor parameter (terminal) | Service dependency property, DI-injected | `context "App\Service\OrderService::$emailSender" --depth 1` | USES shows constructor parameter as terminal (no explicit `new OrderService()` in code) | AC-F3 |
| v6-F-INT-04 | Order::$id USED BY shows accesses from multiple methods | Property accessed in >= 3 methods | `context "App\Entity\Order::$id" --depth 2` | USED BY shows accesses from createOrder(), getOrder(), NotificationService etc. | AC-F4 |
| v6-F-INT-05 | Order::$customerEmail USED BY depth 5: full chain | Promoted property through 3 boundary crossings | `context "App\Entity\Order::$customerEmail" --depth 5` | Traces through OrderOutput::$customerEmail -> controller reads -> OrderResponse::__construct() | AC-F6 |
| v6-F-INT-06 | OrderService::$emailSender USED BY shows send() call on receiver | Service dependency used as receiver | `context "App\Service\OrderService::$emailSender" --depth 3` | USED BY shows $this->emailSender [receiver] -> send() method_call with args | AC-F7 |
| v6-F-INT-07 | JSON output for property trace | Property query with --json | `context "App\Entity\Order::$customerEmail" --depth 3 --json` | JSON includes property-specific trace fields | AC-F8 |

### 4.3 Unit Tests

| ID | Scenario | GIVEN | WHEN | THEN | AC |
|----|----------|-------|------|------|----|
| v6-F-UNIT-01 | Mutable property: direct assignment USES | Property with assigned_from to non-constructor Value | Build USES tree | Shows assignment source value | AC-F5 |
| v6-F-UNIT-02 | Depth limit on property chain | Deep chain exceeding max_depth | Build property trace with depth 3 | Tree stops at depth 3 cleanly | AC-F9 |
| v6-F-UNIT-03 | Dead property: never read | Property node with no calls edges targeting it | Build USED BY | Empty tree, no crash | AC-F10 |

### 4.4 Test Commands

```bash
cd /Users/michal/dev/ai/kloc/kloc-cli
SOT=../kloc-reference-project-php/contract-tests/output/sot.json

# v6-F-INT-01: Order::$customerEmail USES
uv run kloc-cli context 'App\Entity\Order::$customerEmail' --depth 3 --sot $SOT
# CHECK: USES shows who sets this property
# CHECK: [1] createOrder() passes $input->customerEmail via Order::__construct()
# CHECK: [2] $input->customerEmail reads CreateOrderInput::$customerEmail
# CHECK: [3] controller source (OrderController::create())

# v6-F-INT-02: Order::$customerEmail USED BY
uv run kloc-cli context 'App\Entity\Order::$customerEmail' --depth 3 --sot $SOT
# CHECK: USED BY shows who reads this property
# CHECK: [1] $savedOrder->customerEmail accesses in createOrder()
# CHECK: [2] send().$to, OrderOutput::__construct().$customerEmail

# v6-F-INT-03: Service dependency USES (terminal)
uv run kloc-cli context 'App\Service\OrderService::$emailSender' --depth 1 --sot $SOT
# CHECK: USES shows constructor parameter (terminal)

# v6-F-INT-04: Order::$id USED BY (multiple methods)
uv run kloc-cli context 'App\Entity\Order::$id' --depth 2 --sot $SOT
# CHECK: Accesses from createOrder(), getOrder(), NotificationService

# v6-F-INT-05: Full chain depth 5
uv run kloc-cli context 'App\Entity\Order::$customerEmail' --depth 5 --sot $SOT
# CHECK: USED BY traces through OrderOutput -> OrderController -> OrderResponse (3 crossings)

# v6-F-INT-06: Service dependency USED BY
uv run kloc-cli context 'App\Service\OrderService::$emailSender' --depth 3 --sot $SOT
# CHECK: USED BY shows send() method call on $this->emailSender receiver

# JSON verification
uv run kloc-cli context 'App\Entity\Order::$customerEmail' --depth 3 --json --sot $SOT | python3 -m json.tool
```

### 4.5 Verification Checklist (from MUST CHECK 4)

```
[ ] Query Order::$customerEmail USES shows createOrder() passes $input->customerEmail (via assigned_from edge -> Value(parameter))
[ ] Query Order::$customerEmail USED BY shows $savedOrder->customerEmail -> send().$to and OrderOutput::__construct().$customerEmail
[ ] assigned_from edges (63 existing) are followed, not invented
[ ] calls edges (166 existing) to Property are followed, not invented
[ ] Service dependency ($emailSender) USED BY shows send() call
[ ] Property with no assigned_from: empty USES, no crash
[ ] Cross-method depth works through promoted property chains
[ ] Existing tests pass (no regressions)
[ ] VERIFY: all data comes from existing graph edges, no virtual Property -> Value links invented
```

---

## 5. Regression Testing

### 5.1 Regression Commands (output must be UNCHANGED)

```bash
cd /Users/michal/dev/ai/kloc/kloc-cli
SOT=../kloc-reference-project-php/contract-tests/output/sot.json

# CRITICAL: Capture baseline BEFORE any changes
uv run kloc-cli context 'App\Service\OrderService::createOrder()' --depth 2 --sot $SOT > /tmp/regression_method_v5.txt

# After ALL changes (D + C + E + F):
uv run kloc-cli context 'App\Service\OrderService::createOrder()' --depth 2 --sot $SOT > /tmp/regression_method_v6.txt
diff /tmp/regression_method_v5.txt /tmp/regression_method_v6.txt
# MUST: diff shows NO changes (or only expected changes from D FQN fix: :: -> .)

# Additional regression commands:
uv run kloc-cli context 'App\Entity\Order' --sot $SOT > /tmp/regression_class.txt
uv run kloc-cli context 'App\Service\OrderService::createOrder()' --depth 2 --json --sot $SOT > /tmp/regression_method_json.txt
```

### 5.2 Allowed Regression Differences

The following changes are EXPECTED and acceptable in regression output:

| Change | Reason | Affects |
|--------|--------|---------|
| Argument FQN `::` -> `.` | ISSUE-D FQN fix | All argument displays |
| Parameter name corrections on named args | ISSUE-D direct resolution | send() args |

All other output differences are regressions and must be investigated.

### 5.3 Full Test Suite

```bash
# Run all tests for each component
cd /Users/michal/dev/ai/kloc/kloc-mapper && uv run pytest tests/ -v
cd /Users/michal/dev/ai/kloc/kloc-cli && uv run pytest tests/ -v

# Contract test validation
cd /Users/michal/dev/ai/kloc/kloc-contracts && python3 validate.py ../kloc-reference-project-php/contract-tests/output/sot.json
```

---

## 6. Edge Case Matrix

### ISSUE-C Edge Cases

| Edge Case | Expected Behavior | Test Coverage |
|-----------|-------------------|---------------|
| Method call on receiver | `on:` line with access_chain, symbol, on_kind | v6-C-INT-01, v6-C-INT-06 |
| Instantiation (new) | No `on:` line (no receiver) | v6-C-INT-02 |
| Static call | No `on:` line (no receiver) | v6-C-UNIT-01 |
| Chained method call ($this->getService()->method()) | `on: $this->getService()` with no property FQN | Manual verification |
| Part 2: standalone property access | `on:` may show queried variable as receiver (redundant but consistent) | Manual verification |

### ISSUE-D Edge Cases

| Edge Case | Expected Behavior | Test Coverage |
|-----------|-------------------|---------------|
| Built-in functions (sprintf) | parameter=null, position fallback | v6-D-INT-05 |
| Variadic arguments | parameter may be null for extra args | Manual verification |
| Old sot.json (no parameter field) | Position fallback, no crash | v6-D-INT-05 |
| Interface vs implementation param names | Uses interface parameter (correct for call) | Implicit in v6-D-INT-03 |
| Literal with no Value node (id: 0) | No argument edge (value_id null), no parameter | Manual verification |

### ISSUE-E Edge Cases

| Edge Case | Expected Behavior | Test Coverage |
|-----------|-------------------|---------------|
| Interface method (no body) | Terminal node, no crash | v6-E-INT-05 |
| Recursive method | Depth limit stops expansion | v6-E-UNIT-01 |
| Cycle in data flow | Visited set prevents re-traversal | v6-E-UNIT-02 |
| Return value crossing | follows produces -> assigned_from | v6-E-INT-08 |
| Multiple callers | All appear as branches | v6-E-INT-07 |
| Promoted constructor crossing | promoted property consumers shown | v6-E-INT-06 |

### ISSUE-F Edge Cases

| Edge Case | Expected Behavior | Test Coverage |
|-----------|-------------------|---------------|
| Property never read | Empty USED BY, no crash | v6-F-UNIT-03 |
| DI-injected service property | USES shows constructor param as terminal | v6-F-INT-03 |
| Mutable property with direct assignment | USES shows assignment source | v6-F-UNIT-01 |
| Readonly promoted property | USES guaranteed one source per call site | Implicit in v6-F-INT-01 |
| Property accessed in many methods | All access sites shown | v6-F-INT-04 |
| Deep property chain (>5 boundaries) | Stops at depth limit | v6-F-UNIT-02 |

---

## 7. Cross-Issue Interaction Notes

### D enables E, E enables F
ISSUE-D's `parameter` field is the foundation for all cross-method tracing. Without it, E cannot reliably match caller arguments to callee parameters. F depends on E's cross-method infrastructure to trace through promoted properties.

### C is independent but beneficial before E/F
ISSUE-C's receiver chain pattern will be automatically reused by E and F when displaying consumers at deeper depths. Implementing C first avoids duplicating the receiver resolution code.

### sot.json regeneration
ISSUE-D requires regenerating sot.json (mapper changes). After regeneration, ALL tests (C, E, F) must use the new sot.json. Capture baseline regression output BEFORE regeneration.

### FQN change (D) affects all output
The `::` to `.` FQN fix in ISSUE-D changes ALL argument display throughout the CLI. This is an expected, acceptable change in regression output. Document the expected diff.

### Graph data integrity principle
The CLI reads sot.json. It does NOT invent relations. All cross-method tracing must be grounded in actual graph data (nodes, edges, fields). Verify this in every test.

---

## 8. Phase Gate Checklist

### Phase 1: ISSUE-D -- Argument-Parameter Linking Foundation

```
[ ] kloc-contracts: parameter field in sot-json.json schema
[ ] kloc-mapper: Edge model has parameter field
[ ] kloc-mapper: calls_mapper.py stores parameter FQN
[ ] kloc-mapper: Argument FQN uses . separator
[ ] sot.json regenerated successfully
[ ] v6-D-INT-01: argument edges have parameter field
[ ] v6-D-INT-03: Named args (send()) correct
[ ] v6-D-INT-06: Argument FQNs use . not ::
[ ] v6-D-INT-09: createOrder().$input finds Value
[ ] v6-D-CONTRACT-01: Schema validates
[ ] v6-D-UNIT-01: Edge round-trip
[ ] v6-REG-02: All existing tests pass
[ ] Baseline regression captured for comparison
```

### Phase 2: ISSUE-C -- Consumer Access Chain Fix (parallel with Phase 1)

```
[ ] _build_value_consumer_chain() Part 1: access chain populated
[ ] _build_value_consumer_chain() Part 2: access chain populated
[ ] _build_value_consumer_chain() Part 3: access chain populated
[ ] v6-C-INT-01: send() shows on: $this->emailSender
[ ] v6-C-INT-02: __construct() does NOT show on:
[ ] v6-C-INT-04: JSON includes access_chain fields
[ ] v6-C-INT-05: Consistency with method-level
[ ] v6-C-INT-06: checkAvailability() shows on: $this->inventoryChecker
[ ] v6-REG-01: Method-level output unchanged
[ ] All existing tests pass
```

### Phase 3: ISSUE-E -- Cross-Method Parameter Tracing

```
[ ] USES direction: parameter traces back to callers
[ ] USED BY direction: value crosses into callee parameters
[ ] v6-E-INT-01: $input USES shows OrderController
[ ] v6-E-INT-02: $order USED BY crosses into process()
[ ] v6-E-INT-04: depth=5 shows 3 boundary crossings
[ ] v6-E-INT-05: Interface method terminal
[ ] v6-E-INT-08: Return value path works
[ ] v6-E-UNIT-01: Depth limit prevents infinite recursion
[ ] v6-E-UNIT-02: Cycle detection works
[ ] v6-E-INT-09: JSON includes crossing indicator
[ ] All existing tests pass
[ ] VERIFY: no virtual relations invented
```

### Phase 4: ISSUE-F -- Property Cross-Method Tracing

```
[ ] Property USES: promoted property -> constructor args -> call sites
[ ] Property USED BY: property accesses -> downstream consumers
[ ] v6-F-INT-01: Order::$customerEmail USES shows call sites
[ ] v6-F-INT-02: Order::$customerEmail USED BY shows accesses
[ ] v6-F-INT-03: Service dependency terminal
[ ] v6-F-INT-04: Order::$id multiple methods
[ ] v6-F-INT-05: Full chain depth 5
[ ] v6-F-INT-06: Service dependency USED BY
[ ] v6-F-UNIT-03: Dead property no crash
[ ] All existing tests pass
[ ] VERIFY: no virtual Property -> Value links invented
```

### Final Gate (all issues complete)

```
[ ] All v6-C tests pass
[ ] All v6-D tests pass
[ ] All v6-E tests pass
[ ] All v6-F tests pass
[ ] All regression tests pass
[ ] Full test suite green for kloc-mapper and kloc-cli
[ ] Contract schema validates regenerated sot.json
[ ] Regression diff shows ONLY expected changes (FQN :: -> .)
[ ] Manual verification: createOrder().$input USES shows callers
[ ] Manual verification: Order::$customerEmail shows full lineage
[ ] Manual verification: createOrder() method query unchanged (except FQN fix)
[ ] JSON output verified for all issues
```

---

## 9. Test Infrastructure

### 9.1 Fixtures

| Fixture | Used By | Status | Action |
|---------|---------|--------|--------|
| Reference project sot.json | All v6 integration tests | EXISTS but needs REGENERATION after D | Regenerate with updated mapper |
| Pre-v6 sot.json (backup) | v6-D-INT-05, v6-E-INT-10 | NEEDS CREATION | Save copy before regeneration |
| In-memory fixtures | v6-C-UNIT-01, v6-D-UNIT-*, v6-E-UNIT-*, v6-F-UNIT-* | NEEDS CREATION | Minimal sot.json with cycles, recursive patterns |

### 9.2 Test Files

| File | Purpose | Test Type |
|------|---------|-----------|
| `kloc-cli/tests/test_usage_flow.py` | Extend with v6 integration test classes | Integration |
| `kloc-mapper/tests/test_models.py` | Extend with Edge parameter field tests | Unit |
| `kloc-mapper/tests/test_mapper.py` | Extend with parameter FQN and Argument FQN tests | Integration |

### 9.3 Baseline Capture (MUST do before any changes)

```bash
cd /Users/michal/dev/ai/kloc/kloc-cli
SOT=../kloc-reference-project-php/contract-tests/output/sot.json

# Capture current output for regression comparison
uv run kloc-cli context 'App\Service\OrderService::createOrder()' --depth 2 --sot $SOT > /tmp/v6_baseline_method.txt
uv run kloc-cli context 'App\Service\OrderService::createOrder().local$savedOrder@45' --depth 2 --sot $SOT > /tmp/v6_baseline_savedOrder.txt
uv run kloc-cli context 'App\Service\OrderService::createOrder().$input' --depth 2 --sot $SOT > /tmp/v6_baseline_input.txt
uv run kloc-cli context 'App\Entity\Order' --sot $SOT > /tmp/v6_baseline_class.txt
uv run kloc-cli context 'App\Entity\Order::$customerEmail' --sot $SOT > /tmp/v6_baseline_property.txt
```

---

## 10. Risk Assessment

| Risk | Level | Mitigation |
|------|-------|------------|
| sot.json regeneration breaks existing tests | MEDIUM | Capture baseline, run tests before/after |
| FQN change (:: -> .) is a wide blast radius | MEDIUM | Expected change, document in regression notes |
| Cross-method recursion infinite loops | HIGH | Depth limit + cycle detection (v6-E-UNIT-01/02) |
| Performance degradation with deep traversal | LOW | depth parameter limits expansion |
| Virtual relation invention (violating core principle) | HIGH | VERIFY checklist item in every phase gate |
| Backward compat with old sot.json | LOW | Position fallback tested (v6-D-INT-05) |
| Promoted property resolution fails | MEDIUM | assigned_from edge (63 exist) must be followed correctly |
