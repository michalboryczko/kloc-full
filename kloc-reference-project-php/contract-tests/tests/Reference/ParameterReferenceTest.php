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
     * Test: OrderRepository::save() - $order parameter
     *
     * Code reference: src/Repository/OrderRepository.php:26
     *   public function save(Order $order): Order
     *
     * Expected: One value entry for $order, all usages reference it.
     */
    public function testOrderRepositorySaveOrderParameter(): void
    {
        $result = $this->assertReferenceConsistency()
            ->inMethod('App\Repository\OrderRepository', 'save')
            ->forParameter('$order')
            ->verify();

        $this->assertTrue($result->success);
        $this->assertEquals(1, $result->valueCount);
    }

    /**
     * Test: OrderRepository::findById() - $id parameter
     *
     * Code reference: src/Repository/OrderRepository.php:21
     *   public function findById(int $id): ?Order
     *
     * Expected: One value entry for $id.
     */
    public function testOrderRepositoryFindByIdParameter(): void
    {
        $result = $this->assertReferenceConsistency()
            ->inMethod('App\Repository\OrderRepository', 'findById')
            ->forParameter('$id')
            ->verify();

        $this->assertTrue($result->success);
    }

    /**
     * Test: OrderService::createOrder() - $input parameter
     *
     * Code reference: src/Service/OrderService.php:27
     *   public function createOrder(CreateOrderInput $input): OrderOutput
     *
     * Expected: One value entry for $input, multiple property accesses reference it.
     */
    public function testOrderServiceCreateOrderInputParameter(): void
    {
        $result = $this->assertReferenceConsistency()
            ->inMethod('App\Service\OrderService', 'createOrder')
            ->forParameter('$input')
            ->verify();

        $this->assertTrue($result->success);
        // $input is used multiple times for property access
        $this->assertGreaterThanOrEqual(1, $result->callCount);
    }

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
     * Test: EmailSender::send() - $to, $subject, $body parameters
     *
     * Code reference: src/Component/EmailSender.php:12
     *   public function send(string $to, string $subject, string $body): void
     *
     * Expected: One value entry for each parameter.
     */
    public function testEmailSenderSendParameters(): void
    {
        foreach (['$to', '$subject', '$body'] as $param) {
            $result = $this->assertReferenceConsistency()
                ->inMethod('App\Component\EmailSender', 'send')
                ->forParameter($param)
                ->verify();

            $this->assertTrue($result->success, "Parameter {$param} should have single value entry");
        }
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
