# Specification: scip-php Missing Features Implementation

**Created**: 2026-02-01
**Status**: Draft
**Priority**: High
**Identified By**: Contract test coverage analysis (`feature/increase-coverage`)

## Overview

Contract testing revealed 15 documented behaviors in `calls-schema.json` that are not yet implemented in the scip-php indexer. This specification details each missing feature with acceptance criteria and implementation guidance.

## Summary of Missing Features

| Feature | Category | Schema Reference | Priority |
|---------|----------|------------------|----------|
| Static Method Calls | Call Kind | `method_static` | High |
| Static Property Access | Call Kind | `access_static` | High |
| Array Access | Call Kind | `access_array` | High |
| Nullsafe Method Calls | Call Kind | `method_nullsafe` | Medium |
| Nullsafe Property Access | Call Kind | `access_nullsafe` | Medium |
| Null Coalesce Operator | Operator | `coalesce` | Medium |
| Short Ternary Operator | Operator | `ternary` | Medium |
| Full Ternary Operator | Operator | `ternary_full` | Medium |
| Match Expression | Operator | `match` | Low |

---

## Feature 1: Static Method Calls (`method_static`)

### Documentation Reference
- **Schema**: [`calls-schema.json:129,136`](../reference/kloc-scip/calls-schema.json) - CallKind enum and description
- **Docs**: [`calls-and-data-flow.md:183`](../reference/kloc-scip/calls-and-data-flow.md#call-kinds) - Call Kinds table

### Description
Track static method calls like `Foo::bar()` in calls.json.

### PHP Pattern
```php
class OrderRepository {
    public static function getAll(): array {
        return self::$orders;
    }
}

// Usage
$orders = OrderRepository::getAll();
```

### Expected Output
```json
{
  "id": "src/Service.php:10:12",
  "kind": "method_static",
  "kind_type": "invocation",
  "caller": "scip-php composer . App/Service#process().",
  "callee": "scip-php composer . App/Repository/OrderRepository#getAll().",
  "return_type": "scip-php builtin . array#",
  "location": {"file": "src/Service.php", "line": 10, "col": 12},
  "receiver_value_id": null,
  "arguments": []
}
```

### Acceptance Criteria
- [ ] Static method calls produce `kind: "method_static"`
- [ ] `kind_type` is `"invocation"`
- [ ] `receiver_value_id` is `null` (no instance receiver)
- [ ] `callee` contains the class and method symbol
- [ ] `arguments` array is populated correctly
- [ ] Result value is created with matching ID

### Contract Test Reference
```
tests/CallKind/CallKindTest.php::testStaticMethodCallKind
```

---

## Feature 2: Static Property Access (`access_static`)

### Documentation Reference
- **Schema**: [`calls-schema.json:131,141`](../reference/kloc-scip/calls-schema.json) - CallKind enum and description
- **Docs**: [`calls-and-data-flow.md:188`](../reference/kloc-scip/calls-and-data-flow.md#call-kinds) - Call Kinds table

### Description
Track static property access like `Foo::$bar` in calls.json.

### PHP Pattern
```php
class OrderRepository {
    private static array $orders = [];

    public function save(Order $order): Order {
        self::$orders[$order->id] = $order;
        return $order;
    }
}
```

### Expected Output
```json
{
  "id": "src/Repository.php:15:8",
  "kind": "access_static",
  "kind_type": "access",
  "caller": "scip-php composer . App/Repository/OrderRepository#save().",
  "callee": "scip-php composer . App/Repository/OrderRepository#$orders.",
  "return_type": "scip-php builtin . array#",
  "location": {"file": "src/Repository.php", "line": 15, "col": 8},
  "receiver_value_id": null
}
```

### Acceptance Criteria
- [ ] Static property access produces `kind: "access_static"`
- [ ] `kind_type` is `"access"`
- [ ] `receiver_value_id` is `null`
- [ ] `callee` contains the class and property symbol
- [ ] Result value is created for the access

### Contract Test Reference
```
tests/CallKind/CallKindTest.php::testStaticPropertyAccessKind
```

---

## Feature 3: Array Access (`access_array`)

### Documentation Reference
- **Schema**: [`calls-schema.json:131,143`](../reference/kloc-scip/calls-schema.json) - CallKind enum and description
- **Schema**: [`calls-schema.json:249-254`](../reference/kloc-scip/calls-schema.json) - `key_value_id` field definition
- **Docs**: [`calls-and-data-flow.md:190,481-497`](../reference/kloc-scip/calls-and-data-flow.md#array-access) - Array Access section

### Description
Track array access operations like `$arr['key']` or `$arr[$index]` in calls.json.

### PHP Pattern
```php
public function getOrder(int $id): ?Order {
    return self::$orders[$id] ?? null;
}
```

### Expected Output
```json
{
  "id": "src/Repository.php:20:12",
  "kind": "access_array",
  "kind_type": "access",
  "caller": "scip-php composer . App/Repository/OrderRepository#getOrder().",
  "callee": null,
  "return_type": "scip-php composer . App/Entity/Order#",
  "location": {"file": "src/Repository.php", "line": 20, "col": 12},
  "receiver_value_id": "src/Repository.php:20:12",
  "key_value_id": "src/Repository.php:20:25"
}
```

### Acceptance Criteria
- [ ] Array access produces `kind: "access_array"`
- [ ] `kind_type` is `"access"`
- [ ] `callee` is `null` (no method/property being called)
- [ ] `receiver_value_id` points to the array value
- [ ] `key_value_id` points to the key/index value
- [ ] Result value is created for the accessed element

### Contract Test Reference
```
tests/CallKind/CallKindTest.php::testArrayAccessKind
```

---

## Feature 4: Nullsafe Method Calls (`method_nullsafe`)

### Documentation Reference
- **Schema**: [`calls-schema.json:129,137`](../reference/kloc-scip/calls-schema.json) - CallKind enum and description
- **Docs**: [`calls-and-data-flow.md:184`](../reference/kloc-scip/calls-and-data-flow.md#call-kinds) - Call Kinds table

### Description
Track nullsafe method calls like `$obj?->method()` in calls.json.

### PHP Pattern
```php
public function getEmail(): ?string {
    return $this->order?->getCustomer()?->getEmail();
}
```

### Expected Output
```json
{
  "id": "src/Service.php:25:18",
  "kind": "method_nullsafe",
  "kind_type": "invocation",
  "caller": "scip-php composer . App/Service#getEmail().",
  "callee": "scip-php composer . App/Entity/Order#getCustomer().",
  "return_type": "scip-php union . Customer|null#",
  "location": {"file": "src/Service.php", "line": 25, "col": 18},
  "receiver_value_id": "src/Service.php:25:12",
  "arguments": []
}
```

### Acceptance Criteria
- [ ] Nullsafe method calls produce `kind: "method_nullsafe"`
- [ ] `kind_type` is `"invocation"`
- [ ] `receiver_value_id` points to the receiver value
- [ ] Return type includes `null` in union
- [ ] Chain continues correctly after nullsafe call

### Contract Test Reference
```
tests/CallKind/CallKindTest.php::testNullsafeMethodCallKind
```

---

## Feature 5: Nullsafe Property Access (`access_nullsafe`)

### Documentation Reference
- **Schema**: [`calls-schema.json:131,142`](../reference/kloc-scip/calls-schema.json) - CallKind enum and description
- **Docs**: [`calls-and-data-flow.md:189`](../reference/kloc-scip/calls-and-data-flow.md#call-kinds) - Call Kinds table

### Description
Track nullsafe property access like `$obj?->property` in calls.json.

### PHP Pattern
```php
public function getName(): ?string {
    return $this->customer?->name;
}
```

### Expected Output
```json
{
  "id": "src/Service.php:30:12",
  "kind": "access_nullsafe",
  "kind_type": "access",
  "caller": "scip-php composer . App/Service#getName().",
  "callee": "scip-php composer . App/Entity/Customer#$name.",
  "return_type": "scip-php union . string|null#",
  "location": {"file": "src/Service.php", "line": 30, "col": 12},
  "receiver_value_id": "src/Service.php:30:12"
}
```

### Acceptance Criteria
- [ ] Nullsafe property access produces `kind: "access_nullsafe"`
- [ ] `kind_type` is `"access"`
- [ ] Return type includes `null` in union
- [ ] Result value is created

### Contract Test Reference
```
tests/CallKind/CallKindTest.php::testNullsafePropertyAccessKind
```

---

## Feature 6: Null Coalesce Operator (`coalesce`)

### Documentation Reference
- **Schema**: [`calls-schema.json:132,144`](../reference/kloc-scip/calls-schema.json) - CallKind enum and description
- **Schema**: [`calls-schema.json:256-268`](../reference/kloc-scip/calls-schema.json) - `left_value_id`, `right_value_id` field definitions
- **Docs**: [`calls-and-data-flow.md:191,415-437`](../reference/kloc-scip/calls-and-data-flow.md#null-coalesce-) - Null Coalesce section with example

### Description
Track null coalesce operator `$a ?? $b` in calls.json.

### PHP Pattern
```php
public function getOrder(int $id): Order {
    return self::$orders[$id] ?? new Order(0, '', '', 0, 'pending');
}
```

### Expected Output
```json
{
  "id": "src/Repository.php:45:12",
  "kind": "coalesce",
  "kind_type": "operator",
  "caller": "scip-php composer . App/Repository/OrderRepository#getOrder().",
  "callee": "scip-php operator . coalesce#",
  "return_type": "scip-php composer . App/Entity/Order#",
  "location": {"file": "src/Repository.php", "line": 45, "col": 12},
  "left_value_id": "src/Repository.php:45:12",
  "right_value_id": "src/Repository.php:45:35"
}
```

### Acceptance Criteria
- [ ] Null coalesce produces `kind: "coalesce"`
- [ ] `kind_type` is `"operator"`
- [ ] `callee` is `"scip-php operator . coalesce#"`
- [ ] `left_value_id` points to left operand value
- [ ] `right_value_id` points to right operand value
- [ ] `return_type` is union of (left type without null) and right type
- [ ] Result value is created

### Contract Test Reference
```
tests/Operator/OperatorTest.php::testCoalesceOperator
```

---

## Feature 7: Short Ternary Operator (`ternary`)

### Documentation Reference
- **Schema**: [`calls-schema.json:132,145`](../reference/kloc-scip/calls-schema.json) - CallKind enum and description
- **Schema**: [`calls-schema.json:270-289`](../reference/kloc-scip/calls-schema.json) - `condition_value_id`, `true_value_id`, `false_value_id` field definitions
- **Docs**: [`calls-and-data-flow.md:192,439-457`](../reference/kloc-scip/calls-and-data-flow.md#ternary--) - Ternary section with example

### Description
Track short ternary operator `$a ?: $b` (Elvis operator) in calls.json.

### PHP Pattern
```php
public function getDisplayName(): string {
    return $this->name ?: 'Anonymous';
}
```

### Expected Output
```json
{
  "id": "src/Entity.php:50:12",
  "kind": "ternary",
  "kind_type": "operator",
  "caller": "scip-php composer . App/Entity/User#getDisplayName().",
  "callee": "scip-php operator . ternary#",
  "return_type": "scip-php builtin . string#",
  "location": {"file": "src/Entity.php", "line": 50, "col": 12},
  "condition_value_id": "src/Entity.php:50:12",
  "false_value_id": "src/Entity.php:50:28"
}
```

### Acceptance Criteria
- [ ] Short ternary produces `kind: "ternary"`
- [ ] `kind_type` is `"operator"`
- [ ] `condition_value_id` points to the condition/true value
- [ ] `false_value_id` points to the false branch value
- [ ] `true_value_id` is `null` (condition is reused as true value)
- [ ] Result value is created

### Contract Test Reference
```
tests/Operator/OperatorTest.php::testShortTernaryOperator
```

---

## Feature 8: Full Ternary Operator (`ternary_full`)

### Documentation Reference
- **Schema**: [`calls-schema.json:132,146`](../reference/kloc-scip/calls-schema.json) - CallKind enum and description (Note: `ternary_full` in schema)
- **Schema**: [`calls-schema.json:270-289`](../reference/kloc-scip/calls-schema.json) - `condition_value_id`, `true_value_id`, `false_value_id` field definitions
- **Docs**: [`calls-and-data-flow.md:192,439-457`](../reference/kloc-scip/calls-and-data-flow.md#ternary--) - Ternary section with example

### Description
Track full ternary operator `$a ? $b : $c` in calls.json.

### PHP Pattern
```php
public function getStatus(): string {
    return $this->isActive ? 'active' : 'inactive';
}
```

### Expected Output
```json
{
  "id": "src/Entity.php:55:12",
  "kind": "ternary_full",
  "kind_type": "operator",
  "caller": "scip-php composer . App/Entity/User#getStatus().",
  "callee": "scip-php operator . ternary#",
  "return_type": "scip-php builtin . string#",
  "location": {"file": "src/Entity.php", "line": 55, "col": 12},
  "condition_value_id": "src/Entity.php:55:12",
  "true_value_id": "src/Entity.php:55:30",
  "false_value_id": "src/Entity.php:55:42"
}
```

### Acceptance Criteria
- [ ] Full ternary produces `kind: "ternary_full"`
- [ ] `kind_type` is `"operator"`
- [ ] `condition_value_id` points to condition value
- [ ] `true_value_id` points to true branch value
- [ ] `false_value_id` points to false branch value
- [ ] Result value is created with union type of true/false branches

### Contract Test Reference
```
tests/Operator/OperatorTest.php::testFullTernaryOperator
```

---

## Feature 9: Match Expression (`match`)

### Documentation Reference
- **Schema**: [`calls-schema.json:132,147`](../reference/kloc-scip/calls-schema.json) - CallKind enum and description
- **Schema**: [`calls-schema.json:291-302`](../reference/kloc-scip/calls-schema.json) - `subject_value_id`, `arm_ids` field definitions
- **Docs**: [`calls-and-data-flow.md:193,459-478`](../reference/kloc-scip/calls-and-data-flow.md#match-expression) - Match Expression section with example

### Description
Track match expressions in calls.json.

### PHP Pattern
```php
public function getStatusLabel(): string {
    return match($this->status) {
        'pending' => 'Pending Review',
        'active' => 'Active',
        'completed' => 'Completed',
        default => 'Unknown',
    };
}
```

### Expected Output
```json
{
  "id": "src/Entity.php:60:12",
  "kind": "match",
  "kind_type": "operator",
  "caller": "scip-php composer . App/Entity/Order#getStatusLabel().",
  "callee": "scip-php operator . match#",
  "return_type": "scip-php builtin . string#",
  "location": {"file": "src/Entity.php", "line": 60, "col": 12},
  "subject_value_id": "src/Entity.php:60:18",
  "arm_ids": [
    "src/Entity.php:61:23",
    "src/Entity.php:62:21",
    "src/Entity.php:63:25",
    "src/Entity.php:64:20"
  ]
}
```

### Acceptance Criteria
- [ ] Match expression produces `kind: "match"`
- [ ] `kind_type` is `"operator"`
- [ ] `callee` is `"scip-php operator . match#"`
- [ ] `subject_value_id` points to the match subject value
- [ ] `arm_ids` contains IDs of all arm result values
- [ ] Result value type is union of all arm result types

### Contract Test Reference
```
tests/Operator/OperatorTest.php::testMatchExpression
```

---

## Implementation Notes

### Priority Order
1. **High Priority** (Core functionality):
   - `method_static` - Common in repositories, factories
   - `access_static` - Used with static properties
   - `access_array` - Fundamental PHP pattern

2. **Medium Priority** (PHP 8 features):
   - `method_nullsafe` - PHP 8.0 feature
   - `access_nullsafe` - PHP 8.0 feature
   - `coalesce` - Common null handling
   - `ternary` / `ternary_full` - Common conditionals

3. **Low Priority** (Can defer):
   - `match` - PHP 8.0 feature, less common

### Reference Code Location
Add test patterns to `kloc-reference-project-php/src/` to ensure contract tests have code to validate against.

### Testing Strategy
1. Implement feature in scip-php
2. Run contract tests: `cd kloc-reference-project-php/contract-tests && bin/run.sh test`
3. Skipped tests should now pass
4. Generate updated docs: `bin/run.sh docs`

### Schema Reference
All features are documented in `docs/reference/kloc-scip/calls-schema.json`:
- `CallKind` enum defines all call kinds
- `CallKindType` enum defines categories
- `CallRecord` defines required/optional fields per kind

---

## Validation

When all features are implemented, the following should be true:
- All 106 contract tests pass (currently 91 pass, 15 skipped)
- `bin/run.sh docs` shows 0 skipped tests
- calls.json validates against the full schema
