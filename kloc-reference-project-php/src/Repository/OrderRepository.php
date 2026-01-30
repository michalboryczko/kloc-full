<?php

declare(strict_types=1);

namespace App\Repository;

use App\Entity\Order;
use DateTimeImmutable;

final class OrderRepository
{
    /** @var array<int, Order> */
    private static array $orders = [];

    private static int $nextId = 1;

    private static bool $seeded = false;

    public function __construct()
    {
        $this->seedIfNeeded();
    }

    public function findById(int $id): ?Order
    {
        return self::$orders[$id] ?? null;
    }

    /** @return array<Order> */
    public function findAll(): array
    {
        return array_values(self::$orders);
    }

    public function save(Order $order): Order
    {
        if ($order->id === 0) {
            $newOrder = new Order(
                id: self::$nextId++,
                customerEmail: $order->customerEmail,
                productId: $order->productId,
                quantity: $order->quantity,
                status: $order->status,
                createdAt: $order->createdAt,
            );
            self::$orders[$newOrder->id] = $newOrder;

            return $newOrder;
        }

        self::$orders[$order->id] = $order;

        return $order;
    }

    private function seedIfNeeded(): void
    {
        if (self::$seeded) {
            return;
        }

        self::$seeded = true;

        $this->save(new Order(
            id: 0,
            customerEmail: 'john.doe@example.com',
            productId: 'PROD-001',
            quantity: 2,
            status: 'pending',
            createdAt: new DateTimeImmutable('2024-01-15 10:30:00'),
        ));

        $this->save(new Order(
            id: 0,
            customerEmail: 'jane.smith@example.com',
            productId: 'PROD-002',
            quantity: 1,
            status: 'confirmed',
            createdAt: new DateTimeImmutable('2024-01-16 14:45:00'),
        ));

        $this->save(new Order(
            id: 0,
            customerEmail: 'bob.wilson@example.com',
            productId: 'PROD-003',
            quantity: 5,
            status: 'shipped',
            createdAt: new DateTimeImmutable('2024-01-17 09:15:00'),
        ));
    }
}
