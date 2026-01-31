<?php

declare(strict_types=1);

namespace ContractTests;

use ContractTests\Assertions\ArgumentBindingAssertion;
use ContractTests\Assertions\ChainIntegrityAssertion;
use ContractTests\Assertions\DataIntegrityAssertion;
use ContractTests\Assertions\IntegrityReport;
use ContractTests\Assertions\ReferenceConsistencyAssertion;
use ContractTests\Query\CallQuery;
use ContractTests\Query\MethodScope;
use ContractTests\Query\ValueQuery;
use PHPUnit\Framework\TestCase;

/**
 * Base test class for contract tests.
 *
 * Provides access to the loaded CallsData and query/assertion factories.
 */
abstract class CallsContractTestCase extends TestCase
{
    protected static ?CallsData $calls = null;

    public static function setUpBeforeClass(): void
    {
        if (self::$calls === null) {
            self::$calls = CallsData::load(CALLS_JSON_PATH);
        }
    }

    // ═══════════════════════════════════════════════════════════════
    // Query API
    // ═══════════════════════════════════════════════════════════════

    /**
     * Start a query for values.
     */
    protected function values(): ValueQuery
    {
        return new ValueQuery(self::$calls);
    }

    /**
     * Start a query for calls.
     */
    protected function calls(): CallQuery
    {
        return new CallQuery(self::$calls);
    }

    /**
     * Query within a specific method scope.
     *
     * @param string $class  Fully qualified class name (e.g., 'App\Repository\OrderRepository')
     * @param string $method Method name (e.g., 'save')
     */
    protected function inMethod(string $class, string $method): MethodScope
    {
        return new MethodScope(self::$calls, $class, $method);
    }

    /**
     * Get the loaded CallsData instance.
     */
    protected function callsData(): CallsData
    {
        return self::$calls;
    }

    // ═══════════════════════════════════════════════════════════════
    // Category 1: Reference Consistency
    // ═══════════════════════════════════════════════════════════════

    /**
     * Assert that a parameter/local has exactly one value entry
     * and all usages reference it.
     */
    protected function assertReferenceConsistency(): ReferenceConsistencyAssertion
    {
        return new ReferenceConsistencyAssertion(self::$calls, $this);
    }

    // ═══════════════════════════════════════════════════════════════
    // Category 2: Chain Integrity
    // ═══════════════════════════════════════════════════════════════

    /**
     * Assert chain structure is correct (value->call->value->call...).
     */
    protected function assertChain(): ChainIntegrityAssertion
    {
        return new ChainIntegrityAssertion(self::$calls, $this);
    }

    // ═══════════════════════════════════════════════════════════════
    // Category 3: Argument Binding
    // ═══════════════════════════════════════════════════════════════

    /**
     * Assert argument value_id points to expected value.
     */
    protected function assertArgument(): ArgumentBindingAssertion
    {
        return new ArgumentBindingAssertion(self::$calls, $this);
    }

    // ═══════════════════════════════════════════════════════════════
    // Category 4: Data Integrity
    // ═══════════════════════════════════════════════════════════════

    /**
     * Run integrity checks on the data.
     */
    protected function assertIntegrity(): DataIntegrityAssertion
    {
        return new DataIntegrityAssertion(self::$calls, $this);
    }

    /**
     * Get integrity report without failing.
     */
    protected function integrityReport(): IntegrityReport
    {
        return (new DataIntegrityAssertion(self::$calls, $this))
            ->noParameterDuplicates()
            ->noLocalDuplicatesPerLine()
            ->allReceiverValueIdsExist()
            ->allArgumentValueIdsExist()
            ->allSourceCallIdsExist()
            ->allSourceValueIdsExist()
            ->everyCallHasResultValue()
            ->resultValueTypesMatch()
            ->report();
    }
}
