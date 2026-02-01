<?php

declare(strict_types=1);

namespace App\Service;

use App\Entity\Order;
use App\Repository\OrderRepository;

/**
 * Service for formatting and displaying order information.
 *
 * This class demonstrates nullsafe operators and ternary expressions
 * for contract testing of scip-php indexer.
 *
 * Contract test patterns:
 * - Nullsafe method call (method_nullsafe): $order?->getStatusLabel()
 * - Nullsafe property access (access_nullsafe): $order?->status
 * - Short ternary (ternary): $name ?: 'Anonymous'
 * - Full ternary (ternary_full): $flag ? 'yes' : 'no'
 * - Null coalesce (coalesce): $value ?? 'default'
 */
final readonly class OrderDisplayService
{
    public function __construct(
        private OrderRepository $orderRepository,
    ) {
    }

    /**
     * Get order status or default value.
     *
     * Contract test: Nullsafe property access (access_nullsafe)
     * Pattern: $order?->status
     * Expected: kind=access_nullsafe, kind_type=access, receiver_value_id
     *
     * Contract test: Null coalesce operator (coalesce)
     * Pattern: $order?->status ?? 'unknown'
     * Expected: kind=coalesce, kind_type=operator, left_value_id, right_value_id
     */
    public function getStatusOrDefault(int $orderId): string
    {
        $order = $this->orderRepository->findById($orderId);

        return $order?->status ?? 'unknown';
    }

    /**
     * Format order status for display.
     *
     * Contract test: Nullsafe property access (access_nullsafe)
     * Pattern: $order?->status
     *
     * Contract test: Full ternary operator (ternary_full)
     * Pattern: $order !== null ? OrderStatusHelper::getLabel(...) : 'No order'
     * Expected: kind=ternary_full, kind_type=operator, condition_value_id, true_value_id, false_value_id
     *
     * Contract test: Static method call (method_static)
     * Pattern: OrderStatusHelper::getLabel($status)
     */
    public function formatOrderStatus(int $orderId): string
    {
        $order = $this->orderRepository->findById($orderId);
        $status = $order?->status;

        return $status !== null ? OrderStatusHelper::getLabel($status) : 'No order found';
    }

    /**
     * Get display name with fallback.
     *
     * Contract test: Short ternary operator (ternary) / Elvis operator
     * Pattern: $customerEmail ?: 'anonymous@example.com'
     * Expected: kind=ternary, kind_type=operator, condition_value_id, false_value_id
     *
     * Contract test: Null coalesce operator (coalesce)
     * Pattern: $order?->customerEmail ?? ''
     */
    public function getDisplayName(int $orderId): string
    {
        $order = $this->orderRepository->findById($orderId);
        $customerEmail = $order?->customerEmail ?? '';

        return $customerEmail ?: 'anonymous@example.com';
    }

    /**
     * Get formatted quantity string.
     *
     * Contract test: Nullsafe property access (access_nullsafe)
     * Pattern: $order?->quantity
     *
     * Contract test: Full ternary with method call
     * Pattern: $qty > 1 ? 'items' : 'item'
     */
    public function formatQuantity(int $orderId): string
    {
        $order = $this->orderRepository->findById($orderId);
        $qty = $order?->quantity ?? 0;

        $itemWord = $qty > 1 ? 'items' : 'item';

        return sprintf('%d %s', $qty, $itemWord);
    }

    /**
     * Check if order is in terminal state.
     *
     * Contract test: Nullsafe property access (access_nullsafe)
     * Pattern: $order?->status
     *
     * Contract test: Static method call (method_static)
     * Pattern: OrderStatusHelper::isTerminal($status)
     *
     * Contract test: Full ternary with static method
     */
    public function isOrderComplete(int $orderId): bool
    {
        $order = $this->orderRepository->findById($orderId);
        $status = $order?->status;

        return $status !== null ? OrderStatusHelper::isTerminal($status) : false;
    }

    /**
     * Get order summary with all patterns combined.
     *
     * Contract tests demonstrated:
     * - Nullsafe property access: $order?->id, $order?->customerEmail, $order?->status
     * - Null coalesce: ?? 'N/A'
     * - Short ternary: $email ?: 'anonymous'
     * - Full ternary: $id > 0 ? "Order #$id" : "New Order"
     * - Static method call: OrderStatusHelper::getLabel()
     */
    public function getOrderSummary(int $orderId): string
    {
        $order = $this->orderRepository->findById($orderId);

        $id = $order?->id ?? 0;
        $email = $order?->customerEmail ?? '';
        $status = $order?->status ?? 'unknown';

        $displayEmail = $email ?: 'anonymous';
        $title = $id > 0 ? "Order #{$id}" : 'New Order';
        $statusLabel = OrderStatusHelper::getLabel($status);

        return sprintf('%s for %s - Status: %s', $title, $displayEmail, $statusLabel);
    }

    /**
     * Get customer name using nullsafe method call.
     *
     * Contract test: Nullsafe method call (method_nullsafe)
     * Pattern: $order?->getCustomerName()
     * Expected: kind=method_nullsafe, kind_type=invocation, receiver_value_id
     */
    public function getCustomerNameOrDefault(int $orderId): string
    {
        $order = $this->orderRepository->findById($orderId);

        // Nullsafe method call - key contract test pattern
        return $order?->getCustomerName() ?? 'Guest';
    }

    /**
     * Check if order is pending using nullsafe method call.
     *
     * Contract test: Nullsafe method call (method_nullsafe)
     * Pattern: $order?->isPending()
     */
    public function isOrderPending(int $orderId): bool
    {
        $order = $this->orderRepository->findById($orderId);

        return $order?->isPending() ?? false;
    }
}
