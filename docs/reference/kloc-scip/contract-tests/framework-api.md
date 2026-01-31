# Framework API Reference

## Base Test Class

All contract tests extend `CallsContractTestCase`:

```php
<?php

namespace ContractTests\Tests;

use ContractTests\CallsContractTestCase;

class MyTest extends CallsContractTestCase
{
    public function testSomething(): void
    {
        // Use query and assertion methods from base class
    }
}
```

## Query API

### ValueQuery

Query for values in the `calls.json` output.

```php
// Start a query
$query = $this->values();

// Filter by kind
$query->kind('parameter');  // parameter, local, literal, constant, result

// Filter by symbol
$query->symbol('exact symbol match');
$query->symbolContains('OrderRepository');
$query->symbolMatches('*OrderRepository#save().*');  // wildcard pattern

// Filter by location
$query->inFile('src/Repository/OrderRepository.php');
$query->atLine(26);

// Filter by linkage
$query->hasSourceCallId();   // values assigned from calls
$query->hasSourceValueId();  // values assigned from other values

// Get results
$all = $query->all();        // array of matching values
$first = $query->first();    // first match or null
$one = $query->one();        // exactly one match (fails if != 1)
$count = $query->count();    // count of matches

// Assertions
$query->assertCount(1, 'Should find exactly one');
```

### CallQuery

Query for calls in the `calls.json` output.

```php
// Start a query
$query = $this->calls();

// Filter by kind
$query->kind('method');      // method, method_static, method_nullsafe
$query->kind('access');      // access, access_static, access_nullsafe, access_array
$query->kind('constructor'); // constructor
$query->kind('function');    // function

// Filter by kind type
$query->kindType('invocation');  // method, function, constructor calls
$query->kindType('access');      // property access
$query->kindType('operator');    // ??, ?:, match

// Filter by caller/callee
$query->callerContains('OrderRepository');
$query->callerMatches('*OrderRepository#save().*');
$query->calleeContains('customerEmail');
$query->calleeMatches('*Order#$customerEmail.');

// Filter by receiver
$query->hasReceiver();              // calls with receiver_value_id
$query->withReceiverValueId($id);   // specific receiver

// Filter by location
$query->inFile('src/Service/OrderService.php');
$query->atLine(40);
$query->inMethod('App\Service\OrderService', 'createOrder');

// Get results
$all = $query->all();
$first = $query->first();
$one = $query->one();
$count = $query->count();

// Assertions
$query->assertCount(5, 'Should find 5 calls');
$query->assertAllShareReceiver('All should use same receiver');
```

### MethodScope

Scoped queries within a specific method.

```php
// Create scope
$scope = $this->inMethod('App\Repository\OrderRepository', 'save');

// Query within scope
$params = $scope->values()->kind('parameter')->all();
$calls = $scope->calls()->kind('method')->all();

// Get scope info
$scope->getScopePattern();  // "App/Repository/OrderRepository#save()"
$scope->getClass();         // "App\Repository\OrderRepository"
$scope->getMethod();        // "save"
```

## Assertion API

### ReferenceConsistencyAssertion

Verify that a variable has exactly one value entry and all usages reference it.

```php
$this->assertReferenceConsistency()
    ->inMethod('App\Repository\OrderRepository', 'save')
    ->forParameter('$order')
    ->verify();

$this->assertReferenceConsistency()
    ->inMethod('App\Repository\OrderRepository', 'save')
    ->forLocal('$newOrder')
    ->verify();
```

**Verifies:**
- Exactly one value entry exists for the variable
- All calls using this variable as receiver reference that value ID

### ChainIntegrityAssertion

Verify that call chains are properly linked.

```php
$result = $this->assertChain()
    ->startingFrom('App\Service\OrderService', 'createOrder', '$this')
    ->throughAccess('orderRepository')
    ->throughMethod('save')
    ->verify();

// Access result
$result->stepCount();   // 2
$result->finalType();   // "...Order#"
$result->rootValue();   // starting value
$result->finalValue();  // final result value
$result->steps();       // all steps as array
```

**Verifies:**
- Each step's `receiver_value_id` points to previous step's result
- Each call has a corresponding result value
- Chain structure matches expected pattern

### ArgumentBindingAssertion

Verify that argument values are correctly bound.

```php
// Verify argument points to parameter
$this->assertArgument()
    ->inMethod('App\Service\OrderService', 'createOrder')
    ->atCall('save')
    ->position(0)
    ->pointsToLocal('$order')
    ->verify();

// Verify argument points to result of access
$this->assertArgument()
    ->inMethod('App\Service\OrderService', 'createOrder')
    ->atCall('send')
    ->position(0)
    ->pointsToResultOf('access', 'customerEmail')
    ->verify();

// Verify argument is literal
$this->assertArgument()
    ->inMethod('App\Service\OrderService', 'createOrder')
    ->atLine(32)
    ->position(0)
    ->pointsToLiteral()
    ->verify();
```

**Verifies:**
- Argument `value_id` exists
- Referenced value has expected kind
- Referenced value has expected name/source

### DataIntegrityAssertion

Run data structure integrity checks.

```php
// Configure checks
$this->assertIntegrity()
    ->noParameterDuplicates()
    ->noLocalDuplicatesPerLine()
    ->allReceiverValueIdsExist()
    ->allArgumentValueIdsExist()
    ->allSourceCallIdsExist()
    ->allSourceValueIdsExist()
    ->everyCallHasResultValue()
    ->resultValueTypesMatch()
    ->verify();

// Get report without failing
$report = $this->integrityReport();
if ($report->hasIssues()) {
    echo $report->summary();
}
```

**Checks:**
- `noParameterDuplicates()`: No parameter symbol appears twice
- `noLocalDuplicatesPerLine()`: No local symbol at same line appears twice
- `allReceiverValueIdsExist()`: All `receiver_value_id` point to existing values
- `allArgumentValueIdsExist()`: All argument `value_id` point to existing values
- `allSourceCallIdsExist()`: All `source_call_id` point to existing calls
- `allSourceValueIdsExist()`: All `source_value_id` point to existing values
- `everyCallHasResultValue()`: Every call has corresponding result value
- `resultValueTypesMatch()`: Result value types match call return types

## IntegrityReport

Access integrity check results without failing tests.

```php
$report = $this->integrityReport();

// Check totals
$report->duplicateParameterSymbols;  // int
$report->orphanedReceiverIds;        // int
$report->orphanedArgumentIds;        // int
$report->missingResultValues;        // int
$report->typeMismatches;             // int

// Check overall
$report->hasIssues();    // bool
$report->totalIssues();  // int
$report->summary();      // string description

// Get detailed issues
foreach ($report->issues as $issue) {
    echo $issue;
}
```

## CallsData

Direct access to loaded data.

```php
$data = $this->callsData();

// Counts
$data->valueCount();
$data->callCount();

// Version
$data->version();

// All entries
$data->values();
$data->calls();

// Lookup by ID
$data->getValueById('src/File.php:10:8');
$data->getCallById('src/File.php:10:18');

// Check existence
$data->hasValue('src/File.php:10:8');
$data->hasCall('src/File.php:10:18');

// Indexed access
$data->valuesById();
$data->callsById();
```
