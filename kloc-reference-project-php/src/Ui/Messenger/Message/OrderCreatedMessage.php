<?php

declare(strict_types=1);

namespace App\Ui\Messenger\Message;

/**
 * Message dispatched when an order is created.
 *
 * Uses readonly class with constructor property promotion.
 * Processed asynchronously by OrderCreatedHandler.
 */
final readonly class OrderCreatedMessage
{
    public function __construct(
        public int $orderId,
    ) {
    }
}
