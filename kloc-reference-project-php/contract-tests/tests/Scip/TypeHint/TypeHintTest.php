<?php

declare(strict_types=1);

namespace ContractTests\Tests\Scip\TypeHint;

use ContractTests\Attribute\ContractTest;
use ContractTests\CallsContractTestCase;

/**
 * Tests that type hints create SCIP occurrences.
 *
 * Validates that property type hints, parameter type hints, and return type hints
 * are correctly indexed as reference occurrences to their respective types.
 */
class TypeHintTest extends CallsContractTestCase
{
    /**
     * Verifies that property type hints create reference occurrences.
     *
     * Code reference: src/Service/OrderService.php:20
     *   private OrderRepository $orderRepository,
     *
     * Expected: SCIP contains a reference occurrence for OrderRepository at the type hint.
     */
    #[ContractTest(
        name: 'Property type hint creates SCIP occurrence',
        description: 'Verifies typed properties create reference occurrences in SCIP index. Tests OrderService.$orderRepository typed as OrderRepository.',
        category: 'scip',
    )]
    public function testPropertyTypeHintCreatesOccurrence(): void
    {
        $this->requireScipData();

        // Find reference occurrences for OrderRepository class
        $occurrences = $this->scip()->occurrences()
            ->symbolContains('OrderRepository#')
            ->isReference()
            ->inFile('Service/OrderService.php')
            ->all();

        // Should have at least one reference (the type hint)
        $this->assertGreaterThan(
            0,
            count($occurrences),
            'Expected at least one reference occurrence for OrderRepository in OrderService'
        );

        // Verify at least one occurrence is near the constructor (lines 19-24)
        $foundInConstructor = false;
        foreach ($occurrences as $occ) {
            $line = ($occ['range'][0] ?? -1) + 1; // Convert 0-indexed to 1-indexed
            if ($line >= 19 && $line <= 25) {
                $foundInConstructor = true;
                break;
            }
        }

        $this->assertTrue(
            $foundInConstructor,
            'Expected OrderRepository reference in constructor parameter type hint (lines 19-25)'
        );
    }

    /**
     * Verifies that interface type hints create reference occurrences.
     *
     * Code reference: src/Service/OrderService.php:21
     *   private EmailSenderInterface $emailSender,
     *
     * Expected: SCIP contains a reference occurrence for EmailSenderInterface.
     */
    #[ContractTest(
        name: 'Interface type hint creates SCIP occurrence',
        description: 'Verifies interface type hints create reference occurrences. Tests OrderService.$emailSender typed as EmailSenderInterface.',
        category: 'scip',
    )]
    public function testInterfaceTypeHintCreatesOccurrence(): void
    {
        $this->requireScipData();

        // Find reference occurrences for EmailSenderInterface
        $occurrences = $this->scip()->occurrences()
            ->symbolContains('EmailSenderInterface#')
            ->isReference()
            ->inFile('Service/OrderService.php')
            ->all();

        $this->assertGreaterThan(
            0,
            count($occurrences),
            'Expected at least one reference occurrence for EmailSenderInterface in OrderService'
        );
    }

    /**
     * Verifies that parameter type hints create reference occurrences.
     *
     * Code reference: src/Repository/OrderRepository.php:26
     *   public function save(Order $order): Order
     *
     * Expected: SCIP contains a reference for Order at parameter position.
     */
    #[ContractTest(
        name: 'Parameter type hint creates SCIP occurrence',
        description: 'Verifies method parameter type hints create reference occurrences. Tests OrderRepository.save() parameter typed as Order.',
        category: 'scip',
    )]
    public function testParameterTypeHintCreatesOccurrence(): void
    {
        $this->requireScipData();

        // Find reference occurrences for Order class in OrderRepository
        $occurrences = $this->scip()->occurrences()
            ->symbolContains('Order#')
            ->isReference()
            ->inFile('Repository/OrderRepository.php')
            ->all();

        // Filter to Order entity, not OrderRepository
        $orderEntityOccurrences = array_filter($occurrences, function ($occ) {
            $symbol = $occ['symbol'] ?? '';
            // Match Order# but not OrderRepository# or OrderOutput#
            return preg_match('/Entity\/Order#/', $symbol) === 1;
        });

        $this->assertGreaterThan(
            0,
            count($orderEntityOccurrences),
            'Expected at least one reference occurrence for Order entity in OrderRepository'
        );
    }

    /**
     * Verifies that return type hints create reference occurrences.
     *
     * Code reference: src/Repository/OrderRepository.php:26
     *   public function save(Order $order): Order
     *
     * Expected: SCIP contains a reference for Order at return type position.
     */
    #[ContractTest(
        name: 'Return type hint creates SCIP occurrence',
        description: 'Verifies method return type hints create reference occurrences. Tests OrderRepository.save() return type Order.',
        category: 'scip',
    )]
    public function testReturnTypeHintCreatesOccurrence(): void
    {
        $this->requireScipData();

        // The save method on line 26 has both parameter and return type Order
        // We expect at least 2 references (parameter + return) for Order in that file
        $occurrences = $this->scip()->occurrences()
            ->symbolContains('Entity/Order#')
            ->isReference()
            ->inFile('Repository/OrderRepository.php')
            ->all();

        // Should have at least 2 references (one for param, one for return, possibly more for body)
        $this->assertGreaterThanOrEqual(
            2,
            count($occurrences),
            'Expected at least 2 Order references in OrderRepository (parameter and return type)'
        );
    }

    /**
     * Verifies multiple type hints in same method signature are tracked.
     *
     * Code reference: src/Service/OrderService.php:27
     *   public function createOrder(CreateOrderInput $input): OrderOutput
     *
     * Expected: Both CreateOrderInput and OrderOutput have reference occurrences.
     */
    #[ContractTest(
        name: 'Multiple type hints in same method',
        description: 'Verifies both parameter and return types create separate reference occurrences. Tests createOrder(CreateOrderInput): OrderOutput.',
        category: 'scip',
    )]
    public function testMultipleTypeHintsInSameMethod(): void
    {
        $this->requireScipData();

        // Check CreateOrderInput references
        $inputOccurrences = $this->scip()->occurrences()
            ->symbolContains('CreateOrderInput#')
            ->isReference()
            ->inFile('Service/OrderService.php')
            ->all();

        $this->assertGreaterThan(
            0,
            count($inputOccurrences),
            'Expected CreateOrderInput reference in OrderService'
        );

        // Check OrderOutput references
        $outputOccurrences = $this->scip()->occurrences()
            ->symbolContains('OrderOutput#')
            ->isReference()
            ->inFile('Service/OrderService.php')
            ->all();

        $this->assertGreaterThan(
            0,
            count($outputOccurrences),
            'Expected OrderOutput reference in OrderService'
        );
    }

    /**
     * Verifies nullable type hints are tracked.
     *
     * Code reference: src/Repository/OrderRepository.php:21
     *   public function findById(int $id): ?Order
     *
     * Expected: Order reference exists for nullable return type.
     */
    #[ContractTest(
        name: 'Nullable type hint creates SCIP occurrence',
        description: 'Verifies nullable return types (?Order) create reference occurrences for the base type.',
        category: 'scip',
    )]
    public function testNullableTypeHintCreatesOccurrence(): void
    {
        $this->requireScipData();

        // Find Order references around the findById method (line 21-24)
        $occurrences = $this->scip()->occurrences()
            ->symbolContains('Entity/Order#')
            ->isReference()
            ->inFile('Repository/OrderRepository.php')
            ->betweenLines(21, 24)
            ->all();

        $this->assertGreaterThan(
            0,
            count($occurrences),
            'Expected Order reference for nullable return type ?Order in findById'
        );
    }
}
