<?php

declare(strict_types=1);

namespace App\Service;

use App\Dto\CustomerOutput;
use App\Entity\Customer;
use App\Repository\CustomerRepository;

/**
 * Customer service demonstrating nested property access chains.
 *
 * Contract test scenarios:
 * - Nested property chains: $customer->contact->email, $customer->address->street
 * - Multiple accesses on same nested object should share receivers
 * - Full value flow tracing from entity to service DTO (CustomerOutput)
 *
 * Flow: Entity -> Service (CustomerOutput) -> Controller (CustomerResponse)
 */
final readonly class CustomerService
{
    public function __construct(
        private CustomerRepository $repository,
    ) {
    }

    /**
     * Get customer by ID, returning service DTO.
     *
     * Contract test: Value flow from entity to service DTO
     * - $customer->contact->email flows to output.email
     * - $customer->address->street flows to output.street
     *
     * Code reference for contract tests:
     * Lines 46-49: Nested property access on contact (email, phone)
     * Lines 52-56: Nested property access on address (street, city, etc.)
     * Lines 58-67: Value flow to CustomerOutput constructor
     */
    public function getCustomerById(int $id): ?CustomerOutput
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
        // And the result of address access is shared receiver for street/city/etc.
        $street = $customer->address->street;
        $city = $customer->address->city;
        $postalCode = $customer->address->postalCode;
        $country = $customer->address->country;

        return new CustomerOutput(
            id: $customer->id,
            name: $customer->name,
            email: $email,
            phone: $phone,
            street: $street,
            city: $city,
            postalCode: $postalCode,
            country: $country,
        );
    }

    /**
     * Get customer details with direct nested access pattern.
     *
     * Contract test: Direct nested property access in return
     * Pattern: new CustomerOutput(..., email: $customer->contact->email, ...)
     */
    public function getCustomerDetails(int $id): ?CustomerOutput
    {
        $customer = $this->repository->findById($id);

        if ($customer === null) {
            return null;
        }

        // Direct nested property access in constructor arguments
        // Each nested chain: $customer->contact->X and $customer->address->Y
        return new CustomerOutput(
            id: $customer->id,
            name: $customer->name,
            email: $customer->contact->email,
            phone: $customer->contact->phone,
            street: $customer->address->street,
            city: $customer->address->city,
            postalCode: $customer->address->postalCode,
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
