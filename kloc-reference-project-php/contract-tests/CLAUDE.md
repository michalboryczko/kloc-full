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
  src/
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

Tests must extend `CallsContractTestCase` and reference code in `../src/`:

```php
<?php

namespace ContractTests\Tests\Reference;

use ContractTests\CallsContractTestCase;

class ParameterReferenceTest extends CallsContractTestCase
{
    /**
     * Test: OrderRepository::save() - $order parameter
     *
     * Code reference: src/Repository/OrderRepository.php:26
     *   public function save(Order $order): Order
     *
     * Expected: One value entry for $order, all usages reference it.
     */
    public function testOrderParameterHasSingleValueEntry(): void
    {
        $this->assertReferenceConsistency()
            ->inMethod('App\Repository\OrderRepository', 'save')
            ->forParameter('$order')
            ->verify();
    }
}
```
