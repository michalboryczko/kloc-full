<?php

declare(strict_types=1);

namespace ContractTests\Tests\CallKind;

use ContractTests\Attribute\ContractTest;
use ContractTests\CallsContractTestCase;

/**
 * Tests for experimental kind filtering in calls.json.
 *
 * Per the finish-mvp spec:
 * - Stable kinds (always generated): access, method, constructor, access_static, method_static
 * - Experimental kinds (require --experimental flag): function, access_array, coalesce, ternary, ternary_full, match
 *
 * NOTE: These tests validate the schema change. By default (without --experimental flag),
 * experimental kinds should NOT appear in calls.json.
 *
 * Reference code:
 * - src/Service/OrderDisplayService.php (coalesce, ternary operators)
 * - src/Entity/Address.php (sprintf function calls)
 */
class ExperimentalKindTest extends CallsContractTestCase
{
    /**
     * Stable kinds that should always be generated.
     */
    private const STABLE_KINDS = [
        'access',
        'method',
        'constructor',
        'access_static',
        'method_static',
    ];

    /**
     * Experimental kinds that require --experimental flag.
     */
    private const EXPERIMENTAL_KINDS = [
        'function',
        'access_array',
        'coalesce',
        'ternary',
        'ternary_full',
        'match',
    ];

    /**
     * Deprecated kinds that should no longer exist.
     */
    private const DEPRECATED_KINDS = [
        'access_nullsafe',
        'method_nullsafe',
    ];

    // ═══════════════════════════════════════════════════════════════
    // Stable Kinds Tests
    // ═══════════════════════════════════════════════════════════════

    #[ContractTest(
        name: 'Stable Kinds Always Present',
        description: 'Verifies stable kinds (access, method, constructor) are present in calls.json regardless of experimental flag. These kinds should always be generated.',
        category: 'callkind',
    )]
    public function testStableKindsAlwaysPresent(): void
    {
        // These are the core kinds that must always be present
        $requiredStableKinds = ['access', 'method', 'constructor'];

        foreach ($requiredStableKinds as $kind) {
            $calls = $this->calls()->kind($kind)->all();
            $this->assertNotEmpty(
                $calls,
                sprintf('Stable kind "%s" should have at least one call in calls.json', $kind)
            );
        }
    }

    #[ContractTest(
        name: 'All Call Kinds Are Valid',
        description: 'Verifies every call in calls.json has a kind that is either stable, experimental, or deprecated. No unknown kinds should exist.',
        category: 'schema',
    )]
    public function testAllCallKindsAreValid(): void
    {
        $validKinds = array_merge(
            self::STABLE_KINDS,
            self::EXPERIMENTAL_KINDS,
            self::DEPRECATED_KINDS // Include for transition period
        );

        $allCalls = $this->calls()->all();
        $this->assertNotEmpty($allCalls, 'Should have calls in calls.json');

        $invalidKinds = [];
        foreach ($allCalls as $call) {
            $kind = $call['kind'] ?? '';
            if (!in_array($kind, $validKinds, true)) {
                $invalidKinds[$kind] = ($invalidKinds[$kind] ?? 0) + 1;
            }
        }

        $this->assertEmpty(
            $invalidKinds,
            sprintf(
                'Found invalid call kinds: %s',
                implode(', ', array_map(
                    fn($k, $c) => "{$k} ({$c})",
                    array_keys($invalidKinds),
                    array_values($invalidKinds)
                ))
            )
        );
    }

    // ═══════════════════════════════════════════════════════════════
    // Experimental Kinds Tests (Default Run)
    // ═══════════════════════════════════════════════════════════════

    #[ContractTest(
        name: 'No Function Calls Without Experimental Flag',
        description: 'Verifies function calls (kind=function) are NOT present in default calls.json. Function kind is experimental and requires --experimental flag.',
        category: 'callkind',
    )]
    public function testNoFunctionCallsWithoutExperimentalFlag(): void
    {
        if (self::isExperimentalMode()) {
            $this->markTestSkipped('This test verifies default mode behavior - skipped in experimental mode');
        }

        $functionCalls = $this->calls()->kind('function')->all();

        $this->assertEmpty(
            $functionCalls,
            sprintf(
                'Found %d function calls in default output. Function kind is experimental and should not be present without --experimental flag.',
                count($functionCalls)
            )
        );
    }

    #[ContractTest(
        name: 'No Coalesce Operators Without Experimental Flag',
        description: 'Verifies null coalesce operators (kind=coalesce) are NOT present in default calls.json. Coalesce kind is experimental and requires --experimental flag.',
        category: 'callkind',
    )]
    public function testNoCoalesceWithoutExperimentalFlag(): void
    {
        if (self::isExperimentalMode()) {
            $this->markTestSkipped('This test verifies default mode behavior - skipped in experimental mode');
        }

        $coalesceCalls = $this->calls()->kind('coalesce')->all();

        $this->assertEmpty(
            $coalesceCalls,
            sprintf(
                'Found %d coalesce calls in default output. Coalesce kind is experimental and should not be present without --experimental flag.',
                count($coalesceCalls)
            )
        );
    }

    #[ContractTest(
        name: 'No Ternary Operators Without Experimental Flag',
        description: 'Verifies ternary operators (kind=ternary, ternary_full) are NOT present in default calls.json. Ternary kinds are experimental and require --experimental flag.',
        category: 'callkind',
    )]
    public function testNoTernaryWithoutExperimentalFlag(): void
    {
        if (self::isExperimentalMode()) {
            $this->markTestSkipped('This test verifies default mode behavior - skipped in experimental mode');
        }

        $ternaryCalls = $this->calls()->kind('ternary')->all();
        $ternaryFullCalls = $this->calls()->kind('ternary_full')->all();

        $totalTernary = count($ternaryCalls) + count($ternaryFullCalls);
        $this->assertSame(
            0,
            $totalTernary,
            sprintf(
                'Found %d ternary calls in default output. Ternary kinds are experimental and should not be present without --experimental flag.',
                $totalTernary
            )
        );
    }

    #[ContractTest(
        name: 'No Array Access Without Experimental Flag',
        description: 'Verifies array access (kind=access_array) is NOT present in default calls.json. Array access kind is experimental and requires --experimental flag.',
        category: 'callkind',
    )]
    public function testNoArrayAccessWithoutExperimentalFlag(): void
    {
        if (self::isExperimentalMode()) {
            $this->markTestSkipped('This test verifies default mode behavior - skipped in experimental mode');
        }

        $arrayAccessCalls = $this->calls()->kind('access_array')->all();

        $this->assertEmpty(
            $arrayAccessCalls,
            sprintf(
                'Found %d access_array calls in default output. Array access kind is experimental and should not be present without --experimental flag.',
                count($arrayAccessCalls)
            )
        );
    }

    #[ContractTest(
        name: 'No Match Expressions Without Experimental Flag',
        description: 'Verifies match expressions (kind=match) are NOT present in default calls.json. Match kind is experimental and requires --experimental flag.',
        category: 'callkind',
    )]
    public function testNoMatchWithoutExperimentalFlag(): void
    {
        if (self::isExperimentalMode()) {
            $this->markTestSkipped('This test verifies default mode behavior - skipped in experimental mode');
        }

        $matchCalls = $this->calls()->kind('match')->all();

        $this->assertEmpty(
            $matchCalls,
            sprintf(
                'Found %d match calls in default output. Match kind is experimental and should not be present without --experimental flag.',
                count($matchCalls)
            )
        );
    }

    #[ContractTest(
        name: 'No Experimental Kinds in Default Output',
        description: 'Verifies NO experimental kinds exist in calls.json generated without --experimental flag. This is the comprehensive test for experimental filtering.',
        category: 'callkind',
    )]
    public function testNoExperimentalKindsInDefaultOutput(): void
    {
        if (self::isExperimentalMode()) {
            $this->markTestSkipped('This test verifies default mode behavior - skipped in experimental mode');
        }

        $experimentalFound = [];

        foreach (self::EXPERIMENTAL_KINDS as $kind) {
            $count = $this->calls()->kind($kind)->count();
            if ($count > 0) {
                $experimentalFound[$kind] = $count;
            }
        }

        $this->assertEmpty(
            $experimentalFound,
            sprintf(
                'Found experimental kinds in default output (should require --experimental flag): %s',
                implode(', ', array_map(
                    fn($k, $c) => "{$k} ({$c})",
                    array_keys($experimentalFound),
                    array_values($experimentalFound)
                ))
            )
        );
    }

    // ═══════════════════════════════════════════════════════════════
    // Experimental Kinds Tests (--experimental mode)
    // These tests only run when --experimental flag is passed
    // ═══════════════════════════════════════════════════════════════

    #[ContractTest(
        name: 'Function Calls Present With Experimental Flag',
        description: 'Verifies function calls (kind=function) ARE present when --experimental flag is used. Tests sprintf() and other global function calls.',
        category: 'callkind',
        experimental: true,
    )]
    public function testFunctionCallsPresentWithExperimentalFlag(): void
    {
        $functionCalls = $this->calls()->kind('function')->all();

        $this->assertNotEmpty(
            $functionCalls,
            'Function calls should be present with --experimental flag (e.g., sprintf in Address.php)'
        );
    }

    #[ContractTest(
        name: 'Experimental Kinds Present With Flag',
        description: 'Verifies experimental kinds ARE present in calls.json when --experimental flag is used.',
        category: 'callkind',
        experimental: true,
    )]
    public function testExperimentalKindsPresentWithFlag(): void
    {
        // At least one experimental kind should be present
        $totalExperimental = 0;
        foreach (self::EXPERIMENTAL_KINDS as $kind) {
            $totalExperimental += $this->calls()->kind($kind)->count();
        }

        $this->assertGreaterThan(
            0,
            $totalExperimental,
            sprintf(
                'At least one experimental kind (%s) should be present with --experimental flag',
                implode(', ', self::EXPERIMENTAL_KINDS)
            )
        );
    }

    // ═══════════════════════════════════════════════════════════════
    // Deprecated Kinds Tests
    // ═══════════════════════════════════════════════════════════════

    #[ContractTest(
        name: 'No Deprecated Kinds Exist',
        description: 'Verifies deprecated kinds (access_nullsafe, method_nullsafe) do not exist in calls.json. These have been replaced by access/method with union return types.',
        category: 'callkind',
    )]
    public function testNoDeprecatedKindsExist(): void
    {
        $deprecatedFound = [];

        foreach (self::DEPRECATED_KINDS as $kind) {
            $count = $this->calls()->kind($kind)->count();
            if ($count > 0) {
                $deprecatedFound[$kind] = $count;
            }
        }

        $this->assertEmpty(
            $deprecatedFound,
            sprintf(
                'Found deprecated kinds: %s. These should be removed.',
                implode(', ', array_map(
                    fn($k, $c) => "{$k} ({$c})",
                    array_keys($deprecatedFound),
                    array_values($deprecatedFound)
                ))
            )
        );
    }

    // ═══════════════════════════════════════════════════════════════
    // Kind Statistics (Informational)
    // ═══════════════════════════════════════════════════════════════

    #[ContractTest(
        name: 'Kind Distribution Report',
        description: 'Reports the distribution of call kinds in calls.json. This is an informational test that always passes but outputs statistics.',
        category: 'callkind',
    )]
    public function testKindDistributionReport(): void
    {
        $allKinds = array_merge(self::STABLE_KINDS, self::EXPERIMENTAL_KINDS, self::DEPRECATED_KINDS);
        $distribution = [];

        foreach ($allKinds as $kind) {
            $count = $this->calls()->kind($kind)->count();
            if ($count > 0) {
                $distribution[$kind] = $count;
            }
        }

        // Sort by count descending
        arsort($distribution);

        // Output to stderr for debugging
        $report = "\n=== Call Kind Distribution ===\n";
        foreach ($distribution as $kind => $count) {
            $category = match (true) {
                in_array($kind, self::STABLE_KINDS, true) => '[STABLE]',
                in_array($kind, self::EXPERIMENTAL_KINDS, true) => '[EXPERIMENTAL]',
                in_array($kind, self::DEPRECATED_KINDS, true) => '[DEPRECATED]',
                default => '[UNKNOWN]',
            };
            $report .= sprintf("  %-20s %s %d\n", $kind, $category, $count);
        }
        $report .= sprintf("  %-20s %d\n", 'TOTAL', array_sum($distribution));
        $report .= "==============================\n";

        // Use fwrite to stderr to avoid PHPUnit output buffering
        fwrite(STDERR, $report);

        // Always pass - this is informational
        $this->assertNotEmpty($distribution, 'Should have some calls in calls.json');
    }
}
