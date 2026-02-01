<?php

declare(strict_types=1);

namespace App\Service;

/**
 * Helper class for order status operations.
 *
 * This class demonstrates static method calls and match expressions
 * for contract testing of scip-php indexer.
 *
 * Contract test patterns:
 * - Static method call (method_static): OrderStatusHelper::getLabel()
 * - Match expression (match): match($status) { ... }
 */
final class OrderStatusHelper
{
    /**
     * Valid order statuses.
     *
     * @var array<string, string>
     */
    private static array $statusLabels = [
        'pending' => 'Pending Review',
        'confirmed' => 'Confirmed',
        'processing' => 'Processing',
        'shipped' => 'Shipped',
        'delivered' => 'Delivered',
        'cancelled' => 'Cancelled',
    ];

    /**
     * Get human-readable label for an order status.
     *
     * Contract test: Static method call (method_static)
     * Usage: OrderStatusHelper::getLabel('pending')
     * Expected: kind=method_static, kind_type=invocation
     *
     * Contract test: Match expression (match)
     * Pattern: match($status) { 'pending' => 'Pending Review', ... }
     * Expected: kind=match, kind_type=operator, subject_value_id, arm_ids
     */
    public static function getLabel(string $status): string
    {
        return match ($status) {
            'pending' => 'Pending Review',
            'confirmed' => 'Confirmed',
            'processing' => 'Processing',
            'shipped' => 'Shipped',
            'delivered' => 'Delivered',
            'cancelled' => 'Cancelled',
            default => 'Unknown Status',
        };
    }

    /**
     * Check if status is a terminal state.
     *
     * Contract test: Static method call (method_static)
     * Usage: OrderStatusHelper::isTerminal('delivered')
     * Expected: kind=method_static, kind_type=invocation
     *
     * Contract test: Match expression (match) returning boolean
     * Pattern: match($status) { 'delivered', 'cancelled' => true, ... }
     */
    public static function isTerminal(string $status): bool
    {
        return match ($status) {
            'delivered', 'cancelled' => true,
            default => false,
        };
    }

    /**
     * Get CSS class for status badge.
     *
     * Contract test: Static method call (method_static)
     * Usage: OrderStatusHelper::getStatusClass('pending')
     */
    public static function getStatusClass(string $status): string
    {
        return match ($status) {
            'pending' => 'badge-warning',
            'confirmed', 'processing' => 'badge-info',
            'shipped' => 'badge-primary',
            'delivered' => 'badge-success',
            'cancelled' => 'badge-danger',
            default => 'badge-secondary',
        };
    }

    /**
     * Get all valid statuses.
     *
     * Contract test: Static method call (method_static) returning array
     */
    public static function getValidStatuses(): array
    {
        return array_keys(self::$statusLabels);
    }
}
