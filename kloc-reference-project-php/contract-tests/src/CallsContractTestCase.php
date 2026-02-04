<?php

declare(strict_types=1);

namespace ContractTests;

use ContractTests\Assertions\ArgumentBindingAssertion;
use ContractTests\Assertions\ChainIntegrityAssertion;
use ContractTests\Assertions\DataIntegrityAssertion;
use ContractTests\Assertions\IntegrityReport;
use ContractTests\Assertions\ReferenceConsistencyAssertion;
use ContractTests\Attribute\ContractTest;
use ContractTests\Query\CallQuery;
use ContractTests\Query\MethodScope;
use ContractTests\Query\OccurrenceQuery;
use ContractTests\Query\ScipQuery;
use ContractTests\Query\SymbolQuery;
use ContractTests\Query\ValueQuery;
use PHPUnit\Framework\TestCase;
use ReflectionMethod;

/**
 * Base test class for contract tests.
 *
 * Provides access to the loaded CallsData and query/assertion factories.
 */
abstract class CallsContractTestCase extends TestCase
{
    protected static ?CallsData $calls = null;
    protected static ?ScipData $scip = null;

    public static function setUpBeforeClass(): void
    {
        if (self::$calls === null) {
            self::$calls = CallsData::load(CALLS_JSON_PATH);
        }

        // Load SCIP data if available (optional - won't fail if missing)
        if (self::$scip === null && defined('SCIP_JSON_PATH') && file_exists(SCIP_JSON_PATH)) {
            self::$scip = ScipData::load(SCIP_JSON_PATH);
        }
    }

    protected function setUp(): void
    {
        parent::setUp();
        $this->skipIfExperimentalTest();
    }

    /**
     * Check if the current test is marked as experimental and skip if not in experimental mode.
     */
    private function skipIfExperimentalTest(): void
    {
        $testMethod = $this->name();
        $reflection = new ReflectionMethod($this, $testMethod);
        $attributes = $reflection->getAttributes(ContractTest::class);

        if (empty($attributes)) {
            return;
        }

        $contractTest = $attributes[0]->newInstance();

        if ($contractTest->experimental && !self::isExperimentalMode()) {
            $this->markTestSkipped(sprintf(
                '[EXPERIMENTAL] %s - requires --experimental flag',
                $contractTest->name
            ));
        }
    }

    /**
     * Check if experimental mode is enabled via environment variable.
     */
    public static function isExperimentalMode(): bool
    {
        return getenv('CONTRACT_TESTS_EXPERIMENTAL') === '1';
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
    // SCIP Query API
    // ═══════════════════════════════════════════════════════════════

    /**
     * Start a query for SCIP index data.
     *
     * @throws \RuntimeException if SCIP data is not loaded
     */
    protected function scip(): ScipQuery
    {
        if (self::$scip === null) {
            throw new \RuntimeException(
                'SCIP data not loaded. Make sure index.scip.json exists in output directory.'
            );
        }
        return new ScipQuery(self::$scip);
    }

    /**
     * Get the loaded ScipData instance directly.
     *
     * @throws \RuntimeException if SCIP data is not loaded
     */
    protected function scipData(): ScipData
    {
        if (self::$scip === null) {
            throw new \RuntimeException(
                'SCIP data not loaded. Make sure index.scip.json exists in output directory.'
            );
        }
        return self::$scip;
    }

    /**
     * Check if SCIP data is available.
     */
    protected function hasScipData(): bool
    {
        return self::$scip !== null;
    }

    /**
     * Skip test if SCIP data is not available.
     */
    protected function requireScipData(): void
    {
        if (!$this->hasScipData()) {
            $this->markTestSkipped('SCIP data not available (index.scip.json not found)');
        }
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
