<?php

declare(strict_types=1);

namespace ContractTests\Assertions;

/**
 * Report of data integrity check results.
 */
final class IntegrityReport
{
    /**
     * @param list<string> $issues
     */
    public function __construct(
        public readonly int $duplicateParameterSymbols,
        public readonly int $duplicateLocalSymbols,
        public readonly int $orphanedReceiverIds,
        public readonly int $orphanedArgumentIds,
        public readonly int $orphanedSourceCallIds,
        public readonly int $orphanedSourceValueIds,
        public readonly int $missingResultValues,
        public readonly int $typeMismatches,
        public readonly array $issues,
    ) {
    }

    /**
     * Check if any issues were found.
     */
    public function hasIssues(): bool
    {
        return $this->duplicateParameterSymbols > 0
            || $this->duplicateLocalSymbols > 0
            || $this->orphanedReceiverIds > 0
            || $this->orphanedArgumentIds > 0
            || $this->orphanedSourceCallIds > 0
            || $this->orphanedSourceValueIds > 0
            || $this->missingResultValues > 0
            || $this->typeMismatches > 0;
    }

    /**
     * Get a summary of all issues.
     */
    public function summary(): string
    {
        $parts = [];

        if ($this->duplicateParameterSymbols > 0) {
            $parts[] = sprintf('%d duplicate parameter symbols', $this->duplicateParameterSymbols);
        }

        if ($this->duplicateLocalSymbols > 0) {
            $parts[] = sprintf('%d duplicate local symbols', $this->duplicateLocalSymbols);
        }

        if ($this->orphanedReceiverIds > 0) {
            $parts[] = sprintf('%d orphaned receiver_value_id', $this->orphanedReceiverIds);
        }

        if ($this->orphanedArgumentIds > 0) {
            $parts[] = sprintf('%d orphaned argument value_id', $this->orphanedArgumentIds);
        }

        if ($this->orphanedSourceCallIds > 0) {
            $parts[] = sprintf('%d orphaned source_call_id', $this->orphanedSourceCallIds);
        }

        if ($this->orphanedSourceValueIds > 0) {
            $parts[] = sprintf('%d orphaned source_value_id', $this->orphanedSourceValueIds);
        }

        if ($this->missingResultValues > 0) {
            $parts[] = sprintf('%d missing result values', $this->missingResultValues);
        }

        if ($this->typeMismatches > 0) {
            $parts[] = sprintf('%d type mismatches', $this->typeMismatches);
        }

        if (empty($parts)) {
            return 'No issues found';
        }

        return implode(', ', $parts);
    }

    /**
     * Get total issue count.
     */
    public function totalIssues(): int
    {
        return $this->duplicateParameterSymbols
            + $this->duplicateLocalSymbols
            + $this->orphanedReceiverIds
            + $this->orphanedArgumentIds
            + $this->orphanedSourceCallIds
            + $this->orphanedSourceValueIds
            + $this->missingResultValues
            + $this->typeMismatches;
    }
}
