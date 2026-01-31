<?php

declare(strict_types=1);

namespace ContractTests\Assertions;

/**
 * Result of chain integrity verification.
 */
final class ChainVerificationResult
{
    /**
     * @param list<array{type: string, data: array<string, mixed>}> $steps
     * @param array<string, mixed> $rootValue
     * @param array<string, mixed> $finalValue
     */
    public function __construct(
        private readonly array $steps,
        private readonly array $rootValue,
        private readonly array $finalValue,
        private readonly int $stepCount,
        private readonly ?string $finalType,
    ) {
    }

    /**
     * Get number of steps in the chain.
     */
    public function stepCount(): int
    {
        return $this->stepCount;
    }

    /**
     * Get the final result type.
     */
    public function finalType(): ?string
    {
        return $this->finalType;
    }

    /**
     * Get all steps as array.
     *
     * @return list<array{type: string, data: array<string, mixed>}>
     */
    public function steps(): array
    {
        return $this->steps;
    }

    /**
     * Get the root value (starting variable).
     *
     * @return array<string, mixed>
     */
    public function rootValue(): array
    {
        return $this->rootValue;
    }

    /**
     * Get the final result value.
     *
     * @return array<string, mixed>
     */
    public function finalValue(): array
    {
        return $this->finalValue;
    }

    /**
     * Get a step by index (0-based).
     *
     * @return array{type: string, data: array<string, mixed>}|null
     */
    public function getStep(int $index): ?array
    {
        return $this->steps[$index] ?? null;
    }

    /**
     * Get only the call steps.
     *
     * @return list<array<string, mixed>>
     */
    public function callSteps(): array
    {
        return array_values(array_filter(
            array_map(
                fn($s) => $s['type'] === 'call' ? $s['data'] : null,
                $this->steps
            )
        ));
    }

    /**
     * Get only the value steps.
     *
     * @return list<array<string, mixed>>
     */
    public function valueSteps(): array
    {
        return array_values(array_filter(
            array_map(
                fn($s) => $s['type'] === 'value' ? $s['data'] : null,
                $this->steps
            )
        ));
    }
}
