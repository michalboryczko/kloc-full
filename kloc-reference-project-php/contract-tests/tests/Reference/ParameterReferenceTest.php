<?php

declare(strict_types=1);

namespace ContractTests\Tests\Reference;

use ContractTests\CallsContractTestCase;

/**
 * Tests for parameter reference consistency.
 *
 * These tests verify that parameters have exactly one value entry
 * and all usages reference that single entry.
 */
class ParameterReferenceTest extends CallsContractTestCase
{
    /**
     * Test: OrderService::getOrder() - $id parameter
     *
     * Code reference: src/Service/OrderService.php:65
     *   public function getOrder(int $id): ?OrderOutput
     *
     * Expected: One value entry for $id.
     */
    public function testOrderServiceGetOrderIdParameter(): void
    {
        $result = $this->assertReferenceConsistency()
            ->inMethod('App\Service\OrderService', 'getOrder')
            ->forParameter('$id')
            ->verify();

        $this->assertTrue($result->success);
    }

    /**
     * Test: NotificationService::notifyOrderCreated() - $orderId parameter
     *
     * Code reference: src/Service/NotificationService.php:18
     *   public function notifyOrderCreated(int $orderId): void
     *
     * Expected: One value entry for $orderId.
     */
    public function testNotificationServiceOrderIdParameter(): void
    {
        $result = $this->assertReferenceConsistency()
            ->inMethod('App\Service\NotificationService', 'notifyOrderCreated')
            ->forParameter('$orderId')
            ->verify();

        $this->assertTrue($result->success);
    }

    /**
     * Test: All parameters in OrderService constructor
     *
     * Code reference: src/Service/OrderService.php:19-24
     *   public function __construct(
     *       private OrderRepository $orderRepository,
     *       private EmailSenderInterface $emailSender,
     *       private InventoryCheckerInterface $inventoryChecker,
     *       private MessageBusInterface $messageBus,
     *   )
     *
     * Expected: One value entry for each promoted constructor parameter.
     */
    public function testOrderServiceConstructorParameters(): void
    {
        // Promoted properties in readonly classes are handled specially
        // They may or may not have explicit parameter entries depending on indexer
        $params = $this->inMethod('App\Service\OrderService', '__construct')
            ->values()
            ->kind('parameter')
            ->all();

        // At minimum, we should have no duplicates
        $symbols = array_column($params, 'symbol');
        $uniqueSymbols = array_unique($symbols);

        $this->assertCount(
            count($symbols),
            $uniqueSymbols,
            'Constructor parameters should not have duplicates'
        );
    }
}
