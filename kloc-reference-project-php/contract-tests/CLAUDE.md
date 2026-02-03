# Contract Tests

Contract testing framework for validating scip-php calls.json output.

## Quick Start

```bash
# Run all tests (generates fresh index + runs in Docker)
bin/run.sh test

# Run with experimental call kinds (function, access_array, operators)
bin/run.sh test --experimental

# Run specific test by name
bin/run.sh test --filter testOrderRepository

# Run specific test suite
bin/run.sh test --suite smoke

# Generate documentation
bin/run.sh docs

# Show all options
bin/run.sh help
```

The script handles everything:
1. Generates fresh `calls.json` using scip-php on the host
2. Builds the Docker image (PHP 8.4)
3. Runs PHPUnit tests or generates documentation in the container

## Prerequisites

- Docker and docker-compose
- scip-php Docker image built: `cd ../../scip-php && ./build/build.sh`

## Commands Reference

| Command | Description |
|---------|-------------|
| `bin/run.sh test` | Run all tests with fresh index |
| `bin/run.sh test --experimental` | Run with experimental call kinds enabled |
| `bin/run.sh test --filter <name>` | Run specific test by name/pattern |
| `bin/run.sh test --suite <name>` | Run specific test suite |
| `bin/run.sh docs` | Generate markdown documentation |
| `bin/run.sh docs --format=json` | Generate JSON documentation |
| `bin/run.sh docs --format=csv` | Generate CSV documentation |
| `bin/run.sh docs --output=FILE` | Write documentation to file |

Test suites: `smoke`, `integrity`, `reference`, `chain`, `argument`

## Call Kinds

The indexer produces two categories of call kinds:

**Stable (always generated):**
- `access` - Property access (`$obj->property`)
- `method` - Method call (`$obj->method()`)
- `constructor` - Object instantiation (`new Foo()`)
- `access_static` - Static property access (`Foo::$property`)
- `method_static` - Static method call (`Foo::method()`)

**Experimental (require `--experimental` flag):**
- `function` - Function call (`sprintf()`)
- `access_array` - Array access (`$arr['key']`)
- `coalesce` - Null coalesce operator (`$a ?? $b`)
- `ternary` - Short ternary (`$a ?: $b`)
- `ternary_full` - Full ternary (`$a ? $b : $c`)
- `match` - Match expression (`match($x) { ... }`)

## ContractTest Attribute

Every test method MUST use the `#[ContractTest]` attribute for documentation generation:

```php
use ContractTests\Attribute\ContractTest;

#[ContractTest(
    name: 'OrderRepository::save() $order',
    description: 'Verifies $order parameter has single value entry. Per spec, each parameter should have one value entry at declaration.',
    category: 'reference',
)]
public function testOrderRepositorySaveOrderParameter(): void
```

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `name` | string | Yes | Human-readable test name shown in docs |
| `description` | string | Yes | What the test verifies (detailed) |
| `category` | string | No | Test category: `smoke`, `integrity`, `reference`, `chain`, `argument`, `callkind`, `operator` (auto-detected from class if omitted) |
| `status` | string | No | `active` (default), `skipped`, `pending` |
| `experimental` | bool | No | If `true`, test only runs with `--experimental` flag |

**Note**: Code reference (`ClassName::methodName`) is generated automatically via reflection.

## Documentation Output

Generated documentation includes:
- Summary table (passed/failed/skipped/error counts)
- Tests grouped by category with live execution status
- Failed test details with error messages
- Auto-generated code reference as `ClassName::methodName`

## Key Rules

1. **Always reference kloc-reference-project-php code** in test descriptions
2. **Include file:line references** in docblock comments for context
3. **Tests run once** - index is generated before tests, not per-test
4. **Docker only** - always run tests via Docker for consistent environment
5. **Use #[ContractTest] attribute** - required for documentation generation

## Documentation References

- **Overview**: `docs/reference/kloc-scip/contract-tests/README.md`
- **API Reference**: `docs/reference/kloc-scip/contract-tests/framework-api.md`
- **Test Categories**: `docs/reference/kloc-scip/contract-tests/test-categories.md`
- **Writing Tests**: `docs/reference/kloc-scip/contract-tests/writing-tests.md`
- **Calls Schema**: `docs/reference/kloc-scip/calls-and-data-flow.md`

## Directory Structure

```
contract-tests/
  bin/
    run.sh                  # Main entry point - run this
    generate-docs.php       # Generate test documentation (internal)
  src/
    Attribute/
      ContractTest.php          # Test metadata attribute
    CallsContractTestCase.php   # Base test class
    CallsData.php               # JSON wrapper
    Query/
      ValueQuery.php            # Value queries
      CallQuery.php             # Call queries
      MethodScope.php           # Scoped queries
    Assertions/
      ReferenceConsistencyAssertion.php  # Category 1
      ChainIntegrityAssertion.php        # Category 2
      ArgumentBindingAssertion.php       # Category 3
      DataIntegrityAssertion.php         # Category 4
  tests/
    SmokeTest.php               # Acceptance tests
    Integrity/                  # Category 4 tests
    Reference/                  # Category 1 tests
    Chain/                      # Category 2 tests
    Argument/                   # Category 3 tests
  output/
    calls.json                  # Generated index (gitignored)
    junit.xml                   # PHPUnit results (gitignored)
```

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
