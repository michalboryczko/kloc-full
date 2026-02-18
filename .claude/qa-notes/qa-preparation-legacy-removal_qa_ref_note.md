# QA Notes: QA Preparation Legacy Removal

## Test Scenarios

### Happy Path
- GIVEN kloc-mapper with JSON input WHEN running `uv run kloc-mapper map tests/fixtures/sample.json -o /tmp/out.json` THEN it produces valid SoT JSON output
- GIVEN scip-php with a PHP project WHEN running the indexer THEN it produces `index.json` unified output

### Edge Cases
- GIVEN kloc-mapper WHEN given a `.kloc` input file THEN it prints "Unsupported input format" error and exits with code 1
- GIVEN kloc-mapper WHEN given a `.scip` input file THEN it prints "Unsupported input format" error and exits with code 1

### Error Handling
- GIVEN kloc-mapper with `index` parameter omitted WHEN constructing `SCIPMapper` THEN it raises `ValueError` with message about required index parameter

### Regression
- GIVEN kloc-mapper test suite WHEN running `uv run pytest tests/ -v` THEN all tests pass
- GIVEN scip-php test suite WHEN running PHPUnit THEN all tests pass
- GIVEN scip-php contract tests WHEN running `bin/run.sh test` THEN baseline results are unchanged

### Documentation Accuracy
- GIVEN kloc-mapper/CLAUDE.md WHEN reviewed THEN CLI usage shows only JSON input, no .kloc/.scip
- GIVEN kloc-mapper/CLAUDE.md WHEN reviewed THEN build section has no `--collect-all protobuf`
- GIVEN kloc-mapper/CLAUDE.md WHEN reviewed THEN input formats section describes only unified JSON
- GIVEN scip-php/CLAUDE.md WHEN reviewed THEN output files list shows only `index.json`
- GIVEN scip-php/CLAUDE.md WHEN reviewed THEN no references to ArchiveWriter or CallsWriter

### Build Integrity
- GIVEN kloc-mapper/build.sh WHEN reviewed THEN no `--collect-all protobuf` on any line
- GIVEN kloc-mapper/pyproject.toml WHEN reviewed THEN no protobuf dependency

### Grep Validation (No Remaining Legacy References)
- `grep -r "parse_scip_file" kloc-mapper/src/` returns no matches
- `grep -r "archive\.py" kloc-mapper/src/` returns no matches
- `grep -r "scip_pb2" kloc-mapper/src/` returns no matches
- `grep -r "ArchiveWriter" scip-php/src/` returns no matches
- `grep -r "CallsWriter" scip-php/src/` returns no matches
- `grep -r "writeCallsAndArchive" scip-php/src/` returns no matches

## Automated Test Expectations

### kloc-mapper
- `tests/test_parser.py` -- should pass unchanged (no legacy dependencies)
- `tests/test_models.py` -- should pass unchanged
- `tests/test_mapper.py` -- needs fix for `SCIPMapper()` call, then should pass
- `tests/test_classify_symbol.py` -- should pass unchanged (if exists)

### scip-php
- All PHPUnit tests -- should pass
- Contract tests -- baseline unchanged

## Manual Testing Steps

1. Run kloc-mapper tests: `cd kloc-mapper && uv run pytest tests/ -v`
2. Run scip-php tests: `cd scip-php && docker run --rm -v $(pwd):/app scip-php-dev vendor/bin/phpunit --no-coverage`
3. Verify CLAUDE.md files are accurate by reading them
4. Run grep validation commands listed above
5. Verify build.sh and pyproject.toml are clean
