<?php

declare(strict_types=1);

namespace App\Component;

interface InventoryCheckerInterface
{
    public function checkAvailability(string $productId, int $quantity): bool;
}
