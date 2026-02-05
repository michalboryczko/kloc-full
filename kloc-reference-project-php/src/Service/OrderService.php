<?php

declare(strict_types=1);

namespace App\Service;

use App\Component\EmailSenderInterface;
use App\Component\InventoryCheckerInterface;
use App\Dto\CreateOrderInput;
use App\Dto\OrderOutput;
use App\Entity\Order;
use App\Repository\OrderRepository;
use App\Ui\Messenger\Message\OrderCreatedMessage;
use DateTimeImmutable;
use Symfony\Component\Messenger\MessageBusInterface;

final readonly class OrderService
{
    public function __construct(
        private OrderRepository $orderRepository,
        private EmailSenderInterface $emailSender,
        private InventoryCheckerInterface $inventoryChecker,
        private MessageBusInterface $messageBus,
        private AbstractOrderProcessor $orderProcessor,
    ) {
    }

    public function createOrder(CreateOrderInput $input): OrderOutput
    {
        $this->inventoryChecker->checkAvailability($input->productId, $input->quantity);

        $order = new Order(
            id: 0,
            customerEmail: $input->customerEmail,
            productId: $input->productId,
            quantity: $input->quantity,
            status: 'pending',
            createdAt: new DateTimeImmutable(),
        );

        // Process order through inheritance chain (AbstractOrderProcessor -> StandardOrderProcessor)
        $processedOrder = $this->orderProcessor->process($order);
        $processorName = $this->orderProcessor->getName();

        $savedOrder = $this->orderRepository->save($processedOrder);

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

        $this->messageBus->dispatch(new OrderCreatedMessage($savedOrder->id));

        return new OrderOutput(
            id: $savedOrder->id,
            customerEmail: $savedOrder->customerEmail,
            productId: $savedOrder->productId,
            quantity: $savedOrder->quantity,
            status: $savedOrder->status,
            createdAt: $savedOrder->createdAt,
        );
    }

    public function getOrder(int $id): ?OrderOutput
    {
        $order = $this->orderRepository->findById($id);

        if ($order === null) {
            return null;
        }

        return new OrderOutput(
            id: $order->id,
            customerEmail: $order->customerEmail,
            productId: $order->productId,
            quantity: $order->quantity,
            status: $order->status,
            createdAt: $order->createdAt,
        );
    }
}
