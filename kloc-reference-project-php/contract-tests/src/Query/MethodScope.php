<?php

declare(strict_types=1);

namespace ContractTests\Query;

use ContractTests\CallsData;

/**
 * Scoped query helper for querying within a specific method.
 *
 * Usage:
 *   $scope = $this->inMethod('App\Repository\OrderRepository', 'save');
 *   $params = $scope->values()->kind('parameter')->all();
 *   $calls = $scope->calls()->kind('method')->all();
 */
final class MethodScope
{
    private CallsData $data;
    private string $class;
    private string $method;
    private string $scopePattern;

    public function __construct(CallsData $data, string $class, string $method)
    {
        $this->data = $data;
        // Normalize class: remove leading backslash, keep as-is for pattern matching
        $this->class = ltrim($class, '\\');
        $this->method = $method;

        // Build pattern: Class#method()
        // Convert backslashes to forward slashes for SCIP symbol format
        $scipClass = str_replace('\\', '/', $this->class);
        $this->scopePattern = $scipClass . '#' . $method . '()';
    }

    /**
     * Get a value query scoped to this method.
     *
     * Filters values whose symbol contains the method scope pattern.
     */
    public function values(): ValueQuery
    {
        return (new ValueQuery($this->data))
            ->symbolContains($this->scopePattern);
    }

    /**
     * Get a call query scoped to this method.
     *
     * Filters calls whose caller contains the method scope pattern.
     */
    public function calls(): CallQuery
    {
        return (new CallQuery($this->data))
            ->callerContains($this->scopePattern);
    }

    /**
     * Get the SCIP-style scope pattern for this method.
     */
    public function getScopePattern(): string
    {
        return $this->scopePattern;
    }

    /**
     * Get the class name.
     */
    public function getClass(): string
    {
        return $this->class;
    }

    /**
     * Get the method name.
     */
    public function getMethod(): string
    {
        return $this->method;
    }
}
