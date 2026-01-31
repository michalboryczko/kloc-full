<?php

declare(strict_types=1);

namespace ContractTests\Query;

use ContractTests\CallsData;
use PHPUnit\Framework\Assert;

/**
 * Fluent query builder for filtering calls from calls.json.
 */
final class CallQuery
{
    private CallsData $data;

    /** @var list<callable(array<string, mixed>): bool> */
    private array $filters = [];

    public function __construct(CallsData $data)
    {
        $this->data = $data;
    }

    /**
     * Filter by call kind.
     *
     * @param string $kind One of: method, method_static, method_nullsafe,
     *                     function, constructor, access, access_static,
     *                     access_nullsafe, access_array, coalesce, ternary, match
     */
    public function kind(string $kind): self
    {
        $clone = clone $this;
        $clone->filters[] = static fn(array $c): bool => ($c['kind'] ?? '') === $kind;
        return $clone;
    }

    /**
     * Filter by kind_type category.
     *
     * @param string $kindType One of: invocation, access, operator
     */
    public function kindType(string $kindType): self
    {
        $clone = clone $this;
        $clone->filters[] = static fn(array $c): bool => ($c['kind_type'] ?? '') === $kindType;
        return $clone;
    }

    /**
     * Filter by callee pattern (what's being called).
     *
     * @example ->calleeMatches('*Order#$customerEmail.')
     */
    public function calleeMatches(string $pattern): self
    {
        $regex = '/^' . str_replace(
            ['\\*', '\\?'],
            ['.*', '.'],
            preg_quote($pattern, '/')
        ) . '$/';

        $clone = clone $this;
        $clone->filters[] = static fn(array $c): bool =>
            isset($c['callee']) && preg_match($regex, $c['callee']) === 1;
        return $clone;
    }

    /**
     * Filter by callee containing substring.
     */
    public function calleeContains(string $substring): self
    {
        $clone = clone $this;
        $clone->filters[] = static fn(array $c): bool =>
            isset($c['callee']) && str_contains($c['callee'], $substring);
        return $clone;
    }

    /**
     * Filter by caller pattern (where the call is made).
     *
     * @example ->callerMatches('*OrderRepository#save().*')
     */
    public function callerMatches(string $pattern): self
    {
        $regex = '/^' . str_replace(
            ['\\*', '\\?'],
            ['.*', '.'],
            preg_quote($pattern, '/')
        ) . '$/';

        $clone = clone $this;
        $clone->filters[] = static fn(array $c): bool =>
            isset($c['caller']) && preg_match($regex, $c['caller']) === 1;
        return $clone;
    }

    /**
     * Filter by caller containing substring.
     */
    public function callerContains(string $substring): self
    {
        $clone = clone $this;
        $clone->filters[] = static fn(array $c): bool =>
            isset($c['caller']) && str_contains($c['caller'], $substring);
        return $clone;
    }

    /**
     * Filter by specific receiver_value_id.
     */
    public function withReceiverValueId(string $valueId): self
    {
        $clone = clone $this;
        $clone->filters[] = static fn(array $c): bool =>
            ($c['receiver_value_id'] ?? '') === $valueId;
        return $clone;
    }

    /**
     * Filter calls that have a receiver_value_id.
     */
    public function hasReceiver(): self
    {
        $clone = clone $this;
        $clone->filters[] = static fn(array $c): bool => isset($c['receiver_value_id']);
        return $clone;
    }

    /**
     * Filter by file.
     */
    public function inFile(string $file): self
    {
        $clone = clone $this;
        $clone->filters[] = static function(array $c) use ($file): bool {
            $location = $c['location'] ?? null;
            if (!$location) {
                return false;
            }
            $callFile = $location['file'] ?? '';
            return str_contains($callFile, $file) || $callFile === $file;
        };
        return $clone;
    }

    /**
     * Filter by line number.
     */
    public function atLine(int $line): self
    {
        $clone = clone $this;
        $clone->filters[] = static function(array $c) use ($line): bool {
            $location = $c['location'] ?? null;
            return $location && ($location['line'] ?? 0) === $line;
        };
        return $clone;
    }

    /**
     * Scope to calls within a specific method.
     */
    public function inMethod(string $class, string $method): self
    {
        // Normalize class name: remove leading backslash, convert to SCIP format
        $class = ltrim($class, '\\');
        $class = str_replace('\\', '/', $class);

        $clone = clone $this;
        $clone->filters[] = static fn(array $c): bool =>
            isset($c['caller']) &&
            str_contains($c['caller'], $class . '#' . $method . '()');
        return $clone;
    }

    /**
     * Get all matching calls.
     *
     * @return list<array<string, mixed>>
     */
    public function all(): array
    {
        $results = [];
        foreach ($this->data->calls() as $call) {
            if ($this->matches($call)) {
                $results[] = $call;
            }
        }
        return $results;
    }

    /**
     * Get first matching call or null.
     *
     * @return array<string, mixed>|null
     */
    public function first(): ?array
    {
        foreach ($this->data->calls() as $call) {
            if ($this->matches($call)) {
                return $call;
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
                'Expected exactly 1 call, found %d. Filters applied: %s',
                $count,
                $this->describeFilters()
            )
        );

        return $results[0];
    }

    /**
     * Get count of matching calls.
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
                'Expected %d calls, found %d. Filters: %s',
                $expected,
                $actual,
                $this->describeFilters()
            )
        );
        return $this;
    }

    /**
     * Assert all matching calls share the same receiver_value_id.
     */
    public function assertAllShareReceiver(string $message = ''): self
    {
        $results = $this->all();

        if (count($results) < 2) {
            return $this;
        }

        $receiverIds = array_unique(
            array_filter(
                array_map(fn($c) => $c['receiver_value_id'] ?? null, $results)
            )
        );

        Assert::assertCount(
            1,
            $receiverIds,
            $message ?: sprintf(
                'Expected all calls to share the same receiver_value_id, found %d different: %s',
                count($receiverIds),
                implode(', ', $receiverIds)
            )
        );

        return $this;
    }

    /**
     * @param array<string, mixed> $call
     */
    private function matches(array $call): bool
    {
        foreach ($this->filters as $filter) {
            if (!$filter($call)) {
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
