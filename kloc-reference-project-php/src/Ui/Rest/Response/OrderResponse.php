<?php

declare(strict_types=1);

namespace App\Ui\Rest\Response;

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
}
