<?php

declare(strict_types=1);

namespace ContractTests\Query;

use ContractTests\ScipData;
use PHPUnit\Framework\Assert;

/**
 * Fluent query builder for filtering occurrences from SCIP index.
 *
 * SCIP occurrence structure:
 * - range: [startLine, startCol, endLine, endCol] or [startLine, startCol, endCol]
 * - symbol: SCIP symbol string
 * - symbolRoles: bitmask (1=Definition, 2=Reference, etc.)
 * - overrideDocumentation: optional docs
 * - syntaxKind: identifier syntax kind
 * - diagnostics: optional diagnostics
 */
final class OccurrenceQuery
{
    /**
     * SCIP SymbolRole bit flags
     */
    private const ROLE_DEFINITION = 1;
    private const ROLE_IMPORT = 2;
    private const ROLE_WRITE_ACCESS = 4;
    private const ROLE_READ_ACCESS = 8;
    private const ROLE_GENERATED = 16;
    private const ROLE_TEST = 32;
    private const ROLE_FORWARD_DEFINITION = 64;

    private ScipData $data;

    /** @var list<callable(array<string, mixed>): bool> */
    private array $filters = [];

    /** @var list<string>|null Specific symbols to search, or null for all */
    private ?array $symbolFilter = null;

    /** @var list<string>|null Specific files to search, or null for all */
    private ?array $fileFilter = null;

    public function __construct(ScipData $data)
    {
        $this->data = $data;
    }

    /**
     * Filter occurrences for specific symbols.
     *
     * @param list<string> $symbols
     */
    public function forSymbols(array $symbols): self
    {
        $clone = clone $this;
        $clone->symbolFilter = $symbols;
        return $clone;
    }

    /**
     * Filter by exact symbol.
     */
    public function forSymbol(string $symbol): self
    {
        $clone = clone $this;
        $clone->symbolFilter = [$symbol];
        return $clone;
    }

    /**
     * Filter by symbol containing substring.
     */
    public function symbolContains(string $substring): self
    {
        $clone = clone $this;
        $clone->filters[] = static fn(array $o): bool =>
            isset($o['symbol']) && str_contains($o['symbol'], $substring);
        return $clone;
    }

    /**
     * Filter by symbol pattern (supports * and ? wildcards).
     */
    public function symbolMatches(string $pattern): self
    {
        $regex = '/^' . str_replace(
            ['\\*', '\\?'],
            ['.*', '.'],
            preg_quote($pattern, '/')
        ) . '$/';

        $clone = clone $this;
        $clone->filters[] = static fn(array $o): bool =>
            isset($o['symbol']) && preg_match($regex, $o['symbol']) === 1;
        return $clone;
    }

    /**
     * Filter by file path (exact or substring match).
     */
    public function inFile(string $file): self
    {
        $clone = clone $this;
        $clone->fileFilter = [$file];
        $clone->filters[] = static function(array $o) use ($file): bool {
            $occFile = $o['_file'] ?? '';
            return str_contains($occFile, $file) || $occFile === $file;
        };
        return $clone;
    }

    /**
     * Filter by line number.
     *
     * SCIP range format: [startLine, startCol, endCol] or [startLine, startCol, endLine, endCol]
     * Line numbers are 0-indexed in SCIP.
     */
    public function atLine(int $line): self
    {
        // Convert 1-indexed line to 0-indexed
        $scipLine = $line - 1;

        $clone = clone $this;
        $clone->filters[] = static function(array $o) use ($scipLine): bool {
            $range = $o['range'] ?? [];
            if (empty($range)) {
                return false;
            }
            return $range[0] === $scipLine;
        };
        return $clone;
    }

    /**
     * Filter by line range.
     */
    public function betweenLines(int $startLine, int $endLine): self
    {
        // Convert 1-indexed to 0-indexed
        $scipStart = $startLine - 1;
        $scipEnd = $endLine - 1;

        $clone = clone $this;
        $clone->filters[] = static function(array $o) use ($scipStart, $scipEnd): bool {
            $range = $o['range'] ?? [];
            if (empty($range)) {
                return false;
            }
            $occLine = $range[0];
            return $occLine >= $scipStart && $occLine <= $scipEnd;
        };
        return $clone;
    }

    /**
     * Filter occurrences that are definitions.
     */
    public function isDefinition(): self
    {
        $clone = clone $this;
        $clone->filters[] = static fn(array $o): bool =>
            (($o['symbolRoles'] ?? $o['symbol_roles'] ?? 0) & self::ROLE_DEFINITION) !== 0;
        return $clone;
    }

    /**
     * Filter occurrences that are references (not definitions).
     */
    public function isReference(): self
    {
        $clone = clone $this;
        $clone->filters[] = static fn(array $o): bool =>
            (($o['symbolRoles'] ?? $o['symbol_roles'] ?? 0) & self::ROLE_DEFINITION) === 0;
        return $clone;
    }

    /**
     * Filter occurrences that are imports.
     */
    public function isImport(): self
    {
        $clone = clone $this;
        $clone->filters[] = static fn(array $o): bool =>
            (($o['symbolRoles'] ?? $o['symbol_roles'] ?? 0) & self::ROLE_IMPORT) !== 0;
        return $clone;
    }

    /**
     * Filter occurrences that are write accesses.
     */
    public function isWriteAccess(): self
    {
        $clone = clone $this;
        $clone->filters[] = static fn(array $o): bool =>
            (($o['symbolRoles'] ?? $o['symbol_roles'] ?? 0) & self::ROLE_WRITE_ACCESS) !== 0;
        return $clone;
    }

    /**
     * Filter occurrences that are read accesses.
     */
    public function isReadAccess(): self
    {
        $clone = clone $this;
        $clone->filters[] = static fn(array $o): bool =>
            (($o['symbolRoles'] ?? $o['symbol_roles'] ?? 0) & self::ROLE_READ_ACCESS) !== 0;
        return $clone;
    }

    /**
     * Get all matching occurrences.
     *
     * @return list<array<string, mixed>>
     */
    public function all(): array
    {
        $results = [];

        // If we have a symbol filter, only look at occurrences for those symbols
        if ($this->symbolFilter !== null) {
            foreach ($this->symbolFilter as $symbol) {
                foreach ($this->data->occurrences($symbol) as $occurrence) {
                    if ($this->matches($occurrence)) {
                        $results[] = $occurrence;
                    }
                }
            }
            return $results;
        }

        // If we have a file filter, search by file
        if ($this->fileFilter !== null) {
            foreach ($this->fileFilter as $file) {
                // Try exact match first
                $occurrences = $this->data->occurrencesInFile($file);

                // If no exact match, search all files containing the substring
                if (empty($occurrences)) {
                    foreach ($this->data->filePaths() as $path) {
                        if (str_contains($path, $file)) {
                            $occurrences = array_merge($occurrences, $this->data->occurrencesInFile($path));
                        }
                    }
                }

                foreach ($occurrences as $occurrence) {
                    if ($this->matches($occurrence)) {
                        $results[] = $occurrence;
                    }
                }
            }
            return $results;
        }

        // Otherwise, search all occurrences (expensive)
        foreach ($this->data->documents() as $document) {
            $relativePath = $document['relativePath'] ?? $document['relative_path'] ?? '';
            foreach ($document['occurrences'] ?? [] as $occurrence) {
                $occWithFile = $occurrence;
                $occWithFile['_file'] = $relativePath;
                if ($this->matches($occWithFile)) {
                    $results[] = $occWithFile;
                }
            }
        }

        return $results;
    }

    /**
     * Get first matching occurrence or null.
     *
     * @return array<string, mixed>|null
     */
    public function first(): ?array
    {
        $all = $this->all();
        return $all[0] ?? null;
    }

    /**
     * Assert exactly one match and return it.
     *
     * @return array<string, mixed>
     */
    public function one(): array
    {
        $results = $this->all();
        $count = count($results);

        Assert::assertSame(
            1,
            $count,
            sprintf(
                'Expected exactly 1 occurrence, found %d. Filters applied: %s',
                $count,
                $this->describeFilters()
            )
        );

        return $results[0];
    }

    /**
     * Get count of matching occurrences.
     */
    public function count(): int
    {
        return count($this->all());
    }

    /**
     * Assert count equals expected.
     */
    public function assertCount(int $expected, string $message = ''): self
    {
        $actual = $this->count();
        Assert::assertSame(
            $expected,
            $actual,
            $message ?: sprintf(
                'Expected %d occurrences, found %d. Filters: %s',
                $expected,
                $actual,
                $this->describeFilters()
            )
        );
        return $this;
    }

    /**
     * Assert at least one match exists.
     */
    public function assertExists(string $message = ''): self
    {
        Assert::assertGreaterThan(
            0,
            $this->count(),
            $message ?: 'Expected at least one matching occurrence'
        );
        return $this;
    }

    /**
     * Assert no matches exist.
     */
    public function assertEmpty(string $message = ''): self
    {
        Assert::assertSame(
            0,
            $this->count(),
            $message ?: 'Expected no matching occurrences'
        );
        return $this;
    }

    /**
     * Get roles as human-readable strings for an occurrence.
     *
     * @param array<string, mixed> $occurrence
     * @return list<string>
     */
    public static function getRoleNames(array $occurrence): array
    {
        $roles = [];
        $symbolRoles = $occurrence['symbolRoles'] ?? $occurrence['symbol_roles'] ?? 0;

        if (($symbolRoles & self::ROLE_DEFINITION) !== 0) {
            $roles[] = 'Definition';
        }
        if (($symbolRoles & self::ROLE_IMPORT) !== 0) {
            $roles[] = 'Import';
        }
        if (($symbolRoles & self::ROLE_WRITE_ACCESS) !== 0) {
            $roles[] = 'WriteAccess';
        }
        if (($symbolRoles & self::ROLE_READ_ACCESS) !== 0) {
            $roles[] = 'ReadAccess';
        }
        if (($symbolRoles & self::ROLE_GENERATED) !== 0) {
            $roles[] = 'Generated';
        }
        if (($symbolRoles & self::ROLE_TEST) !== 0) {
            $roles[] = 'Test';
        }
        if (($symbolRoles & self::ROLE_FORWARD_DEFINITION) !== 0) {
            $roles[] = 'ForwardDefinition';
        }

        // If no definition role, it's a reference
        if (($symbolRoles & self::ROLE_DEFINITION) === 0) {
            $roles[] = 'Reference';
        }

        return $roles;
    }

    /**
     * @param array<string, mixed> $occurrence
     */
    private function matches(array $occurrence): bool
    {
        foreach ($this->filters as $filter) {
            if (!$filter($occurrence)) {
                return false;
            }
        }
        return true;
    }

    private function describeFilters(): string
    {
        $desc = sprintf('%d filter(s)', count($this->filters));
        if ($this->symbolFilter !== null) {
            $desc .= sprintf(', symbols: [%s]', implode(', ', $this->symbolFilter));
        }
        if ($this->fileFilter !== null) {
            $desc .= sprintf(', files: [%s]', implode(', ', $this->fileFilter));
        }
        return $desc;
    }
}
