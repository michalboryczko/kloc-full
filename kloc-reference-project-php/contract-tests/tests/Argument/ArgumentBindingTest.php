<?php

declare(strict_types=1);

namespace ContractTests\Tests\Argument;

use ContractTests\Attribute\ContractTest;
use ContractTests\CallsContractTestCase;

/**
 * Tests for argument binding correctness.
 */
class ArgumentBindingTest extends CallsContractTestCase
{
    #[ContractTest(
        name: 'save() Receives $processedOrder Local',
        description: 'Argument 0 of save() points to $processedOrder local variable (processed via AbstractOrderProcessor)',
        category: 'argument',
    )]
    public function testSaveArgumentPointsToProcessedOrderLocal(): void
    {
        $this->assertArgument()
            ->inMethod('App\Service\OrderService', 'createOrder')
            ->atCall('save')
            ->position(0)
            ->pointsToLocal('$processedOrder')
            ->verify();
    }

    #[ContractTest(
        name: 'findById() Receives $orderId Parameter',
        description: 'Argument 0 of findById() points to $orderId parameter',
        category: 'argument',
    )]
    public function testFindByIdArgumentPointsToParameter(): void
    {
        $this->assertArgument()
            ->inMethod('App\Service\NotificationService', 'notifyOrderCreated')
            ->atCall('findById')
            ->position(0)
            ->pointsToParameter('$orderId')
            ->verify();
    }

    #[ContractTest(
        name: 'send() Receives customerEmail Access Result',
        description: 'First argument of send() points to customerEmail property access result',
        category: 'argument',
    )]
    public function testEmailSenderReceivesCustomerEmail(): void
    {
        $this->assertArgument()
            ->inMethod('App\Service\OrderService', 'createOrder')
            ->atCall('send')
            ->position(0)
            ->pointsToResultOf('access', 'customerEmail')
            ->verify();
    }

    #[ContractTest(
        name: 'Order Constructor Arguments',
        description: 'Order constructor receives correct argument types (literal, result)',
        category: 'argument',
    )]
    public function testOrderConstructorArguments(): void
    {
        $constructorCall = $this->calls()
            ->kind('constructor')
            ->callerContains('OrderService#createOrder()')
            ->calleeContains('Order')
            ->first();

        $this->assertNotNull($constructorCall, 'Should find Order constructor call');

        $arguments = $constructorCall['arguments'] ?? [];
        $this->assertNotEmpty($arguments, 'Constructor should have arguments');

        $idArg = $this->findArgumentByPosition($arguments, 0);
        if ($idArg !== null) {
            $idValue = $this->callsData()->getValueById($idArg['value_id']);
            $this->assertEquals('literal', $idValue['kind'] ?? null, 'id argument should be literal');
        }

        $emailArg = $this->findArgumentByPosition($arguments, 1);
        if ($emailArg !== null) {
            $emailValue = $this->callsData()->getValueById($emailArg['value_id']);
            $this->assertEquals('result', $emailValue['kind'] ?? null, 'customerEmail argument should be result');
        }
    }

    #[ContractTest(
        name: 'OrderRepository Constructor in save()',
        description: 'Order constructor in save() receives property access results from $order',
        category: 'argument',
    )]
    public function testOrderRepositoryConstructorArguments(): void
    {
        $constructorCall = $this->calls()
            ->kind('constructor')
            ->callerContains('OrderRepository#save()')
            ->calleeContains('Order')
            ->first();

        $this->assertNotNull($constructorCall, 'Should find Order constructor in save()');

        $arguments = $constructorCall['arguments'] ?? [];
        $this->assertNotEmpty($arguments, 'Constructor should have arguments');

        foreach ($arguments as $arg) {
            $this->assertArrayHasKey('value_id', $arg);
            $valueId = $arg['value_id'];
            if ($valueId !== null) {
                $value = $this->callsData()->getValueById($valueId);
                $this->assertNotNull($value, "Argument at position {$arg['position']} should have value");
            }
        }
    }

    #[ContractTest(
        name: 'dispatch() Receives Constructor Result',
        description: 'MessageBus dispatch() receives OrderCreatedMessage constructor result',
        category: 'argument',
    )]
    public function testMessageBusDispatchArgument(): void
    {
        $dispatchCall = $this->calls()
            ->kind('method')
            ->callerContains('OrderService#createOrder()')
            ->calleeContains('dispatch')
            ->first();

        $this->assertNotNull($dispatchCall, 'Should find dispatch call');

        $arguments = $dispatchCall['arguments'] ?? [];
        $this->assertNotEmpty($arguments, 'dispatch should have argument');

        $arg0 = $this->findArgumentByPosition($arguments, 0);
        $this->assertNotNull($arg0, 'Should have argument at position 0');

        $argValue = $this->callsData()->getValueById($arg0['value_id']);
        $this->assertEquals('result', $argValue['kind'] ?? null, 'Argument should be constructor result');
    }

    #[ContractTest(
        name: 'checkAvailability() Receives Property Access Results',
        description: 'InventoryChecker receives $input property values as arguments',
        category: 'argument',
    )]
    public function testInventoryCheckerArguments(): void
    {
        $call = $this->calls()
            ->kind('method')
            ->callerContains('OrderService#createOrder()')
            ->calleeContains('checkAvailability')
            ->first();

        $this->assertNotNull($call, 'Should find checkAvailability call');

        $arguments = $call['arguments'] ?? [];
        $this->assertGreaterThanOrEqual(2, count($arguments), 'Should have at least 2 arguments');

        foreach ([0, 1] as $pos) {
            $arg = $this->findArgumentByPosition($arguments, $pos);
            if ($arg !== null) {
                $value = $this->callsData()->getValueById($arg['value_id']);
                $this->assertEquals(
                    'result',
                    $value['kind'] ?? null,
                    "Argument at position {$pos} should be access result"
                );
            }
        }
    }

    #[ContractTest(
        name: 'Argument value_expr for Complex Expressions',
        description: 'Verifies arguments with complex expressions (like self::$nextId++) have value_expr when value_id is null.',
        category: 'argument',
    )]
    public function testArgumentValueExprForComplexExpressions(): void
    {
        // Find Order constructor in save() which has self::$nextId++ as first argument
        $constructorCall = $this->calls()
            ->kind('constructor')
            ->callerContains('OrderRepository#save()')
            ->calleeContains('Order')
            ->first();

        $this->assertNotNull($constructorCall, 'Should find Order constructor in save()');

        $arguments = $constructorCall['arguments'] ?? [];
        $idArg = $this->findArgumentByPosition($arguments, 0);

        $this->assertNotNull($idArg, 'Should have first argument');

        // This argument should have value_expr since self::$nextId++ is complex
        if ($idArg['value_id'] === null) {
            $this->assertNotEmpty(
                $idArg['value_expr'] ?? '',
                'Argument with null value_id should have value_expr'
            );
            $this->assertStringContainsString(
                'nextId',
                $idArg['value_expr'] ?? '',
                'value_expr should contain the expression'
            );
        }
    }

    #[ContractTest(
        name: 'Argument Parameter Symbol Present',
        description: 'Verifies arguments have parameter symbol linking to the callee parameter definition.',
        category: 'argument',
    )]
    public function testArgumentParameterSymbolPresent(): void
    {
        // Find a method call with arguments
        $methodCall = $this->calls()
            ->kind('method')
            ->callerContains('OrderService#createOrder()')
            ->calleeContains('save')
            ->first();

        $this->assertNotNull($methodCall, 'Should find save() call');

        $arguments = $methodCall['arguments'] ?? [];
        $this->assertNotEmpty($arguments, 'save() should have arguments');

        $arg0 = $this->findArgumentByPosition($arguments, 0);
        $this->assertNotNull($arg0, 'Should have argument at position 0');

        // Check parameter symbol is present
        $this->assertNotEmpty(
            $arg0['parameter'] ?? '',
            'Argument should have parameter symbol'
        );
        $this->assertStringContainsString(
            '($order)',
            $arg0['parameter'] ?? '',
            'Parameter symbol should reference the parameter name'
        );
    }

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
}
