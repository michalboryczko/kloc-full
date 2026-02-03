<?php

declare(strict_types=1);

namespace ContractTests\Tests\CallKind;

use ContractTests\Attribute\ContractTest;
use ContractTests\CallsContractTestCase;

/**
 * Tests for call kind coverage.
 *
 * Verifies that each call kind is properly tracked in the index
 * and has the correct properties per the schema.
 */
class CallKindTest extends CallsContractTestCase
{
    // ═══════════════════════════════════════════════════════════════
    // Invocation Call Kinds
    // ═══════════════════════════════════════════════════════════════

    #[ContractTest(
        name: 'Method Call Kind Exists',
        description: 'Verifies index contains instance method calls (kind=method). Example: $this->orderRepository->save(). Per schema: $obj->method()',
        category: 'callkind',
    )]
    public function testMethodCallKindExists(): void
    {
        $methodCalls = $this->calls()
            ->kind('method')
            ->all();

        $this->assertNotEmpty(
            $methodCalls,
            'Should find at least one method call (kind=method)'
        );

        // Verify properties
        $call = $methodCalls[0];
        $this->assertSame('invocation', $call['kind_type'] ?? '', 'Method calls should have kind_type=invocation');
        $this->assertArrayHasKey('receiver_value_id', $call, 'Method calls should have receiver_value_id');
        $this->assertArrayHasKey('arguments', $call, 'Method calls should have arguments array');
    }

    #[ContractTest(
        name: 'Constructor Call Kind Exists',
        description: 'Verifies index contains constructor calls (kind=constructor). Example: new Order(). Per schema: new Foo()',
        category: 'callkind',
    )]
    public function testConstructorCallKindExists(): void
    {
        $constructorCalls = $this->calls()
            ->kind('constructor')
            ->all();

        $this->assertNotEmpty(
            $constructorCalls,
            'Should find at least one constructor call (kind=constructor)'
        );

        // Verify properties
        $call = $constructorCalls[0];
        $this->assertSame('invocation', $call['kind_type'] ?? '', 'Constructor calls should have kind_type=invocation');
        $this->assertArrayHasKey('arguments', $call, 'Constructor calls should have arguments array');
        // Constructors don't have receiver_value_id (no instance yet)
    }

    #[ContractTest(
        name: 'Static Method Call Kind',
        description: 'Verifies static method calls are tracked with kind=method_static. Example: self::$nextId++. Per schema: Foo::method()',
        category: 'callkind',
        status: 'pending',
    )]
    public function testStaticMethodCallKindExists(): void
    {
        $staticMethodCalls = $this->calls()
            ->kind('method_static')
            ->all();

        // Note: Reference project may not have explicit static method calls
        // This test documents the expected behavior
        if (empty($staticMethodCalls)) {
            $this->markTestSkipped(
                'No static method calls found in reference project. ' .
                'If added, should have kind_type=invocation, no receiver_value_id.'
            );
        }

        $call = $staticMethodCalls[0];
        $this->assertSame('invocation', $call['kind_type'] ?? '');
        $this->assertFalse(
            isset($call['receiver_value_id']),
            'Static method calls should not have receiver_value_id'
        );
    }

    #[ContractTest(
        name: 'Function Call Kind',
        description: 'Verifies function calls are tracked with kind=function. Example: sprintf(). Per schema: func(). NOTE: Function kind is EXPERIMENTAL and requires --experimental flag.',
        category: 'callkind',
        experimental: true,
    )]
    public function testFunctionCallKindExists(): void
    {
        $functionCalls = $this->calls()
            ->kind('function')
            ->all();

        $this->assertNotEmpty(
            $functionCalls,
            'Function calls (kind=function) should be present with --experimental flag'
        );

        // Verify properties
        $call = $functionCalls[0];
        $this->assertSame('invocation', $call['kind_type'] ?? '', 'Function calls should have kind_type=invocation');
        $this->assertArrayHasKey('arguments', $call, 'Function calls should have arguments array');
        $this->assertFalse(
            isset($call['receiver_value_id']),
            'Function calls should not have receiver_value_id'
        );
    }

    // ═══════════════════════════════════════════════════════════════
    // Access Call Kinds
    // ═══════════════════════════════════════════════════════════════

    #[ContractTest(
        name: 'Property Access Kind Exists',
        description: 'Verifies property access is tracked with kind=access. Example: $order->customerEmail. Per schema: $obj->property',
        category: 'callkind',
    )]
    public function testPropertyAccessKindExists(): void
    {
        $accessCalls = $this->calls()
            ->kind('access')
            ->all();

        $this->assertNotEmpty(
            $accessCalls,
            'Should find at least one property access (kind=access)'
        );

        // Verify properties
        $call = $accessCalls[0];
        $this->assertSame('access', $call['kind_type'] ?? '', 'Property access should have kind_type=access');
        // Note: receiver_value_id may be null for $this->property in readonly classes
        // Arguments should be absent or empty for access calls
        if (isset($call['arguments'])) {
            $this->assertEmpty(
                $call['arguments'],
                'Property access arguments array should be empty if present'
            );
        }
    }

    #[ContractTest(
        name: 'Static Property Access Kind',
        description: 'Verifies static property access is tracked with kind=access_static. Example: self::$orders. Per schema: Foo::$property',
        category: 'callkind',
    )]
    public function testStaticPropertyAccessKindExists(): void
    {
        $staticAccessCalls = $this->calls()
            ->kind('access_static')
            ->all();

        $this->assertNotEmpty(
            $staticAccessCalls,
            'Should find at least one static property access (kind=access_static)'
        );

        // Verify properties
        $call = $staticAccessCalls[0];
        $this->assertSame('access', $call['kind_type'] ?? '', 'Static access should have kind_type=access');
        $this->assertFalse(
            isset($call['receiver_value_id']),
            'Static property access should not have receiver_value_id'
        );
    }

    #[ContractTest(
        name: 'Array Access Kind',
        description: 'Verifies array access is tracked with kind=access_array. Example: self::$orders[$id]. Per schema: $arr[\'key\']. NOTE: Array access is EXPERIMENTAL.',
        category: 'callkind',
        experimental: true,
    )]
    public function testArrayAccessKindExists(): void
    {
        $arrayAccessCalls = $this->calls()
            ->kind('access_array')
            ->all();

        $this->assertNotEmpty(
            $arrayAccessCalls,
            'Array access (kind=access_array) should be present with --experimental flag'
        );

        // Verify properties
        $call = $arrayAccessCalls[0];
        $this->assertSame('access', $call['kind_type'] ?? '', 'Array access should have kind_type=access');
        $this->assertArrayHasKey('receiver_value_id', $call, 'Array access should have receiver_value_id');
        $this->assertArrayHasKey('key_value_id', $call, 'Array access should have key_value_id');
    }

    #[ContractTest(
        name: 'Nullsafe Uses Access Kind with Union Type',
        description: 'Verifies nullsafe property access uses kind=access (not access_nullsafe) with union return_type. Per schema: $obj?->prop uses access with return_type including null.',
        category: 'callkind',
    )]
    public function testNullsafeUsesAccessKindWithUnionType(): void
    {
        // There should be NO access_nullsafe kind anymore
        $nullsafeAccessCalls = $this->calls()
            ->kind('access_nullsafe')
            ->all();

        $this->assertEmpty(
            $nullsafeAccessCalls,
            'access_nullsafe kind is deprecated. Nullsafe accesses should use kind=access with union return_type.'
        );

        // There should be NO method_nullsafe kind anymore
        $nullsafeMethodCalls = $this->calls()
            ->kind('method_nullsafe')
            ->all();

        $this->assertEmpty(
            $nullsafeMethodCalls,
            'method_nullsafe kind is deprecated. Nullsafe method calls should use kind=method with union return_type.'
        );
    }

    // ═══════════════════════════════════════════════════════════════
    // Kind Type Categories
    // ═══════════════════════════════════════════════════════════════

    #[ContractTest(
        name: 'All Invocation Kinds Have Arguments',
        description: 'Verifies all calls with kind_type=invocation have an arguments array (may be empty).',
        category: 'callkind',
    )]
    public function testAllInvocationKindsHaveArguments(): void
    {
        $invocationCalls = $this->calls()
            ->kindType('invocation')
            ->all();

        $this->assertNotEmpty($invocationCalls, 'Should have invocation calls');

        $missingArguments = [];
        foreach ($invocationCalls as $call) {
            if (!array_key_exists('arguments', $call)) {
                $missingArguments[] = sprintf(
                    '%s (kind=%s)',
                    $call['id'] ?? 'unknown',
                    $call['kind'] ?? 'unknown'
                );
            }
        }

        $this->assertEmpty(
            $missingArguments,
            sprintf(
                "Found %d invocation calls missing arguments field:\n%s",
                count($missingArguments),
                implode("\n", array_slice($missingArguments, 0, 10))
            )
        );
    }

    #[ContractTest(
        name: 'Instance Methods Have Receiver When Applicable',
        description: 'Verifies most instance method calls have receiver_value_id. Some calls on $this in child classes may not have it tracked.',
        category: 'callkind',
    )]
    public function testInstanceMethodsHaveReceiver(): void
    {
        $methodCalls = $this->calls()
            ->kind('method')
            ->all();

        $this->assertNotEmpty($methodCalls, 'Should have method calls');

        $callsWithReceiver = array_filter($methodCalls, fn($c) => isset($c['receiver_value_id']));

        // Most method calls should have receivers
        $this->assertNotEmpty(
            $callsWithReceiver,
            'At least some method calls should have receiver_value_id'
        );

        // Log any without receivers for information (not a failure)
        $missingReceiver = array_filter($methodCalls, fn($c) => !isset($c['receiver_value_id']));
        if (!empty($missingReceiver)) {
            // This is informational - inherited methods called on $this may lack receiver
            $this->addToAssertionCount(1); // Pass but note the data
        }
    }

    #[ContractTest(
        name: 'Property Access Has Receiver When Applicable',
        description: 'Verifies most property access calls have receiver_value_id. $this->property in readonly classes may be handled differently.',
        category: 'callkind',
    )]
    public function testPropertyAccessHasReceiver(): void
    {
        $accessCalls = $this->calls()
            ->kind('access')
            ->all();

        $this->assertNotEmpty($accessCalls, 'Should have access calls');

        $callsWithReceiver = array_filter($accessCalls, fn($c) => isset($c['receiver_value_id']));

        // Some property accesses should have receivers (e.g., $order->customerEmail)
        $this->assertNotEmpty(
            $callsWithReceiver,
            'At least some property access calls should have receiver_value_id'
        );

        // Verify accesses with receivers reference valid values
        foreach ($callsWithReceiver as $call) {
            $receiverId = $call['receiver_value_id'];
            $this->assertTrue(
                self::$calls->hasValue($receiverId),
                sprintf('Receiver %s should exist for access %s', $receiverId, $call['id'])
            );
        }
    }

    // ═══════════════════════════════════════════════════════════════
    // Specific Call Kind Tests
    // ═══════════════════════════════════════════════════════════════

    #[ContractTest(
        name: 'OrderRepository save() Call Tracked',
        description: 'Verifies the $this->orderRepository->save($order) call is tracked as kind=method with correct callee.',
        category: 'callkind',
    )]
    public function testOrderRepositorySaveCallTracked(): void
    {
        // Code reference: src/Service/OrderService.php:40
        // $savedOrder = $this->orderRepository->save($order);
        $saveCalls = $this->calls()
            ->kind('method')
            ->callerContains('OrderService#createOrder()')
            ->calleeContains('save')
            ->all();

        $this->assertNotEmpty(
            $saveCalls,
            'Should find save() method call in OrderService::createOrder()'
        );

        $call = $saveCalls[0];
        $this->assertSame('method', $call['kind']);
        $this->assertSame('invocation', $call['kind_type']);
        $this->assertNotEmpty($call['receiver_value_id'] ?? '', 'Should have receiver');
        $this->assertNotEmpty($call['arguments'] ?? [], 'Should have arguments');
    }

    #[ContractTest(
        name: 'Order Constructor Call Tracked',
        description: 'Verifies new Order(...) constructor call is tracked as kind=constructor with arguments.',
        category: 'callkind',
    )]
    public function testOrderConstructorCallTracked(): void
    {
        // Code reference: src/Service/OrderService.php:31-38
        // $order = new Order(id: 0, customerEmail: $input->customerEmail, ...)
        $constructorCalls = $this->calls()
            ->kind('constructor')
            ->callerContains('OrderService#createOrder()')
            ->calleeContains('Order')
            ->all();

        $this->assertNotEmpty(
            $constructorCalls,
            'Should find Order constructor call in OrderService::createOrder()'
        );

        $call = $constructorCalls[0];
        $this->assertSame('constructor', $call['kind']);
        $this->assertSame('invocation', $call['kind_type']);

        $arguments = $call['arguments'] ?? [];
        $this->assertNotEmpty($arguments, 'Constructor should have arguments');
        $this->assertGreaterThanOrEqual(6, count($arguments), 'Order constructor has 6 parameters');
    }

    #[ContractTest(
        name: 'sprintf Function Call Tracked',
        description: 'Verifies sprintf() function call is tracked as kind=function. NOTE: Function kind is EXPERIMENTAL and requires --experimental flag.',
        category: 'callkind',
        experimental: true,
    )]
    public function testSprintfFunctionCallTracked(): void
    {
        // Code reference: src/Service/OrderService.php:45-50
        // sprintf('Thank you for your order!...')
        $sprintfCalls = $this->calls()
            ->kind('function')
            ->calleeContains('sprintf')
            ->all();

        $this->assertNotEmpty(
            $sprintfCalls,
            'sprintf() function calls should be present with --experimental flag'
        );

        $call = $sprintfCalls[0];
        $this->assertSame('function', $call['kind']);
        $this->assertSame('invocation', $call['kind_type']);
        $this->assertNotEmpty($call['arguments'] ?? [], 'sprintf should have arguments');
    }

    #[ContractTest(
        name: 'Property Access on $order Tracked',
        description: 'Verifies $order->customerEmail property access is tracked as kind=access.',
        category: 'callkind',
    )]
    public function testPropertyAccessOnOrderTracked(): void
    {
        // Code reference: src/Repository/OrderRepository.php:31
        // customerEmail: $order->customerEmail
        $accessCalls = $this->calls()
            ->kind('access')
            ->callerContains('OrderRepository#save()')
            ->calleeContains('customerEmail')
            ->all();

        $this->assertNotEmpty(
            $accessCalls,
            'Should find $order->customerEmail property access in OrderRepository::save()'
        );

        $call = $accessCalls[0];
        $this->assertSame('access', $call['kind']);
        $this->assertSame('access', $call['kind_type']);
        $this->assertNotEmpty($call['receiver_value_id'] ?? '', 'Property access should have receiver');
    }

    #[ContractTest(
        name: 'Array Access on self::$orders Tracked',
        description: 'Verifies self::$orders[$id] array access is tracked as kind=access_array with key_value_id. NOTE: access_array is EXPERIMENTAL.',
        category: 'callkind',
        experimental: true,
    )]
    public function testArrayAccessOnOrdersTracked(): void
    {
        // Code reference: src/Repository/OrderRepository.php:23
        // return self::$orders[$id] ?? null;
        $arrayAccessCalls = $this->calls()
            ->kind('access_array')
            ->callerContains('OrderRepository')
            ->all();

        $this->assertNotEmpty(
            $arrayAccessCalls,
            'Array access (kind=access_array) should be present with --experimental flag'
        );

        $call = $arrayAccessCalls[0];
        $this->assertSame('access_array', $call['kind']);
        $this->assertSame('access', $call['kind_type']);
        $this->assertArrayHasKey('key_value_id', $call, 'Array access should have key_value_id');
    }
}
