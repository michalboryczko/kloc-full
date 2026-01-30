<?php

declare(strict_types=1);

namespace App\Ui\Rest\Controller;

use App\Dto\CreateOrderInput;
use App\Service\OrderService;
use App\Ui\Rest\Request\CreateOrderRequest;
use App\Ui\Rest\Response\OrderResponse;
use Symfony\Bundle\FrameworkBundle\Controller\AbstractController;
use Symfony\Component\HttpFoundation\JsonResponse;
use Symfony\Component\HttpFoundation\Response;
use Symfony\Component\HttpKernel\Attribute\MapRequestPayload;
use Symfony\Component\Routing\Attribute\Route;

#[Route('/api/orders')]
final class OrderController extends AbstractController
{
    public function __construct(
        private readonly OrderService $orderService,
    ) {
    }

    #[Route('/{id}', name: 'order_get', methods: ['GET'])]
    public function get(int $id): JsonResponse
    {
        $output = $this->orderService->getOrder($id);

        if ($output === null) {
            return $this->json(
                ['error' => 'Order not found'],
                Response::HTTP_NOT_FOUND,
            );
        }

        return $this->json(new OrderResponse(
            id: $output->id,
            customerEmail: $output->customerEmail,
            productId: $output->productId,
            quantity: $output->quantity,
            status: $output->status,
            createdAt: $output->createdAt->format('c'),
        ));
    }

    #[Route('', name: 'order_create', methods: ['POST'])]
    public function create(
        #[MapRequestPayload] CreateOrderRequest $request,
    ): JsonResponse {
        $input = new CreateOrderInput(
            customerEmail: $request->customerEmail,
            productId: $request->productId,
            quantity: $request->quantity,
        );

        $output = $this->orderService->createOrder($input);

        return $this->json(
            new OrderResponse(
                id: $output->id,
                customerEmail: $output->customerEmail,
                productId: $output->productId,
                quantity: $output->quantity,
                status: $output->status,
                createdAt: $output->createdAt->format('c'),
            ),
            Response::HTTP_CREATED,
        );
    }
}
