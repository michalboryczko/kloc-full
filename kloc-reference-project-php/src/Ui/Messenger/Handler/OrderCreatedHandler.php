<?php

declare(strict_types=1);

namespace App\Ui\Messenger\Handler;

use App\Service\NotificationService;
use App\Ui\Messenger\Message\OrderCreatedMessage;
use Symfony\Component\Messenger\Attribute\AsMessageHandler;

#[AsMessageHandler]
final readonly class OrderCreatedHandler
{
    public function __construct(
        private NotificationService $notificationService,
    ) {
    }

    public function __invoke(OrderCreatedMessage $message): void
    {
        $this->notificationService->notifyOrderCreated($message->orderId);
    }
}
