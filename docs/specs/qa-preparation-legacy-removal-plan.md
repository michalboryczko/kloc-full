# Implementation Plan: QA Preparation Legacy Removal

## Overview

Remove backward-compatibility legacy code from kloc-mapper and scip-php. WIP commits already exist. This plan covers verification, test fixes, documentation updates, dependency cleanup, and clean commits.

## Codebase Exploration Findings

### kloc-mapper -- Remaining Legacy References

| File | Line(s) | Reference | Action |
|------|---------|-----------|--------|
| `CLAUDE.md` | 22-47, 71, 98 | .kloc/.scip input, protobuf, archive.py | REWRITE |
| `README.md` | 7, 33-67, 144-193 | .kloc/.scip input, protobuf, archive.py, scip_pb2.py | REWRITE |
| `build.sh` | 43, 84 | `--collect-all protobuf` | REMOVE lines |
| `pyproject.toml` | 11 | `protobuf>=4.0.0` dependency | REMOVE |
| `src/json_parser.py` | 4-5 | Comment referencing protobuf/archive.py | UPDATE comment |
| `tests/test_mapper.py` | 11, 15, 22 | Uses `SCIPMapper(SCIP_PATH)` without `index` param | FIX test |

### scip-php -- Remaining Legacy References

| File | Line(s) | Reference | Action |
|------|---------|-----------|--------|
| `CLAUDE.md` | 25, 41 | `calls.json`, `index.kloc` output files | REWRITE |
| `README.md` | 3, 9-10, 41-42, 57-99 | calls.json, index.kloc output | REWRITE |
| `src/DocIndexer.php` | Various | Comments referencing "calls.json" | KEEP (data concept still valid in unified JSON) |
| `src/Indexer.php` | 154-155 | PHPDoc referencing index.scip | KEEP (still produces index.scip internally) |

### kloc-contracts

| File | Line(s) | Reference | Action |
|------|---------|-----------|--------|
| `scip-php-output.json` | 33 | "protobuf" in description | UPDATE |

## Phased Implementation

### Phase 1: kloc-mapper Verification & Fixes (developer-1)

- [ ] Fix `tests/test_mapper.py` -- update `SCIPMapper(SCIP_PATH)` call to provide `index` parameter or skip test properly
- [ ] Run `uv run pytest tests/ -v` and verify all tests pass
- [ ] Remove `protobuf>=4.0.0` from `pyproject.toml`
- [ ] Remove `--collect-all protobuf` from `build.sh` (lines 43, 84)
- [ ] Update `CLAUDE.md` -- rewrite CLI usage, input formats, build section
- [ ] Update `README.md` -- rewrite to reflect JSON-only input
- [ ] Update `src/json_parser.py` docstring (lines 4-5) to remove legacy references
- [ ] Run tests again after all changes

### Phase 2: scip-php Verification & Fixes (developer-2)

- [ ] Verify `bin/scip-php` runs without errors (no missing class references)
- [ ] Run scip-php tests and verify all pass
- [ ] Update `CLAUDE.md` -- remove `calls.json`, `index.kloc` output references
- [ ] Update `README.md` -- rewrite to reflect unified JSON output
- [ ] Run tests again after all changes

### Phase 3: Cross-project Cleanup

- [ ] Update `kloc-contracts/scip-php-output.json` description
- [ ] Final grep across both projects for remaining legacy references
- [ ] Verify no import/require statements reference deleted files

### Phase 4: Clean Commits

- [ ] kloc-mapper: Amend WIP commit or add clean commit on top
- [ ] scip-php: Amend WIP commit or add clean commit on top

## File Manifest

| Action | File Path | Owner |
|--------|-----------|-------|
| MODIFY | `kloc-mapper/tests/test_mapper.py` | developer-1 |
| MODIFY | `kloc-mapper/pyproject.toml` | developer-1 |
| MODIFY | `kloc-mapper/build.sh` | developer-1 |
| MODIFY | `kloc-mapper/CLAUDE.md` | developer-1 |
| MODIFY | `kloc-mapper/README.md` | developer-1 |
| MODIFY | `kloc-mapper/src/json_parser.py` | developer-1 |
| MODIFY | `scip-php/CLAUDE.md` | developer-2 |
| MODIFY | `scip-php/README.md` | developer-2 |
| MODIFY | `kloc-contracts/scip-php-output.json` | developer-1 |

## File Ownership

| Developer | Files | Rationale |
|-----------|-------|-----------|
| developer-1 | kloc-mapper/*, kloc-contracts/* | Python/build changes |
| developer-2 | scip-php/* | PHP changes |

## Test Cases

1. `uv run pytest kloc-mapper/tests/ -v` -- all tests pass
2. `uv run kloc-mapper map input.json -o out.json` -- works
3. `uv run kloc-mapper map input.kloc -o out.json` -- rejects with error
4. scip-php PHPUnit -- all tests pass
5. Grep for `archive.py`, `scip_pb2`, `parse_scip_file` in kloc-mapper/src/ -- no matches
6. Grep for `ArchiveWriter`, `CallsWriter` in scip-php/src/ -- no matches
7. No `--collect-all protobuf` in build.sh
8. No `protobuf` in pyproject.toml dependencies

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| test_mapper.py uses deleted protobuf parsing | Tests fail | Fix fixture to use JSON parser or skip test |
| Removing protobuf dep breaks uv.lock | Build fails | Regenerate lock file after removing dep |
| README changes lose useful information | Developer confusion | Preserve architectural explanation, just update formats |
