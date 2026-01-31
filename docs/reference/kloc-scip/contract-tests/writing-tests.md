# Writing Contract Tests

This guide explains how to write new contract tests for the `scip-php` indexer.

## Prerequisites

1. Understand the [calls.json schema](../calls-and-data-flow.md)
2. Identify the PHP code pattern you want to test in `kloc-reference-project-php/src/`
3. Know the expected behavior according to the specification

## Test Structure

Every test class extends `CallsContractTestCase`:

```php
<?php

declare(strict_types=1);

namespace ContractTests\Tests\Category;

use ContractTests\CallsContractTestCase;

class MyFeatureTest extends CallsContractTestCase
{
    /**
     * Test: ClassName::methodName() - specific scenario
     *
     * Code reference: src/Path/To/File.php:line
     *   public function methodName(Type $param): ReturnType
     *
     * Expected: Description of expected behavior
     */
    public function testDescriptiveName(): void
    {
        // Test implementation
    }
}
```

## Best Practices

### 1. Always Reference Source Code

Include file path and line number for the code being tested:

```php
/**
 * Code reference: src/Repository/OrderRepository.php:26
 *   public function save(Order $order): Order
 */
```

### 2. Use Descriptive Test Names

Test names should describe what is being verified:

```php
public function testOrderParameterHasSingleValueEntry(): void
public function testSavedOrderChainHasCorrectLinkage(): void
public function testEmailSenderReceivesCustomerEmailArgument(): void
```

### 3. Include Expected Behavior

Document what the test expects to find:

```php
/**
 * Expected: One value entry for $order parameter.
 * All property accesses on $order should reference this single value ID.
 */
```

### 4. Provide Context on Failure

Use assertion messages that explain what went wrong:

```php
$this->assertNotEmpty(
    $values,
    'Should find value entry for $order parameter. ' .
    'Reference: src/Repository/OrderRepository.php:26'
);
```

## Writing Query Tests

Use queries to find specific entries:

```php
public function testFindSpecificParameter(): void
{
    // Find by kind and symbol
    $values = $this->values()
        ->kind('parameter')
        ->symbolContains('OrderRepository#save().($order)')
        ->all();

    $this->assertCount(1, $values, 'Should find exactly one parameter');

    // Verify properties
    $param = $values[0];
    $this->assertEquals('parameter', $param['kind']);
    $this->assertStringContainsString('Order', $param['type']);
}
```

## Writing Assertion Tests

Use fluent assertions for complex verifications:

```php
public function testParameterReferenceConsistency(): void
{
    $result = $this->assertReferenceConsistency()
        ->inMethod('App\Repository\OrderRepository', 'save')
        ->forParameter('$order')
        ->verify();

    // Optionally check result details
    $this->assertEquals(1, $result->valueCount);
    $this->assertGreaterThan(0, $result->callCount);
}
```

## Common Patterns

### Testing Parameter Values

```php
public function testParameterExists(): void
{
    $params = $this->inMethod('App\Repository\OrderRepository', 'save')
        ->values()
        ->kind('parameter')
        ->all();

    $this->assertNotEmpty($params);

    // Find specific parameter
    $orderParam = null;
    foreach ($params as $p) {
        if (str_contains($p['symbol'], '($order)')) {
            $orderParam = $p;
            break;
        }
    }

    $this->assertNotNull($orderParam, 'Should find $order parameter');
    $this->assertStringContainsString('Order', $orderParam['type']);
}
```

### Testing Method Calls

```php
public function testMethodCallExists(): void
{
    $calls = $this->inMethod('App\Service\OrderService', 'createOrder')
        ->calls()
        ->kind('method')
        ->calleeContains('save')
        ->all();

    $this->assertCount(1, $calls);

    $saveCall = $calls[0];
    $this->assertArrayHasKey('receiver_value_id', $saveCall);
    $this->assertArrayHasKey('arguments', $saveCall);
}
```

### Testing Property Access

```php
public function testPropertyAccess(): void
{
    $accesses = $this->inMethod('App\Service\OrderService', 'createOrder')
        ->calls()
        ->kind('access')
        ->calleeContains('customerEmail')
        ->all();

    $this->assertNotEmpty($accesses);

    foreach ($accesses as $access) {
        // Verify each access has receiver
        $this->assertArrayHasKey('receiver_value_id', $access);
        $receiverId = $access['receiver_value_id'];

        // Verify receiver exists
        $receiver = $this->callsData()->getValueById($receiverId);
        $this->assertNotNull($receiver, "Receiver {$receiverId} should exist");
    }
}
```

### Testing Argument Binding

```php
public function testArgumentBinding(): void
{
    // Find the call
    $call = $this->calls()
        ->callerContains('OrderService#createOrder()')
        ->calleeContains('save')
        ->one();

    // Check arguments
    $args = $call['arguments'];
    $this->assertNotEmpty($args);

    // Verify first argument
    $arg0 = $args[0];
    $this->assertEquals(0, $arg0['position']);
    $this->assertArrayHasKey('value_id', $arg0);

    // Verify argument points to local variable
    $argValue = $this->callsData()->getValueById($arg0['value_id']);
    $this->assertEquals('local', $argValue['kind']);
    $this->assertStringContainsString('order', $argValue['symbol']);
}
```

### Testing Chain Integrity

```php
public function testChainIntegrity(): void
{
    // Start from a local variable
    $localVar = $this->values()
        ->kind('local')
        ->symbolContains('savedOrder')
        ->first();

    $this->assertNotNull($localVar);

    // Find calls using this as receiver
    $calls = $this->calls()
        ->withReceiverValueId($localVar['id'])
        ->all();

    $this->assertNotEmpty($calls);

    // Verify each call has result value
    foreach ($calls as $call) {
        $resultValue = $this->callsData()->getValueById($call['id']);
        $this->assertNotNull($resultValue, "Call {$call['id']} should have result value");
        $this->assertEquals('result', $resultValue['kind']);
    }
}
```

## Debugging Tips

### Dump All Values in Scope

```php
$values = $this->inMethod('App\Repository\OrderRepository', 'save')
    ->values()
    ->all();

foreach ($values as $v) {
    echo sprintf(
        "%s: %s (id=%s)\n",
        $v['kind'],
        $v['symbol'] ?? '<no symbol>',
        $v['id']
    );
}
```

### Check calls.json Directly

```php
$path = CALLS_JSON_PATH;
$json = file_get_contents($path);
$data = json_decode($json, true);

// Find entries matching pattern
$matches = array_filter(
    $data['values'],
    fn($v) => str_contains($v['symbol'] ?? '', 'OrderRepository')
);
```

### Inspect Linkages

```php
// Find a value and trace its usage
$value = $this->values()->kind('parameter')->symbolContains('order')->first();

echo "Value ID: {$value['id']}\n";

// Find calls that use this value
$calls = $this->calls()->withReceiverValueId($value['id'])->all();
echo "Used as receiver in " . count($calls) . " calls\n";

foreach ($calls as $call) {
    echo "  - {$call['kind']} to {$call['callee']}\n";
}
```

## Adding New Test Files

1. Create file in appropriate directory:
   - `tests/Integrity/` for data structure tests
   - `tests/Reference/` for variable reference tests
   - `tests/Chain/` for chain linkage tests
   - `tests/Argument/` for argument binding tests

2. Extend `CallsContractTestCase`

3. Add descriptive PHPDoc with code references

4. Run tests to verify:
   ```bash
   vendor/bin/phpunit tests/YourNewTest.php
   ```

5. Update `phpunit.xml` if adding new directory (usually not needed)
