<?php

declare(strict_types=1);

namespace App\Dto;

final readonly class CreateOrderInput
{
    public function __construct(
        public string $customerEmail,
        public string $productId,
        public int $quantity,
    ) {
    }
}
