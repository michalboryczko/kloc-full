<?php

declare(strict_types=1);

namespace App\Entity;

use DateTimeImmutable;

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

    /**
     * Get customer display name from email.
     * Used for nullsafe method call contract testing.
     */
    public function getCustomerName(): string
    {
        $parts = explode('@', $this->customerEmail);
        return $parts[0];
    }

    /**
     * Check if order is in pending status.
     */
    public function isPending(): bool
    {
        return $this->status === 'pending';
    }
}
