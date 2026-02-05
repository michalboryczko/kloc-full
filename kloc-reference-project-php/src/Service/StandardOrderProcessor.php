<?php

declare(strict_types=1);

namespace App\Service;

use App\Entity\Order;

/**
 * Standard order processor implementing the template method pattern.
 *
 * Contract test patterns:
 * - Class extends (extends): StandardOrderProcessor extends AbstractOrderProcessor
 * - Method override: getName() overridden from parent
 * - Abstract method implementation: preProcess() implemented
 */
final class StandardOrderProcessor extends AbstractOrderProcessor
{
    /**
     * Implementation of abstract preProcess method.
     *
     * Contract test: Child class implements parent abstract method.
     */
    protected function preProcess(Order $order): void
    {
        // Validate order before processing
        if (empty($order->status)) {
            $order->status = 'pending';
        }
    }

    /**
     * Override the doProcess method from parent.
     *
     * Contract test: Method override from non-abstract parent method.
     */
    protected function doProcess(Order $order): void
    {
        // Standard processing logic
        $order->status = 'processing';
    }

    /**
     * Override getName to return processor-specific name.
     *
     * Contract test: Public method override with same return type.
     */
    public function getName(): string
    {
        return 'standard';
    }
}
