<?php

declare(strict_types=1);

namespace ContractTests\Tests\Reference;

use ContractTests\Attribute\ContractTest;
use ContractTests\CallsContractTestCase;

/**
 * Tests for parameter reference consistency.
 */
class ParameterReferenceTest extends CallsContractTestCase
{
    #[ContractTest(
        name: 'OrderService::getOrder() $id',
        description: 'Verifies $id parameter in getOrder() has exactly one value entry. Per the spec, each parameter should have a single value entry at declaration, with all usages referencing that entry.',
        category: 'reference',
    )]
    public function testOrderServiceGetOrderIdParameter(): void
    {
        $result = $this->assertReferenceConsistency()
            ->inMethod('App\Service\OrderService', 'getOrder')
            ->forParameter('$id')
            ->verify();

        $this->assertTrue($result->success);
    }

    #[ContractTest(
        name: 'NotificationService::notifyOrderCreated() $orderId',
        description: 'Verifies $orderId parameter in notifyOrderCreated() has exactly one value entry and is correctly referenced when passed to findById() call.',
        category: 'reference',
    )]
    public function testNotificationServiceOrderIdParameter(): void
    {
        $result = $this->assertReferenceConsistency()
            ->inMethod('App\Service\NotificationService', 'notifyOrderCreated')
            ->forParameter('$orderId')
            ->verify();

        $this->assertTrue($result->success);
    }

    #[ContractTest(
        name: 'OrderService Constructor Parameters',
        description: 'Verifies promoted constructor parameters ($orderRepository, $emailSender, $inventoryChecker, $messageBus) have no duplicate symbol entries. Readonly class promoted properties are handled specially by the indexer.',
        category: 'reference',
    )]
    public function testOrderServiceConstructorParameters(): void
    {
        $params = $this->inMethod('App\Service\OrderService', '__construct')
            ->values()
            ->kind('parameter')
            ->all();

        $symbols = array_column($params, 'symbol');
        $uniqueSymbols = array_unique($symbols);

        $this->assertCount(
            count($symbols),
            $uniqueSymbols,
            'Constructor parameters should not have duplicates'
        );
    }
}
