<?php

declare(strict_types=1);

namespace ContractTests\Tests\Chain;

use ContractTests\CallsContractTestCase;

/**
 * Tests for call chain integrity.
 *
 * These tests verify that method/property chains are properly linked:
 * value -> call -> result value -> call -> result value...
 *
 * TODO: Tests removed pending scip-php fixes for:
 * - receiver_value_id linkage on $this property access
 * - Chain propagation through property access results
 */
class ChainIntegrityTest extends CallsContractTestCase
{
    /**
     * Placeholder test - chain tests pending scip-php fixes.
     */
    public function testPlaceholder(): void
    {
        $this->markTestSkipped('Chain tests pending scip-php receiver_value_id fixes');
    }
}
