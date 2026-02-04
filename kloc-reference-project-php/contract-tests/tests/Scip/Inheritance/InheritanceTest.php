<?php

declare(strict_types=1);

namespace ContractTests\Tests\Scip\Inheritance;

use ContractTests\Attribute\ContractTest;
use ContractTests\CallsContractTestCase;

/**
 * Tests that inheritance relationships are captured in SCIP index.
 *
 * Validates that `implements` relationships between classes and interfaces
 * are correctly indexed with proper relationship types.
 */
class InheritanceTest extends CallsContractTestCase
{
    /**
     * Verifies EmailSender implements EmailSenderInterface creates relationship.
     *
     * Code reference: src/Component/EmailSender.php:7
     *   final class EmailSender implements EmailSenderInterface
     *
     * Expected: EmailSender symbol has implementation relationship to EmailSenderInterface.
     */
    #[ContractTest(
        name: 'Class implements interface creates relationship',
        description: 'Verifies EmailSender has relationship with kind=implementation to EmailSenderInterface.',
        category: 'scip',
    )]
    public function testClassImplementsInterfaceCreatesRelationship(): void
    {
        $this->requireScipData();

        // Find EmailSender class symbol
        $emailSenderSymbol = $this->scip()->symbols()
            ->symbolContains('EmailSender#')
            ->isClass()
            ->first();

        $this->assertNotNull(
            $emailSenderSymbol,
            'Expected to find EmailSender class symbol in SCIP index'
        );

        // Check for relationships
        $relationships = $emailSenderSymbol['info']['relationships'] ?? [];

        // Look for implementation relationship (note: SCIP uses snake_case: is_implementation)
        $hasImplementation = false;
        $implementedSymbol = null;
        foreach ($relationships as $rel) {
            // Check both camelCase and snake_case variants
            if (!empty($rel['isImplementation']) || !empty($rel['is_implementation'])) {
                $hasImplementation = true;
                $implementedSymbol = $rel['symbol'] ?? null;
                break;
            }
        }

        $this->assertTrue(
            $hasImplementation,
            'Expected EmailSender to have implementation relationship'
        );

        if ($implementedSymbol !== null) {
            $this->assertStringContainsString(
                'EmailSenderInterface',
                $implementedSymbol,
                'Expected implementation to point to EmailSenderInterface'
            );
        }
    }

    /**
     * Verifies InventoryChecker implements InventoryCheckerInterface.
     *
     * Code reference: src/Component/InventoryChecker.php:7
     *   final class InventoryChecker implements InventoryCheckerInterface
     */
    #[ContractTest(
        name: 'InventoryChecker implements interface relationship',
        description: 'Verifies InventoryChecker has implementation relationship to InventoryCheckerInterface.',
        category: 'scip',
    )]
    public function testInventoryCheckerImplementsInterface(): void
    {
        $this->requireScipData();

        // Find InventoryChecker class symbol
        $checkerSymbol = $this->scip()->symbols()
            ->symbolContains('InventoryChecker#')
            ->isClass()
            ->first();

        $this->assertNotNull(
            $checkerSymbol,
            'Expected to find InventoryChecker class symbol in SCIP index'
        );

        $relationships = $checkerSymbol['info']['relationships'] ?? [];

        $hasImplementation = false;
        foreach ($relationships as $rel) {
            if (!empty($rel['isImplementation']) || !empty($rel['is_implementation'])) {
                $hasImplementation = true;
                break;
            }
        }

        $this->assertTrue(
            $hasImplementation,
            'Expected InventoryChecker to have implementation relationship'
        );
    }

    /**
     * Verifies all implementing classes have implementation relationships.
     *
     * Expected: Each class implementing an interface has relationship with isImplementation=true.
     */
    #[ContractTest(
        name: 'All implementing classes have relationships',
        description: 'Verifies every class implementing an interface has proper implementation relationship in SCIP.',
        category: 'scip',
    )]
    public function testAllImplementingClassesHaveRelationships(): void
    {
        $this->requireScipData();

        // Known implementing classes in the project
        $implementingClasses = [
            'EmailSender',
            'InventoryChecker',
        ];

        foreach ($implementingClasses as $className) {
            $symbol = $this->scip()->symbols()
                ->symbolContains($className . '#')
                ->isClass()
                ->first();

            $this->assertNotNull(
                $symbol,
                "Expected to find {$className} class symbol"
            );

            $relationships = $symbol['info']['relationships'] ?? [];
            $hasImplementation = false;

            foreach ($relationships as $rel) {
                if (!empty($rel['isImplementation']) || !empty($rel['is_implementation'])) {
                    $hasImplementation = true;
                    break;
                }
            }

            $this->assertTrue(
                $hasImplementation,
                "Expected {$className} to have implementation relationship"
            );
        }
    }

    /**
     * Verifies interface symbols exist for implemented interfaces.
     *
     * Expected: Both EmailSenderInterface and InventoryCheckerInterface have symbols.
     */
    #[ContractTest(
        name: 'Interface symbols exist',
        description: 'Verifies interfaces (EmailSenderInterface, InventoryCheckerInterface) have symbol definitions.',
        category: 'scip',
    )]
    public function testInterfaceSymbolsExist(): void
    {
        $this->requireScipData();

        $interfaces = [
            'EmailSenderInterface',
            'InventoryCheckerInterface',
        ];

        foreach ($interfaces as $interfaceName) {
            $symbol = $this->scip()->symbols()
                ->symbolContains($interfaceName . '#')
                ->first();

            $this->assertNotNull(
                $symbol,
                "Expected to find {$interfaceName} symbol in SCIP index"
            );
        }
    }

    /**
     * Verifies interface has definition occurrences.
     *
     * Code reference: src/Component/EmailSenderInterface.php:7
     *   interface EmailSenderInterface
     *
     * Note: SCIP may report multiple definition occurrences (interface, methods).
     */
    #[ContractTest(
        name: 'Interface has definition occurrence',
        description: 'Verifies interfaces have definition occurrences in their source files.',
        category: 'scip',
    )]
    public function testInterfaceHasDefinitionOccurrence(): void
    {
        $this->requireScipData();

        // Find definition occurrence for EmailSenderInterface (interface symbol only)
        // The interface symbol ends with just # (not a method)
        $occurrences = $this->scip()->occurrences()
            ->symbolMatches('*EmailSenderInterface#')
            ->isDefinition()
            ->inFile('Component/EmailSenderInterface.php')
            ->all();

        // Filter to just the interface itself, not methods
        $interfaceOnly = array_filter($occurrences, function ($occ) {
            $symbol = $occ['symbol'] ?? '';
            // Interface symbol ends with # not #methodName().
            return preg_match('/EmailSenderInterface#$/', $symbol) === 1;
        });

        $this->assertGreaterThanOrEqual(
            1,
            count($interfaceOnly),
            'Expected at least one definition occurrence for EmailSenderInterface symbol'
        );
    }
}
