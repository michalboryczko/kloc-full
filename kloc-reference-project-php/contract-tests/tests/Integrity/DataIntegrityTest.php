<?php

declare(strict_types=1);

namespace ContractTests\Tests\Integrity;

use ContractTests\CallsContractTestCase;

/**
 * Tests for overall data integrity of the calls.json output.
 *
 * These tests verify the structural correctness of the index:
 * - No duplicate entries
 * - No orphaned references
 * - Complete result values
 * - Type consistency
 */
class DataIntegrityTest extends CallsContractTestCase
{
    /**
     * Test: No duplicate parameter symbols
     *
     * Each parameter should have exactly one value entry.
     * Duplicates indicate an indexer bug.
     */
    public function testNoParameterDuplicates(): void
    {
        $this->assertIntegrity()
            ->noParameterDuplicates()
            ->verify();
    }

    /**
     * Test: No duplicate local variable symbols per line
     *
     * Each local variable assignment at a specific line should have
     * exactly one value entry. Different lines can have the same variable
     * name (reassignment creates new symbol with different @line).
     */
    public function testNoLocalDuplicates(): void
    {
        $this->assertIntegrity()
            ->noLocalDuplicatesPerLine()
            ->verify();
    }

    /**
     * Test: All receiver_value_id references exist
     *
     * Every call's receiver_value_id should point to an existing value.
     * Orphaned references indicate missing value entries.
     */
    public function testAllReceiverIdsExist(): void
    {
        $this->assertIntegrity()
            ->allReceiverValueIdsExist()
            ->verify();
    }

    /**
     * Test: All argument value_id references exist
     *
     * Every argument's value_id should point to an existing value.
     * Orphaned references indicate missing value entries.
     */
    public function testAllArgumentIdsExist(): void
    {
        $this->assertIntegrity()
            ->allArgumentValueIdsExist()
            ->verify();
    }

    /**
     * Test: All source_call_id references exist
     *
     * Every value's source_call_id should point to an existing call.
     * Orphaned references indicate missing call entries.
     */
    public function testAllSourceCallIdsExist(): void
    {
        $this->assertIntegrity()
            ->allSourceCallIdsExist()
            ->verify();
    }

    /**
     * Test: All source_value_id references exist
     *
     * Every value's source_value_id should point to an existing value.
     * Orphaned references indicate missing value entries.
     */
    public function testAllSourceValueIdsExist(): void
    {
        $this->assertIntegrity()
            ->allSourceValueIdsExist()
            ->verify();
    }

    /**
     * Test: Every call has a corresponding result value
     *
     * Each call should have a result value with the same ID.
     * Missing result values break chain integrity.
     */
    public function testEveryCallHasResultValue(): void
    {
        $this->assertIntegrity()
            ->everyCallHasResultValue()
            ->verify();
    }

    /**
     * Test: Result value types match call return types
     *
     * The type field of result values should match the return_type
     * of their source call.
     */
    public function testResultTypesMatch(): void
    {
        $this->assertIntegrity()
            ->resultValueTypesMatch()
            ->verify();
    }

    /**
     * Test: Generate full integrity report
     *
     * Get a complete report of all integrity issues for debugging.
     */
    public function testFullIntegrityReport(): void
    {
        $report = $this->integrityReport();

        // For now, just verify we can generate the report
        $this->assertIsInt($report->totalIssues());

        // Output summary for visibility if there are issues
        if ($report->hasIssues()) {
            fwrite(STDERR, "\nIntegrity issues found: " . $report->summary() . "\n");
        }
    }
}
