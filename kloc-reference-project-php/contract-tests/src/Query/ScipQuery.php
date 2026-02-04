<?php

declare(strict_types=1);

namespace ContractTests\Query;

use ContractTests\ScipData;

/**
 * Entry point for querying SCIP index data.
 *
 * Provides access to symbol and occurrence queries.
 */
final class ScipQuery
{
    public function __construct(
        private readonly ScipData $data,
    ) {
    }

    /**
     * Start a query for symbols.
     */
    public function symbols(): SymbolQuery
    {
        return new SymbolQuery($this->data);
    }

    /**
     * Start a query for occurrences.
     */
    public function occurrences(): OccurrenceQuery
    {
        return new OccurrenceQuery($this->data);
    }

    /**
     * Query a specific symbol by name or pattern.
     *
     * @example ->symbol('App\Service\OrderService')
     * @example ->symbol('*OrderService#')
     */
    public function symbol(string $nameOrPattern): SymbolQuery
    {
        // Convert PHP class name to SCIP format if needed
        $normalized = $this->normalizeSymbolName($nameOrPattern);

        // If it contains wildcards, use pattern matching
        if (str_contains($normalized, '*') || str_contains($normalized, '?')) {
            return (new SymbolQuery($this->data))->symbolMatches($normalized);
        }

        // Try exact match first
        return (new SymbolQuery($this->data))->symbolContains($normalized);
    }

    /**
     * Get occurrences at a specific location.
     */
    public function occurrenceAt(string $file, int $line): OccurrenceQuery
    {
        return (new OccurrenceQuery($this->data))
            ->inFile($file)
            ->atLine($line);
    }

    /**
     * Get the underlying ScipData.
     */
    public function data(): ScipData
    {
        return $this->data;
    }

    /**
     * Normalize a PHP class name to SCIP symbol format.
     *
     * PHP: App\Service\OrderService
     * SCIP: App/Service/OrderService#
     */
    private function normalizeSymbolName(string $name): string
    {
        // If already contains SCIP-style separators, return as-is
        if (str_contains($name, '/') || str_contains($name, '#')) {
            return $name;
        }

        // Convert backslash to forward slash
        return str_replace('\\', '/', $name);
    }
}
