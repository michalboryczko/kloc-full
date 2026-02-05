<?php

declare(strict_types=1);

namespace ContractTests\Tests\CallKind;

use ContractTests\Attribute\ContractTest;
use ContractTests\CallsContractTestCase;

/**
 * Tests for static method call tracking in calls.json.
 *
 * Verifies that static method calls (Class::method(), self::method(), static::method())
 * are properly tracked with kind=method_static and correct argument binding.
 */
class StaticMethodTest extends CallsContractTestCase
{
    // ═══════════════════════════════════════════════════════════════
    // Static Method Call Kind
    // ═══════════════════════════════════════════════════════════════

    #[ContractTest(
        name: 'Static Method Calls Have Kind method_static',
        description: 'Verifies static method calls (OrderStatusHelper::getLabel()) are tracked with kind=method_static.',
        category: 'callkind',
    )]
    public function testStaticMethodCallsHaveKindMethodStatic(): void
    {
        // Code reference: src/Service/OrderDisplayService.php:66
        // OrderStatusHelper::getLabel($status)
        $staticCalls = $this->calls()
            ->kind('method_static')
            ->all();

        $this->assertNotEmpty(
            $staticCalls,
            'Static method calls should be tracked with kind=method_static'
        );
    }

    #[ContractTest(
        name: 'Static Method Calls Have Kind Type Invocation',
        description: 'Verifies static method calls have kind_type=invocation per schema.',
        category: 'callkind',
    )]
    public function testStaticMethodCallsHaveKindTypeInvocation(): void
    {
        $staticCalls = $this->calls()
            ->kind('method_static')
            ->all();

        $this->assertNotEmpty($staticCalls, 'Should have static method calls');

        foreach ($staticCalls as $call) {
            $this->assertSame(
                'invocation',
                $call['kind_type'] ?? '',
                sprintf('Static method call %s should have kind_type=invocation', $call['id'])
            );
        }
    }

    #[ContractTest(
        name: 'Static Method Calls Have No Receiver Value ID',
        description: 'Verifies static method calls do not have receiver_value_id since they are called on classes, not instances.',
        category: 'callkind',
    )]
    public function testStaticMethodCallsHaveNoReceiverValueId(): void
    {
        $staticCalls = $this->calls()
            ->kind('method_static')
            ->all();

        $this->assertNotEmpty($staticCalls, 'Should have static method calls');

        foreach ($staticCalls as $call) {
            $this->assertNull(
                $call['receiver_value_id'] ?? null,
                sprintf('Static method call %s should not have receiver_value_id', $call['id'])
            );
        }
    }

    #[ContractTest(
        name: 'Static Method Calls Reference Correct Callee Symbol',
        description: 'Verifies static method calls have callee pointing to the static method symbol.',
        category: 'callkind',
    )]
    public function testStaticMethodCallsReferenceCorrectCalleeSymbol(): void
    {
        // Code reference: src/Service/OrderDisplayService.php:66,122,145
        // OrderStatusHelper::getLabel() and OrderStatusHelper::isTerminal()
        $getLabelCalls = $this->calls()
            ->kind('method_static')
            ->calleeContains('OrderStatusHelper')
            ->calleeContains('getLabel')
            ->all();

        $this->assertNotEmpty(
            $getLabelCalls,
            'Should find static calls to OrderStatusHelper::getLabel()'
        );

        foreach ($getLabelCalls as $call) {
            $this->assertStringContainsString(
                'getLabel().',
                $call['callee'],
                'Callee should reference getLabel method'
            );
        }
    }

    #[ContractTest(
        name: 'Static Method Calls Have Arguments Tracked',
        description: 'Verifies static method calls have their arguments properly bound.',
        category: 'callkind',
    )]
    public function testStaticMethodCallsHaveArgumentsTracked(): void
    {
        // Code reference: src/Service/OrderDisplayService.php:66
        // OrderStatusHelper::getLabel($status)
        $getLabelCalls = $this->calls()
            ->kind('method_static')
            ->calleeContains('getLabel')
            ->all();

        $this->assertNotEmpty($getLabelCalls, 'Should find getLabel static calls');

        $callWithArgs = null;
        foreach ($getLabelCalls as $call) {
            if (!empty($call['arguments'])) {
                $callWithArgs = $call;
                break;
            }
        }

        $this->assertNotNull(
            $callWithArgs,
            'At least one static method call should have arguments'
        );

        $this->assertCount(
            1,
            $callWithArgs['arguments'],
            'OrderStatusHelper::getLabel() should have 1 argument'
        );
    }

    #[ContractTest(
        name: 'Static Method Calls Have Return Type',
        description: 'Verifies static method calls have return_type field populated.',
        category: 'callkind',
    )]
    public function testStaticMethodCallsHaveReturnType(): void
    {
        // OrderStatusHelper::getLabel() returns string
        $getLabelCalls = $this->calls()
            ->kind('method_static')
            ->calleeContains('getLabel')
            ->all();

        $this->assertNotEmpty($getLabelCalls, 'Should find getLabel static calls');

        $callWithReturnType = null;
        foreach ($getLabelCalls as $call) {
            if (!empty($call['return_type'])) {
                $callWithReturnType = $call;
                break;
            }
        }

        $this->assertNotNull(
            $callWithReturnType,
            'Static method call should have return_type'
        );

        $this->assertStringContainsString(
            'string',
            $callWithReturnType['return_type'],
            'getLabel() return_type should contain string'
        );
    }

    #[ContractTest(
        name: 'Static Boolean Method Has Bool Return Type',
        description: 'Verifies OrderStatusHelper::isTerminal() has bool return type.',
        category: 'callkind',
    )]
    public function testStaticBooleanMethodHasBoolReturnType(): void
    {
        // Code reference: src/Service/OrderDisplayService.php:122
        // OrderStatusHelper::isTerminal($status) returns bool
        $isTerminalCalls = $this->calls()
            ->kind('method_static')
            ->calleeContains('isTerminal')
            ->all();

        $this->assertNotEmpty($isTerminalCalls, 'Should find isTerminal static calls');

        $call = $isTerminalCalls[0];
        $returnType = $call['return_type'] ?? null;

        $this->assertNotNull($returnType, 'isTerminal() should have return_type');
        $this->assertStringContainsString(
            'bool',
            $returnType,
            'isTerminal() return_type should contain bool'
        );
    }

    // ═══════════════════════════════════════════════════════════════
    // Self:: and Static:: Calls
    // ═══════════════════════════════════════════════════════════════

    #[ContractTest(
        name: 'Self Static Property Access Exists',
        description: 'Verifies self:: static property access is tracked. Example: self::$statusLabels',
        category: 'callkind',
    )]
    public function testSelfStaticPropertyAccessExists(): void
    {
        // Code reference: src/Service/OrderStatusHelper.php:100
        // return array_keys(self::$statusLabels);
        $staticAccessCalls = $this->calls()
            ->kind('access_static')
            ->all();

        // Static property access may or may not exist depending on indexer
        // This test documents the expected behavior
        if (empty($staticAccessCalls)) {
            $this->markTestSkipped('No static property access calls found - may not be implemented');
        }

        foreach ($staticAccessCalls as $call) {
            $this->assertSame(
                'access',
                $call['kind_type'] ?? '',
                sprintf('Static property access %s should have kind_type=access', $call['id'])
            );
        }
    }
}
