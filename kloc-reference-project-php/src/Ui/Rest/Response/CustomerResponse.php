<?php

declare(strict_types=1);

namespace App\Ui\Rest\Response;

/**
 * REST response DTO for customer data.
 *
 * Contract test: Full value flow tracing
 * Values in this response should be traceable back through:
 * Controller -> CustomerOutput -> Entity (Customer->contact->email, Customer->address->street)
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
        public string $postalCode,
        public string $country,
    ) {
    }
}
