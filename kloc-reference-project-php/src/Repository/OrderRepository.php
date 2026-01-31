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

    public function __construct()
    {
    }

    public function findById(int $id): ?Order
    {
        return self::$orders[$id] ?? null;
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
}
