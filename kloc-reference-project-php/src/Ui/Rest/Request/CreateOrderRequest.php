<?php

declare(strict_types=1);

namespace App\Ui\Rest\Request;

use Symfony\Component\Validator\Constraints as Assert;

/**
 * Request DTO for creating a new order.
 *
 * Uses readonly class with constructor property promotion.
 * Validated using Symfony Validator constraints.
 */
final readonly class CreateOrderRequest
{
    public function __construct(
        #[Assert\NotBlank]
        #[Assert\Email]
        public string $customerEmail,

        #[Assert\NotBlank]
        public string $productId,

        #[Assert\Positive]
        public int $quantity,
    ) {
    }
}
