<?php

declare(strict_types=1);

namespace App\Entity;

use DateTimeImmutable;

/**
 * Order entity representing a customer order.
 *
 * Uses readonly class with constructor property promotion.
 */
final readonly class Order
{
    public function __construct(
        public int $id,
        public string $customerEmail,
        public string $productId,
        public int $quantity,
        public string $status,
        public DateTimeImmutable $createdAt,
    ) {
    }
}
