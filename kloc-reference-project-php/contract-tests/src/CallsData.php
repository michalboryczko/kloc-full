<?php

declare(strict_types=1);

namespace ContractTests;

use JsonException;
use RuntimeException;

/**
 * Immutable wrapper around parsed calls.json with indexed lookups.
 *
 * This class loads the calls.json file and provides efficient access to
 * values and calls by their IDs, as well as raw array access for queries.
 */
final class CallsData
{
    private string $version;

    /** @var array<string, array<string, mixed>> */
    private array $valuesById = [];

    /** @var array<string, array<string, mixed>> */
    private array $callsById = [];

    /** @var list<array<string, mixed>> */
    private array $values = [];

    /** @var list<array<string, mixed>> */
    private array $calls = [];

    private function __construct()
    {
    }

    /**
     * Load calls data from a JSON file.
     *
     * @throws RuntimeException if file doesn't exist
     * @throws JsonException if JSON is invalid
     */
    public static function load(string $path): self
    {
        if (!file_exists($path)) {
            throw new RuntimeException(
                sprintf('calls.json not found at: %s', $path)
            );
        }

        $content = file_get_contents($path);
        if ($content === false) {
            throw new RuntimeException(
                sprintf('Failed to read calls.json at: %s', $path)
            );
        }

        $data = json_decode($content, true, 512, JSON_THROW_ON_ERROR);

        return self::fromArray($data);
    }

    /**
     * Create CallsData from a parsed array.
     *
     * @param array{version: string, values: list<array<string, mixed>>, calls: list<array<string, mixed>>} $data
     */
    public static function fromArray(array $data): self
    {
        $instance = new self();
        $instance->version = $data['version'] ?? '0.0';
        $instance->values = $data['values'] ?? [];
        $instance->calls = $data['calls'] ?? [];

        // Build indices
        foreach ($instance->values as $value) {
            if (isset($value['id'])) {
                $instance->valuesById[$value['id']] = $value;
            }
        }

        foreach ($instance->calls as $call) {
            if (isset($call['id'])) {
                $instance->callsById[$call['id']] = $call;
            }
        }

        return $instance;
    }

    /**
     * Get the schema version.
     */
    public function version(): string
    {
        return $this->version;
    }

    /**
     * Get all values as a list.
     *
     * @return list<array<string, mixed>>
     */
    public function values(): array
    {
        return $this->values;
    }

    /**
     * Get all calls as a list.
     *
     * @return list<array<string, mixed>>
     */
    public function calls(): array
    {
        return $this->calls;
    }

    /**
     * Get total count of values.
     */
    public function valueCount(): int
    {
        return count($this->values);
    }

    /**
     * Get total count of calls.
     */
    public function callCount(): int
    {
        return count($this->calls);
    }

    /**
     * Get a value by its ID.
     *
     * @return array<string, mixed>|null
     */
    public function getValueById(string $id): ?array
    {
        return $this->valuesById[$id] ?? null;
    }

    /**
     * Get a call by its ID.
     *
     * @return array<string, mixed>|null
     */
    public function getCallById(string $id): ?array
    {
        return $this->callsById[$id] ?? null;
    }

    /**
     * Check if a value exists with the given ID.
     */
    public function hasValue(string $id): bool
    {
        return isset($this->valuesById[$id]);
    }

    /**
     * Check if a call exists with the given ID.
     */
    public function hasCall(string $id): bool
    {
        return isset($this->callsById[$id]);
    }

    /**
     * Get all values indexed by ID.
     *
     * @return array<string, array<string, mixed>>
     */
    public function valuesById(): array
    {
        return $this->valuesById;
    }

    /**
     * Get all calls indexed by ID.
     *
     * @return array<string, array<string, mixed>>
     */
    public function callsById(): array
    {
        return $this->callsById;
    }
}
