<?php

declare(strict_types=1);

namespace App\Service;

use App\Component\EmailSenderInterface;
use App\Repository\OrderRepository;

final readonly class NotificationService
{
    public function __construct(
        private OrderRepository $orderRepository,
        private EmailSenderInterface $emailSender,
    ) {
    }

    public function notifyOrderCreated(int $orderId): void
    {
        $order = $this->orderRepository->findById($orderId);

        if ($order === null) {
            return;
        }

        $this->emailSender->send(
            to: $order->customerEmail,
            subject: 'Order #' . $order->id . ' is being processed',
            body: sprintf(
                'Your order #%d is now being processed. We will notify you when it ships. ' .
                'Order details: Product %s, Quantity: %d.',
                $order->id,
                $order->productId,
                $order->quantity,
            ),
        );
    }
}
