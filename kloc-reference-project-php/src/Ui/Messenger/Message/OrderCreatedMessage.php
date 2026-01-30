<?php

declare(strict_types=1);

namespace App\Ui\Messenger\Message;

final readonly class OrderCreatedMessage
{
    public function __construct(
        public int $orderId,
    ) {
    }
}
