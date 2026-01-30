<?php

declare(strict_types=1);

namespace App\Ui\Rest\Response;

use App\Entity\Order;

/**
 * Response DTO for a single order.
 *
 * Uses readonly class with constructor property promotion.
 */
final readonly class OrderResponse
{
    public function __construct(
        public int $id,
        public string $customerEmail,
        public string $productId,
        public int $quantity,
        public string $status,
        public string $createdAt,
    ) {
    }

    public static function fromEntity(Order $order): self
    {
        return new self(
            id: $order->id,
            customerEmail: $order->customerEmail,
            productId: $order->productId,
            quantity: $order->quantity,
            status: $order->status,
            createdAt: $order->createdAt->format('c'),
        );
    }
}
