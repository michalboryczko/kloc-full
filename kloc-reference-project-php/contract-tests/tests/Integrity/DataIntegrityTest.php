<?php

declare(strict_types=1);

namespace ContractTests\Tests\Integrity;

use ContractTests\Attribute\ContractTest;
use ContractTests\CallsContractTestCase;

/**
 * Tests for overall data integrity of the calls.json output.
 */
class DataIntegrityTest extends CallsContractTestCase
{
    #[ContractTest(
        name: 'Receiver IDs Exist',
        description: 'Verifies every call receiver_value_id references an existing value entry. Orphaned references indicate missing value entries in the index.',
        category: 'integrity',
    )]
    public function testAllReceiverIdsExist(): void
    {
        $this->assertIntegrity()
            ->allReceiverValueIdsExist()
            ->verify();
    }

    #[ContractTest(
        name: 'Argument IDs Exist',
        description: 'Verifies every argument value_id references an existing value entry. Orphaned references indicate missing value entries for argument sources.',
        category: 'integrity',
    )]
    public function testAllArgumentIdsExist(): void
    {
        $this->assertIntegrity()
            ->allArgumentValueIdsExist()
            ->verify();
    }

    #[ContractTest(
        name: 'Source Call IDs Exist',
        description: 'Verifies every value source_call_id references an existing call entry. Result values must point to the call that produced them.',
        category: 'integrity',
    )]
    public function testAllSourceCallIdsExist(): void
    {
        $this->assertIntegrity()
            ->allSourceCallIdsExist()
            ->verify();
    }

    #[ContractTest(
        name: 'Source Value IDs Exist',
        description: 'Verifies every value source_value_id references an existing value entry. Used for assignment tracking where a value derives from another value.',
        category: 'integrity',
    )]
    public function testAllSourceValueIdsExist(): void
    {
        $this->assertIntegrity()
            ->allSourceValueIdsExist()
            ->verify();
    }

    #[ContractTest(
        name: 'Every Call Has Result Value',
        description: 'Verifies each call has a corresponding result value with the same ID. Missing result values break chain integrity as subsequent calls cannot reference the result.',
        category: 'integrity',
    )]
    public function testEveryCallHasResultValue(): void
    {
        $this->assertIntegrity()
            ->everyCallHasResultValue()
            ->verify();
    }

    #[ContractTest(
        name: 'Result Types Match',
        description: 'Verifies result value type field matches the return_type of their source call. Type mismatches indicate incorrect type inference in the indexer.',
        category: 'integrity',
    )]
    public function testResultTypesMatch(): void
    {
        $this->assertIntegrity()
            ->resultValueTypesMatch()
            ->verify();
    }

    #[ContractTest(
        name: 'Full Integrity Report',
        description: 'Generates a complete integrity report counting all issue types: duplicate symbols, orphaned references, missing result values, type mismatches. Outputs summary to stderr for debugging.',
        category: 'integrity',
    )]
    public function testFullIntegrityReport(): void
    {
        $report = $this->integrityReport();

        $this->assertIsInt($report->totalIssues());

        if ($report->hasIssues()) {
            fwrite(STDERR, "\nIntegrity issues found: " . $report->summary() . "\n");
        }
    }
}
