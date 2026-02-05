<?php

declare(strict_types=1);

namespace ContractTests\Tests\Scip\Occurrence;

use ContractTests\Attribute\ContractTest;
use ContractTests\CallsContractTestCase;
use ContractTests\Query\OccurrenceQuery;

/**
 * Tests that occurrences are captured correctly in SCIP index.
 *
 * Validates that definitions and references are tracked with accurate
 * file locations and proper symbol linking.
 */
class OccurrenceTest extends CallsContractTestCase
{
    /**
     * Verifies every symbol has at least one occurrence.
     *
     * Expected: All symbols in SCIP have at least one occurrence (definition or reference).
     */
    #[ContractTest(
        name: 'Every symbol has at least one occurrence',
        description: 'Verifies every symbol in the SCIP index has at least one occurrence (definition or reference).',
        category: 'scip',
    )]
    public function testEverySymbolHasOccurrence(): void
    {
        $this->requireScipData();

        $allSymbols = $this->scipData()->symbols();
        $symbolsWithoutOccurrences = [];

        foreach ($allSymbols as $symbolName => $symbolInfo) {
            $occurrences = $this->scipData()->occurrences($symbolName);
            if (count($occurrences) === 0) {
                $symbolsWithoutOccurrences[] = $symbolName;
            }
        }

        $this->assertEmpty(
            $symbolsWithoutOccurrences,
            sprintf(
                'Symbols without occurrences: %s',
                implode(', ', array_slice($symbolsWithoutOccurrences, 0, 5))
            )
        );
    }

    /**
     * Verifies definition occurrences have correct file locations.
     *
     * Code reference: src/Entity/Order.php
     *   final readonly class Order
     *
     * Expected: Order class definition occurrence is at correct file.
     * Note: The exact line may vary based on class structure (around lines 8-12).
     */
    #[ContractTest(
        name: 'Definition occurrence has correct location',
        description: 'Verifies Order class definition is in src/Entity/Order.php with valid location.',
        category: 'scip',
    )]
    public function testDefinitionOccurrenceHasCorrectLocation(): void
    {
        $this->requireScipData();

        // Find the Order class symbol definition (ends with #)
        $occurrences = $this->scip()->occurrences()
            ->symbolMatches('*Entity/Order#')
            ->isDefinition()
            ->inFile('Entity/Order.php')
            ->all();

        // Filter to just the class symbol
        $classOccurrences = array_filter($occurrences, function ($occ) {
            $symbol = $occ['symbol'] ?? '';
            return preg_match('/Order#$/', $symbol) === 1;
        });

        $this->assertGreaterThan(
            0,
            count($classOccurrences),
            'Expected to find Order class definition occurrence'
        );

        $occurrence = reset($classOccurrences);

        // Check file
        $file = $occurrence['_file'] ?? '';
        $this->assertStringContainsString('Entity/Order.php', $file);

        // Check line (should be in a reasonable range - lines 1-15 for class definition)
        $range = $occurrence['range'] ?? [];
        $this->assertNotEmpty($range, 'Expected occurrence to have range');

        $line = $range[0] + 1; // Convert to 1-indexed
        $this->assertGreaterThanOrEqual(1, $line, 'Expected valid line number');
        $this->assertLessThanOrEqual(15, $line, 'Expected definition near top of file');
    }

    /**
     * Verifies reference occurrences track type usage across files.
     *
     * Expected: Order class has references in multiple files where it's used as type hint.
     */
    #[ContractTest(
        name: 'References tracked across files',
        description: 'Verifies Order class has reference occurrences in files that use it as type hints.',
        category: 'scip',
    )]
    public function testReferencesTrackedAcrossFiles(): void
    {
        $this->requireScipData();

        $references = $this->scip()->occurrences()
            ->symbolContains('Entity/Order#')
            ->isReference()
            ->all();

        $this->assertGreaterThan(
            0,
            count($references),
            'Expected at least one reference to Order class'
        );

        // Collect files with references
        $filesWithReferences = [];
        foreach ($references as $ref) {
            $file = $ref['_file'] ?? 'unknown';
            $filesWithReferences[$file] = true;
        }

        // Order should be referenced in at least OrderRepository and OrderService
        $expectedFiles = ['Repository/OrderRepository.php', 'Service/OrderService.php'];
        foreach ($expectedFiles as $expectedFile) {
            $found = false;
            foreach (array_keys($filesWithReferences) as $file) {
                if (str_contains($file, $expectedFile)) {
                    $found = true;
                    break;
                }
            }
            $this->assertTrue(
                $found,
                "Expected Order reference in {$expectedFile}"
            );
        }
    }

    /**
     * Verifies method call creates reference occurrence.
     *
     * Code reference: src/Service/OrderService.php:40
     *   $savedOrder = $this->orderRepository->save($order);
     *
     * Expected: SCIP contains reference occurrence for save() method.
     */
    #[ContractTest(
        name: 'Method call creates reference occurrence',
        description: 'Verifies method calls like $this->orderRepository->save() create reference occurrences.',
        category: 'scip',
    )]
    public function testMethodCallCreatesReferenceOccurrence(): void
    {
        $this->requireScipData();

        // Find reference occurrences for save method
        $occurrences = $this->scip()->occurrences()
            ->symbolContains('OrderRepository#save()')
            ->isReference()
            ->inFile('Service/OrderService.php')
            ->all();

        $this->assertGreaterThan(
            0,
            count($occurrences),
            'Expected reference occurrence for OrderRepository.save() in OrderService'
        );

        // Verify one is near line 45
        $foundAtLine45 = false;
        foreach ($occurrences as $occ) {
            $line = ($occ['range'][0] ?? -1) + 1;
            if ($line >= 43 && $line <= 47) {
                $foundAtLine45 = true;
                break;
            }
        }

        $this->assertTrue(
            $foundAtLine45,
            'Expected save() reference near line 45 in OrderService'
        );
    }

    /**
     * Verifies property access creates reference occurrence.
     *
     * Code reference: src/Service/OrderService.php:48
     *   to: $savedOrder->customerEmail,
     *
     * Expected: SCIP contains reference occurrence for customerEmail property.
     */
    #[ContractTest(
        name: 'Property access creates reference occurrence',
        description: 'Verifies property accesses like $savedOrder->customerEmail create reference occurrences.',
        category: 'scip',
    )]
    public function testPropertyAccessCreatesReferenceOccurrence(): void
    {
        $this->requireScipData();

        // Find reference occurrences for customerEmail property
        $occurrences = $this->scip()->occurrences()
            ->symbolContains('Order#$customerEmail')
            ->isReference()
            ->inFile('Service/OrderService.php')
            ->all();

        $this->assertGreaterThan(
            0,
            count($occurrences),
            'Expected reference occurrence for Order.customerEmail in OrderService'
        );
    }

    /**
     * Verifies occurrence symbol roles are correctly set.
     *
     * Expected: Definition occurrences have role=Definition (bit 1).
     */
    #[ContractTest(
        name: 'Occurrence roles are correct',
        description: 'Verifies definition occurrences have the Definition role bit set.',
        category: 'scip',
    )]
    public function testOccurrenceRolesAreCorrect(): void
    {
        $this->requireScipData();

        // Get a definition occurrence
        $definition = $this->scip()->occurrences()
            ->symbolContains('Entity/Order#')
            ->isDefinition()
            ->first();

        $this->assertNotNull($definition);

        $symbolRoles = $definition['symbolRoles'] ?? $definition['symbol_roles'] ?? 0;
        $roleNames = OccurrenceQuery::getRoleNames($definition);

        $this->assertContains(
            'Definition',
            $roleNames,
            'Expected definition occurrence to have Definition role'
        );

        // Get a reference occurrence
        $reference = $this->scip()->occurrences()
            ->symbolContains('Entity/Order#')
            ->isReference()
            ->first();

        $this->assertNotNull($reference);

        $refRoleNames = OccurrenceQuery::getRoleNames($reference);
        $this->assertContains(
            'Reference',
            $refRoleNames,
            'Expected reference occurrence to have Reference role'
        );
    }

    /**
     * Verifies new Object() creates reference occurrence.
     *
     * Code reference: src/Service/OrderService.php:31
     *   $order = new Order(...)
     *
     * Expected: SCIP contains reference occurrence for Order at constructor call.
     */
    #[ContractTest(
        name: 'Constructor call creates reference',
        description: 'Verifies new Order() constructor calls create reference occurrences for the class.',
        category: 'scip',
    )]
    public function testConstructorCallCreatesReference(): void
    {
        $this->requireScipData();

        // Find Order references in OrderService around the constructor call
        $occurrences = $this->scip()->occurrences()
            ->symbolContains('Entity/Order#')
            ->isReference()
            ->inFile('Service/OrderService.php')
            ->betweenLines(30, 40)
            ->all();

        $this->assertGreaterThan(
            0,
            count($occurrences),
            'Expected Order reference at new Order() call location'
        );
    }

    /**
     * Verifies file paths in occurrences are consistent.
     *
     * Expected: All occurrences have valid _file paths.
     */
    #[ContractTest(
        name: 'Occurrence file paths are consistent',
        description: 'Verifies all occurrences have valid relative file paths.',
        category: 'scip',
    )]
    public function testOccurrenceFilePathsAreConsistent(): void
    {
        $this->requireScipData();

        // Get some occurrences
        $occurrences = $this->scip()->occurrences()
            ->symbolContains('Order')
            ->all();

        $this->assertGreaterThan(0, count($occurrences));

        $invalidPaths = [];
        foreach ($occurrences as $occ) {
            $file = $occ['_file'] ?? null;
            if ($file === null || $file === '') {
                $invalidPaths[] = $occ['symbol'] ?? 'unknown';
            }
        }

        $this->assertEmpty(
            $invalidPaths,
            sprintf('Occurrences with invalid file paths: %s', implode(', ', array_slice($invalidPaths, 0, 5)))
        );
    }
}
