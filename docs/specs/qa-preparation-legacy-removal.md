# Feature: QA Preparation Legacy Removal

## Goal

Remove all backward-compatibility legacy code from kloc-mapper and scip-php that was preserved during the qa-preparation feature implementation. The pipeline now uses unified JSON exclusively (scip-php produces `index.json`, kloc-mapper consumes `index.json`). Legacy `.kloc` archive, `.scip` protobuf, and `calls.json`/`index.kloc` output paths are no longer needed.

## Context

All 5 qa-preparation pipeline issues (ISSUE-01 through ISSUE-15) have been implemented on the `feature/qa-preparation` branch. WIP commits already removed most legacy code:

- **kloc-mapper** (commit `7ca3872`): Deleted `archive.py`, `scip_pb2.py`. Modified `cli.py`, `mapper.py`, `parser.py`.
- **scip-php** (commit `fe6aebe`): Deleted `ArchiveWriter.php`, `CallsWriter.php`, and their tests. Modified `Indexer.php`, `bin/scip-php`, `bin/scip-php.sh`.

## What Was Removed

### kloc-mapper
| File | Action | Detail |
|------|--------|--------|
| `src/archive.py` | DELETED | .kloc ZIP archive loader |
| `src/scip_pb2.py` | DELETED | Protobuf bindings for SCIP |
| `src/cli.py` | MODIFIED | Removed .kloc/.scip input handlers, JSON-only |
| `src/mapper.py` | MODIFIED | Removed parse_scip_file import, made `index` param required |
| `src/parser.py` | MODIFIED | Removed `parse_scip_file` function and protobuf import |

### scip-php
| File | Action | Detail |
|------|--------|--------|
| `src/Calls/ArchiveWriter.php` | DELETED | .kloc ZIP archive creator |
| `src/Calls/CallsWriter.php` | DELETED | Standalone calls.json writer |
| `tests/Calls/ArchiveWriterTest.php` | DELETED | Tests for archive writer |
| `tests/Calls/CallsWriterTest.php` | DELETED | Tests for calls writer |
| `src/Indexer.php` | MODIFIED | Removed writeCallsAndArchive method |
| `bin/scip-php` | MODIFIED | Removed protobuf write, uses writeUnifiedJson only |
| `bin/scip-php.sh` | MODIFIED | Removed legacy output references |

## What Still Needs Verification and Updates

### 1. Code Verification
- Verify kloc-mapper runs without import errors
- Verify scip-php runs without class-not-found errors

### 2. Test Fixes
- `kloc-mapper/tests/test_mapper.py` calls `SCIPMapper(SCIP_PATH)` without `index` parameter -- will fail since `index` is now required
- Run all kloc-mapper tests, fix any breakage
- Run all scip-php tests, fix any breakage

### 3. Documentation Updates
- `kloc-mapper/CLAUDE.md` -- references .kloc/.scip input formats, protobuf, archive.py, `--collect-all protobuf`
- `scip-php/CLAUDE.md` -- references `calls.json`, `index.kloc` as output files
- `kloc-mapper/README.md` -- references .kloc/.scip input formats, protobuf, archive.py, scip_pb2.py
- `scip-php/README.md` -- references calls.json and index.kloc output files

### 4. Build Script Cleanup
- `kloc-mapper/build.sh` lines 43, 84: Remove `--collect-all protobuf`
- `kloc-mapper/CLAUDE.md` Docker example: Remove `--collect-all protobuf`

### 5. Dependency Cleanup
- `kloc-mapper/pyproject.toml`: Remove `protobuf>=4.0.0` dependency

### 6. Remaining Legacy References
- `kloc-mapper/src/json_parser.py` lines 4-5: Comment references "protobuf-based" and "archive.py" (informational, low priority)
- `scip-php/src/DocIndexer.php`: Multiple comments reference "calls.json" (these are accurate -- the data still exists, just in unified JSON format)
- `kloc-contracts/scip-php-output.json`: References "protobuf" in description field

## Acceptance Criteria

1. GIVEN kloc-mapper with WIP changes WHEN running `uv run kloc-mapper map input.json -o out.json` THEN it processes JSON input without errors
2. GIVEN kloc-mapper with WIP changes WHEN running `uv run kloc-mapper map input.kloc -o out.json` THEN it prints error "Unsupported input format" and exits
3. GIVEN kloc-mapper tests WHEN running `uv run pytest tests/ -v` THEN all tests pass (after fixing test_mapper.py)
4. GIVEN scip-php with WIP changes WHEN running the indexer THEN it produces unified JSON output without errors
5. GIVEN scip-php tests WHEN running PHPUnit THEN all tests pass
6. GIVEN kloc-mapper/CLAUDE.md WHEN reviewed THEN it contains no references to .kloc archives, .scip files, protobuf, or archive.py
7. GIVEN scip-php/CLAUDE.md WHEN reviewed THEN it contains no references to calls.json output files, index.kloc, ArchiveWriter, or CallsWriter
8. GIVEN kloc-mapper/build.sh WHEN reviewed THEN it contains no `--collect-all protobuf` flags
9. GIVEN kloc-mapper/pyproject.toml WHEN reviewed THEN it does not list `protobuf` as a dependency
10. GIVEN a grep for `parse_scip_file`, `archive.py`, `scip_pb2` across kloc-mapper/src/ WHEN executed THEN returns no matches
11. GIVEN the feature branch WHEN completed THEN clean commits exist (not WIP) on both kloc-mapper and scip-php
