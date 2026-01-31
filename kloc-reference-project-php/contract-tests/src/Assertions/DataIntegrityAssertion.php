<?php

declare(strict_types=1);

namespace ContractTests\Assertions;

use ContractTests\CallsData;
use PHPUnit\Framework\Assert;
use PHPUnit\Framework\TestCase;

/**
 * Assertion builder for verifying overall data integrity.
 *
 * Data integrity checks:
 * - No duplicate parameter symbols
 * - No orphaned receiver_value_id references
 * - No orphaned argument value_id references
 * - Every call has a corresponding result value
 * - Type consistency between calls and result values
 *
 * Usage:
 *   $this->assertIntegrity()
 *       ->noParameterDuplicates()
 *       ->allReceiverValueIdsExist()
 *       ->allArgumentValueIdsExist()
 *       ->verify();
 */
final class DataIntegrityAssertion
{
    private CallsData $data;
    private TestCase $testCase;

    private bool $checkParameterDuplicates = false;
    private bool $checkLocalDuplicates = false;
    private bool $checkReceiverIds = false;
    private bool $checkArgumentIds = false;
    private bool $checkSourceCallIds = false;
    private bool $checkSourceValueIds = false;
    private bool $checkResultValues = false;
    private bool $checkResultTypes = false;
    private bool $checkParameterLines = false;

    public function __construct(CallsData $data, TestCase $testCase)
    {
        $this->data = $data;
        $this->testCase = $testCase;
    }

    /**
     * Assert no parameter symbol appears more than once.
     */
    public function noParameterDuplicates(): self
    {
        $clone = clone $this;
        $clone->checkParameterDuplicates = true;
        return $clone;
    }

    /**
     * Assert no local symbol (with same @line) appears more than once.
     */
    public function noLocalDuplicatesPerLine(): self
    {
        $clone = clone $this;
        $clone->checkLocalDuplicates = true;
        return $clone;
    }

    /**
     * Assert all receiver_value_id point to existing values.
     */
    public function allReceiverValueIdsExist(): self
    {
        $clone = clone $this;
        $clone->checkReceiverIds = true;
        return $clone;
    }

    /**
     * Assert all argument value_id point to existing values.
     */
    public function allArgumentValueIdsExist(): self
    {
        $clone = clone $this;
        $clone->checkArgumentIds = true;
        return $clone;
    }

    /**
     * Assert all source_call_id point to existing calls.
     */
    public function allSourceCallIdsExist(): self
    {
        $clone = clone $this;
        $clone->checkSourceCallIds = true;
        return $clone;
    }

    /**
     * Assert all source_value_id point to existing values.
     */
    public function allSourceValueIdsExist(): self
    {
        $clone = clone $this;
        $clone->checkSourceValueIds = true;
        return $clone;
    }

    /**
     * Assert every call has a corresponding result value with same ID.
     */
    public function everyCallHasResultValue(): self
    {
        $clone = clone $this;
        $clone->checkResultValues = true;
        return $clone;
    }

    /**
     * Assert result value types match their call's return_type.
     */
    public function resultValueTypesMatch(): self
    {
        $clone = clone $this;
        $clone->checkResultTypes = true;
        return $clone;
    }

    /**
     * Assert parameter values only appear at method signature lines.
     */
    public function parameterValuesAtSignatureOnly(): self
    {
        $clone = clone $this;
        $clone->checkParameterLines = true;
        return $clone;
    }

    /**
     * Run all configured checks.
     *
     * @throws \PHPUnit\Framework\AssertionFailedError on first failure
     */
    public function verify(): void
    {
        if ($this->checkParameterDuplicates) {
            $this->verifyNoParameterDuplicates();
        }

        if ($this->checkLocalDuplicates) {
            $this->verifyNoLocalDuplicates();
        }

        if ($this->checkReceiverIds) {
            $this->verifyReceiverIdsExist();
        }

        if ($this->checkArgumentIds) {
            $this->verifyArgumentIdsExist();
        }

        if ($this->checkSourceCallIds) {
            $this->verifySourceCallIdsExist();
        }

        if ($this->checkSourceValueIds) {
            $this->verifySourceValueIdsExist();
        }

        if ($this->checkResultValues) {
            $this->verifyResultValuesExist();
        }

        if ($this->checkResultTypes) {
            $this->verifyResultTypesMatch();
        }
    }

    /**
     * Run all configured checks and return report (no throw).
     */
    public function report(): IntegrityReport
    {
        $issues = [];

        if ($this->checkParameterDuplicates) {
            $duplicates = $this->findParameterDuplicates();
            if (!empty($duplicates)) {
                foreach ($duplicates as $symbol => $ids) {
                    $issues[] = sprintf(
                        'Duplicate parameter symbol: %s (ids: %s)',
                        $symbol,
                        implode(', ', $ids)
                    );
                }
            }
        }

        if ($this->checkReceiverIds) {
            $orphaned = $this->findOrphanedReceiverIds();
            foreach ($orphaned as $callId => $receiverId) {
                $issues[] = sprintf(
                    'Orphaned receiver_value_id: call %s references non-existent value %s',
                    $callId,
                    $receiverId
                );
            }
        }

        if ($this->checkArgumentIds) {
            $orphaned = $this->findOrphanedArgumentIds();
            foreach ($orphaned as $info) {
                $issues[] = sprintf(
                    'Orphaned argument value_id: call %s arg %d references non-existent value %s',
                    $info['call_id'],
                    $info['position'],
                    $info['value_id']
                );
            }
        }

        if ($this->checkSourceCallIds) {
            $orphaned = $this->findOrphanedSourceCallIds();
            foreach ($orphaned as $valueId => $sourceCallId) {
                $issues[] = sprintf(
                    'Orphaned source_call_id: value %s references non-existent call %s',
                    $valueId,
                    $sourceCallId
                );
            }
        }

        if ($this->checkSourceValueIds) {
            $orphaned = $this->findOrphanedSourceValueIds();
            foreach ($orphaned as $valueId => $sourceValueId) {
                $issues[] = sprintf(
                    'Orphaned source_value_id: value %s references non-existent value %s',
                    $valueId,
                    $sourceValueId
                );
            }
        }

        return new IntegrityReport(
            duplicateParameterSymbols: count($this->findParameterDuplicates()),
            duplicateLocalSymbols: $this->checkLocalDuplicates ? count($this->findLocalDuplicates()) : 0,
            orphanedReceiverIds: count($this->findOrphanedReceiverIds()),
            orphanedArgumentIds: count($this->findOrphanedArgumentIds()),
            orphanedSourceCallIds: count($this->findOrphanedSourceCallIds()),
            orphanedSourceValueIds: count($this->findOrphanedSourceValueIds()),
            missingResultValues: $this->checkResultValues ? $this->countMissingResultValues() : 0,
            typeMismatches: $this->checkResultTypes ? $this->countTypeMismatches() : 0,
            issues: $issues
        );
    }

    private function verifyNoParameterDuplicates(): void
    {
        $duplicates = $this->findParameterDuplicates();
        Assert::assertEmpty(
            $duplicates,
            sprintf(
                'Found %d duplicate parameter symbols: %s',
                count($duplicates),
                implode(', ', array_keys($duplicates))
            )
        );
    }

    private function verifyNoLocalDuplicates(): void
    {
        $duplicates = $this->findLocalDuplicates();
        Assert::assertEmpty(
            $duplicates,
            sprintf(
                'Found %d duplicate local variable symbols: %s',
                count($duplicates),
                implode(', ', array_keys($duplicates))
            )
        );
    }

    private function verifyReceiverIdsExist(): void
    {
        $orphaned = $this->findOrphanedReceiverIds();
        Assert::assertEmpty(
            $orphaned,
            sprintf(
                'Found %d calls with orphaned receiver_value_id references: %s',
                count($orphaned),
                implode(', ', array_keys($orphaned))
            )
        );
    }

    private function verifyArgumentIdsExist(): void
    {
        $orphaned = $this->findOrphanedArgumentIds();
        Assert::assertEmpty(
            $orphaned,
            sprintf(
                'Found %d arguments with orphaned value_id references',
                count($orphaned)
            )
        );
    }

    private function verifySourceCallIdsExist(): void
    {
        $orphaned = $this->findOrphanedSourceCallIds();
        Assert::assertEmpty(
            $orphaned,
            sprintf(
                'Found %d values with orphaned source_call_id references: %s',
                count($orphaned),
                implode(', ', array_keys($orphaned))
            )
        );
    }

    private function verifySourceValueIdsExist(): void
    {
        $orphaned = $this->findOrphanedSourceValueIds();
        Assert::assertEmpty(
            $orphaned,
            sprintf(
                'Found %d values with orphaned source_value_id references: %s',
                count($orphaned),
                implode(', ', array_keys($orphaned))
            )
        );
    }

    private function verifyResultValuesExist(): void
    {
        $missing = $this->countMissingResultValues();
        Assert::assertEquals(
            0,
            $missing,
            sprintf('Found %d calls without corresponding result values', $missing)
        );
    }

    private function verifyResultTypesMatch(): void
    {
        $mismatches = $this->countTypeMismatches();
        Assert::assertEquals(
            0,
            $mismatches,
            sprintf('Found %d type mismatches between calls and result values', $mismatches)
        );
    }

    /**
     * @return array<string, list<string>>
     */
    private function findParameterDuplicates(): array
    {
        $symbols = [];
        foreach ($this->data->values() as $value) {
            if (($value['kind'] ?? '') !== 'parameter') {
                continue;
            }
            $symbol = $value['symbol'] ?? '';
            if ($symbol === '') {
                continue;
            }
            $symbols[$symbol][] = $value['id'];
        }

        return array_filter($symbols, fn($ids) => count($ids) > 1);
    }

    /**
     * @return array<string, list<string>>
     */
    private function findLocalDuplicates(): array
    {
        $symbols = [];
        foreach ($this->data->values() as $value) {
            if (($value['kind'] ?? '') !== 'local') {
                continue;
            }
            $symbol = $value['symbol'] ?? '';
            if ($symbol === '') {
                continue;
            }
            // Local symbols include @line, so same symbol = same declaration = duplicate
            $symbols[$symbol][] = $value['id'];
        }

        return array_filter($symbols, fn($ids) => count($ids) > 1);
    }

    /**
     * @return array<string, string>
     */
    private function findOrphanedReceiverIds(): array
    {
        $orphaned = [];
        foreach ($this->data->calls() as $call) {
            $receiverId = $call['receiver_value_id'] ?? null;
            if ($receiverId !== null && !$this->data->hasValue($receiverId)) {
                $orphaned[$call['id']] = $receiverId;
            }
        }
        return $orphaned;
    }

    /**
     * @return list<array{call_id: string, position: int, value_id: string}>
     */
    private function findOrphanedArgumentIds(): array
    {
        $orphaned = [];
        foreach ($this->data->calls() as $call) {
            $arguments = $call['arguments'] ?? [];
            foreach ($arguments as $arg) {
                $valueId = $arg['value_id'] ?? null;
                if ($valueId !== null && !$this->data->hasValue($valueId)) {
                    $orphaned[] = [
                        'call_id' => $call['id'],
                        'position' => $arg['position'] ?? -1,
                        'value_id' => $valueId,
                    ];
                }
            }
        }
        return $orphaned;
    }

    /**
     * @return array<string, string>
     */
    private function findOrphanedSourceCallIds(): array
    {
        $orphaned = [];
        foreach ($this->data->values() as $value) {
            $sourceCallId = $value['source_call_id'] ?? null;
            if ($sourceCallId !== null && !$this->data->hasCall($sourceCallId)) {
                $orphaned[$value['id']] = $sourceCallId;
            }
        }
        return $orphaned;
    }

    /**
     * @return array<string, string>
     */
    private function findOrphanedSourceValueIds(): array
    {
        $orphaned = [];
        foreach ($this->data->values() as $value) {
            $sourceValueId = $value['source_value_id'] ?? null;
            if ($sourceValueId !== null && !$this->data->hasValue($sourceValueId)) {
                $orphaned[$value['id']] = $sourceValueId;
            }
        }
        return $orphaned;
    }

    private function countMissingResultValues(): int
    {
        $missing = 0;
        foreach ($this->data->calls() as $call) {
            // Every call should have a result value with the same ID
            if (!$this->data->hasValue($call['id'])) {
                $missing++;
            }
        }
        return $missing;
    }

    private function countTypeMismatches(): int
    {
        $mismatches = 0;
        foreach ($this->data->calls() as $call) {
            $returnType = $call['return_type'] ?? null;
            if ($returnType === null) {
                continue;
            }

            $resultValue = $this->data->getValueById($call['id']);
            if ($resultValue === null) {
                continue;
            }

            $valueType = $resultValue['type'] ?? null;
            if ($valueType !== null && $valueType !== $returnType) {
                $mismatches++;
            }
        }
        return $mismatches;
    }
}
