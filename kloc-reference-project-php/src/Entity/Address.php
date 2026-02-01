<?php

declare(strict_types=1);

namespace App\Entity;

/**
 * Address information for a customer.
 * Used for testing nested object property access chains.
 */
final readonly class Address
{
    public function __construct(
        public string $street,
        public string $city,
        public string $postalCode,
        public string $country,
    ) {
    }

    public function getFullAddress(): string
    {
        return sprintf('%s, %s %s, %s', $this->street, $this->city, $this->postalCode, $this->country);
    }
}
