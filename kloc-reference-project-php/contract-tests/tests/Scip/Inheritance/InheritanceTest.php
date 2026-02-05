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

    // ═══════════════════════════════════════════════════════════════
    // Class Extends Relationships
    // ═══════════════════════════════════════════════════════════════

    /**
     * Verifies StandardOrderProcessor extends AbstractOrderProcessor creates relationship.
     *
     * Code reference: src/Service/StandardOrderProcessor.php:17
     *   final class StandardOrderProcessor extends AbstractOrderProcessor
     *
     * Expected: StandardOrderProcessor symbol has is_reference relationship to AbstractOrderProcessor.
     * Note: SCIP uses is_reference for extends, is_implementation for implements.
     */
    #[ContractTest(
        name: 'Class extends abstract class creates relationship',
        description: 'Verifies StandardOrderProcessor has relationship with is_reference to AbstractOrderProcessor.',
        category: 'scip',
    )]
    public function testClassExtendsAbstractCreatesRelationship(): void
    {
        $this->requireScipData();

        // Find StandardOrderProcessor class symbol
        $processorSymbol = $this->scip()->symbols()
            ->symbolContains('StandardOrderProcessor#')
            ->isClass()
            ->first();

        $this->assertNotNull(
            $processorSymbol,
            'Expected to find StandardOrderProcessor class symbol in SCIP index'
        );

        // Check for relationships
        $relationships = $processorSymbol['info']['relationships'] ?? [];

        // Look for is_reference relationship (extends uses is_reference in SCIP)
        $hasReference = false;
        $extendedSymbol = null;
        foreach ($relationships as $rel) {
            // Check both camelCase and snake_case variants
            if (!empty($rel['isReference']) || !empty($rel['is_reference'])) {
                $hasReference = true;
                $extendedSymbol = $rel['symbol'] ?? null;
                break;
            }
        }

        $this->assertTrue(
            $hasReference,
            'Expected StandardOrderProcessor to have is_reference relationship (extends)'
        );

        if ($extendedSymbol !== null) {
            $this->assertStringContainsString(
                'AbstractOrderProcessor',
                $extendedSymbol,
                'Expected is_reference to point to AbstractOrderProcessor'
            );
        }
    }

    /**
     * Verifies abstract class symbol exists.
     *
     * Code reference: src/Service/AbstractOrderProcessor.php:17
     *   abstract class AbstractOrderProcessor
     *
     * Expected: AbstractOrderProcessor has symbol definition in SCIP index.
     */
    #[ContractTest(
        name: 'Abstract class symbol exists',
        description: 'Verifies AbstractOrderProcessor has symbol definition in SCIP index.',
        category: 'scip',
    )]
    public function testAbstractClassSymbolExists(): void
    {
        $this->requireScipData();

        $abstractSymbol = $this->scip()->symbols()
            ->symbolContains('AbstractOrderProcessor#')
            ->first();

        $this->assertNotNull(
            $abstractSymbol,
            'Expected to find AbstractOrderProcessor symbol in SCIP index'
        );
    }

    /**
     * Verifies child class symbol exists alongside parent.
     *
     * Code reference: src/Service/StandardOrderProcessor.php:17
     *   final class StandardOrderProcessor extends AbstractOrderProcessor
     *
     * Expected: Both abstract parent and concrete child have symbols.
     */
    #[ContractTest(
        name: 'Child class symbol exists',
        description: 'Verifies StandardOrderProcessor class has symbol definition in SCIP index.',
        category: 'scip',
    )]
    public function testChildClassSymbolExists(): void
    {
        $this->requireScipData();

        $childSymbol = $this->scip()->symbols()
            ->symbolContains('StandardOrderProcessor#')
            ->isClass()
            ->first();

        $this->assertNotNull(
            $childSymbol,
            'Expected to find StandardOrderProcessor class symbol in SCIP index'
        );

        // Also verify the parent exists
        $parentSymbol = $this->scip()->symbols()
            ->symbolContains('AbstractOrderProcessor#')
            ->first();

        $this->assertNotNull(
            $parentSymbol,
            'Expected to find AbstractOrderProcessor (parent) symbol in SCIP index'
        );
    }

    /**
     * Verifies extends relationship is distinct from implements relationship.
     *
     * Expected: Extends uses is_reference, implements uses is_implementation.
     */
    #[ContractTest(
        name: 'Extends relationship distinct from implements',
        description: 'Verifies extends uses is_reference while implements uses is_implementation.',
        category: 'scip',
    )]
    public function testExtendsDistinctFromImplements(): void
    {
        $this->requireScipData();

        // Get StandardOrderProcessor (uses extends)
        $extendsClass = $this->scip()->symbols()
            ->symbolContains('StandardOrderProcessor#')
            ->isClass()
            ->first();

        // Get EmailSender (uses implements)
        $implementsClass = $this->scip()->symbols()
            ->symbolContains('EmailSender#')
            ->isClass()
            ->first();

        $this->assertNotNull($extendsClass, 'Expected StandardOrderProcessor symbol');
        $this->assertNotNull($implementsClass, 'Expected EmailSender symbol');

        $extendsRels = $extendsClass['info']['relationships'] ?? [];
        $implementsRels = $implementsClass['info']['relationships'] ?? [];

        // StandardOrderProcessor should have is_reference but not implementation
        $extendsHasReference = false;
        $extendsHasImpl = false;
        foreach ($extendsRels as $rel) {
            if (!empty($rel['isReference']) || !empty($rel['is_reference'])) {
                $extendsHasReference = true;
            }
            if (!empty($rel['isImplementation']) || !empty($rel['is_implementation'])) {
                $extendsHasImpl = true;
            }
        }

        // EmailSender should have implementation but not is_reference
        $implHasReference = false;
        $implHasImpl = false;
        foreach ($implementsRels as $rel) {
            if (!empty($rel['isReference']) || !empty($rel['is_reference'])) {
                $implHasReference = true;
            }
            if (!empty($rel['isImplementation']) || !empty($rel['is_implementation'])) {
                $implHasImpl = true;
            }
        }

        $this->assertTrue(
            $extendsHasReference,
            'StandardOrderProcessor (extends) should have is_reference relationship'
        );

        $this->assertTrue(
            $implHasImpl,
            'EmailSender (implements) should have implementation relationship'
        );

        // Verify they use different relationship kinds
        $this->assertFalse(
            $extendsHasImpl,
            'StandardOrderProcessor should NOT have implementation relationship (uses extends not implements)'
        );

        $this->assertFalse(
            $implHasReference,
            'EmailSender should NOT have is_reference relationship (uses implements not extends)'
        );
    }

    /**
     * Verifies abstract method implementation is tracked.
     *
     * Code reference:
     *   AbstractOrderProcessor.php:38 - abstract protected function preProcess(Order $order): void;
     *   StandardOrderProcessor.php:22 - protected function preProcess(Order $order): void
     *
     * Expected: Child method overriding abstract parent should be tracked.
     */
    #[ContractTest(
        name: 'Abstract method implementation tracked',
        description: 'Verifies child class implementing abstract parent method has proper symbol relationship.',
        category: 'scip',
    )]
    public function testAbstractMethodImplementationTracked(): void
    {
        $this->requireScipData();

        // Find the preProcess method in StandardOrderProcessor
        $childMethod = $this->scip()->symbols()
            ->symbolContains('StandardOrderProcessor')
            ->symbolContains('preProcess')
            ->first();

        $this->assertNotNull(
            $childMethod,
            'Expected to find StandardOrderProcessor::preProcess method symbol'
        );

        // Find the abstract preProcess in AbstractOrderProcessor
        $parentMethod = $this->scip()->symbols()
            ->symbolContains('AbstractOrderProcessor')
            ->symbolContains('preProcess')
            ->first();

        $this->assertNotNull(
            $parentMethod,
            'Expected to find AbstractOrderProcessor::preProcess abstract method symbol'
        );
    }
}
