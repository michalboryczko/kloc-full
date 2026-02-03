<?php

declare(strict_types=1);

namespace ContractTests\Tests\CallKind;

use ContractTests\Attribute\ContractTest;
use ContractTests\CallsContractTestCase;

/**
 * Tests for nullsafe operator handling in calls.json.
 *
 * Per the finish-mvp spec:
 * - Nullsafe property access ($obj?->prop) uses kind="access" (not access_nullsafe)
 * - Nullsafe method call ($obj?->method()) uses kind="method" (not method_nullsafe)
 * - Return type should be union T|null to capture nullsafe semantics
 *
 * Reference code: src/Service/OrderDisplayService.php
 * - Nullsafe property accesses: lines 45, 64, 82, 99, 139-141
 * - Nullsafe method calls: lines 162, 175
 */
class NullsafeKindTest extends CallsContractTestCase
{
    // ═══════════════════════════════════════════════════════════════
    // Nullsafe Property Access Tests
    // ═══════════════════════════════════════════════════════════════

    #[ContractTest(
        name: 'Nullsafe Property Access Uses kind=access',
        description: 'Verifies nullsafe property access ($order?->status) uses kind="access" not "access_nullsafe". Per finish-mvp spec, nullsafe semantics are captured via union return type.',
        category: 'callkind',
            )]
    public function testNullsafePropertyAccessUsesAccessKind(): void
    {
        // Code reference: src/Service/OrderDisplayService.php:64
        // $status = $order?->status;
        // Note: With nullsafe kinds removed, this should be kind=access, not access_nullsafe
        // First, check what kinds exist for status property access
        $accessCalls = $this->calls()
            ->kind('access')
            ->inFile('OrderDisplayService.php')
            ->calleeContains('Order#$status')
            ->all();

        $this->assertNotEmpty(
            $accessCalls,
            'Should find property access calls for $order->status in OrderDisplayService'
        );

        // Verify it's kind=access, not access_nullsafe
        foreach ($accessCalls as $call) {
            $this->assertSame('access', $call['kind'], 'Nullsafe property access should have kind=access');
            $this->assertSame('access', $call['kind_type'], 'Property access should have kind_type=access');
        }
    }

    #[ContractTest(
        name: 'Nullsafe Property Access Has Union Return Type',
        description: 'Verifies nullsafe property access ($order?->status) has union return_type containing null. For string property, should be "null|string" union.',
        category: 'callkind',
            )]
    public function testNullsafePropertyAccessHasUnionReturnType(): void
    {
        // Code reference: src/Service/OrderDisplayService.php:64
        // $status = $order?->status; // status is type string
        // Note: With nullsafe kinds removed, these should be kind=access with union return type
        $accessCalls = $this->calls()
            ->kind('access')
            ->inFile('OrderDisplayService.php')
            ->calleeContains('Order#$status')
            ->all();

        $this->assertNotEmpty($accessCalls, 'Should find status property access (kind=access, not access_nullsafe)');

        $foundUnionType = false;
        foreach ($accessCalls as $call) {
            $returnType = $call['return_type'] ?? null;
            if ($returnType !== null && str_contains($returnType, 'null')) {
                $foundUnionType = true;
                // Union type should contain both null and string
                $this->assertStringContainsString(
                    'union',
                    $returnType,
                    'Nullsafe return type should be a union type'
                );
            }
        }

        $this->assertTrue(
            $foundUnionType,
            'At least one nullsafe property access should have union return_type with null'
        );
    }

    #[ContractTest(
        name: 'No access_nullsafe Kind Exists',
        description: 'Verifies calls.json contains ZERO calls with kind="access_nullsafe". This kind has been removed in favor of access with union return type.',
        category: 'callkind',
            )]
    public function testNoAccessNullsafeKindExists(): void
    {
        $nullsafeCalls = $this->calls()
            ->kind('access_nullsafe')
            ->all();

        $this->assertEmpty(
            $nullsafeCalls,
            sprintf(
                'Should find ZERO access_nullsafe calls. Found %d. This kind should be removed.',
                count($nullsafeCalls)
            )
        );
    }

    // ═══════════════════════════════════════════════════════════════
    // Nullsafe Method Call Tests
    // ═══════════════════════════════════════════════════════════════

    #[ContractTest(
        name: 'Nullsafe Method Call Uses kind=method',
        description: 'Verifies nullsafe method call ($order?->getCustomerName()) uses kind="method" not "method_nullsafe". Per finish-mvp spec, nullsafe semantics are captured via union return type.',
        category: 'callkind',
            )]
    public function testNullsafeMethodCallUsesMethodKind(): void
    {
        // Code reference: src/Service/OrderDisplayService.php:162
        // return $order?->getCustomerName() ?? 'Guest';
        // Note: With nullsafe kinds removed, this should be kind=method, not method_nullsafe
        $methodCalls = $this->calls()
            ->kind('method')
            ->inFile('OrderDisplayService.php')
            ->calleeContains('getCustomerName')
            ->all();

        $this->assertNotEmpty(
            $methodCalls,
            'Should find method calls for getCustomerName() in OrderDisplayService (kind=method, not method_nullsafe)'
        );

        foreach ($methodCalls as $call) {
            $this->assertSame('method', $call['kind'], 'Nullsafe method call should have kind=method');
            $this->assertSame('invocation', $call['kind_type'], 'Method call should have kind_type=invocation');
        }
    }

    #[ContractTest(
        name: 'Nullsafe Method Call Has Union Return Type',
        description: 'Verifies nullsafe method call ($order?->getCustomerName()) has union return_type containing null. For method returning string, should be "null|string" union.',
        category: 'callkind',
            )]
    public function testNullsafeMethodCallHasUnionReturnType(): void
    {
        // Code reference: src/Service/OrderDisplayService.php:162
        // $order?->getCustomerName() // returns string
        // Note: With nullsafe kinds removed, this should be kind=method with union return type
        $methodCalls = $this->calls()
            ->kind('method')
            ->inFile('OrderDisplayService.php')
            ->calleeContains('getCustomerName')
            ->all();

        $this->assertNotEmpty($methodCalls, 'Should find getCustomerName method call (kind=method, not method_nullsafe)');

        $foundUnionType = false;
        foreach ($methodCalls as $call) {
            $returnType = $call['return_type'] ?? null;
            if ($returnType !== null && str_contains($returnType, 'null')) {
                $foundUnionType = true;
                $this->assertStringContainsString(
                    'union',
                    $returnType,
                    'Nullsafe return type should be a union type'
                );
            }
        }

        $this->assertTrue(
            $foundUnionType,
            'At least one nullsafe method call should have union return_type with null'
        );
    }

    #[ContractTest(
        name: 'Nullsafe Boolean Method Has Union Return Type',
        description: 'Verifies nullsafe method call returning bool ($order?->isPending()) has union return_type "null|bool".',
        category: 'callkind',
            )]
    public function testNullsafeBooleanMethodHasUnionReturnType(): void
    {
        // Code reference: src/Service/OrderDisplayService.php:175
        // return $order?->isPending() ?? false; // isPending returns bool
        $methodCalls = $this->calls()
            ->kind('method')
            ->inFile('OrderDisplayService.php')
            ->calleeContains('isPending')
            ->all();

        $this->assertNotEmpty($methodCalls, 'Should find isPending method call');

        $foundUnionType = false;
        foreach ($methodCalls as $call) {
            $returnType = $call['return_type'] ?? null;
            if ($returnType !== null && str_contains($returnType, 'null')) {
                $foundUnionType = true;
                // Should be null|bool union
                $this->assertStringContainsString(
                    'union',
                    $returnType,
                    'Nullsafe return type should be a union type'
                );
            }
        }

        $this->assertTrue(
            $foundUnionType,
            'Nullsafe method call returning bool should have union return_type with null'
        );
    }

    #[ContractTest(
        name: 'No method_nullsafe Kind Exists',
        description: 'Verifies calls.json contains ZERO calls with kind="method_nullsafe". This kind has been removed in favor of method with union return type.',
        category: 'callkind',
            )]
    public function testNoMethodNullsafeKindExists(): void
    {
        $nullsafeCalls = $this->calls()
            ->kind('method_nullsafe')
            ->all();

        $this->assertEmpty(
            $nullsafeCalls,
            sprintf(
                'Should find ZERO method_nullsafe calls. Found %d. This kind should be removed.',
                count($nullsafeCalls)
            )
        );
    }

    // ═══════════════════════════════════════════════════════════════
    // Nullsafe Chain Integrity Tests
    // ═══════════════════════════════════════════════════════════════

    #[ContractTest(
        name: 'Multiple Nullsafe Accesses Share Receiver',
        description: 'Verifies multiple nullsafe property accesses on same variable share the same receiver_value_id. In getOrderSummary(): $order?->id, $order?->customerEmail, $order?->status should all point to same $order value.',
        category: 'chain',
            )]
    public function testMultipleNullsafeAccessesShareReceiver(): void
    {
        // Code reference: src/Service/OrderDisplayService.php:139-141
        // $id = $order?->id ?? 0;
        // $email = $order?->customerEmail ?? '';
        // $status = $order?->status ?? 'unknown';
        $accessCalls = $this->calls()
            ->kind('access')
            ->callerContains('OrderDisplayService#getOrderSummary()')
            ->hasReceiver()
            ->all();

        $this->assertGreaterThanOrEqual(
            3,
            count($accessCalls),
            'Should find at least 3 property accesses in getOrderSummary()'
        );

        // All accesses on $order should share the same receiver_value_id
        $receiverIds = array_unique(
            array_filter(
                array_map(fn($c) => $c['receiver_value_id'] ?? null, $accessCalls)
            )
        );

        $this->assertCount(
            1,
            $receiverIds,
            sprintf(
                'All property accesses on $order should share the same receiver_value_id, found %d different: %s',
                count($receiverIds),
                implode(', ', $receiverIds)
            )
        );
    }

    #[ContractTest(
        name: 'Nullsafe Access Has Receiver Value',
        description: 'Verifies nullsafe property access has receiver_value_id pointing to a valid value in the values array.',
        category: 'chain',
            )]
    public function testNullsafeAccessHasReceiverValue(): void
    {
        // Get any nullsafe property access
        $accessCalls = $this->calls()
            ->kind('access')
            ->inFile('OrderDisplayService.php')
            ->hasReceiver()
            ->all();

        $this->assertNotEmpty($accessCalls, 'Should find property access calls with receivers');

        foreach (array_slice($accessCalls, 0, 5) as $call) { // Check first 5
            $receiverId = $call['receiver_value_id'];
            $this->assertTrue(
                self::$calls->hasValue($receiverId),
                sprintf('Receiver %s should exist in values array for call %s', $receiverId, $call['id'])
            );
        }
    }

    #[ContractTest(
        name: 'Nullsafe Access Result Value Exists',
        description: 'Verifies nullsafe property access creates a result value with source_call_id pointing to the access call.',
        category: 'chain',
            )]
    public function testNullsafeAccessResultValueExists(): void
    {
        $accessCalls = $this->calls()
            ->kind('access')
            ->inFile('OrderDisplayService.php')
            ->all();

        $this->assertNotEmpty($accessCalls, 'Should find property access calls');

        foreach (array_slice($accessCalls, 0, 5) as $call) { // Check first 5
            $callId = $call['id'];

            // Result value should exist with same ID as call
            $this->assertTrue(
                self::$calls->hasValue($callId),
                sprintf('Result value should exist for access call %s', $callId)
            );

            // Verify it's a result value
            $resultValue = self::$calls->getValueById($callId);
            $this->assertSame(
                'result',
                $resultValue['kind'] ?? '',
                sprintf('Value for call %s should be kind=result', $callId)
            );

            // Verify source_call_id points back to itself
            $this->assertSame(
                $callId,
                $resultValue['source_call_id'] ?? '',
                'Result value source_call_id should equal its own id'
            );
        }
    }
}
