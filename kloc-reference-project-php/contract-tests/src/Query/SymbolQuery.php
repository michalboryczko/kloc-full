<?php

declare(strict_types=1);

namespace ContractTests\Query;

use ContractTests\ScipData;
use PHPUnit\Framework\Assert;

/**
 * Fluent query builder for filtering symbols from SCIP index.
 */
final class SymbolQuery
{
    private ScipData $data;

    /** @var list<callable(string, array<string, mixed>): bool> */
    private array $filters = [];

    public function __construct(ScipData $data)
    {
        $this->data = $data;
    }

    /**
     * Filter by exact symbol name.
     */
    public function symbol(string $symbol): self
    {
        $clone = clone $this;
        $clone->filters[] = static fn(string $name, array $info): bool => $name === $symbol;
        return $clone;
    }

    /**
     * Filter by symbol containing substring.
     */
    public function symbolContains(string $substring): self
    {
        $clone = clone $this;
        $clone->filters[] = static fn(string $name, array $info): bool =>
            str_contains($name, $substring);
        return $clone;
    }

    /**
     * Filter by symbol pattern (supports * and ? wildcards).
     *
     * @example ->symbolMatches('*OrderService#')
     * @example ->symbolMatches('scip-php composer . App/Service/*')
     */
    public function symbolMatches(string $pattern): self
    {
        $regex = '/^' . str_replace(
            ['\\*', '\\?'],
            ['.*', '.'],
            preg_quote($pattern, '/')
        ) . '$/';

        $clone = clone $this;
        $clone->filters[] = static fn(string $name, array $info): bool =>
            preg_match($regex, $name) === 1;
        return $clone;
    }

    /**
     * Filter symbols that have documentation.
     */
    public function hasDocumentation(): self
    {
        $clone = clone $this;
        $clone->filters[] = static fn(string $name, array $info): bool =>
            !empty($info['documentation']);
        return $clone;
    }

    /**
     * Filter symbols that have relationships (extends, implements, etc.).
     */
    public function hasRelationships(): self
    {
        $clone = clone $this;
        $clone->filters[] = static fn(string $name, array $info): bool =>
            !empty($info['relationships']);
        return $clone;
    }

    /**
     * Filter by relationship type.
     *
     * @param string $kind One of: 'implementation', 'type_definition', 'reference'
     */
    public function hasRelationshipKind(string $kind): self
    {
        $clone = clone $this;
        $clone->filters[] = static function(string $name, array $info) use ($kind): bool {
            $relationships = $info['relationships'] ?? [];
            foreach ($relationships as $rel) {
                // Check isImplementation, isTypeDefinition, isReference flags
                if ($kind === 'implementation' && !empty($rel['isImplementation'])) {
                    return true;
                }
                if ($kind === 'type_definition' && !empty($rel['isTypeDefinition'])) {
                    return true;
                }
                if ($kind === 'reference' && !empty($rel['isReference'])) {
                    return true;
                }
            }
            return false;
        };
        return $clone;
    }

    /**
     * Filter symbols that are classes (symbol ends with #).
     */
    public function isClass(): self
    {
        $clone = clone $this;
        $clone->filters[] = static fn(string $name, array $info): bool =>
            str_ends_with($name, '#');
        return $clone;
    }

    /**
     * Filter symbols that are methods (symbol contains #methodName()).
     */
    public function isMethod(): self
    {
        $clone = clone $this;
        $clone->filters[] = static fn(string $name, array $info): bool =>
            preg_match('/#[^#]+\(\)\.$/', $name) === 1;
        return $clone;
    }

    /**
     * Filter symbols that are properties (symbol contains #$propertyName.).
     */
    public function isProperty(): self
    {
        $clone = clone $this;
        $clone->filters[] = static fn(string $name, array $info): bool =>
            preg_match('/#\$[^.]+\.$/', $name) === 1;
        return $clone;
    }

    /**
     * Get all matching symbols.
     *
     * @return list<array{symbol: string, info: array<string, mixed>}>
     */
    public function all(): array
    {
        $results = [];
        foreach ($this->data->symbols() as $name => $info) {
            if ($this->matches($name, $info)) {
                $results[] = ['symbol' => $name, 'info' => $info];
            }
        }
        return $results;
    }

    /**
     * Get first matching symbol or null.
     *
     * @return array{symbol: string, info: array<string, mixed>}|null
     */
    public function first(): ?array
    {
        foreach ($this->data->symbols() as $name => $info) {
            if ($this->matches($name, $info)) {
                return ['symbol' => $name, 'info' => $info];
            }
        }
        return null;
    }

    /**
     * Assert exactly one match and return it.
     *
     * @return array{symbol: string, info: array<string, mixed>}
     */
    public function one(): array
    {
        $results = $this->all();
        $count = count($results);

        Assert::assertSame(
            1,
            $count,
            sprintf(
                'Expected exactly 1 symbol, found %d. Filters applied: %s',
                $count,
                $this->describeFilters()
            )
        );

        return $results[0];
    }

    /**
     * Get count of matching symbols.
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
                'Expected %d symbols, found %d. Filters: %s',
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
            $message ?: 'Expected at least one matching symbol'
        );
        return $this;
    }

    /**
     * Get occurrences for matching symbols.
     */
    public function occurrences(): OccurrenceQuery
    {
        $symbols = array_map(fn($s) => $s['symbol'], $this->all());
        return (new OccurrenceQuery($this->data))->forSymbols($symbols);
    }

    /**
     * Get relationships for matching symbols.
     *
     * @return list<array<string, mixed>>
     */
    public function relationships(): array
    {
        $relationships = [];
        foreach ($this->all() as $match) {
            $rels = $match['info']['relationships'] ?? [];
            foreach ($rels as $rel) {
                $relationships[] = $rel;
            }
        }
        return $relationships;
    }

    /**
     * @param array<string, mixed> $info
     */
    private function matches(string $name, array $info): bool
    {
        foreach ($this->filters as $filter) {
            if (!$filter($name, $info)) {
                return false;
            }
        }
        return true;
    }

    private function describeFilters(): string
    {
        return sprintf('%d filter(s)', count($this->filters));
    }
}
