<?php

declare(strict_types=1);

namespace App\Dto;

use DateTimeImmutable;

final readonly class OrderOutput
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
