<?php

declare(strict_types=1);

namespace ContractTests\Tests\Argument;

use ContractTests\CallsContractTestCase;

/**
 * Tests for argument binding correctness.
 *
 * These tests verify that method/function arguments are correctly
 * linked to their source values.
 */
class ArgumentBindingTest extends CallsContractTestCase
{
    /**
     * Test: OrderService::createOrder() calls save($order)
     *
     * Code reference: src/Service/OrderService.php:40
     *   $savedOrder = $this->orderRepository->save($order);
     *
     * Expected: Argument 0 points to $order local variable.
     */
    public function testSaveArgumentPointsToOrderLocal(): void
    {
        $this->assertArgument()
            ->inMethod('App\Service\OrderService', 'createOrder')
            ->atCall('save')
            ->position(0)
            ->pointsToLocal('$order')
            ->verify();
    }

    /**
     * Test: NotificationService::notifyOrderCreated() calls findById($orderId)
     *
     * Code reference: src/Service/NotificationService.php:20
     *   $order = $this->orderRepository->findById($orderId);
     *
     * Expected: Argument 0 points to $orderId parameter.
     */
    public function testFindByIdArgumentPointsToParameter(): void
    {
        $this->assertArgument()
            ->inMethod('App\Service\NotificationService', 'notifyOrderCreated')
            ->atCall('findById')
            ->position(0)
            ->pointsToParameter('$orderId')
            ->verify();
    }

    /**
     * Test: EmailSender::send() receives customerEmail access result
     *
     * Code reference: src/Service/OrderService.php:42-43
     *   $this->emailSender->send(
     *       to: $savedOrder->customerEmail,
     *
     * Expected: First argument points to result of customerEmail access.
     */
    public function testEmailSenderReceivesCustomerEmail(): void
    {
        $this->assertArgument()
            ->inMethod('App\Service\OrderService', 'createOrder')
            ->atCall('send')
            ->position(0)
            ->pointsToResultOf('access', 'customerEmail')
            ->verify();
    }

    /**
     * Test: Constructor Order() receives correct arguments
     *
     * Code reference: src/Service/OrderService.php:31-38
     *   $order = new Order(
     *       id: 0,
     *       customerEmail: $input->customerEmail,
     *       productId: $input->productId,
     *       ...
     *   );
     *
     * Expected: Constructor has multiple arguments with correct bindings.
     */
    public function testOrderConstructorArguments(): void
    {
        // Find the constructor call
        $constructorCall = $this->calls()
            ->kind('constructor')
            ->callerContains('OrderService#createOrder()')
            ->calleeContains('Order')
            ->first();

        $this->assertNotNull($constructorCall, 'Should find Order constructor call');

        $arguments = $constructorCall['arguments'] ?? [];
        $this->assertNotEmpty($arguments, 'Constructor should have arguments');

        // First argument (id) should be a literal
        $idArg = $this->findArgumentByPosition($arguments, 0);
        if ($idArg !== null) {
            $idValue = $this->callsData()->getValueById($idArg['value_id']);
            $this->assertEquals('literal', $idValue['kind'] ?? null, 'id argument should be literal');
        }

        // Second argument (customerEmail) should be result of access
        $emailArg = $this->findArgumentByPosition($arguments, 1);
        if ($emailArg !== null) {
            $emailValue = $this->callsData()->getValueById($emailArg['value_id']);
            $this->assertEquals('result', $emailValue['kind'] ?? null, 'customerEmail argument should be result');
        }
    }

    /**
     * Test: OrderRepository::save() in OrderRepository (internal call to new Order)
     *
     * Code reference: src/Repository/OrderRepository.php:29-36
     *   $newOrder = new Order(
     *       id: self::$nextId++,
     *       customerEmail: $order->customerEmail,
     *       ...
     *   );
     *
     * Expected: Constructor receives property access results from $order parameter.
     */
    public function testOrderRepositoryConstructorArguments(): void
    {
        // Find constructor call in save method
        $constructorCall = $this->calls()
            ->kind('constructor')
            ->callerContains('OrderRepository#save()')
            ->calleeContains('Order')
            ->first();

        $this->assertNotNull($constructorCall, 'Should find Order constructor in save()');

        $arguments = $constructorCall['arguments'] ?? [];
        $this->assertNotEmpty($arguments, 'Constructor should have arguments');

        // Verify arguments exist and have value_ids
        foreach ($arguments as $arg) {
            $this->assertArrayHasKey('value_id', $arg);
            $valueId = $arg['value_id'];
            if ($valueId !== null) {
                $value = $this->callsData()->getValueById($valueId);
                $this->assertNotNull($value, "Argument at position {$arg['position']} should have value");
            }
        }
    }

    /**
     * Test: MessageBus dispatch receives OrderCreatedMessage
     *
     * Code reference: src/Service/OrderService.php:53
     *   $this->messageBus->dispatch(new OrderCreatedMessage($savedOrder->id));
     *
     * Expected: dispatch receives constructor result as argument.
     */
    public function testMessageBusDispatchArgument(): void
    {
        // Find dispatch call
        $dispatchCall = $this->calls()
            ->kind('method')
            ->callerContains('OrderService#createOrder()')
            ->calleeContains('dispatch')
            ->first();

        $this->assertNotNull($dispatchCall, 'Should find dispatch call');

        $arguments = $dispatchCall['arguments'] ?? [];
        $this->assertNotEmpty($arguments, 'dispatch should have argument');

        // First argument should be result of constructor
        $arg0 = $this->findArgumentByPosition($arguments, 0);
        $this->assertNotNull($arg0, 'Should have argument at position 0');

        $argValue = $this->callsData()->getValueById($arg0['value_id']);
        $this->assertEquals('result', $argValue['kind'] ?? null, 'Argument should be constructor result');
    }

    /**
     * Test: InventoryChecker receives $input property values
     *
     * Code reference: src/Service/OrderService.php:29
     *   $this->inventoryChecker->checkAvailability($input->productId, $input->quantity);
     *
     * Expected: Both arguments are results of property access on $input.
     */
    public function testInventoryCheckerArguments(): void
    {
        // Find checkAvailability call
        $call = $this->calls()
            ->kind('method')
            ->callerContains('OrderService#createOrder()')
            ->calleeContains('checkAvailability')
            ->first();

        $this->assertNotNull($call, 'Should find checkAvailability call');

        $arguments = $call['arguments'] ?? [];
        $this->assertGreaterThanOrEqual(2, count($arguments), 'Should have at least 2 arguments');

        // Both arguments should be access results
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
     * Find an argument by position in arguments array.
     *
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
