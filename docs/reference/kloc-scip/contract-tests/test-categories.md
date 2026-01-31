# Test Categories

Contract tests are organized into categories based on what aspect of the `calls.json` output they verify.

## Category 1: Reference Consistency

**File**: `tests/Reference/ParameterReferenceTest.php`

Reference consistency verifies that variables are tracked correctly:

1. **One Value Per Declaration**: Each parameter or local variable has exactly one value entry at its declaration site
2. **Consistent References**: All usages of the variable reference the same value ID via `receiver_value_id`

### Example Test

```php
public function testOrderParameterHasSingleValueEntry(): void
{
    $this->assertReferenceConsistency()
        ->inMethod('App\Repository\OrderRepository', 'save')
        ->forParameter('$order')
        ->verify();
}
```

### What This Verifies

For code like:
```php
public function save(Order $order): Order
{
    if ($order->id === 0) {              // usage 1
        $newOrder = new Order(...);
        self::$orders[$newOrder->id] = $newOrder;
        return $newOrder;
    }
    self::$orders[$order->id] = $order;  // usage 2
    return $order;                        // usage 3
}
```

Expected in `calls.json`:
- ONE value entry with `kind: "parameter"` and symbol containing `($order)`
- All access calls on `$order` (line 3, 8, 9) have `receiver_value_id` pointing to that one value

## Category 2: Chain Integrity

**File**: `tests/Chain/ChainIntegrityTest.php`

Chain integrity verifies that method/property chains are properly linked:

1. **Value->Call Linkage**: Each call's `receiver_value_id` points to a value
2. **Call->Result Linkage**: Each call has a result value with matching ID
3. **Result->Source Linkage**: Each result value's `source_call_id` points back

### Example Test

```php
public function testOrderRepositoryChain(): void
{
    $result = $this->assertChain()
        ->startingFrom('App\Service\OrderService', 'createOrder', '$this')
        ->throughAccess('orderRepository')
        ->throughMethod('save')
        ->verify();

    $this->assertEquals(2, $result->stepCount());
    $this->assertStringContainsString('Order', $result->finalType());
}
```

### What This Verifies

For code like:
```php
$savedOrder = $this->orderRepository->save($order);
```

Expected chain:
```
$this (parameter/local value)
  -> access "orderRepository" (call)
    -> result (OrderRepository type)
      -> method "save" (call)
        -> result (Order type)
          -> $savedOrder (local value, source_call_id points here)
```

## Category 3: Argument Binding

**File**: `tests/Argument/ArgumentBindingTest.php`

Argument binding verifies that method arguments are correctly linked to their values:

1. **Value ID Exists**: Each argument's `value_id` points to an existing value
2. **Correct Kind**: The referenced value has the expected kind (parameter, local, result)
3. **Correct Source**: The referenced value matches the expected variable/expression

### Example Test

```php
public function testSaveArgumentPointsToLocalOrder(): void
{
    $this->assertArgument()
        ->inMethod('App\Service\OrderService', 'createOrder')
        ->atCall('save')
        ->position(0)
        ->pointsToLocal('$order')
        ->verify();
}
```

### What This Verifies

For code like:
```php
$order = new Order(...);
$savedOrder = $this->orderRepository->save($order);
```

Expected:
- The `save` call has an argument at position 0
- That argument's `value_id` points to the `$order` local value
- The local value has symbol containing `local$order`

### Argument Position

Arguments are 0-indexed by position in the call:

```php
$this->emailSender->send(
    to: $savedOrder->customerEmail,    // position 0
    subject: 'Order Confirmation',      // position 1
    body: $message,                      // position 2
);
```

## Category 4: Data Integrity

**File**: `tests/Integrity/DataIntegrityTest.php`

Data integrity verifies overall structure correctness:

1. **No Duplicates**: No duplicate symbol entries
2. **No Orphans**: All ID references point to existing entries
3. **Complete Results**: Every call has a corresponding result value
4. **Type Consistency**: Result types match call return types

### Example Test

```php
public function testNoOrphanedReferences(): void
{
    $this->assertIntegrity()
        ->allReceiverValueIdsExist()
        ->allArgumentValueIdsExist()
        ->allSourceCallIdsExist()
        ->verify();
}
```

### What This Verifies

- Every `receiver_value_id` in calls points to an existing value
- Every `value_id` in arguments points to an existing value
- Every `source_call_id` in values points to an existing call
- Every `source_value_id` in values points to an existing value

## Test File Organization

```
tests/
  SmokeTest.php              # Basic framework validation
  Integrity/
    DataIntegrityTest.php    # Category 4 tests
  Reference/
    ParameterReferenceTest.php  # Category 1 tests
  Chain/
    ChainIntegrityTest.php   # Category 2 tests
  Argument/
    ArgumentBindingTest.php  # Category 3 tests
```

## Running by Category

```bash
# All tests
vendor/bin/phpunit

# Specific category
vendor/bin/phpunit --testsuite=smoke
vendor/bin/phpunit --testsuite=integrity
vendor/bin/phpunit --testsuite=reference
vendor/bin/phpunit --testsuite=chain
vendor/bin/phpunit --testsuite=argument
```
