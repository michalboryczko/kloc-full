# Contract Tests Framework

A PHPUnit-based testing framework for validating `calls.json` output from the `scip-php` indexer against expected behavior.

## Purpose

The contract tests framework ensures that the `scip-php` indexer produces correct, consistent output by testing against real PHP code in `kloc-reference-project-php/src/`.

## Quick Start

```bash
# Navigate to contract-tests directory
cd kloc-reference-project-php/contract-tests

# Option 1: Run with Docker (recommended for CI)
docker compose build
docker compose run --rm contract-tests

# Option 2: Run locally (requires PHP 8.3+ and scip-php binary)
composer install

# Generate index first
../../scip-php/build/scip-php ..

# Move calls.json to output directory
mv ../calls.json output/

# Run tests
vendor/bin/phpunit
```

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
vendor/bin/phpunit --testsuite=smoke
vendor/bin/phpunit --testsuite=integrity
vendor/bin/phpunit --testsuite=reference
vendor/bin/phpunit --testsuite=chain
vendor/bin/phpunit --testsuite=argument

# Run single test file
vendor/bin/phpunit tests/SmokeTest.php

# Run single test method
vendor/bin/phpunit --filter testOrderRepositorySaveParameterExists
```

## Configuration

Configuration is in `config.php`:

| Variable | Default | Description |
|----------|---------|-------------|
| `scip_binary` | `../../scip-php/build/scip-php` | Path to scip-php binary |
| `project_root` | `../` | Path to PHP project to index |
| `output_dir` | `./output` | Where to write generated index |

Environment variables override config file values:
- `SCIP_PHP_BINARY`
- `PROJECT_ROOT`
- `OUTPUT_DIR`
- `SKIP_INDEX_GENERATION` - Skip regeneration if calls.json exists
- `FORCE_INDEX_GENERATION` - Force regeneration even if exists

## Documentation

- [Framework API](./framework-api.md) - Query and assertion API reference
- [Test Categories](./test-categories.md) - Detailed category documentation
- [Writing Tests](./writing-tests.md) - How to write new contract tests

## References

- [Calls Schema](../calls-and-data-flow.md) - Full specification
- [Schema Documentation](../calls-schema-docs.md) - Quick reference
