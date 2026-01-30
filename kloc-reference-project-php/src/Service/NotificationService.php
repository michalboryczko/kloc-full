<?php

declare(strict_types=1);

namespace App\Service;

use App\Component\EmailSenderInterface;
use App\Repository\OrderRepository;

/**
 * Service for sending notifications about orders.
 *
 * Uses EmailSender component for sending emails.
 */
final readonly class NotificationService
{
    public function __construct(
        private OrderRepository $orderRepository,
        private EmailSenderInterface $emailSender,
    ) {
    }

    /**
     * Notify about order creation (called from async handler).
     */
    public function notifyOrderCreated(int $orderId): void
    {
        $order = $this->orderRepository->findById($orderId);

        if ($order === null) {
            return;
        }

        // Send follow-up notification email
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
