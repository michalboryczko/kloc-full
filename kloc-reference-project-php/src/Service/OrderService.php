<?php

declare(strict_types=1);

namespace App\Service;

use App\Component\EmailSenderInterface;
use App\Entity\Order;
use App\Repository\OrderRepository;
use App\Ui\Rest\Request\CreateOrderRequest;
use App\Ui\Rest\Response\OrderListResponse;
use App\Ui\Rest\Response\OrderResponse;
use DateTimeImmutable;

/**
 * Service for order business logic.
 *
 * Coordinates between repository and components.
 */
final readonly class OrderService
{
    public function __construct(
        private OrderRepository $orderRepository,
        private EmailSenderInterface $emailSender,
    ) {
    }

    public function createOrder(CreateOrderRequest $request): OrderResponse
    {
        // Create order entity
        $order = new Order(
            id: 0, // Will be assigned by repository
            customerEmail: $request->customerEmail,
            productId: $request->productId,
            quantity: $request->quantity,
            status: 'pending',
            createdAt: new DateTimeImmutable(),
        );

        // Save to repository
        $savedOrder = $this->orderRepository->save($order);

        // Send confirmation email via component
        $this->emailSender->send(
            to: $savedOrder->customerEmail,
            subject: 'Order Confirmation #' . $savedOrder->id,
            body: sprintf(
                'Thank you for your order! Your order #%d for product %s (qty: %d) has been received.',
                $savedOrder->id,
                $savedOrder->productId,
                $savedOrder->quantity,
            ),
        );

        return OrderResponse::fromEntity($savedOrder);
    }

    public function getOrder(int $id): ?OrderResponse
    {
        $order = $this->orderRepository->findById($id);

        if ($order === null) {
            return null;
        }

        return OrderResponse::fromEntity($order);
    }

    public function listOrders(): OrderListResponse
    {
        $orders = $this->orderRepository->findAll();

        $orderResponses = array_map(
            fn(Order $order) => OrderResponse::fromEntity($order),
            $orders,
        );

        return new OrderListResponse(
            orders: $orderResponses,
            total: count($orderResponses),
        );
    }
}
