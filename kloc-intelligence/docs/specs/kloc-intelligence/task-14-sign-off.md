# Task 14: Contract Tests & Behavioral Validation -- Sign-Off

## Status: COMPLETE

## Summary

Contract tests, edge case coverage, output format validation, and diff report
tooling have been implemented for kloc-intelligence context queries.

## Test Coverage

### 42/42 Snapshot Tests Pass

All 42 context snapshot cases from `tests/cases.json` pass against the golden
baseline in `tests/snapshot-1802262244.json`. These cover:

- 7 Class targets (Order, OrderService, OrderOutput, Customer, Address, AbstractOrderProcessor)
- 8 Interface targets (OrderRepositoryInterface, CustomerRepositoryInterface, EmailSenderInterface, OrderProcessorInterface, InventoryCheckerInterface, BaseRepositoryInterface)
- 11 Method targets (constructors, service methods, repository methods, abstract methods)
- 9 Property targets (promoted/readonly, regular, foreign-key-like)
- 7 Value targets (parameters, locals, results)

### All NodeKinds Tested

The snapshot corpus covers all major NodeKinds:

| NodeKind | Cases | Example |
|----------|-------|---------|
| Class | 7 | `App\Entity\Order` |
| Interface | 8 | `App\Repository\OrderRepositoryInterface` |
| Method | 11 | `App\Service\OrderService::createOrder()` |
| Property | 9 | `App\Entity\Order::$id` |
| Value | 7 | `App\Service\OrderService::createOrder().$input` |

Entry-level kinds also cover: PropertyGroup, method (lowercase variant).

### Schema Validation Passes

All 42 snapshot outputs validate against the contract schema at
`kloc-contracts/kloc-cli-context.json` using the `jsonschema` library.

Validation checks:
- Required fields: `target`, `maxDepth`, `usedBy`, `uses`
- Target structure: `fqn`, `file`, `line`, optional `signature`
- Entry structure: `depth`, `fqn`, `kind`, `children` (recursive)
- Definition structure: `fqn`, `kind`, optional fields by kind
- camelCase field naming throughout
- No forbidden snake_case keys in output
- All line numbers 1-based (no 0 or negative values)
- No unexpected None values in non-nullable fields

### Edge Cases Covered

| Edge Case | Test File | Description |
|-----------|-----------|-------------|
| Empty results | `test_edge_cases.py` | Interface with minimal implementors |
| Deep depth | `test_edge_cases.py` | depth=5 on short chain stops early |
| Limit enforcement | `test_edge_cases.py` | limit=1 constrains results |
| Constructor redirect | `test_edge_cases.py` | `__construct()` -> Class USED BY |
| Unknown symbol | `test_edge_cases.py` | Raises ValueError cleanly |

### Contract Tests

| Area | Test File | Tests |
|------|-----------|-------|
| Required fields | `test_contract.py` | 6 tests |
| Target structure | `test_contract.py` | 5 tests |
| camelCase naming | `test_contract.py` | 6 tests |
| 1-based line numbers | `test_contract.py` | 5 tests |
| Null field omission | `test_contract.py` | 2 tests |
| Entry structure | `test_contract.py` | 2 tests |
| Bidirectional structure | `test_contract.py` | 2 tests |
| Definition structure | `test_contract.py` | 4 tests |
| JSON Schema validation | `test_contract.py` | 8 tests |

## New Files

| File | Purpose |
|------|---------|
| `tests/test_contract.py` | Contract tests ported from reference project |
| `tests/test_edge_cases.py` | Edge case tests (Neo4j integration) |
| `tests/test_output_format.py` | Output format validation against schema |
| `tests/diff_report.py` | Diff report utility for 42 snapshot cases |

## Diff Report

The diff report utility (`tests/diff_report.py`) can be run against Neo4j to
verify all 42 cases match the golden baseline:

```bash
cd kloc-intelligence
uv run python tests/diff_report.py
```

Expected output: `42/42 (100%)` pass rate.

## Dependencies Added

- `jsonschema>=4.26.0` (dev dependency for schema validation)
