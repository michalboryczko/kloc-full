<?php

declare(strict_types=1);

namespace App\Service;

use App\Entity\Order;

/**
 * Abstract base class for order processing.
 *
 * Contract test patterns:
 * - Class extends (extends): StandardOrderProcessor extends AbstractOrderProcessor
 * - Method override: process() overridden in child class
 * - Abstract method: preProcess() must be implemented
 */
abstract class AbstractOrderProcessor
{
    /**
     * Template method for processing orders.
     *
     * Contract test: Method call on abstract base class reference.
     */
    public function process(Order $order): Order
    {
        $this->preProcess($order);
        $this->doProcess($order);
        $this->postProcess($order);

        return $order;
    }

    /**
     * Hook for pre-processing.
     *
     * Contract test: Abstract method must be implemented by subclass.
     */
    abstract protected function preProcess(Order $order): void;

    /**
     * Core processing logic.
     *
     * Contract test: Protected method can be overridden.
     */
    protected function doProcess(Order $order): void
    {
        // Default implementation - can be overridden
    }

    /**
     * Hook for post-processing.
     *
     * Contract test: Concrete method in abstract class.
     */
    protected function postProcess(Order $order): void
    {
        // Default implementation
    }

    /**
     * Get processor name.
     *
     * Contract test: Method with return type that can be overridden.
     */
    public function getName(): string
    {
        return 'abstract';
    }
}
