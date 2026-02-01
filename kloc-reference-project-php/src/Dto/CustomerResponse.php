<?php

declare(strict_types=1);

namespace App\Dto;

/**
 * Response DTO for customer data.
 *
 * Contract test: Value flow tracking
 * The street and email values should be traceable from this response
 * back to the original Customer->Address->street and Customer->Contact->email.
 */
final readonly class CustomerResponse
{
    public function __construct(
        public int $id,
        public string $name,
        public string $email,
        public string $phone,
        public string $street,
        public string $city,
        public string $country,
    ) {
    }
}
