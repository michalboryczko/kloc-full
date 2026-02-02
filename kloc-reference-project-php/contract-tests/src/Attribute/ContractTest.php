<?php

declare(strict_types=1);

namespace ContractTests\Attribute;

use Attribute;

/**
 * Metadata attribute for contract tests.
 *
 * Used to generate documentation and track test coverage.
 * Code reference (class::method) is generated dynamically via reflection.
 */
#[Attribute(Attribute::TARGET_METHOD)]
final readonly class ContractTest
{
    /**
     * @param string $name Human-readable test name
     * @param string $description What the test verifies
     * @param string $category Test category (smoke, integrity, reference, chain, argument)
     * @param string $status Test status (active, skipped, pending)
     * @param bool $experimental Whether this test requires --experimental flag
     */
    public function __construct(
        public string $name,
        public string $description,
        public string $category = '',
        public string $status = 'active',
        public bool $experimental = false,
    ) {}
}
