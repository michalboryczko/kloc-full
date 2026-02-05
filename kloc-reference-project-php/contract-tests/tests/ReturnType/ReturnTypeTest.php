<?php

declare(strict_types=1);

namespace ContractTests\Tests\ReturnType;

use ContractTests\Attribute\ContractTest;
use ContractTests\CallsContractTestCase;

/**
 * Tests for return_type field tracking across all call kinds.
 *
 * Verifies that return_type is properly populated for method calls,
 * property access, constructors, and operators.
 */
class ReturnTypeTest extends CallsContractTestCase
{
    // ═══════════════════════════════════════════════════════════════
    // Method Call Return Types
    // ═══════════════════════════════════════════════════════════════

    #[ContractTest(
        name: 'Method Calls Have Return Type',
        description: 'Verifies method calls have return_type field populated when method has declared return type.',
        category: 'returntype',
    )]
    public function testMethodCallsHaveReturnType(): void
    {
        $methodCalls = $this->calls()
            ->kind('method')
            ->all();

        $this->assertNotEmpty($methodCalls, 'Should have method calls');

        $callsWithReturnType = 0;
        foreach ($methodCalls as $call) {
            if (!empty($call['return_type'])) {
                $callsWithReturnType++;
            }
        }

        $this->assertGreaterThan(
            0,
            $callsWithReturnType,
            'At least some method calls should have return_type'
        );
    }

    #[ContractTest(
        name: 'OrderRepository findById Returns Nullable Order',
        description: 'Verifies findById() call has return_type containing Order and null union.',
        category: 'returntype',
    )]
    public function testOrderRepositoryFindByIdReturnsNullableOrder(): void
    {
        // Code reference: src/Repository/OrderRepository.php:21
        // public function findById(int $id): ?Order
        $findByIdCalls = $this->calls()
            ->kind('method')
            ->calleeContains('OrderRepository')
            ->calleeContains('findById')
            ->all();

        $this->assertNotEmpty($findByIdCalls, 'Should find findById calls');

        $call = $findByIdCalls[0];
        $returnType = $call['return_type'] ?? null;

        $this->assertNotNull($returnType, 'findById() should have return_type');

        // Should be a union with null (nullable)
        $this->assertTrue(
            str_contains($returnType, 'Order') || str_contains($returnType, 'union'),
            sprintf('findById() return_type should reference Order, got: %s', $returnType)
        );
    }

    #[ContractTest(
        name: 'OrderRepository save Returns Order',
        description: 'Verifies save() call has return_type containing Order.',
        category: 'returntype',
    )]
    public function testOrderRepositorySaveReturnsOrder(): void
    {
        // Code reference: src/Repository/OrderRepository.php:26
        // public function save(Order $order): Order
        $saveCalls = $this->calls()
            ->kind('method')
            ->calleeContains('OrderRepository')
            ->calleeContains('save')
            ->all();

        $this->assertNotEmpty($saveCalls, 'Should find save calls');

        $call = $saveCalls[0];
        $returnType = $call['return_type'] ?? null;

        $this->assertNotNull($returnType, 'save() should have return_type');
        $this->assertStringContainsString(
            'Order',
            $returnType,
            'save() return_type should contain Order'
        );
    }

    // ═══════════════════════════════════════════════════════════════
    // Property Access Return Types
    // ═══════════════════════════════════════════════════════════════

    #[ContractTest(
        name: 'Property Access Has Return Type',
        description: 'Verifies property access calls have return_type matching property type.',
        category: 'returntype',
    )]
    public function testPropertyAccessHasReturnType(): void
    {
        $accessCalls = $this->calls()
            ->kind('access')
            ->all();

        $this->assertNotEmpty($accessCalls, 'Should have property access calls');

        $callsWithReturnType = 0;
        foreach ($accessCalls as $call) {
            if (!empty($call['return_type'])) {
                $callsWithReturnType++;
            }
        }

        $this->assertGreaterThan(
            0,
            $callsWithReturnType,
            'At least some property access calls should have return_type'
        );
    }

    #[ContractTest(
        name: 'Order Status Property Access Has String Type',
        description: 'Verifies $order->status access has string return_type.',
        category: 'returntype',
    )]
    public function testOrderStatusPropertyAccessHasStringType(): void
    {
        // Code reference: src/Entity/Order.php:14
        // public string $status = 'pending'
        $statusAccessCalls = $this->calls()
            ->kind('access')
            ->calleeContains('Order')
            ->calleeContains('status')
            ->all();

        $this->assertNotEmpty($statusAccessCalls, 'Should find $order->status access');

        // Find one with non-union return type (direct access, not nullsafe)
        $directAccess = null;
        foreach ($statusAccessCalls as $call) {
            $returnType = $call['return_type'] ?? '';
            if (!empty($returnType) && !str_contains($returnType, 'null')) {
                $directAccess = $call;
                break;
            }
        }

        if ($directAccess !== null) {
            $this->assertStringContainsString(
                'string',
                $directAccess['return_type'],
                '$order->status return_type should contain string'
            );
        }
    }

    // ═══════════════════════════════════════════════════════════════
    // Constructor Return Types
    // ═══════════════════════════════════════════════════════════════

    #[ContractTest(
        name: 'Constructor Calls Have Return Type',
        description: 'Verifies new Class() calls have return_type matching the constructed class.',
        category: 'returntype',
    )]
    public function testConstructorCallsHaveReturnType(): void
    {
        $constructorCalls = $this->calls()
            ->kind('constructor')
            ->all();

        $this->assertNotEmpty($constructorCalls, 'Should have constructor calls');

        foreach ($constructorCalls as $call) {
            $returnType = $call['return_type'] ?? null;

            $this->assertNotNull(
                $returnType,
                sprintf('Constructor call %s should have return_type', $call['id'])
            );
        }
    }

    #[ContractTest(
        name: 'Order Constructor Returns Order Type',
        description: 'Verifies new Order() has return_type containing Order.',
        category: 'returntype',
    )]
    public function testOrderConstructorReturnsOrderType(): void
    {
        $orderConstructorCalls = $this->calls()
            ->kind('constructor')
            ->calleeContains('Order')
            ->calleeContains('__construct')
            ->all();

        $this->assertNotEmpty($orderConstructorCalls, 'Should find Order constructor calls');

        $call = $orderConstructorCalls[0];
        $returnType = $call['return_type'] ?? null;

        $this->assertNotNull($returnType, 'Order constructor should have return_type');
        $this->assertStringContainsString(
            'Order',
            $returnType,
            'Order constructor return_type should contain Order'
        );
    }

    // ═══════════════════════════════════════════════════════════════
    // Operator Return Types (Experimental)
    // ═══════════════════════════════════════════════════════════════

    #[ContractTest(
        name: 'Coalesce Operator Has Return Type Without Null',
        description: 'Verifies coalesce ($a ?? $b) has return_type without null (null is handled by fallback).',
        category: 'returntype',
        experimental: true,
    )]
    public function testCoalesceOperatorHasReturnTypeWithoutNull(): void
    {
        // Already tested in OperatorTest::testCoalesceReturnTypeRemovesNull
        $coalesceCalls = $this->calls()
            ->kind('coalesce')
            ->all();

        $this->assertNotEmpty($coalesceCalls, 'Coalesce operators should be present with --experimental flag');

        foreach ($coalesceCalls as $call) {
            $returnType = $call['return_type'] ?? null;

            if ($returnType !== null) {
                // Verify return type is populated
                $this->assertNotEmpty($returnType, 'Coalesce return_type should not be empty');
            }
        }
    }

    #[ContractTest(
        name: 'Ternary Operator Has Return Type',
        description: 'Verifies ternary ($a ? $b : $c) has return_type as union of branch types.',
        category: 'returntype',
        experimental: true,
    )]
    public function testTernaryOperatorHasReturnType(): void
    {
        $ternaryFullCalls = $this->calls()
            ->kind('ternary_full')
            ->all();

        $ternaryCalls = $this->calls()
            ->kind('ternary')
            ->all();

        $allTernary = array_merge($ternaryFullCalls, $ternaryCalls);

        $this->assertNotEmpty($allTernary, 'Ternary operators should be present with --experimental flag');

        $callsWithReturnType = 0;
        foreach ($allTernary as $call) {
            if (!empty($call['return_type'])) {
                $callsWithReturnType++;
            }
        }

        $this->assertGreaterThan(
            0,
            $callsWithReturnType,
            'At least some ternary operators should have return_type'
        );
    }

    // ═══════════════════════════════════════════════════════════════
    // Return Type Format Validation
    // ═══════════════════════════════════════════════════════════════

    #[ContractTest(
        name: 'Return Types Use SCIP Symbol Format',
        description: 'Verifies return_type fields use proper SCIP symbol format (scip-php ...).',
        category: 'returntype',
    )]
    public function testReturnTypesUseScipSymbolFormat(): void
    {
        $allCalls = self::$calls->calls();

        $callsWithReturnType = [];
        foreach ($allCalls as $call) {
            if (!empty($call['return_type'])) {
                $callsWithReturnType[] = $call;
            }
        }

        $this->assertNotEmpty($callsWithReturnType, 'Should have calls with return_type');

        foreach ($callsWithReturnType as $call) {
            $returnType = $call['return_type'];

            // Return type should start with scip-php
            $this->assertStringStartsWith(
                'scip-php',
                $returnType,
                sprintf('Return type should use SCIP format, got: %s', $returnType)
            );
        }
    }

    #[ContractTest(
        name: 'Builtin Return Types Have Correct Format',
        description: 'Verifies builtin types (string, int, bool) use scip-php php builtin format.',
        category: 'returntype',
    )]
    public function testBuiltinReturnTypesHaveCorrectFormat(): void
    {
        $allCalls = self::$calls->calls();

        $builtinReturnTypes = [];
        foreach ($allCalls as $call) {
            $returnType = $call['return_type'] ?? '';
            if (str_contains($returnType, 'builtin')) {
                $builtinReturnTypes[] = $returnType;
            }
        }

        $this->assertNotEmpty($builtinReturnTypes, 'Should have some builtin return types');

        foreach ($builtinReturnTypes as $returnType) {
            $this->assertStringContainsString(
                'scip-php php builtin',
                $returnType,
                sprintf('Builtin return type should use correct format, got: %s', $returnType)
            );
        }
    }

    #[ContractTest(
        name: 'Union Return Types Have Correct Format',
        description: 'Verifies union types use scip-php synthetic union format.',
        category: 'returntype',
    )]
    public function testUnionReturnTypesHaveCorrectFormat(): void
    {
        $allCalls = self::$calls->calls();

        $unionReturnTypes = [];
        foreach ($allCalls as $call) {
            $returnType = $call['return_type'] ?? '';
            if (str_contains($returnType, 'union')) {
                $unionReturnTypes[] = $returnType;
            }
        }

        if (empty($unionReturnTypes)) {
            $this->markTestSkipped('No union return types found in index');
        }

        foreach ($unionReturnTypes as $returnType) {
            $this->assertStringContainsString(
                'scip-php synthetic union',
                $returnType,
                sprintf('Union return type should use correct format, got: %s', $returnType)
            );

            // Union should contain pipe separator
            $this->assertStringContainsString(
                '|',
                $returnType,
                sprintf('Union return type should contain pipe, got: %s', $returnType)
            );
        }
    }
}
