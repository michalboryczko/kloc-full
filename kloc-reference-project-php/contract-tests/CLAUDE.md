# Contract Tests

Contract testing framework for validating scip-php calls.json output.

## Quick Start

```bash
# Run the complete test flow (generates index + runs tests in Docker)
./run-tests.sh
```

That's it. The script handles everything:
1. Generates `calls.json` using scip-php on the host
2. Builds the Docker image (PHP 8.4)
3. Runs PHPUnit tests in the container

## Prerequisites

- Docker and docker-compose
- scip-php binary built at `../../scip-php/build/scip-php`

## Generate Documentation

Generate test documentation with live execution status:

```bash
# Run tests + generate markdown documentation
docker compose run --rm -e SKIP_INDEX_GENERATION=1 contract-tests \
  php bin/generate-docs.php

# Generate JSON format (for tooling)
docker compose run --rm -e SKIP_INDEX_GENERATION=1 contract-tests \
  php bin/generate-docs.php --format=json

# Generate CSV format
docker compose run --rm -e SKIP_INDEX_GENERATION=1 contract-tests \
  php bin/generate-docs.php --format=csv

# Use cached results (skip running tests)
docker compose run --rm -e SKIP_INDEX_GENERATION=1 contract-tests \
  php bin/generate-docs.php --skip-tests

# Write to file
docker compose run --rm -e SKIP_INDEX_GENERATION=1 contract-tests \
  php bin/generate-docs.php --output=TESTS.md
```

Output includes:
- Summary table (passed/failed/skipped/error counts)
- Tests grouped by category with live execution status
- Failed test details with error messages

## Manual Docker Commands

```bash
# If you already have calls.json in output/
docker compose build
docker compose run --rm -e SKIP_INDEX_GENERATION=1 contract-tests

# Run specific test suite
docker compose run --rm -e SKIP_INDEX_GENERATION=1 contract-tests \
  vendor/bin/phpunit --testsuite=smoke

# Run single test
docker compose run --rm -e SKIP_INDEX_GENERATION=1 contract-tests \
  vendor/bin/phpunit --filter testOrderRepositorySaveParameterExists
```

## Documentation

All framework documentation is in the main repository:

- **Overview**: `docs/reference/kloc-scip/contract-tests/README.md`
- **API Reference**: `docs/reference/kloc-scip/contract-tests/framework-api.md`
- **Test Categories**: `docs/reference/kloc-scip/contract-tests/test-categories.md`
- **Writing Tests**: `docs/reference/kloc-scip/contract-tests/writing-tests.md`

## Specification References

- **Calls Schema**: `docs/reference/kloc-scip/calls-and-data-flow.md`
- **Schema Docs**: `docs/reference/kloc-scip/calls-schema-docs.md`

## Key Rules

1. **Always reference kloc-reference-project-php code** in test descriptions
2. **Include file:line references** for the code being tested
3. **Tests run once** - index is generated before tests, not per-test
4. **Docker only** - always run tests via Docker for consistent environment
5. **Use #[ContractTest] attribute** - for documentation generation

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `SCIP_PHP_BINARY` | `../../scip-php/build/scip-php` | Path to scip-php binary (host) |
| `SKIP_INDEX_GENERATION` | - | Skip regeneration, use existing calls.json |
| `FORCE_INDEX_GENERATION` | - | Force regeneration even if exists |

## Directory Structure

```
contract-tests/
  run-tests.sh              # Main entry point - run this
  Dockerfile                # PHP 8.4 image
  docker-compose.yml        # Container config
  bin/
    generate-docs.php       # Generate test documentation
  src/
    Attribute/
      ContractTest.php      # Test metadata attribute
    CallsContractTestCase.php   # Base test class
    CallsData.php               # JSON wrapper
    Query/
      ValueQuery.php            # Value queries
      CallQuery.php             # Call queries
      MethodScope.php           # Scoped queries
    Assertions/
      ReferenceConsistencyAssertion.php  # Category 1
      ChainIntegrityAssertion.php        # Category 2
      ChainVerificationResult.php
      ArgumentBindingAssertion.php       # Category 3
      DataIntegrityAssertion.php         # Category 4
      IntegrityReport.php
    Setup/
      IndexGenerator.php        # Runs scip-php
  tests/
    bootstrap.php               # One-time index generation
    SmokeTest.php               # Acceptance tests
    Integrity/                  # Category 4 tests
    Reference/                  # Category 1 tests
    Chain/                      # Category 2 tests
    Argument/                   # Category 3 tests
  output/
    calls.json                  # Generated index (gitignored)
    junit.xml                   # PHPUnit results (gitignored)
    test-metadata.json          # Cached test metadata (gitignored)
```

## Test Suites

| Suite | Description |
|-------|-------------|
| `smoke` | Critical acceptance tests (must pass) |
| `integrity` | Data integrity checks (duplicates, orphans) |
| `reference` | Parameter/local reference consistency |
| `chain` | Method chain linkage verification |
| `argument` | Argument binding validation |

## Writing Tests

Tests must:
1. Extend `CallsContractTestCase`
2. Use `#[ContractTest]` attribute for metadata
3. Include docblock with detailed description
4. Reference code in `../src/`

```php
<?php

namespace ContractTests\Tests\Reference;

use ContractTests\Attribute\ContractTest;
use ContractTests\CallsContractTestCase;

class ParameterReferenceTest extends CallsContractTestCase
{
    /**
     * Verifies $order parameter in save() has exactly one value entry.
     * Per the spec, each parameter should have a single value entry at
     * declaration, with all usages referencing that entry.
     *
     * Code reference: src/Repository/OrderRepository.php:26
     *   public function save(Order $order): Order
     */
    #[ContractTest(
        name: 'OrderRepository::save() $order',
        description: 'Verifies $order parameter has single value entry',
        codeRef: 'src/Repository/OrderRepository.php:26',
        category: 'reference',
    )]
    public function testOrderRepositorySaveOrderParameter(): void
    {
        $result = $this->assertReferenceConsistency()
            ->inMethod('App\Repository\OrderRepository', 'save')
            ->forParameter('$order')
            ->verify();

        $this->assertTrue($result->success);
    }
}
```

## ContractTest Attribute

The `#[ContractTest]` attribute provides metadata for documentation generation:

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `name` | string | Yes | Human-readable test name |
| `description` | string | Yes | What the test verifies |
| `codeRef` | string | No | Code reference (file:line) |
| `category` | string | No | Test category (auto-detected from class if omitted) |
| `status` | string | No | Declared status: `active`, `skipped`, `pending` (default: `active`) |
