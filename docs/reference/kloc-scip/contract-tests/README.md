# Contract Tests Framework

A PHPUnit-based testing framework for validating `calls.json` output from the `scip-php` indexer against expected behavior.

## Purpose

The contract tests framework ensures that the `scip-php` indexer produces correct, consistent output by testing against real PHP code in `kloc-reference-project-php/src/`.

## Quick Start

```bash
# Navigate to contract-tests directory
cd kloc-reference-project-php/contract-tests

# Run all tests (generates fresh index + runs in Docker)
bin/run.sh test

# Run specific test by name
bin/run.sh test --filter testOrderRepository

# Run specific test suite
bin/run.sh test --suite smoke

# Generate documentation
bin/run.sh docs

# Show all options
bin/run.sh help
```

The script handles everything: generates fresh `calls.json`, builds Docker image, runs tests.

## Test Categories

| Category | Description | Test Class |
|----------|-------------|------------|
| Smoke | Basic validation that framework works | `SmokeTest` |
| Integrity | Data structure validation | `DataIntegrityTest` |
| Reference | Variable reference consistency | `ParameterReferenceTest` |
| Chain | Call chain linkage | `ChainIntegrityTest` |
| Argument | Argument binding correctness | `ArgumentBindingTest` |

## Key Concepts

### Values

Values are data holders in the `calls.json` output:
- **parameter**: Method/function parameters
- **local**: Local variables
- **literal**: String, int, array literals
- **constant**: Class and global constants
- **result**: Return values from calls

### Calls

Calls are operations in the code:
- **method**: Instance method calls (`$obj->method()`)
- **method_static**: Static method calls (`Foo::method()`)
- **constructor**: Object creation (`new Foo()`)
- **function**: Function calls (`strlen()`)
- **access**: Property access (`$obj->prop`)

### ID Linkage

Each entry has a unique ID based on source position (`file:line:col`). Key linkages:
- `receiver_value_id`: Points from call to value being called on
- `source_call_id`: Points from result value to its source call
- `value_id` (in arguments): Points from argument to value being passed

## Running Specific Tests

```bash
# Run by test suite
bin/run.sh test --suite smoke
bin/run.sh test --suite integrity
bin/run.sh test --suite reference
bin/run.sh test --suite chain
bin/run.sh test --suite argument

# Run single test by name
bin/run.sh test --filter testOrderRepositorySaveParameterExists
```

## ContractTest Attribute

Every test method MUST use the `#[ContractTest]` attribute:

```php
use ContractTests\Attribute\ContractTest;

#[ContractTest(
    name: 'OrderRepository::save() $order',
    description: 'Verifies $order parameter has single value entry',
    category: 'reference',
)]
public function testOrderRepositorySaveOrderParameter(): void
```

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `name` | string | Yes | Human-readable test name |
| `description` | string | Yes | What the test verifies |
| `category` | string | No | `smoke`, `integrity`, `reference`, `chain`, `argument` |
| `status` | string | No | `active` (default), `skipped`, `pending` |

**Note**: Code reference (`ClassName::methodName`) is auto-generated via reflection.

## Generating Documentation

```bash
# Generate markdown documentation
bin/run.sh docs

# Generate JSON format
bin/run.sh docs --format=json

# Generate CSV format
bin/run.sh docs --format=csv

# Write to file
bin/run.sh docs --output=TESTS.md
```

## Configuration

Configuration is in `config.php`:

| Variable | Default | Description |
|----------|---------|-------------|
| `output_dir` | `./output` | Directory containing generated index |
| `calls_json` | `calls.json` | Name of calls.json file |

**Note:** Index generation is handled by `bin/run.sh` using the scip-php Docker image.
Tests only read the pre-generated `calls.json` file.

## Documentation

- [Framework API](./framework-api.md) - Query and assertion API reference
- [Test Categories](./test-categories.md) - Detailed category documentation
- [Writing Tests](./writing-tests.md) - How to write new contract tests

## References

- [Calls Schema](../calls-and-data-flow.md) - Full specification
- [Schema Documentation](../calls-schema-docs.md) - Quick reference
