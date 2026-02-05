<?php

declare(strict_types=1);

namespace ContractTests\Tests\Reference;

use ContractTests\Attribute\ContractTest;
use ContractTests\CallsContractTestCase;

/**
 * Tests for the "One Value Per Declaration Rule".
 *
 * Per the spec (docs/reference/kloc-scip/calls-and-data-flow.md), each variable
 * (parameter or local) should have exactly ONE value entry at its declaration
 * or assignment site. Multiple usages of the same variable should all reference
 * this single value via receiver_value_id.
 *
 * These tests validate that the indexer does NOT create duplicate value entries
 * for each usage of a variable.
 */
class OneValuePerDeclarationTest extends CallsContractTestCase
{
    // ═══════════════════════════════════════════════════════════════
    // Category 1: Parameter Value Uniqueness
    // ═══════════════════════════════════════════════════════════════

    /**
     * Verifies $order parameter in OrderRepository::save() has exactly one
     * value entry at the declaration site (line 26), not at each usage site.
     *
     * The $order parameter is used 8 times in the method body (lines 28, 31-35, 42, 44).
     * Per the spec, there should be only ONE parameter value entry.
     *
     * Code reference: src/Repository/OrderRepository.php:26
     *   public function save(Order $order): Order
     */
    #[ContractTest(
        name: 'OrderRepository::save() $order - Single Value Entry',
        description: 'Verifies $order parameter has exactly ONE value entry at declaration (line 26), not 8 entries for each usage. This is the key test for the "One Value Per Declaration Rule".',
        category: 'reference',
    )]
    public function testOrderRepositorySaveOrderParameterSingleEntry(): void
    {
        // Query all parameter values with $order symbol in this method
        $orderParams = $this->inMethod('App\Repository\OrderRepository', 'save')
            ->values()
            ->kind('parameter')
            ->symbolContains('($order)')
            ->all();

        // There should be exactly ONE parameter value entry
        $this->assertCount(
            1,
            $orderParams,
            sprintf(
                'Expected exactly 1 parameter value entry for $order, found %d. ' .
                'The indexer should create ONE value at declaration, not one per usage. ' .
                'Found values: %s',
                count($orderParams),
                json_encode(array_column($orderParams, 'id'))
            )
        );

        // The single entry should be at the declaration site (line 26)
        $paramValue = $orderParams[0];
        $line = $paramValue['location']['line'] ?? 0;

        $this->assertSame(
            26,
            $line,
            sprintf(
                'Parameter $order value should be at declaration line 26, found at line %d. ' .
                'Value ID: %s',
                $line,
                $paramValue['id']
            )
        );
    }

    /**
     * Verifies that all property accesses on $order in save() share the same
     * receiver_value_id pointing to the parameter declaration.
     *
     * Code reference: src/Repository/OrderRepository.php:31-35
     *   customerEmail: $order->customerEmail,
     *   productId: $order->productId,
     *   quantity: $order->quantity,
     *   status: $order->status,
     *   createdAt: $order->createdAt,
     */
    #[ContractTest(
        name: 'OrderRepository::save() $order - All Accesses Share Receiver',
        description: 'Verifies all 5 property accesses on $order (lines 31-35) have the same receiver_value_id pointing to the single parameter value at declaration.',
        category: 'reference',
    )]
    public function testOrderRepositorySaveAllAccessesShareReceiver(): void
    {
        $scope = $this->inMethod('App\Repository\OrderRepository', 'save');

        // Get the single parameter value for $order
        $orderParam = $scope->values()
            ->kind('parameter')
            ->symbolContains('($order)')
            ->one();

        $paramValueId = $orderParam['id'];

        // Get all property access calls on Order properties in this method
        $propertyAccessCalls = $scope->calls()
            ->kind('access')
            ->calleeContains('Order#')
            ->hasReceiver()
            ->all();

        // Filter to accesses on lines 31-35 (the $order property accesses)
        // This excludes $newOrder->id on line 37 which correctly has a different receiver
        $orderPropertyAccesses = array_filter($propertyAccessCalls, function ($call) {
            // Match properties: customerEmail, productId, quantity, status, createdAt
            // These are the 5 accesses on $order in the constructor call (lines 31-35)
            $callee = $call['callee'] ?? '';
            $location = $call['location'] ?? [];
            $line = $location['line'] ?? 0;
            // Only include lines 31-35 where $order property accesses occur
            return preg_match('/Order#\$(customerEmail|productId|quantity|status|createdAt)\./', $callee)
                && $line >= 31 && $line <= 35;
        });

        $this->assertNotEmpty(
            $orderPropertyAccesses,
            'Should find property accesses on $order (e.g., $order->customerEmail)'
        );

        // All these accesses should have receiver_value_id pointing to the param
        $mismatchedCalls = [];
        foreach ($orderPropertyAccesses as $call) {
            $receiverId = $call['receiver_value_id'] ?? '';
            if ($receiverId !== $paramValueId) {
                $mismatchedCalls[] = [
                    'call_id' => $call['id'],
                    'callee' => $call['callee'] ?? '?',
                    'receiver_value_id' => $receiverId,
                    'expected' => $paramValueId,
                ];
            }
        }

        $this->assertEmpty(
            $mismatchedCalls,
            sprintf(
                'All property accesses on $order should reference the parameter value %s. ' .
                'Found %d mismatched calls: %s',
                $paramValueId,
                count($mismatchedCalls),
                json_encode($mismatchedCalls, JSON_PRETTY_PRINT)
            )
        );
    }

    /**
     * Verifies $input parameter in OrderService::createOrder() has exactly one
     * value entry, despite being used 4 times.
     *
     * Code reference: src/Service/OrderService.php:27
     *   public function createOrder(CreateOrderInput $input): OrderOutput
     */
    #[ContractTest(
        name: 'OrderService::createOrder() $input - Single Value Entry',
        description: 'Verifies $input parameter has exactly ONE value entry at declaration, not 4 entries for each usage (lines 29, 33-35).',
        category: 'reference',
    )]
    public function testOrderServiceCreateOrderInputParameterSingleEntry(): void
    {
        $inputParams = $this->inMethod('App\Service\OrderService', 'createOrder')
            ->values()
            ->kind('parameter')
            ->symbolContains('($input)')
            ->all();

        $this->assertCount(
            1,
            $inputParams,
            sprintf(
                'Expected exactly 1 parameter value entry for $input, found %d. ' .
                'Values at: %s',
                count($inputParams),
                json_encode(array_map(fn($v) => $v['location']['line'] ?? '?', $inputParams))
            )
        );

        // Should be at declaration line (28)
        $this->assertSame(
            28,
            $inputParams[0]['location']['line'] ?? 0,
            'Parameter $input should be at declaration line 28'
        );
    }

    // ═══════════════════════════════════════════════════════════════
    // Category 2: Local Variable Value Uniqueness
    // ═══════════════════════════════════════════════════════════════

    /**
     * Verifies $savedOrder local in OrderService::createOrder() has exactly one
     * value entry at the assignment site (line 45), not at each usage site.
     *
     * The $savedOrder local is used 8 times after assignment (lines 43, 44, 47-49, 53, 56-62).
     *
     * Code reference: src/Service/OrderService.php:40
     *   $savedOrder = $this->orderRepository->save($order);
     */
    #[ContractTest(
        name: 'OrderService::createOrder() $savedOrder - Single Value Entry',
        description: 'Verifies $savedOrder local has exactly ONE value entry at assignment (line 45), not multiple entries for each of its 8 usages.',
        category: 'reference',
    )]
    public function testOrderServiceCreateOrderSavedOrderLocalSingleEntry(): void
    {
        $savedOrderLocals = $this->inMethod('App\Service\OrderService', 'createOrder')
            ->values()
            ->kind('local')
            ->symbolContains('local$savedOrder')
            ->all();

        $this->assertCount(
            1,
            $savedOrderLocals,
            sprintf(
                'Expected exactly 1 local value entry for $savedOrder, found %d. ' .
                'The indexer should create ONE value at assignment, not one per usage. ' .
                'Values at lines: %s',
                count($savedOrderLocals),
                json_encode(array_map(fn($v) => $v['location']['line'] ?? '?', $savedOrderLocals))
            )
        );

        // Should be at assignment line (45)
        $localValue = $savedOrderLocals[0];
        $this->assertSame(
            45,
            $localValue['location']['line'] ?? 0,
            'Local $savedOrder should be at assignment line 45'
        );

        // Should have source_call_id pointing to the save() call
        $this->assertArrayHasKey(
            'source_call_id',
            $localValue,
            'Local $savedOrder should have source_call_id (assigned from method call)'
        );
    }

    /**
     * Verifies that all property accesses on $savedOrder share the same
     * receiver_value_id pointing to the local variable assignment.
     *
     * Code reference: src/Service/OrderService.php:43-62
     */
    #[ContractTest(
        name: 'OrderService::createOrder() $savedOrder - All Accesses Share Receiver',
        description: 'Verifies all property accesses on $savedOrder have the same receiver_value_id pointing to the single local value at assignment line 45.',
        category: 'reference',
    )]
    public function testOrderServiceCreateOrderSavedOrderAllAccessesShareReceiver(): void
    {
        $scope = $this->inMethod('App\Service\OrderService', 'createOrder');

        // Get the single local value for $savedOrder
        $savedOrderLocal = $scope->values()
            ->kind('local')
            ->symbolContains('local$savedOrder')
            ->one();

        $localValueId = $savedOrderLocal['id'];

        // Get all property access calls in this method that access Order properties
        $propertyAccessCalls = $scope->calls()
            ->kind('access')
            ->calleeContains('Order#')
            ->hasReceiver()
            ->all();

        // Filter to accesses on Order properties (from $savedOrder)
        // These are after line 45 where $savedOrder is assigned
        $savedOrderAccesses = array_filter($propertyAccessCalls, function ($call) {
            $line = $call['location']['line'] ?? 0;
            // $savedOrder is used after line 45
            return $line > 40;
        });

        $this->assertNotEmpty(
            $savedOrderAccesses,
            'Should find property accesses on $savedOrder (after line 45)'
        );

        // All these accesses should have receiver_value_id pointing to the local
        $mismatchedCalls = [];
        foreach ($savedOrderAccesses as $call) {
            $receiverId = $call['receiver_value_id'] ?? '';
            if ($receiverId !== $localValueId) {
                $mismatchedCalls[] = [
                    'call_id' => $call['id'],
                    'line' => $call['location']['line'] ?? '?',
                    'callee' => $call['callee'] ?? '?',
                    'receiver_value_id' => $receiverId,
                    'expected' => $localValueId,
                ];
            }
        }

        $this->assertEmpty(
            $mismatchedCalls,
            sprintf(
                'All property accesses on $savedOrder should reference the local value %s. ' .
                'Found %d mismatched calls: %s',
                $localValueId,
                count($mismatchedCalls),
                json_encode($mismatchedCalls, JSON_PRETTY_PRINT)
            )
        );
    }

    /**
     * Verifies $order local in NotificationService::notifyOrderCreated() has
     * exactly one value entry at assignment (line 20).
     *
     * Code reference: src/Service/NotificationService.php:20
     *   $order = $this->orderRepository->findById($orderId);
     */
    #[ContractTest(
        name: 'NotificationService::notifyOrderCreated() $order - Single Value Entry',
        description: 'Verifies $order local has exactly ONE value entry at assignment (line 20), not entries for each of its 6 usages (lines 22, 27-34).',
        category: 'reference',
    )]
    public function testNotificationServiceOrderLocalSingleEntry(): void
    {
        $orderLocals = $this->inMethod('App\Service\NotificationService', 'notifyOrderCreated')
            ->values()
            ->kind('local')
            ->symbolContains('local$order')
            ->all();

        $this->assertCount(
            1,
            $orderLocals,
            sprintf(
                'Expected exactly 1 local value entry for $order, found %d. ' .
                'Values at lines: %s',
                count($orderLocals),
                json_encode(array_map(fn($v) => $v['location']['line'] ?? '?', $orderLocals))
            )
        );

        // Should be at assignment line (20)
        $this->assertSame(
            20,
            $orderLocals[0]['location']['line'] ?? 0,
            'Local $order should be at assignment line 20'
        );
    }

    // ═══════════════════════════════════════════════════════════════
    // Category 3: Chain Integrity (Multiple Accesses Same Receiver)
    // ═══════════════════════════════════════════════════════════════

    /**
     * Verifies that multiple property accesses on $order within the Order
     * constructor call (lines 31-35) all share the same receiver_value_id.
     *
     * This tests the chain integrity: when the same variable is used as
     * receiver for multiple property accesses, all should reference the
     * single declaration value.
     *
     * Code reference: src/Repository/OrderRepository.php:31-35
     */
    #[ContractTest(
        name: 'OrderRepository::save() - Property Access Chain on $order',
        description: 'Verifies the 5 consecutive property accesses on $order (customerEmail, productId, quantity, status, createdAt) all share the same receiver_value_id.',
        category: 'reference',
    )]
    public function testOrderRepositoryPropertyAccessChainSharesReceiver(): void
    {
        $scope = $this->inMethod('App\Repository\OrderRepository', 'save');

        // Get property access calls on lines 31-35
        $chainedAccesses = [];
        for ($line = 31; $line <= 35; $line++) {
            $callsAtLine = $scope->calls()
                ->kind('access')
                ->atLine($line)
                ->hasReceiver()
                ->all();

            foreach ($callsAtLine as $call) {
                $chainedAccesses[] = $call;
            }
        }

        $this->assertGreaterThanOrEqual(
            5,
            count($chainedAccesses),
            'Should find at least 5 property accesses on lines 31-35'
        );

        // Extract unique receiver_value_ids
        $receiverIds = array_unique(
            array_filter(
                array_map(fn($c) => $c['receiver_value_id'] ?? null, $chainedAccesses)
            )
        );

        $this->assertCount(
            1,
            $receiverIds,
            sprintf(
                'All 5 property accesses on $order should share the same receiver_value_id. ' .
                'Found %d different receiver IDs: %s',
                count($receiverIds),
                implode(', ', $receiverIds)
            )
        );
    }

    /**
     * Verifies that the shared receiver_value_id for $order accesses
     * actually points to a parameter value entry (not a result or other kind).
     *
     * This validates that the receiver chain correctly terminates at the
     * parameter declaration.
     */
    #[ContractTest(
        name: 'OrderRepository::save() - Receiver Points to Parameter',
        description: 'Verifies the shared receiver_value_id for $order property accesses points to a parameter value (kind=parameter), not a result or duplicate entry.',
        category: 'reference',
    )]
    public function testOrderRepositoryReceiverPointsToParameter(): void
    {
        $scope = $this->inMethod('App\Repository\OrderRepository', 'save');

        // Get the parameter value
        $paramValue = $scope->values()
            ->kind('parameter')
            ->symbolContains('($order)')
            ->first();

        $this->assertNotNull($paramValue, 'Should find $order parameter value');

        // Get a property access call
        $accessCall = $scope->calls()
            ->kind('access')
            ->calleeContains('Order#$customerEmail')
            ->first();

        $this->assertNotNull($accessCall, 'Should find $order->customerEmail access');

        // The receiver should point to the parameter
        $this->assertSame(
            $paramValue['id'],
            $accessCall['receiver_value_id'] ?? '',
            sprintf(
                'Property access receiver_value_id should point to parameter value. ' .
                'Expected: %s, Got: %s',
                $paramValue['id'],
                $accessCall['receiver_value_id'] ?? 'null'
            )
        );

        // Verify the parameter is kind=parameter (not accidentally a result or local)
        $this->assertSame(
            'parameter',
            $paramValue['kind'],
            'The receiver value should be kind=parameter'
        );
    }

    // ═══════════════════════════════════════════════════════════════
    // Category 4: Data Integrity - No Duplicate Symbols
    // ═══════════════════════════════════════════════════════════════

    /**
     * Verifies no parameter in OrderRepository has duplicate symbol entries.
     *
     * If the indexer creates value entries at usage sites instead of just
     * declaration, we'd see multiple entries with the same symbol.
     */
    #[ContractTest(
        name: 'OrderRepository - No Duplicate Parameter Symbols',
        description: 'Verifies no parameter in OrderRepository has duplicate symbol entries (which would indicate values created at usage sites).',
        category: 'reference',
    )]
    public function testOrderRepositoryNoDuplicateParameterSymbols(): void
    {
        $params = $this->values()
            ->kind('parameter')
            ->inFile('OrderRepository.php')
            ->all();

        $symbols = array_column($params, 'symbol');
        $symbolCounts = array_count_values($symbols);
        $duplicates = array_filter($symbolCounts, fn($count) => $count > 1);

        $this->assertEmpty(
            $duplicates,
            sprintf(
                'Found duplicate parameter symbols in OrderRepository: %s',
                json_encode($duplicates)
            )
        );
    }

    /**
     * Verifies no local variable in OrderService::createOrder() has duplicate
     * symbol entries (same @line suffix).
     */
    #[ContractTest(
        name: 'OrderService::createOrder() - No Duplicate Local Symbols',
        description: 'Verifies no local variable has duplicate symbol entries with the same @line suffix.',
        category: 'reference',
    )]
    public function testOrderServiceCreateOrderNoDuplicateLocalSymbols(): void
    {
        $locals = $this->inMethod('App\Service\OrderService', 'createOrder')
            ->values()
            ->kind('local')
            ->all();

        $symbols = array_column($locals, 'symbol');
        $symbolCounts = array_count_values($symbols);
        $duplicates = array_filter($symbolCounts, fn($count) => $count > 1);

        $this->assertEmpty(
            $duplicates,
            sprintf(
                'Found duplicate local symbols in createOrder(): %s',
                json_encode($duplicates)
            )
        );
    }
}
