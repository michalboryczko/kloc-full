# QA Reference Note: Value Context v5 (Issues A-B)

**Date:** 2026-02-11
**Feature Branch:** feature/cli-context-fix
**Scope:** kloc-cli only (both issues)
**v5 Spec:** `docs/specs/value-context.md` (22 acceptance criteria total)
**v5 Todo:** `docs/todo/context-command-v5/` (detailed examples)
**Issues:** A (Value data flow traversal), B (Value definition enhancement)
**Depends on:** v4 feature (shipped: impl execution flow, local variable identity)
**Note:** CLI-only. No scip-php, kloc-mapper, or kloc-contracts changes needed.

---

## 0. Acceptance Criteria Traceability Matrix

### ISSUE-A: Value Node Data Flow Traversal (12 ACs)

| Spec AC | Description | Test Scenarios | Test Type |
|---------|-------------|----------------|-----------|
| AC 1 | Local variable USES shows source call at depth 1 | v5-A-INT-01 | Integration |
| AC 2 | USES traces recursively at depth >= 2 | v5-A-INT-02, v5-A-INT-03 | Integration |
| AC 3 | Parameter USES is empty | v5-A-INT-04 | Integration |
| AC 4 | USED BY shows receiver property accesses grouped by consuming Call, sorted by line | v5-A-INT-05, v5-A-INT-06 | Integration |
| AC 5 | USED BY forward trace at depth >= 2 (promoted property, callee body) | v5-A-INT-07 | Integration |
| AC 6 | Value used directly as argument shows Call with param mapping in USED BY | v5-A-INT-08 | Integration |
| AC 7 | JSON output uses same ContextEntry structure as method queries | v5-A-INT-09, v5-A-INT-10 | Integration |
| AC 8 | Literal with no receiver/argument edges has empty USED BY | v5-A-UNIT-01 | Unit |
| AC 9 | Result Value USES shows producing Call at depth 1 | v5-A-INT-11 | Integration |
| AC 10 | Method/Function context queries unchanged (regression) | v5-REG-01 | Regression |
| AC 11 | USES entries reuse existing argument display format | v5-A-INT-01 (implicit) | Integration |
| AC 12 | USED BY entries sorted by source line number | v5-A-INT-05 | Integration |

### ISSUE-B: Value Node Definition Enhancement (10 ACs)

| Spec AC | Description | Test Scenarios | Test Type |
|---------|-------------|----------------|-----------|
| AC 13 | Local variable shows "Kind: Value (local)" | v5-B-INT-01 | Integration |
| AC 14 | Parameter shows "Kind: Value (parameter)" | v5-B-INT-02 | Integration |
| AC 15 | Result shows "Kind: Value (result)" | v5-B-INT-03 | Integration |
| AC 16 | Value with type_of edge shows "Type: {class FQN}" | v5-B-INT-04 | Integration |
| AC 17 | Value with no type_of edge omits Type line | v5-B-INT-05 | Integration |
| AC 18 | Local with assigned_from chain shows "Source: {method}() result (line N)" | v5-B-INT-06 | Integration |
| AC 19 | Parameter omits Source | v5-B-INT-02 (implicit) | Integration |
| AC 20 | Any Value shows "Scope: {containing method FQN}" | v5-B-INT-01 (implicit) | Integration |
| AC 21 | JSON includes value_kind, type, source in definition | v5-B-INT-07 | Integration |
| AC 22 | Method/Function/Class/Property definitions unchanged (regression) | v5-REG-02 | Regression |

---

## 1. Test Scenarios: ISSUE-A -- Value Data Flow Traversal

### 1.1 Integration Tests (reference project sot.json)

| ID | Scenario | GIVEN | WHEN | THEN | AC |
|----|----------|-------|------|------|----|
| v5-A-INT-01 | Local variable USES: $savedOrder source call | `$savedOrder` local at line 45 with `assigned_from` -> result -> save() Call | `context "...local$savedOrder@45" --depth 1` | USES has 1 entry at depth 1: FQN contains `OrderRepositoryInterface::save()`, kind is "Call", has arguments list with `$processedOrder` | AC 1, 11 |
| v5-A-INT-02 | Recursive USES: $savedOrder depth 3 | Same node | `context "...local$savedOrder@45" --depth 3` | USES traces 3 levels: [1] save() -> [2] process() -> [3] Order::__construct(). Each level has correct arguments. | AC 2 |
| v5-A-INT-03 | Recursive USES: depth limits expansion | Same node | `context "...local$savedOrder@45" --depth 1` | USES has exactly 1 entry at depth 1, no children (depth 2+ not expanded) | AC 2 |
| v5-A-INT-04 | Parameter USES is empty | `$input` parameter of createOrder() | `context "...createOrder().$input"` | USES list is empty (len == 0) | AC 3 |
| v5-A-INT-05 | USED BY: $savedOrder receiver accesses sorted by line | `$savedOrder` with 12 receiver edges | `context "...local$savedOrder@45"` | USED BY has >= 4 entries. First entry's line < second entry's line < third entry's line (sorted). Entries include send(), sprintf()/related, OrderCreatedMessage, OrderOutput consumers. | AC 4, 12 |
| v5-A-INT-06 | USED BY: $input property accesses grouped by Call | `$input` parameter with 5 receiver edges | `context "...createOrder().$input"` | USED BY has >= 2 entries: one for checkAvailability() (line 30) and one for Order::__construct() (line 32). Each groups multiple property accesses. | AC 4 |
| v5-A-INT-07 | USED BY forward trace: depth 2 promoted property | `$savedOrder` | `context "...local$savedOrder@45" --depth 2` | At least one USED BY depth-1 entry (OrderCreatedMessage::__construct or OrderOutput::__construct) has children at depth 2 referencing promoted properties | AC 5 |
| v5-A-INT-08 | USED BY: direct argument pass ($processedOrder) | `$processedOrder` local at line 42, passed as arg to save() (no receiver/property access) | `context "...local$processedOrder@42"` | USED BY has >= 1 entry: save() Call, with parameter mapping showing `$processedOrder` as `$order` arg | AC 6 |
| v5-A-INT-09 | JSON output: USES structure | `$savedOrder` | `context "...local$savedOrder@45" --json` | JSON `uses` array has entries with `depth`, `fqn`, `kind`, `arguments` fields. Arguments have `param_name`, `value_expr` etc. | AC 7 |
| v5-A-INT-10 | JSON output: USED BY structure | `$savedOrder` | `context "...local$savedOrder@45" --json` | JSON `used_by` array has entries with `depth`, `fqn`, `kind` fields. Entries are in line-number order. | AC 7 |
| v5-A-INT-11 | Result Value USES: producing Call | A result Value node (e.g., `file:line:(result)`) | `context "file:line:(result)"` | USES has >= 1 entry showing the Call that produced this result | AC 9 |

### 1.2 Unit Tests (in-memory fixtures)

| ID | Scenario | GIVEN | WHEN | THEN | AC |
|----|----------|-------|------|------|----|
| v5-A-UNIT-01 | Literal value has empty USED BY | Value node with value_kind="literal", no incoming receiver/argument edges | Execute context query | USED BY list is empty | AC 8 |
| v5-A-UNIT-02 | Value with no assigned_from has empty USES | Value node with no outgoing assigned_from edges | Execute context query | USES list is empty | AC 1 (inverse) |
| v5-A-UNIT-03 | Reverse lookup: receiver edges found | Value node with 3 incoming receiver edges | Access `incoming[value_id]["receiver"]` | Returns 3 edges with correct source Call IDs | Graph API |
| v5-A-UNIT-04 | Reverse lookup: argument edges found | Value node with 2 incoming argument edges | Access `incoming[value_id]["argument"]` | Returns 2 edges with correct source Call IDs | Graph API |

### 1.3 Regression Tests

| ID | Scenario | GIVEN | WHEN | THEN | AC |
|----|----------|-------|------|------|----|
| v5-REG-01 | Method context query unchanged | `OrderService::createOrder()` (Method node) | `context "OrderService::createOrder()" --depth 2` | Result has non-empty USES with execution flow entries. Output structure identical to v4 (same entry types, same argument format, same impl blocks). | AC 10 |
| v5-REG-02 | Class context query unchanged | `App\Entity\Order` (Class node) | `context "Order"` | USES shows structural type references. USED BY shows callers. No change from v4. | AC 22 |

---

## 2. Test Scenarios: ISSUE-B -- Value Definition Enhancement

### 2.1 Integration Tests (reference project sot.json)

| ID | Scenario | GIVEN | WHEN | THEN | AC |
|----|----------|-------|------|------|----|
| v5-B-INT-01 | Local definition: value_kind, scope | `$savedOrder` (value_kind="local") in createOrder() | `context "...local$savedOrder@45"` | definition.kind == "Value", definition.value_kind == "local", definition.declared_in.fqn contains "createOrder()" | AC 13, 20 |
| v5-B-INT-02 | Parameter definition: value_kind, no source | `$input` (value_kind="parameter") in createOrder() | `context "...createOrder().$input"` | definition.value_kind == "parameter", definition.source is None, definition.declared_in.fqn contains "createOrder()" | AC 14, 19 |
| v5-B-INT-03 | Result definition: value_kind | A result Value node | `context "file:line:(result)"` | definition.value_kind == "result" | AC 15 |
| v5-B-INT-04 | Type resolution from type_of edge | `$savedOrder` has type_of -> Order | `context "...local$savedOrder@45"` | definition.type_info is not None, definition.type_info["fqn"] contains "Order", definition.type_info["name"] == "Order" | AC 16 |
| v5-B-INT-05 | No type_of: type_info omitted | `$order@32` (local without type_of edge) | `context "...local$order@32"` | definition.type_info is None | AC 17 |
| v5-B-INT-06 | Source from assigned_from chain | `$savedOrder` has assigned_from -> produces -> save() Call | `context "...local$savedOrder@45"` | definition.source is not None, definition.source["method_name"] contains "save" | AC 18 |
| v5-B-INT-07 | JSON definition includes new fields | `$savedOrder` | `context "...local$savedOrder@45" --json` | JSON definition object has "value_kind": "local", "type" object with "fqn" and "name", "source" object with "method_fqn" and "line", "declared_in" object | AC 21 |

### 2.2 Unit Tests (in-memory fixtures)

| ID | Scenario | GIVEN | WHEN | THEN | AC |
|----|----------|-------|------|------|----|
| v5-B-UNIT-01 | Literal definition: minimal fields | Value with value_kind="literal", no type_of, no assigned_from | Execute context query | definition.value_kind == "literal", definition.type_info is None, definition.source is None | Edge case |
| v5-B-UNIT-02 | Union type: multiple type_of edges | Value with 2 type_of edges (Order + Serializable) | Execute context query | definition.type_info shows both types (pipe-separated or list) | Edge case |
| v5-B-UNIT-03 | Promoted constructor parameter: source shows property | Parameter Value with assigned_from -> Property node | Execute context query | definition.source shows the promoted property relationship | Edge case |

---

## 3. Edge Case Matrix

### ISSUE-A Edge Cases

| Edge Case | Expected Behavior | Test Coverage |
|-----------|-------------------|---------------|
| Parameter Values | USES empty. USED BY shows consumption within method body. | v5-A-INT-04, v5-A-INT-06 |
| Literal values | USES empty, USED BY empty (unless literal is used as argument). | v5-A-UNIT-01 |
| Result values without variable name | USES shows producing Call. USED BY shows where result flows. | v5-A-INT-11 |
| Promoted constructor parameters | USED BY shows promoted property assignment at depth 1. | v5-A-INT-07 |
| Chained property access ($obj->prop->method()) | At depth 2+, result of property access becomes receiver of next call. | Manual verification |
| Multiple assignments to same variable | Each creates separate Value with @line suffix. Distinct queries. | Implicit (separate nodes) |
| Variable with no consumers | USES shows source chain. USED BY is empty. | v5-A-UNIT-02 (inverse) + Manual |
| Value with no source (constructed inline) | USES may be empty if no assigned_from edge. | v5-A-UNIT-02 |
| Depth exhaustion | USES/USED BY stop expanding at max_depth. No error. | v5-A-INT-03 |

### ISSUE-B Edge Cases

| Edge Case | Expected Behavior | Test Coverage |
|-----------|-------------------|---------------|
| Promoted constructor parameter | value_kind="parameter", source shows promoted property. | v5-B-UNIT-03 |
| No type_of edge | Type line omitted from definition. | v5-B-INT-05 |
| Union types (multiple type_of edges) | Types shown pipe-separated: "Type: User\|null". | v5-B-UNIT-02 |
| Literal values | value_kind="literal". No type, no source. | v5-B-UNIT-01 |
| Constant values | value_kind="constant". No type, no source. | Manual verification |
| Scope resolution via contains parent | All Value nodes should resolve to a containing Method/Function. | v5-B-INT-01 (implicit) |

---

## 4. Test Infrastructure

### 4.1 Test Files

| File | Purpose | Test Type |
|------|---------|-----------|
| `kloc-cli/tests/test_usage_flow.py` | Extend with `TestV5IssueAValueDataFlow` and `TestV5IssueBValueDefinition` classes | Integration |
| `kloc-cli/tests/test_index.py` | Extend with Value reverse-lookup unit tests | Unit |
| `kloc-cli/tests/test_value_context.py` (new, optional) | Dedicated file if test count is large | Unit + Integration |

### 4.2 Fixture Requirements

| Fixture | Used By | Status | Action |
|---------|---------|--------|--------|
| Reference project sot.json | All v5-A-INT-*, v5-B-INT-* tests | EXISTS at `kloc-reference-project-php/contract-tests/output/sot.json` | No regeneration needed (CLI-only changes) |
| In-memory Value fixtures | v5-A-UNIT-*, v5-B-UNIT-* tests | NEEDS CREATION | Create minimal sot.json fixtures with Value/Call nodes, receiver/argument/assigned_from/type_of edges |

### 4.3 Helper Functions (extend or create)

| Helper | Purpose | Location |
|--------|---------|----------|
| `find_entry_by_fqn()` | Find USES/USED BY entry by FQN substring | Already in test_usage_flow.py |
| `find_entry_by_line()` | Find USED BY entry by source line number (NEW) | Add to test_usage_flow.py |
| `resolve_value_node()` | Resolve a Value node by FQN substring (NEW) | Add to test helpers |
| `assert_definition_field()` | Assert DefinitionInfo has specific value_kind/type_info/source (NEW) | Add to test helpers |

### 4.4 Existing Test Impact

| File | Tests Affected | Impact | Cause |
|------|----------------|--------|-------|
| `test_usage_flow.py` | Tests querying Method nodes | NO CHANGE expected -- Value changes are additive | v5 does not modify Method/Function paths |
| `test_integration.py` | Structural graph tests | NO CHANGE -- graph API unchanged | v5 adds no new edge types |
| `test_index.py` | Index unit tests | NO CHANGE for existing tests. NEW tests added for reverse lookups if new methods created. | New methods only |
| `test_callee_verification.py` | find_call_for_usage tests | NO CHANGE | v5 does not modify call matching |
| `test_reference_type.py` | _infer_reference_type tests | NO CHANGE | v5 does not modify reference type inference |

**Key point: v5 has LOW regression risk.** Both issues add NEW code paths for Value nodes. The existing Method/Function/Class paths are not modified. Regression tests v5-REG-01 and v5-REG-02 confirm this.

---

## 5. Regression Risks

| Risk | Level | What Could Break | Mitigation |
|------|-------|-----------------|------------|
| ContextResult structure change | LOW | Adding Value entries to uses/used_by could affect consumers that iterate all entries | Entries only added when querying Value nodes, not Method/Class |
| DefinitionInfo new fields | LOW | Adding value_kind/type_info/source fields to DefinitionInfo. Existing code may not handle them. | New fields have None defaults. Tree/JSON renderers must check before displaying. |
| JSON output schema change | MEDIUM | JSON output for Value queries will have new structure. Any downstream consumer parsing JSON needs updating. | New fields are additive. Existing fields unchanged. |
| depth parameter interaction | LOW | Recursive source/consumer chain respects max_depth. Edge case: depth=0 should still show definition. | v5-A-INT-03 tests depth limiting |
| Performance with many receiver edges | LOW | $savedOrder has 12 receiver edges -- max in reference project. Real codebases may have more. | Existing limit parameter caps output |
| Interaction with v4 [local]/[param] tags | LOW | Value query receiver info in USED BY must be consistent with v4 on: line format | Verify in v5-A-INT-05 |

---

## 6. Phase Gate Checklist

### Phase 1: ISSUE-B -- Value Definition Enhancement (S effort, do first)

```
[ ] DefinitionInfo dataclass has new fields: value_kind, type_info, source
[ ] v5-B-INT-01: $savedOrder definition shows "Value (local)" with scope
[ ] v5-B-INT-02: $input definition shows "Value (parameter)" with no source
[ ] v5-B-INT-03: Result value definition shows "Value (result)"
[ ] v5-B-INT-04: $savedOrder definition shows Type: App\Entity\Order
[ ] v5-B-INT-05: $order@32 definition omits Type (no type_of edge)
[ ] v5-B-INT-06: $savedOrder definition shows Source: save() result
[ ] v5-B-INT-07: JSON definition includes value_kind, type, source
[ ] v5-B-UNIT-01: Literal definition has minimal fields
[ ] v5-B-UNIT-02: Union type shown correctly
[ ] v5-B-UNIT-03: Promoted constructor param shows property source
[ ] v5-REG-02: Method/Class/Property definitions unchanged
[ ] All existing tests pass (no regression)
```

### Phase 2: ISSUE-A -- Value Data Flow Traversal (L effort)

```
[ ] New graph API methods added (if needed): get_calls_with_receiver(), get_calls_with_argument()
[ ] _build_value_source_chain() implemented and tested
[ ] _build_value_consumer_chain() implemented and tested
[ ] v5-A-INT-01: $savedOrder USES shows save() at depth 1
[ ] v5-A-INT-02: $savedOrder USES traces 3 levels deep
[ ] v5-A-INT-03: Depth 1 limits to single entry
[ ] v5-A-INT-04: $input USES is empty (parameter)
[ ] v5-A-INT-05: $savedOrder USED BY sorted by line number
[ ] v5-A-INT-06: $input USED BY groups by consuming Call
[ ] v5-A-INT-07: Depth 2 USED BY shows promoted properties
[ ] v5-A-INT-08: $processedOrder USED BY shows direct arg pass
[ ] v5-A-INT-09: JSON USES has ContextEntry structure
[ ] v5-A-INT-10: JSON USED BY has ContextEntry structure
[ ] v5-A-INT-11: Result value USES shows producing Call
[ ] v5-A-UNIT-01: Literal with no edges has empty USED BY
[ ] v5-A-UNIT-02: Value with no assigned_from has empty USES
[ ] v5-A-UNIT-03: Receiver edge reverse lookup works
[ ] v5-A-UNIT-04: Argument edge reverse lookup works
[ ] v5-REG-01: Method context query unchanged
[ ] All existing tests pass (no regression)
```

### Final Gate (both issues complete)

```
[ ] All v5-A-INT tests pass
[ ] All v5-A-UNIT tests pass
[ ] All v5-B-INT tests pass
[ ] All v5-B-UNIT tests pass
[ ] v5-REG-01 and v5-REG-02 pass (regression)
[ ] Full existing test suite green
[ ] Manual verification: kloc context "$savedOrder" shows source chain + consumers
[ ] Manual verification: kloc context "$input" shows parameter definition + consumers
[ ] Manual verification: kloc context "OrderService::createOrder()" unchanged
[ ] JSON output verified for both issues
```

---

## 7. Manual Testing Steps

```bash
cd /Users/michal/dev/ai/kloc/kloc-cli
SOT=../kloc-reference-project-php/contract-tests/output/sot.json

# ISSUE-B: Definition enhancement
uv run kloc-cli context 'App\Service\OrderService::createOrder().local$savedOrder@45' --sot $SOT
# Check:
# [x] Kind: Value (local)
# [x] Type: App\Entity\Order
# [x] Source: save() result (line 45)
# [x] Scope: App\Service\OrderService::createOrder()

uv run kloc-cli context 'App\Service\OrderService::createOrder().$input' --sot $SOT
# Check:
# [x] Kind: Value (parameter)
# [x] Type: App\Dto\CreateOrderInput
# [x] No Source line (parameter)
# [x] Scope: App\Service\OrderService::createOrder()

# ISSUE-A: Data flow traversal
uv run kloc-cli context 'App\Service\OrderService::createOrder().local$savedOrder@45' --depth 3 --sot $SOT
# Check USES:
# [x] [1] OrderRepositoryInterface::save() with args ($processedOrder)
# [x] [2] AbstractOrderProcessor::process() with args ($order)
# [x] [3] Order::__construct() with args ($input->customerEmail, etc.)
#
# Check USED BY:
# [x] [1] EmailSenderInterface::send() (line 48) with property accesses
# [x] [1] sprintf() or related (line 51) with property accesses
# [x] [1] OrderCreatedMessage::__construct() (line 58)
# [x] [1] OrderOutput::__construct() (line 61)
# [x] Entries sorted by line number

uv run kloc-cli context 'App\Service\OrderService::createOrder().local$processedOrder@42' --sot $SOT
# Check:
# [x] USES: [1] AbstractOrderProcessor::process()
# [x] USED BY: [1] OrderRepositoryInterface::save() with $processedOrder as $order arg

# JSON verification
uv run kloc-cli context 'App\Service\OrderService::createOrder().local$savedOrder@45' --json --sot $SOT | python3 -m json.tool
# Check:
# [x] definition has value_kind, type, source, declared_in
# [x] uses array has ContextEntry structure with depth/fqn/arguments
# [x] used_by array has entries in line-number order

# Regression: Method query unchanged
uv run kloc-cli context 'App\Service\OrderService::createOrder()' --depth 2 --sot $SOT
# Check: Output identical to v4 (execution flow, impl blocks, [local]/[param] tags)

# Regression: Class query unchanged
uv run kloc-cli context 'App\Entity\Order' --sot $SOT
# Check: Output identical to v4 (structural type references, usages)
```

---

## 8. Cross-Issue Interaction Notes

### ISSUE-A + ISSUE-B combined
Both issues are independent and apply additively. ISSUE-B populates the DEFINITION section. ISSUE-A populates the USES/USED BY sections. They do not conflict. The DEFINITION section changes are visible even at depth 0. USES/USED BY require depth >= 1.

### v5 + v4 interaction
v5 builds on v4's local variable identity (`local$name@line` FQN format, `[local]/[param]` tags). The new source chain and consumer chain in v5 ISSUE-A will use the same receiver display format with `[local]/[param]` tags established in v4 ISSUE-B.

### No upstream changes
Both issues are CLI-only. The sot.json fixture does not need regeneration. All 411 Value nodes and their ~1,350 edge connections already exist in the reference project's sot.json.
