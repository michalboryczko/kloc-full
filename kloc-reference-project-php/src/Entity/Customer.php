<?php

declare(strict_types=1);

namespace App\Entity;

/**
 * Customer entity with nested Contact and Address objects.
 *
 * Used for testing:
 * - Nested property access chains: $customer->contact->email, $customer->address->street
 * - Shared receiver verification: Multiple accesses on same nested object
 * - Value flow tracking: From entity through service to response
 */
final readonly class Customer
{
    public function __construct(
        public int $id,
        public string $name,
        public Contact $contact,
        public Address $address,
    ) {
    }

    public function getContactEmail(): string
    {
        return $this->contact->email;
    }

    public function getStreetAddress(): string
    {
        return $this->address->street;
    }
}
