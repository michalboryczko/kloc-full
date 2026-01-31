<?php

declare(strict_types=1);

namespace ContractTests\Tests\Chain;

use ContractTests\Attribute\ContractTest;
use ContractTests\CallsContractTestCase;

/**
 * Tests for call chain integrity.
 *
 * Verifies that method/property chains are properly linked:
 * value -> call -> result value -> call -> result value...
 */
class ChainIntegrityTest extends CallsContractTestCase
{
    #[ContractTest(
        name: 'Chain Tests Placeholder',
        description: 'Chain tests pending scip-php receiver_value_id fixes',
        category: 'chain',
        status: 'skipped',
    )]
    public function testPlaceholder(): void
    {
        $this->markTestSkipped('Chain tests pending scip-php receiver_value_id fixes');
    }
}
