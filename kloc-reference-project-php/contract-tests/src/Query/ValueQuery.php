<?php

declare(strict_types=1);

namespace ContractTests\Query;

use ContractTests\CallsData;
use PHPUnit\Framework\Assert;

/**
 * Fluent query builder for filtering values from calls.json.
 */
final class ValueQuery
{
    private CallsData $data;

    /** @var list<callable(array<string, mixed>): bool> */
    private array $filters = [];

    public function __construct(CallsData $data)
    {
        $this->data = $data;
    }

    /**
     * Filter by value kind.
     *
     * @param string $kind One of: parameter, local, literal, constant, result
     */
    public function kind(string $kind): self
    {
        $clone = clone $this;
        $clone->filters[] = static fn(array $v): bool => ($v['kind'] ?? '') === $kind;
        return $clone;
    }

    /**
     * Filter by exact symbol.
     */
    public function symbol(string $symbol): self
    {
        $clone = clone $this;
        $clone->filters[] = static fn(array $v): bool => ($v['symbol'] ?? '') === $symbol;
        return $clone;
    }

    /**
     * Filter by symbol containing substring.
     */
    public function symbolContains(string $substring): self
    {
        $clone = clone $this;
        $clone->filters[] = static fn(array $v): bool =>
            isset($v['symbol']) && str_contains($v['symbol'], $substring);
        return $clone;
    }

    /**
     * Filter by symbol pattern (supports * wildcard).
     *
     * @example ->symbolMatches('*OrderRepository#save().($order)')
     */
    public function symbolMatches(string $pattern): self
    {
        // Convert wildcard pattern to regex
        $regex = '/^' . str_replace(
            ['\\*', '\\?'],
            ['.*', '.'],
            preg_quote($pattern, '/')
        ) . '$/';

        $clone = clone $this;
        $clone->filters[] = static fn(array $v): bool =>
            isset($v['symbol']) && preg_match($regex, $v['symbol']) === 1;
        return $clone;
    }

    /**
     * Filter by caller pattern (method scope).
     *
     * @example ->inCaller('*OrderRepository#save().*')
     */
    public function inCaller(string $pattern): self
    {
        $regex = '/^' . str_replace(
            ['\\*', '\\?'],
            ['.*', '.'],
            preg_quote($pattern, '/')
        ) . '$/';

        $clone = clone $this;
        $clone->filters[] = static fn(array $v): bool =>
            isset($v['symbol']) && preg_match($regex, $v['symbol']) === 1;
        return $clone;
    }

    /**
     * Filter by file.
     */
    public function inFile(string $file): self
    {
        $clone = clone $this;
        $clone->filters[] = static function(array $v) use ($file): bool {
            $location = $v['location'] ?? null;
            if (!$location) {
                return false;
            }
            $valueFile = $location['file'] ?? '';
            return str_contains($valueFile, $file) || $valueFile === $file;
        };
        return $clone;
    }

    /**
     * Filter by line number.
     */
    public function atLine(int $line): self
    {
        $clone = clone $this;
        $clone->filters[] = static function(array $v) use ($line): bool {
            $location = $v['location'] ?? null;
            return $location && ($location['line'] ?? 0) === $line;
        };
        return $clone;
    }

    /**
     * Filter values that have source_call_id.
     */
    public function hasSourceCallId(): self
    {
        $clone = clone $this;
        $clone->filters[] = static fn(array $v): bool => isset($v['source_call_id']);
        return $clone;
    }

    /**
     * Filter values that have source_value_id.
     */
    public function hasSourceValueId(): self
    {
        $clone = clone $this;
        $clone->filters[] = static fn(array $v): bool => isset($v['source_value_id']);
        return $clone;
    }

    /**
     * Get all matching values.
     *
     * @return list<array<string, mixed>>
     */
    public function all(): array
    {
        $results = [];
        foreach ($this->data->values() as $value) {
            if ($this->matches($value)) {
                $results[] = $value;
            }
        }
        return $results;
    }

    /**
     * Get first matching value or null.
     *
     * @return array<string, mixed>|null
     */
    public function first(): ?array
    {
        foreach ($this->data->values() as $value) {
            if ($this->matches($value)) {
                return $value;
            }
        }
        return null;
    }

    /**
     * Assert exactly one match and return it.
     *
     * @return array<string, mixed>
     * @throws \PHPUnit\Framework\AssertionFailedError if count != 1
     */
    public function one(): array
    {
        $results = $this->all();
        $count = count($results);

        Assert::assertSame(
            1,
            $count,
            sprintf(
                'Expected exactly 1 value, found %d. Filters applied: %s',
                $count,
                $this->describeFilters()
            )
        );

        return $results[0];
    }

    /**
     * Get count of matching values.
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
                'Expected %d values, found %d. Filters: %s',
                $expected,
                $actual,
                $this->describeFilters()
            )
        );
        return $this;
    }

    /**
     * @param array<string, mixed> $value
     */
    private function matches(array $value): bool
    {
        foreach ($this->filters as $filter) {
            if (!$filter($value)) {
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
