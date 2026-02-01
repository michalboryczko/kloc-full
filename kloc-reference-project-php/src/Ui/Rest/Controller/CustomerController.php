<?php

declare(strict_types=1);

namespace App\Ui\Rest\Controller;

use App\Service\CustomerService;
use App\Ui\Rest\Response\CustomerResponse;
use Symfony\Bundle\FrameworkBundle\Controller\AbstractController;
use Symfony\Component\HttpFoundation\JsonResponse;
use Symfony\Component\HttpFoundation\Response;
use Symfony\Component\Routing\Attribute\Route;

/**
 * REST controller for customer operations.
 *
 * Contract test: Full flow from Entity to Response
 * Flow: Entity (Customer->contact->email, Customer->address->street)
 *       -> Service (returns CustomerOutput with nested property values)
 *       -> Controller (converts CustomerOutput to CustomerResponse)
 *
 * This demonstrates the complete data flow that contract tests verify:
 * - Entity nested property access chains
 * - Service DTO creation from entity properties
 * - Controller DTO transformation
 */
#[Route('/api/customers')]
final class CustomerController extends AbstractController
{
    public function __construct(
        private readonly CustomerService $customerService,
    ) {
    }

    /**
     * Get customer by ID.
     *
     * Contract test: Value flow tracing from response to entity
     * Each property in CustomerResponse can be traced back:
     * - response.street <- output.street <- $customer->address->street <- Address entity
     * - response.email <- output.email <- $customer->contact->email <- Contact entity
     */
    #[Route('/{id}', name: 'customer_get', methods: ['GET'])]
    public function get(int $id): JsonResponse
    {
        $output = $this->customerService->getCustomerById($id);

        if ($output === null) {
            return $this->json(
                ['error' => 'Customer not found'],
                Response::HTTP_NOT_FOUND,
            );
        }

        // Contract test: This transformation creates the final response
        // Each property access on $output can be traced back to entity
        return $this->json(new CustomerResponse(
            id: $output->id,
            name: $output->name,
            email: $output->email,
            phone: $output->phone,
            street: $output->street,
            city: $output->city,
            postalCode: $output->postalCode,
            country: $output->country,
        ));
    }

    /**
     * Get customer summary with formatted address.
     *
     * Contract test: Method calls on nested objects
     * Uses CustomerService methods that call entity methods
     */
    #[Route('/{id}/summary', name: 'customer_summary', methods: ['GET'])]
    public function summary(int $id): JsonResponse
    {
        $output = $this->customerService->getCustomerById($id);

        if ($output === null) {
            return $this->json(
                ['error' => 'Customer not found'],
                Response::HTTP_NOT_FOUND,
            );
        }

        // Build summary using output properties
        $summary = sprintf(
            '%s (%s) - %s, %s',
            $output->name,
            $output->email,
            $output->street,
            $output->city,
        );

        return $this->json([
            'id' => $output->id,
            'summary' => $summary,
        ]);
    }
}
