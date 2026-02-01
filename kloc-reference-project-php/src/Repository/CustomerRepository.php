<?php

declare(strict_types=1);

namespace App\Repository;

use App\Entity\Address;
use App\Entity\Contact;
use App\Entity\Customer;

/**
 * In-memory customer repository for testing.
 */
final class CustomerRepository
{
    /** @var array<int, Customer> */
    private static array $customers = [];

    private static int $nextId = 1;

    public function __construct()
    {
    }

    public function findById(int $id): ?Customer
    {
        return self::$customers[$id] ?? null;
    }

    public function save(Customer $customer): Customer
    {
        if ($customer->id === 0) {
            $newCustomer = new Customer(
                id: self::$nextId++,
                name: $customer->name,
                contact: $customer->contact,
                address: $customer->address,
            );
            self::$customers[$newCustomer->id] = $newCustomer;

            return $newCustomer;
        }

        self::$customers[$customer->id] = $customer;

        return $customer;
    }

    /**
     * Create a customer with nested objects.
     *
     * Contract test pattern: Constructor with nested object creation
     * - new Contact(email, phone)
     * - new Address(street, city, postalCode, country)
     * - new Customer(id, name, contact, address)
     */
    public function createCustomer(
        string $name,
        string $email,
        string $phone,
        string $street,
        string $city,
        string $postalCode,
        string $country,
    ): Customer {
        $contact = new Contact(
            email: $email,
            phone: $phone,
        );

        $address = new Address(
            street: $street,
            city: $city,
            postalCode: $postalCode,
            country: $country,
        );

        $customer = new Customer(
            id: 0,
            name: $name,
            contact: $contact,
            address: $address,
        );

        return $this->save($customer);
    }

    public static function clearAll(): void
    {
        self::$customers = [];
        self::$nextId = 1;
    }
}
