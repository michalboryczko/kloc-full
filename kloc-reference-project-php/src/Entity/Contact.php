<?php

declare(strict_types=1);

namespace App\Entity;

/**
 * Contact information for a customer.
 * Used for testing nested object property access chains.
 */
final readonly class Contact
{
    public function __construct(
        public string $email,
        public string $phone,
    ) {
    }

    public function getFormattedEmail(): string
    {
        return strtolower($this->email);
    }
}
