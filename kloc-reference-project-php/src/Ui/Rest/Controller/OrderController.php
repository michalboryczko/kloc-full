<?php

declare(strict_types=1);

namespace App\Ui\Rest\Controller;

use App\Service\OrderService;
use App\Ui\Messenger\Message\OrderCreatedMessage;
use App\Ui\Rest\Request\CreateOrderRequest;
use App\Ui\Rest\Response\OrderListResponse;
use App\Ui\Rest\Response\OrderResponse;
use Symfony\Bundle\FrameworkBundle\Controller\AbstractController;
use Symfony\Component\HttpFoundation\JsonResponse;
use Symfony\Component\HttpFoundation\Response;
use Symfony\Component\HttpKernel\Attribute\MapRequestPayload;
use Symfony\Component\Messenger\MessageBusInterface;
use Symfony\Component\Routing\Attribute\Route;

/**
 * REST API controller for order management.
 *
 * Demonstrates:
 * - Route attributes for routing
 * - MapRequestPayload for request body mapping
 * - MessageBusInterface for async message dispatch
 * - Service injection via constructor
 */
#[Route('/api/orders')]
final class OrderController extends AbstractController
{
    public function __construct(
        private readonly OrderService $orderService,
        private readonly MessageBusInterface $messageBus,
    ) {
    }

    /**
     * List all orders.
     */
    #[Route('', name: 'order_list', methods: ['GET'])]
    public function list(): JsonResponse
    {
        $response = $this->orderService->listOrders();

        return $this->json($response);
    }

    /**
     * Get a single order by ID.
     */
    #[Route('/{id}', name: 'order_get', methods: ['GET'])]
    public function get(int $id): JsonResponse
    {
        $response = $this->orderService->getOrder($id);

        if ($response === null) {
            return $this->json(
                ['error' => 'Order not found'],
                Response::HTTP_NOT_FOUND,
            );
        }

        return $this->json($response);
    }

    /**
     * Create a new order.
     *
     * - Calls OrderService to create the order
     * - OrderService uses EmailSender component to send confirmation
     * - Dispatches OrderCreatedMessage for async processing
     */
    #[Route('', name: 'order_create', methods: ['POST'])]
    public function create(
        #[MapRequestPayload] CreateOrderRequest $request,
    ): JsonResponse {
        // Create order via service (triggers EmailSender component)
        $response = $this->orderService->createOrder($request);

        // Dispatch async message for additional processing
        $this->messageBus->dispatch(new OrderCreatedMessage($response->id));

        return $this->json($response, Response::HTTP_CREATED);
    }
}
