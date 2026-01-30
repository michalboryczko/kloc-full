<?php

declare(strict_types=1);

namespace App\Ui\Messenger\Handler;

use App\Service\NotificationService;
use App\Ui\Messenger\Message\OrderCreatedMessage;
use Symfony\Component\Messenger\Attribute\AsMessageHandler;

/**
 * Handler for OrderCreatedMessage.
 *
 * Demonstrates:
 * - AsMessageHandler attribute for auto-registration
 * - Service injection via constructor
 * - Calling service layer from handler
 */
#[AsMessageHandler]
final readonly class OrderCreatedHandler
{
    public function __construct(
        private NotificationService $notificationService,
    ) {
    }

    public function __invoke(OrderCreatedMessage $message): void
    {
        // Trigger follow-up notification via service layer
        $this->notificationService->notifyOrderCreated($message->orderId);
    }
}
