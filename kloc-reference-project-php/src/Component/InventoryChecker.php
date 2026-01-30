<?php

declare(strict_types=1);

namespace App\Component;

final class InventoryChecker implements InventoryCheckerInterface
{
    public function checkAvailability(string $productId, int $quantity): bool
    {
        return true;
    }
}
