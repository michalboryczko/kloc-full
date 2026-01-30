<?php

declare(strict_types=1);

namespace App\Ui\Rest\Response;

/**
 * Response DTO for a list of orders.
 *
 * Uses readonly class with constructor property promotion.
 */
final readonly class OrderListResponse
{
    /**
     * @param array<OrderResponse> $orders
     */
    public function __construct(
        public array $orders,
        public int $total,
    ) {
    }
}
