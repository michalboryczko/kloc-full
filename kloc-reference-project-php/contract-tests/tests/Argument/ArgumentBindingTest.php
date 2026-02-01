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
        name: 'save() Receives $order Local',
        description: 'Argument 0 of save() points to $order local variable',
        category: 'argument',
    )]
    public function testSaveArgumentPointsToOrderLocal(): void
    {
        $this->assertArgument()
            ->inMethod('App\Service\OrderService', 'createOrder')
            ->atCall('save')
            ->position(0)
            ->pointsToLocal('$order')
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
