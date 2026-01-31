<?php

declare(strict_types=1);

namespace ContractTests\Tests\Chain;

use ContractTests\CallsContractTestCase;

/**
 * Tests for call chain integrity.
 *
 * These tests verify that method/property chains are properly linked:
 * value -> call -> result value -> call -> result value...
 */
class ChainIntegrityTest extends CallsContractTestCase
{
    /**
     * Test: NotificationService chain - $this->orderRepository->findById()
     *
     * Code reference: src/Service/NotificationService.php:20
     *   $order = $this->orderRepository->findById($orderId);
     *
     * Expected: Chain from $this through orderRepository access to findById method.
     */
    public function testNotificationServiceOrderRepositoryChain(): void
    {
        $result = $this->assertChain()
            ->startingFrom('App\Service\NotificationService', 'notifyOrderCreated', '$this')
            ->throughAccess('orderRepository')
            ->throughMethod('findById')
            ->verify();

        $this->assertEquals(2, $result->stepCount());
    }

    /**
     * Test: OrderService chain - $this->orderRepository->save()
     *
     * Code reference: src/Service/OrderService.php:40
     *   $savedOrder = $this->orderRepository->save($order);
     *
     * Expected: Chain from $this through orderRepository access to save method.
     */
    public function testOrderServiceSaveChain(): void
    {
        $result = $this->assertChain()
            ->startingFrom('App\Service\OrderService', 'createOrder', '$this')
            ->throughAccess('orderRepository')
            ->throughMethod('save')
            ->verify();

        $this->assertEquals(2, $result->stepCount());
        // Final type should be Order
        $this->assertStringContainsString('Order', $result->finalType() ?? '');
    }

    /**
     * Test: Property access on saved order - $savedOrder->customerEmail
     *
     * Code reference: src/Service/OrderService.php:43
     *   to: $savedOrder->customerEmail,
     *
     * Expected: Local variable $savedOrder should have property access calls.
     */
    public function testSavedOrderPropertyAccess(): void
    {
        // Find the $savedOrder local variable
        $savedOrderLocal = $this->values()
            ->kind('local')
            ->symbolContains('createOrder().local$savedOrder')
            ->first();

        $this->assertNotNull($savedOrderLocal, 'Should find $savedOrder local variable');

        // Find property accesses using this as receiver
        $accesses = $this->calls()
            ->kind('access')
            ->withReceiverValueId($savedOrderLocal['id'])
            ->all();

        $this->assertNotEmpty($accesses, '$savedOrder should have property accesses');

        // Verify each access has a result value
        foreach ($accesses as $access) {
            $resultValue = $this->callsData()->getValueById($access['id']);
            $this->assertNotNull(
                $resultValue,
                "Access call {$access['id']} should have result value"
            );
            $this->assertEquals('result', $resultValue['kind']);
        }
    }

    /**
     * Test: NotificationService email chain - $order->customerEmail
     *
     * Code reference: src/Service/NotificationService.php:27
     *   to: $order->customerEmail,
     *
     * Expected: $order local should have customerEmail access.
     */
    public function testNotificationServiceEmailAccess(): void
    {
        // Find the $order local variable in notifyOrderCreated
        $orderLocal = $this->values()
            ->kind('local')
            ->symbolContains('notifyOrderCreated().local$order')
            ->first();

        $this->assertNotNull($orderLocal, 'Should find $order local variable');

        // Find customerEmail access
        $emailAccess = $this->calls()
            ->kind('access')
            ->withReceiverValueId($orderLocal['id'])
            ->calleeContains('customerEmail')
            ->first();

        $this->assertNotNull($emailAccess, 'Should find customerEmail access on $order');

        // Verify result value exists
        $resultValue = $this->callsData()->getValueById($emailAccess['id']);
        $this->assertNotNull($resultValue, 'customerEmail access should have result value');
    }

    /**
     * Test: Chained property accesses share correct receivers
     *
     * Code reference: src/Service/OrderService.php:43-50
     *   $savedOrder->customerEmail
     *   $savedOrder->id
     *   $savedOrder->productId
     *   $savedOrder->quantity
     *
     * Expected: All accesses on $savedOrder share the same receiver_value_id.
     */
    public function testSavedOrderAccessesShareReceiver(): void
    {
        // Find $savedOrder local
        $savedOrderLocal = $this->values()
            ->kind('local')
            ->symbolContains('createOrder().local$savedOrder')
            ->first();

        if ($savedOrderLocal === null) {
            $this->markTestSkipped('Could not find $savedOrder local');
        }

        // Get all accesses with this receiver
        $accesses = $this->calls()
            ->kind('access')
            ->withReceiverValueId($savedOrderLocal['id'])
            ->all();

        // Should have multiple accesses (customerEmail, id, productId, quantity, status, createdAt)
        $this->assertGreaterThanOrEqual(
            3,
            count($accesses),
            '$savedOrder should have multiple property accesses'
        );

        // Verify all share the same receiver
        $this->calls()
            ->kind('access')
            ->withReceiverValueId($savedOrderLocal['id'])
            ->assertAllShareReceiver('All $savedOrder accesses should share receiver');
    }

    /**
     * Test: Input parameter property chain - $input->productId
     *
     * Code reference: src/Service/OrderService.php:29, 34
     *   $this->inventoryChecker->checkAvailability($input->productId, ...)
     *   productId: $input->productId,
     *
     * Expected: $input parameter should have productId accesses.
     */
    public function testInputPropertyAccesses(): void
    {
        // Find $input parameter
        $inputParam = $this->values()
            ->kind('parameter')
            ->symbolContains('createOrder().($input)')
            ->first();

        $this->assertNotNull($inputParam, 'Should find $input parameter');

        // Find property accesses
        $accesses = $this->calls()
            ->kind('access')
            ->withReceiverValueId($inputParam['id'])
            ->all();

        $this->assertNotEmpty($accesses, '$input should have property accesses');

        // Check for specific properties
        $callees = array_column($accesses, 'callee');
        $calleeStr = implode(' ', $callees);

        $this->assertStringContainsString('productId', $calleeStr, 'Should access productId');
        $this->assertStringContainsString('customerEmail', $calleeStr, 'Should access customerEmail');
        $this->assertStringContainsString('quantity', $calleeStr, 'Should access quantity');
    }
}
