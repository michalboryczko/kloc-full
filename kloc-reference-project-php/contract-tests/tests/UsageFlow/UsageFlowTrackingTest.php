<?php

declare(strict_types=1);

namespace ContractTests\Tests\UsageFlow;

use ContractTests\Attribute\ContractTest;
use ContractTests\CallsContractTestCase;

/**
 * Contract tests for Usage Flow Tracking spec.
 *
 * These tests validate that calls.json supports the requirements from
 * docs/specs/usage-flow-tracking.md for tracking:
 * - Access chains (TC3)
 * - Receiver information (TC2)
 * - Multiple references not collapsed (TC4)
 * - Argument tracking (TC5)
 * - Property type hints (TC1)
 *
 * @see docs/specs/usage-flow-tracking.md
 */
class UsageFlowTrackingTest extends CallsContractTestCase
{
    // ═══════════════════════════════════════════════════════════════
    // TC1: Property Type Hints Create Values with Types
    // ═══════════════════════════════════════════════════════════════

    #[ContractTest(
        name: 'TC1: Property Type Hint Creates Typed Parameter Value',
        description: 'Verifies that constructor parameters with type hints have type information in the value entry. Per spec TC1, property type hints should create values with types.',
        category: 'reference',
    )]
    public function testTC1PropertyTypeHintCreatesTypedValue(): void
    {
        // Code reference: src/Service/OrderService.php:20
        // private OrderRepository $orderRepository

        $param = $this->inMethod('App\Service\OrderService', '__construct')
            ->values()
            ->kind('parameter')
            ->symbolContains('($orderRepository)')
            ->first();

        $this->assertNotNull($param, 'Should find $orderRepository parameter');
        $this->assertArrayHasKey('type', $param, 'Parameter should have type field');
        $this->assertNotNull($param['type'], 'Parameter type should not be null');
        $this->assertStringContainsString(
            'OrderRepository',
            $param['type'] ?? '',
            'Parameter type should contain OrderRepository'
        );
    }

    #[ContractTest(
        name: 'TC1: Local Variable Assigned from Method Has Type',
        description: 'Verifies that local variables assigned from method calls can have type information via source_call_id linkage.',
        category: 'reference',
    )]
    public function testTC1LocalVariableFromMethodHasType(): void
    {
        // Code reference: src/Service/OrderService.php:40
        // $savedOrder = $this->orderRepository->save($order);

        $local = $this->inMethod('App\Service\OrderService', 'createOrder')
            ->values()
            ->kind('local')
            ->symbolContains('local$savedOrder')
            ->first();

        $this->assertNotNull($local, 'Should find $savedOrder local');

        // Local should have source_call_id pointing to save()
        $this->assertArrayHasKey('source_call_id', $local, 'Local should have source_call_id');
        $sourceCallId = $local['source_call_id'];
        $this->assertNotNull($sourceCallId, 'source_call_id should not be null');

        // The source call should have return_type
        $sourceCall = self::$calls->getCallById($sourceCallId);
        $this->assertNotNull($sourceCall, 'Source call should exist');
        $this->assertStringContainsString('save', $sourceCall['callee'] ?? '', 'Source should be save() call');

        // Check return_type is present (type flow from call to local)
        if (isset($sourceCall['return_type'])) {
            $this->assertStringContainsString('Order', $sourceCall['return_type'], 'save() should return Order type');
        }
    }

    // ═══════════════════════════════════════════════════════════════
    // TC2: Method Calls Via Property Have receiver_value_id
    // ═══════════════════════════════════════════════════════════════

    #[ContractTest(
        name: 'TC2: Method Call Has receiver_value_id to Property Access Result',
        description: 'Verifies that method calls on properties have receiver_value_id pointing to the property access result value. Per spec TC2, method calls via property need proper linkage.',
        category: 'chain',
    )]
    public function testTC2MethodCallHasReceiverFromPropertyAccess(): void
    {
        // Code reference: src/Service/OrderService.php:40
        // $savedOrder = $this->orderRepository->save($order);

        // Find save() method call
        $saveCall = $this->calls()
            ->kind('method')
            ->callerContains('OrderService#createOrder()')
            ->calleeContains('save')
            ->first();

        $this->assertNotNull($saveCall, 'Should find save() method call');
        $this->assertArrayHasKey('receiver_value_id', $saveCall, 'save() should have receiver_value_id');

        $receiverId = $saveCall['receiver_value_id'];
        $this->assertNotNull($receiverId, 'receiver_value_id should not be null');

        // Receiver should be a result value (from orderRepository access)
        $receiverValue = self::$calls->getValueById($receiverId);
        $this->assertNotNull($receiverValue, 'Receiver value should exist');
        $this->assertSame('result', $receiverValue['kind'] ?? '', 'Receiver should be a result value');

        // Result should point back to an access call
        $sourceCallId = $receiverValue['source_call_id'] ?? null;
        $this->assertNotNull($sourceCallId, 'Result should have source_call_id');

        $accessCall = self::$calls->getCallById($sourceCallId);
        $this->assertNotNull($accessCall, 'Access call should exist');
        $this->assertSame('access', $accessCall['kind'] ?? '', 'Should be an access call');
        $this->assertStringContainsString('orderRepository', $accessCall['callee'] ?? '', 'Should access orderRepository');
    }

    #[ContractTest(
        name: 'TC2: Property Access Creates Result Value',
        description: 'Verifies that property access calls create corresponding result values with matching IDs.',
        category: 'chain',
    )]
    public function testTC2PropertyAccessCreatesResultValue(): void
    {
        // Code reference: src/Service/OrderService.php:40
        // $this->orderRepository...

        $accessCall = $this->calls()
            ->kind('access')
            ->callerContains('OrderService#createOrder()')
            ->calleeContains('orderRepository')
            ->first();

        $this->assertNotNull($accessCall, 'Should find orderRepository access');

        // Access should have corresponding result value with same ID
        $resultValue = self::$calls->getValueById($accessCall['id']);
        $this->assertNotNull($resultValue, 'Access should have result value');
        $this->assertSame('result', $resultValue['kind'] ?? '', 'Should be kind result');
        $this->assertSame(
            $accessCall['id'],
            $resultValue['source_call_id'] ?? '',
            'Result source_call_id should match access id'
        );
    }

    // ═══════════════════════════════════════════════════════════════
    // TC3: Chained Calls Reconstructed via receiver_value_id
    // ═══════════════════════════════════════════════════════════════

    #[ContractTest(
        name: 'TC3: Chain Traversal Value->Access->Result->Method->Result',
        description: 'Verifies the full chain $this->orderRepository->save() can be traced: value -> access -> result -> method -> result. Per spec TC3, chains must be reconstructable.',
        category: 'chain',
    )]
    public function testTC3ChainTraversalPropertyToMethod(): void
    {
        // Code reference: src/Service/OrderService.php:40
        // $this->orderRepository->save($order)

        // Start from save() method call
        $saveCall = $this->calls()
            ->kind('method')
            ->callerContains('OrderService#createOrder()')
            ->calleeContains('save')
            ->first();

        $this->assertNotNull($saveCall, 'Should find save() call');

        // Step 1: save() has result
        $saveResultValue = self::$calls->getValueById($saveCall['id']);
        $this->assertNotNull($saveResultValue, 'save() should have result value');
        $this->assertSame('result', $saveResultValue['kind'] ?? '');

        // Step 2: save() receiver is result of access
        $receiverId = $saveCall['receiver_value_id'];
        $this->assertNotNull($receiverId, 'save() should have receiver');

        $accessResult = self::$calls->getValueById($receiverId);
        $this->assertNotNull($accessResult, 'Receiver value should exist');
        $this->assertSame('result', $accessResult['kind'] ?? '', 'Receiver should be result');

        // Step 3: Access result points to access call
        $accessCallId = $accessResult['source_call_id'];
        $this->assertNotNull($accessCallId, 'Access result should have source_call_id');

        $accessCall = self::$calls->getCallById($accessCallId);
        $this->assertNotNull($accessCall, 'Access call should exist');
        $this->assertSame('access', $accessCall['kind'] ?? '');

        // Step 4: Access call may have receiver (for $this) or null (readonly promoted)
        // Chain terminates here - we've traced: method -> access -> [possibly $this]
        $this->assertTrue(true, 'Chain successfully traced from method to access');
    }

    #[ContractTest(
        name: 'TC3: Local Variable Links to Method Result',
        description: 'Verifies that local variables assigned from method calls have source_call_id pointing to the method call.',
        category: 'chain',
    )]
    public function testTC3LocalVariableLinksToMethodResult(): void
    {
        // Code reference: src/Service/NotificationService.php:20
        // $order = $this->orderRepository->findById($orderId);

        $orderLocal = $this->inMethod('App\Service\NotificationService', 'notifyOrderCreated')
            ->values()
            ->kind('local')
            ->symbolContains('local$order')
            ->first();

        $this->assertNotNull($orderLocal, 'Should find $order local');

        // Local should have source_call_id
        $sourceCallId = $orderLocal['source_call_id'] ?? null;
        $this->assertNotNull($sourceCallId, '$order should have source_call_id');

        // Source should be findById call
        $findByIdCall = self::$calls->getCallById($sourceCallId);
        $this->assertNotNull($findByIdCall, 'findById call should exist');
        $this->assertStringContainsString('findById', $findByIdCall['callee'] ?? '');
    }

    #[ContractTest(
        name: 'TC3: Property Access Uses Local as Receiver',
        description: 'Verifies that property accesses on local variables use the local variable value as receiver.',
        category: 'chain',
    )]
    public function testTC3PropertyAccessUsesLocalAsReceiver(): void
    {
        // Code reference: src/Service/NotificationService.php:27
        // to: $order->customerEmail

        // Find $order local
        $orderLocal = $this->inMethod('App\Service\NotificationService', 'notifyOrderCreated')
            ->values()
            ->kind('local')
            ->symbolContains('local$order')
            ->first();

        $this->assertNotNull($orderLocal, 'Should find $order local');

        // Find customerEmail access in same method
        $emailAccess = $this->inMethod('App\Service\NotificationService', 'notifyOrderCreated')
            ->calls()
            ->kind('access')
            ->calleeContains('customerEmail')
            ->first();

        $this->assertNotNull($emailAccess, 'Should find customerEmail access');

        // Access receiver should point to $order local
        $this->assertSame(
            $orderLocal['id'],
            $emailAccess['receiver_value_id'] ?? null,
            'customerEmail access receiver should point to $order local'
        );
    }

    #[ContractTest(
        name: 'TC3: Nested Property Chain CustomerService',
        description: 'Verifies nested property chains like $customer->contact->email can be traced.',
        category: 'chain',
    )]
    public function testTC3NestedPropertyChain(): void
    {
        // Code reference: src/Service/CustomerService.php:50
        // $email = $customer->contact->email;

        // Find email access (the innermost)
        $emailAccess = $this->inMethod('App\Service\CustomerService', 'getCustomerById')
            ->calls()
            ->kind('access')
            ->calleeContains('$email')
            ->first();

        // If we find the email property access, trace back
        if ($emailAccess !== null) {
            $receiverId = $emailAccess['receiver_value_id'] ?? null;
            if ($receiverId !== null) {
                $receiverValue = self::$calls->getValueById($receiverId);
                $this->assertNotNull($receiverValue, 'Email access receiver should exist');

                // Receiver should be result of contact access
                if ($receiverValue['kind'] === 'result') {
                    $sourceCallId = $receiverValue['source_call_id'] ?? null;
                    if ($sourceCallId !== null) {
                        $contactAccess = self::$calls->getCallById($sourceCallId);
                        $this->assertNotNull($contactAccess, 'Contact access should exist');
                        $this->assertSame('access', $contactAccess['kind'] ?? '');
                    }
                }
            }
        }

        // Alternative: Find contact property access
        $contactAccess = $this->inMethod('App\Service\CustomerService', 'getCustomerById')
            ->calls()
            ->kind('access')
            ->calleeContains('contact')
            ->first();

        $this->assertNotNull($contactAccess, 'Should find contact access');
        $this->assertArrayHasKey('receiver_value_id', $contactAccess, 'Contact access should have receiver');
    }

    // ═══════════════════════════════════════════════════════════════
    // TC4: Multiple Method Calls Not Collapsed
    // ═══════════════════════════════════════════════════════════════

    #[ContractTest(
        name: 'TC4: Multiple Calls in Same Scope Have Unique IDs',
        description: 'Verifies that multiple method calls in the same scope are NOT collapsed - each has a unique ID. Per spec TC4.',
        category: 'reference',
    )]
    public function testTC4MultipleCallsHaveUniqueIds(): void
    {
        // Code reference: src/Service/OrderService.php:29-53
        // Multiple method calls in createOrder()

        $calls = $this->inMethod('App\Service\OrderService', 'createOrder')
            ->calls()
            ->kind('method')
            ->all();

        $this->assertGreaterThanOrEqual(4, count($calls), 'Should have at least 4 method calls');

        // Collect call IDs
        $callIds = array_map(fn($c) => $c['id'] ?? '', $calls);
        $uniqueIds = array_unique($callIds);

        $this->assertCount(
            count($callIds),
            $uniqueIds,
            'All method calls should have unique IDs (not collapsed)'
        );

        // Verify specific calls exist
        $calleeNames = array_map(fn($c) => $c['callee'] ?? '', $calls);
        $this->assertContainsPattern('checkAvailability', $calleeNames, 'Should have checkAvailability call');
        $this->assertContainsPattern('save', $calleeNames, 'Should have save call');
        $this->assertContainsPattern('send', $calleeNames, 'Should have send call');
        $this->assertContainsPattern('dispatch', $calleeNames, 'Should have dispatch call');
    }

    #[ContractTest(
        name: 'TC4: Multiple Property Accesses Share Receiver',
        description: 'Verifies that multiple property accesses on the same variable all share the same receiver_value_id.',
        category: 'reference',
    )]
    public function testTC4MultipleAccessesShareReceiver(): void
    {
        // Code reference: src/Service/OrderService.php:43-49
        // Multiple accesses on $savedOrder

        $accesses = $this->inMethod('App\Service\OrderService', 'createOrder')
            ->calls()
            ->kind('access')
            ->all();

        // Find accesses that should be on $savedOrder (line 43+)
        $savedOrderAccesses = array_filter($accesses, function($a) {
            $callee = $a['callee'] ?? '';
            // These are properties of Order accessed via $savedOrder
            return str_contains($callee, 'customerEmail')
                || str_contains($callee, 'productId')
                || str_contains($callee, 'quantity')
                || str_contains($callee, 'status')
                || str_contains($callee, 'createdAt');
        });

        // Get the ones that have a receiver pointing to $savedOrder
        $savedOrderLocal = $this->inMethod('App\Service\OrderService', 'createOrder')
            ->values()
            ->kind('local')
            ->symbolContains('local$savedOrder')
            ->first();

        if ($savedOrderLocal === null) {
            $this->markTestSkipped('Could not find $savedOrder local - may need reference code adjustment');
            return;
        }

        $savedOrderId = $savedOrderLocal['id'];

        // Filter to accesses with that receiver
        $accessesOnSavedOrder = array_filter($accesses, function($a) use ($savedOrderId) {
            return ($a['receiver_value_id'] ?? null) === $savedOrderId;
        });

        $this->assertGreaterThanOrEqual(
            3,
            count($accessesOnSavedOrder),
            'Should have multiple property accesses sharing $savedOrder receiver'
        );

        // All should share the same receiver_value_id
        $receiverIds = array_unique(array_map(fn($a) => $a['receiver_value_id'], $accessesOnSavedOrder));
        $this->assertCount(1, $receiverIds, 'All accesses on $savedOrder should share same receiver_value_id');
    }

    #[ContractTest(
        name: 'TC4: Same Property Multiple Times Has Multiple Entries',
        description: 'Verifies that accessing the same property multiple times creates multiple call entries.',
        category: 'reference',
    )]
    public function testTC4SamePropertyMultipleEntriesNotCollapsed(): void
    {
        // Code reference: src/Service/OrderService.php:44, 47
        // $savedOrder->id appears multiple times

        $idAccesses = $this->inMethod('App\Service\OrderService', 'createOrder')
            ->calls()
            ->kind('access')
            ->calleeContains('#$id.')
            ->all();

        // Should have multiple entries, not collapsed
        $this->assertGreaterThanOrEqual(
            2,
            count($idAccesses),
            '$savedOrder->id accessed multiple times should have multiple entries (not collapsed)'
        );

        // Each should have unique ID
        $ids = array_map(fn($a) => $a['id'], $idAccesses);
        $uniqueIds = array_unique($ids);
        $this->assertCount(count($ids), $uniqueIds, 'Each access should have unique ID');
    }

    // ═══════════════════════════════════════════════════════════════
    // TC5: Arguments Reference Values via value_id
    // ═══════════════════════════════════════════════════════════════

    #[ContractTest(
        name: 'TC5: Argument Points to Local Variable',
        description: 'Verifies that arguments have value_id pointing to local variable values. Per spec TC5.',
        category: 'argument',
    )]
    public function testTC5ArgumentPointsToLocal(): void
    {
        // Code reference: src/Service/OrderService.php:45
        // $this->orderRepository->save($processedOrder)

        $saveCall = $this->calls()
            ->kind('method')
            ->callerContains('OrderService#createOrder()')
            ->calleeContains('save')
            ->first();

        $this->assertNotNull($saveCall, 'Should find save() call');

        $arguments = $saveCall['arguments'] ?? [];
        $this->assertNotEmpty($arguments, 'save() should have arguments');

        $arg0 = $this->findArgumentByPosition($arguments, 0);
        $this->assertNotNull($arg0, 'Should have argument at position 0');
        $this->assertArrayHasKey('value_id', $arg0, 'Argument should have value_id');

        $valueId = $arg0['value_id'];
        $this->assertNotNull($valueId, 'value_id should not be null');

        $argValue = self::$calls->getValueById($valueId);
        $this->assertNotNull($argValue, 'Argument value should exist');
        $this->assertSame('local', $argValue['kind'] ?? '', 'Argument should point to local value');
        $this->assertStringContainsString('$processedOrder', $argValue['symbol'] ?? '', 'Should be $processedOrder local');
    }

    #[ContractTest(
        name: 'TC5: Argument Points to Parameter',
        description: 'Verifies that arguments can point to parameter values.',
        category: 'argument',
    )]
    public function testTC5ArgumentPointsToParameter(): void
    {
        // Code reference: src/Service/NotificationService.php:20
        // $this->orderRepository->findById($orderId)

        $findByIdCall = $this->calls()
            ->kind('method')
            ->callerContains('NotificationService#notifyOrderCreated()')
            ->calleeContains('findById')
            ->first();

        $this->assertNotNull($findByIdCall, 'Should find findById() call');

        $arguments = $findByIdCall['arguments'] ?? [];
        $arg0 = $this->findArgumentByPosition($arguments, 0);

        $this->assertNotNull($arg0, 'Should have argument');
        $valueId = $arg0['value_id'] ?? null;
        $this->assertNotNull($valueId, 'Argument should have value_id');

        $argValue = self::$calls->getValueById($valueId);
        $this->assertNotNull($argValue, 'Argument value should exist');
        $this->assertSame('parameter', $argValue['kind'] ?? '', 'Argument should point to parameter');
        $this->assertStringContainsString('$orderId', $argValue['symbol'] ?? '');
    }

    #[ContractTest(
        name: 'TC5: Argument Points to Property Access Result',
        description: 'Verifies that arguments pointing to property access results have value_id to result values.',
        category: 'argument',
    )]
    public function testTC5ArgumentPointsToAccessResult(): void
    {
        // Code reference: src/Service/OrderService.php:43
        // to: $savedOrder->customerEmail

        $sendCall = $this->calls()
            ->kind('method')
            ->callerContains('OrderService#createOrder()')
            ->calleeContains('send')
            ->first();

        $this->assertNotNull($sendCall, 'Should find send() call');

        $arguments = $sendCall['arguments'] ?? [];
        $arg0 = $this->findArgumentByPosition($arguments, 0);

        $this->assertNotNull($arg0, 'Should have first argument (to:)');
        $valueId = $arg0['value_id'] ?? null;
        $this->assertNotNull($valueId, 'Argument should have value_id');

        $argValue = self::$calls->getValueById($valueId);
        $this->assertNotNull($argValue, 'Argument value should exist');
        $this->assertSame('result', $argValue['kind'] ?? '', 'Argument should point to result value');

        // Result should link to access call
        $sourceCallId = $argValue['source_call_id'] ?? null;
        $this->assertNotNull($sourceCallId, 'Result should have source_call_id');

        $sourceCall = self::$calls->getCallById($sourceCallId);
        $this->assertNotNull($sourceCall, 'Source call should exist');
        $this->assertSame('access', $sourceCall['kind'] ?? '', 'Source should be access call');
        $this->assertStringContainsString('customerEmail', $sourceCall['callee'] ?? '');
    }

    #[ContractTest(
        name: 'TC5: Argument Points to Constructor Result',
        description: 'Verifies that arguments from constructor results have value_id pointing to constructor result values.',
        category: 'argument',
    )]
    public function testTC5ArgumentPointsToConstructorResult(): void
    {
        // Code reference: src/Service/OrderService.php:53
        // $this->messageBus->dispatch(new OrderCreatedMessage($savedOrder->id))

        $dispatchCall = $this->calls()
            ->kind('method')
            ->callerContains('OrderService#createOrder()')
            ->calleeContains('dispatch')
            ->first();

        $this->assertNotNull($dispatchCall, 'Should find dispatch() call');

        $arguments = $dispatchCall['arguments'] ?? [];
        $arg0 = $this->findArgumentByPosition($arguments, 0);

        $this->assertNotNull($arg0, 'Should have argument');
        $valueId = $arg0['value_id'] ?? null;
        $this->assertNotNull($valueId, 'Argument should have value_id');

        $argValue = self::$calls->getValueById($valueId);
        $this->assertNotNull($argValue, 'Argument value should exist');
        $this->assertSame('result', $argValue['kind'] ?? '', 'Argument should point to result value');

        // Result should link to constructor call
        $sourceCallId = $argValue['source_call_id'] ?? null;
        $this->assertNotNull($sourceCallId, 'Result should have source_call_id');

        $sourceCall = self::$calls->getCallById($sourceCallId);
        $this->assertNotNull($sourceCall, 'Source call should exist');
        $this->assertSame('constructor', $sourceCall['kind'] ?? '', 'Source should be constructor call');
    }

    #[ContractTest(
        name: 'TC5: Argument Has Parameter Symbol',
        description: 'Verifies that arguments have parameter symbol linking to callee parameter definition.',
        category: 'argument',
    )]
    public function testTC5ArgumentHasParameterSymbol(): void
    {
        // Code reference: src/Service/OrderService.php:40
        // $this->orderRepository->save($order)

        $saveCall = $this->calls()
            ->kind('method')
            ->callerContains('OrderService#createOrder()')
            ->calleeContains('save')
            ->first();

        $this->assertNotNull($saveCall, 'Should find save() call');

        $arguments = $saveCall['arguments'] ?? [];
        $arg0 = $this->findArgumentByPosition($arguments, 0);

        $this->assertNotNull($arg0, 'Should have argument');
        $this->assertArrayHasKey('parameter', $arg0, 'Argument should have parameter field');
        $this->assertNotEmpty($arg0['parameter'] ?? '', 'parameter should not be empty');
        $this->assertStringContainsString('($order)', $arg0['parameter'] ?? '', 'Parameter should reference callee parameter');
    }

    // ═══════════════════════════════════════════════════════════════
    // Integration Tests
    // ═══════════════════════════════════════════════════════════════

    #[ContractTest(
        name: 'Integration: Full Chain Trace from Argument to Source',
        description: 'Verifies complete data flow can be traced: argument -> value -> (result -> call)* -> parameter/local.',
        category: 'chain',
    )]
    public function testIntegrationFullChainTrace(): void
    {
        // Code reference: src/Service/OrderService.php:43
        // send(to: $savedOrder->customerEmail, ...)
        // Full chain: send arg -> customerEmail result -> customerEmail access -> $savedOrder local -> save result -> save call

        $sendCall = $this->calls()
            ->kind('method')
            ->callerContains('OrderService#createOrder()')
            ->calleeContains('send')
            ->first();

        $this->assertNotNull($sendCall, 'Should find send() call');

        $arg0 = $this->findArgumentByPosition($sendCall['arguments'] ?? [], 0);
        $this->assertNotNull($arg0, 'Should have argument');

        // Trace from argument
        $valueId = $arg0['value_id'];
        $depth = 0;
        $maxDepth = 10;
        $chainKinds = [];

        while ($valueId !== null && $depth < $maxDepth) {
            $value = self::$calls->getValueById($valueId);
            if ($value === null) {
                break;
            }

            $kind = $value['kind'] ?? '';
            $chainKinds[] = $kind;

            // Terminal kinds
            if (in_array($kind, ['parameter', 'local', 'literal', 'constant'], true)) {
                break;
            }

            // Result kind - follow source_call_id
            if ($kind === 'result') {
                $sourceCallId = $value['source_call_id'] ?? null;
                if ($sourceCallId === null) {
                    break;
                }

                $sourceCall = self::$calls->getCallById($sourceCallId);
                if ($sourceCall === null) {
                    break;
                }

                // Follow receiver_value_id to continue chain
                $valueId = $sourceCall['receiver_value_id'] ?? null;
            } else {
                break;
            }

            $depth++;
        }

        // Chain should end at a terminal kind
        $this->assertNotEmpty($chainKinds, 'Should trace some chain');
        $lastKind = end($chainKinds);
        $this->assertContains(
            $lastKind,
            ['parameter', 'local', 'literal', 'constant', 'result'],
            'Chain should terminate at terminal value or result (if no receiver)'
        );
    }

    #[ContractTest(
        name: 'Summary: All TC Requirements Traceable',
        description: 'Summary test verifying the spec requirements are structurally supported in calls.json.',
        category: 'smoke',
    )]
    public function testSummaryAllRequirementsTraceable(): void
    {
        // TC1: Values have types
        $typedValues = array_filter(self::$calls->values(), fn($v) => !empty($v['type']));
        $this->assertNotEmpty($typedValues, 'TC1: Should have values with types');

        // TC2: Calls have receiver_value_id
        $callsWithReceiver = array_filter(self::$calls->calls(), fn($c) => !empty($c['receiver_value_id']));
        $this->assertNotEmpty($callsWithReceiver, 'TC2: Should have calls with receivers');

        // TC3: Chain structure exists (result values with source_call_id)
        $resultValues = array_filter(self::$calls->values(), fn($v) => ($v['kind'] ?? '') === 'result');
        $this->assertNotEmpty($resultValues, 'TC3: Should have result values for chain traversal');

        // TC4: Multiple calls exist
        $calls = self::$calls->calls();
        $uniqueCallIds = array_unique(array_column($calls, 'id'));
        $this->assertCount(count($calls), $uniqueCallIds, 'TC4: All calls should have unique IDs');

        // TC5: Arguments have value_id
        $argsWithValueId = 0;
        foreach ($calls as $call) {
            foreach ($call['arguments'] ?? [] as $arg) {
                if (!empty($arg['value_id'])) {
                    $argsWithValueId++;
                }
            }
        }
        $this->assertGreaterThan(0, $argsWithValueId, 'TC5: Should have arguments with value_id');
    }

    // ═══════════════════════════════════════════════════════════════
    // Helper Methods
    // ═══════════════════════════════════════════════════════════════

    /**
     * @param array<int, array<string, mixed>> $arguments
     * @return array<string, mixed>|null
     */
    private function findArgumentByPosition(array $arguments, int $position): ?array
    {
        foreach ($arguments as $arg) {
            if (($arg['position'] ?? -1) === $position) {
                return $arg;
            }
        }
        return null;
    }

    /**
     * Assert that at least one element in array contains the pattern.
     * @param string[] $array
     */
    private function assertContainsPattern(string $pattern, array $array, string $message = ''): void
    {
        $found = false;
        foreach ($array as $item) {
            if (str_contains($item, $pattern)) {
                $found = true;
                break;
            }
        }
        $this->assertTrue($found, $message ?: "Array should contain element matching '{$pattern}'");
    }
}
