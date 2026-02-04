<?php

declare(strict_types=1);

namespace ContractTests\Tests\Scip\Symbol;

use ContractTests\Attribute\ContractTest;
use ContractTests\CallsContractTestCase;

/**
 * Tests that symbols are properly defined in the SCIP index.
 *
 * Validates that classes, methods, and properties have correct definition
 * occurrences with proper symbol kinds.
 */
class SymbolTest extends CallsContractTestCase
{
    /**
     * Verifies class symbol definition occurrence exists.
     *
     * Code reference: src/Entity/Order.php:9
     *   final readonly class Order
     *
     * Expected: SCIP contains definition occurrence for Order# symbol.
     * Note: SCIP indexes multiple symbols per class (class, properties, methods).
     */
    #[ContractTest(
        name: 'Class has definition occurrence',
        description: 'Verifies Order class symbol has a definition occurrence with role=Definition in SCIP index.',
        category: 'scip',
    )]
    public function testClassDefinitionOccurrenceExists(): void
    {
        $this->requireScipData();

        // Find definition occurrence for Order class symbol specifically (ends with #)
        $occurrences = $this->scip()->occurrences()
            ->symbolMatches('*Entity/Order#')
            ->isDefinition()
            ->inFile('Entity/Order.php')
            ->all();

        // Filter to just the class symbol (ends with # not #method or #$prop)
        $classOnly = array_filter($occurrences, function ($occ) {
            $symbol = $occ['symbol'] ?? '';
            return preg_match('/Order#$/', $symbol) === 1;
        });

        $this->assertGreaterThanOrEqual(
            1,
            count($classOnly),
            'Expected at least one definition occurrence for Order class symbol'
        );

        // Verify it's in the expected line range (around line 9)
        $occurrence = reset($classOnly);
        $line = ($occurrence['range'][0] ?? -1) + 1;
        $this->assertGreaterThanOrEqual(
            8,
            $line,
            'Expected Order class definition near line 9'
        );
        $this->assertLessThanOrEqual(
            12,
            $line,
            'Expected Order class definition near line 9'
        );
    }

    /**
     * Verifies method definition occurrence exists.
     *
     * Code reference: src/Entity/Order.php:25
     *   public function getCustomerName(): string
     *
     * Expected: SCIP contains definition occurrence for Order#getCustomerName(). symbol.
     */
    #[ContractTest(
        name: 'Method has definition occurrence',
        description: 'Verifies Order.getCustomerName() has a definition occurrence in SCIP index.',
        category: 'scip',
    )]
    public function testMethodDefinitionOccurrenceExists(): void
    {
        $this->requireScipData();

        // Find definition occurrence for getCustomerName method
        $occurrences = $this->scip()->occurrences()
            ->symbolContains('Order#getCustomerName()')
            ->isDefinition()
            ->inFile('Entity/Order.php')
            ->all();

        $this->assertCount(
            1,
            $occurrences,
            'Expected exactly one definition occurrence for getCustomerName method'
        );

        // Verify it's at the correct line (25)
        $occurrence = $occurrences[0];
        $line = ($occurrence['range'][0] ?? -1) + 1;
        $this->assertEquals(
            25,
            $line,
            'Expected getCustomerName method definition at line 25'
        );
    }

    /**
     * Verifies property definition occurrence exists.
     *
     * Code reference: src/Entity/Order.php:13
     *   public string $customerEmail,
     *
     * Expected: SCIP contains definition occurrence for Order#$customerEmail. symbol.
     */
    #[ContractTest(
        name: 'Property has definition occurrence',
        description: 'Verifies Order.$customerEmail has a definition occurrence in SCIP index.',
        category: 'scip',
    )]
    public function testPropertyDefinitionOccurrenceExists(): void
    {
        $this->requireScipData();

        // Find definition occurrence for customerEmail property
        $occurrences = $this->scip()->occurrences()
            ->symbolContains('Order#$customerEmail')
            ->isDefinition()
            ->inFile('Entity/Order.php')
            ->all();

        $this->assertCount(
            1,
            $occurrences,
            'Expected exactly one definition occurrence for customerEmail property'
        );

        // Verify it's at the correct line (13)
        $occurrence = $occurrences[0];
        $line = ($occurrence['range'][0] ?? -1) + 1;
        $this->assertEquals(
            13,
            $line,
            'Expected customerEmail property definition at line 13'
        );
    }

    /**
     * Verifies interface method definition exists.
     *
     * Code reference: src/Component/EmailSenderInterface.php:9
     *   public function send(string $to, string $subject, string $body): void;
     *
     * Expected: SCIP contains definition occurrence for EmailSenderInterface#send(). symbol.
     */
    #[ContractTest(
        name: 'Interface method has definition',
        description: 'Verifies EmailSenderInterface.send() has a definition occurrence in SCIP index.',
        category: 'scip',
    )]
    public function testInterfaceMethodDefinitionExists(): void
    {
        $this->requireScipData();

        // Find definition occurrence for send method in interface
        // Filter to just the method definition (ends with send().)
        $occurrences = $this->scip()->occurrences()
            ->symbolMatches('*EmailSenderInterface#send().')
            ->isDefinition()
            ->inFile('Component/EmailSenderInterface.php')
            ->all();

        // Filter to just the method symbol (not parameters)
        $methodOnly = array_filter($occurrences, function ($occ) {
            $symbol = $occ['symbol'] ?? '';
            return preg_match('/#send\(\)\.$/', $symbol) === 1;
        });

        $this->assertGreaterThanOrEqual(
            1,
            count($methodOnly),
            'Expected at least one definition occurrence for EmailSenderInterface.send() method'
        );
    }

    /**
     * Verifies all classes have definition occurrences.
     *
     * Expected: Every class symbol (ending with #) has at least one definition occurrence.
     */
    #[ContractTest(
        name: 'All classes have definitions',
        description: 'Verifies every class in the project has at least one definition occurrence in SCIP.',
        category: 'scip',
    )]
    public function testAllClassesHaveDefinitionOccurrences(): void
    {
        $this->requireScipData();

        // Get all class symbols
        $classSymbols = $this->scip()->symbols()
            ->isClass()
            ->all();

        $this->assertGreaterThan(
            0,
            count($classSymbols),
            'Expected at least one class symbol in SCIP index'
        );

        $classesWithoutDefinition = [];

        foreach ($classSymbols as $classData) {
            $symbol = $classData['symbol'];

            // Find definition occurrences for this symbol
            $definitions = $this->scip()->occurrences()
                ->forSymbol($symbol)
                ->isDefinition()
                ->all();

            if (count($definitions) === 0) {
                $classesWithoutDefinition[] = $symbol;
            }
        }

        $this->assertEmpty(
            $classesWithoutDefinition,
            sprintf(
                'Expected all classes to have definition occurrences. Missing: %s',
                implode(', ', $classesWithoutDefinition)
            )
        );
    }

    /**
     * Verifies symbol count is reasonable for the project.
     *
     * Expected: SCIP index contains a reasonable number of symbols.
     */
    #[ContractTest(
        name: 'Symbol count is reasonable',
        description: 'Verifies the SCIP index contains expected number of symbols for the project size.',
        category: 'scip',
    )]
    public function testSymbolCountIsReasonable(): void
    {
        $this->requireScipData();

        $totalSymbols = $this->scipData()->symbolCount();

        // Reference project has ~20 source files, expecting 50+ symbols
        $this->assertGreaterThan(
            50,
            $totalSymbols,
            'Expected at least 50 symbols in SCIP index for reference project'
        );

        // But not too many (sanity check)
        $this->assertLessThan(
            1000,
            $totalSymbols,
            'Unexpectedly high symbol count in SCIP index'
        );
    }

    /**
     * Verifies constructor symbols exist for classes with constructors.
     *
     * Code reference: src/Entity/Order.php:11-18
     *   public function __construct(...)
     *
     * Expected: SCIP contains definition for Order#__construct(). symbol.
     */
    #[ContractTest(
        name: 'Constructor has definition',
        description: 'Verifies class constructors have definition occurrences. Tests Order.__construct().',
        category: 'scip',
    )]
    public function testConstructorDefinitionExists(): void
    {
        $this->requireScipData();

        // Find definition occurrence for Order constructor
        $occurrences = $this->scip()->occurrences()
            ->symbolContains('Order#__construct()')
            ->isDefinition()
            ->inFile('Entity/Order.php')
            ->all();

        $this->assertCount(
            1,
            $occurrences,
            'Expected exactly one definition occurrence for Order constructor'
        );

        // Should be at line 11
        $occurrence = $occurrences[0];
        $line = ($occurrence['range'][0] ?? -1) + 1;
        $this->assertEquals(
            11,
            $line,
            'Expected Order constructor definition at line 11'
        );
    }
}
