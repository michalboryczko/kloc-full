# Contract Tests

Contract testing framework for validating scip-php calls.json output.

## Quick Start

```bash
# Build and run tests with Docker
docker compose build
docker compose run --rm contract-tests

# Or without Docker (requires PHP 8.3+)
composer install
# Generate index first
../../scip-php/build/scip-php ..
mv ../calls.json output/
vendor/bin/phpunit
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
3. **Tests run once** - index is generated in bootstrap.php, not per-test
4. **Configuration** via config.php or environment variables

## Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `SCIP_PHP_BINARY` | `../../scip-php/build/scip-php` | Path to scip-php binary |
| `PROJECT_ROOT` | `../` | Path to kloc-reference-project-php root |
| `OUTPUT_DIR` | `./output` | Where to write generated index |
| `SKIP_INDEX_GENERATION` | - | Skip regeneration if calls.json exists |
| `FORCE_INDEX_GENERATION` | - | Force regeneration even if exists |

## Directory Structure

```
contract-tests/
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

## Running Tests

```bash
# All tests
vendor/bin/phpunit

# By test suite
vendor/bin/phpunit --testsuite=smoke
vendor/bin/phpunit --testsuite=integrity
vendor/bin/phpunit --testsuite=reference
vendor/bin/phpunit --testsuite=chain
vendor/bin/phpunit --testsuite=argument

# Single test
vendor/bin/phpunit --filter testOrderRepositorySaveParameterExists
```

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
