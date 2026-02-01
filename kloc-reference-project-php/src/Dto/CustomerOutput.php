<?php

declare(strict_types=1);

namespace App\Dto;

/**
 * Service layer DTO for customer data.
 *
 * Flow: Entity (Customer->contact->email, Customer->address->street)
 *       -> Service (CustomerService returns CustomerOutput)
 *       -> Controller (converts to CustomerResponse)
 */
final readonly class CustomerOutput
{
    public function __construct(
        public int $id,
        public string $name,
        public string $email,
        public string $phone,
        public string $street,
        public string $city,
        public string $postalCode,
        public string $country,
    ) {
    }
}
