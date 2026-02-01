<?php

declare(strict_types=1);

namespace App\Service;

use App\Dto\CustomerResponse;
use App\Entity\Customer;
use App\Repository\CustomerRepository;

/**
 * Customer service demonstrating nested property access chains.
 *
 * Contract test scenarios:
 * - Nested property chains: $customer->contact->email, $customer->address->street
 * - Multiple accesses on same nested object should share receivers
 * - Full value flow tracing from entity to response DTO
 */
final readonly class CustomerService
{
    public function __construct(
        private CustomerRepository $repository,
    ) {
    }

    /**
     * Get customer details demonstrating nested property access chains.
     *
     * Contract test: Nested chain receiver sharing
     * - $customer->contact->email and $customer->contact->phone share contact receiver
     * - $customer->contact and $customer->address share $customer receiver
     * - Values flow from entity properties to response DTO
     *
     * Code reference for contract tests:
     * Lines 40-45: Nested property access on contact (email, phone)
     * Lines 48-51: Nested property access on address (street, city)
     * Lines 53-61: Value flow to CustomerResponse constructor
     */
    public function getCustomerDetails(int $id): ?CustomerResponse
    {
        $customer = $this->repository->findById($id);

        if ($customer === null) {
            return null;
        }

        // These should share the same $customer receiver for 'contact' access
        // And the result of contact access is shared receiver for email/phone
        $email = $customer->contact->email;
        $phone = $customer->contact->phone;

        // These should share the same $customer receiver for 'address' access
        // And the result of address access is shared receiver for street/city
        $street = $customer->address->street;
        $city = $customer->address->city;

        return new CustomerResponse(
            id: $customer->id,
            name: $customer->name,
            email: $email,
            phone: $phone,
            street: $street,
            city: $city,
            country: $customer->address->country,
        );
    }

    /**
     * Demonstrate nested method call chains.
     *
     * Contract test: Method call on nested object result
     * $customer->contact->getFormattedEmail() should have:
     * - getFormattedEmail() receiver = result of contact access
     * - contact access receiver = $customer parameter
     */
    public function getFormattedCustomerEmail(Customer $customer): string
    {
        // Nested method call: $customer->contact->getFormattedEmail()
        return $customer->contact->getFormattedEmail();
    }

    /**
     * Demonstrate nested property access with method chain.
     *
     * Contract test: Property + method chain
     * $customer->address->getFullAddress() should have:
     * - getFullAddress() receiver = result of address access
     * - address access receiver = $customer parameter
     */
    public function getCustomerFullAddress(Customer $customer): string
    {
        // Nested method call: $customer->address->getFullAddress()
        return $customer->address->getFullAddress();
    }

    /**
     * Demonstrate multiple nested chains in single expression.
     *
     * Contract test: Multiple chains sharing receivers
     * Both contact and address accesses share $customer receiver
     */
    public function getCustomerSummary(Customer $customer): string
    {
        // Multiple nested chains - contact and address share $customer receiver
        return sprintf(
            '%s (%s) - %s',
            $customer->name,
            $customer->contact->email,
            $customer->address->city,
        );
    }
}
