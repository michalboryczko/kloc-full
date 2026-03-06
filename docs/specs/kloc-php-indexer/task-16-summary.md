# Task 16 Summary: Full Pipeline Integration & Output Parity

**Status:** COMPLETE (subtasks 16.1, 16.2, 16.5 done; 16.3, 16.4 skipped — no PHP binary available)
**Date:** 2026-03-06

## What Was Implemented

### Subtask 16.1 & 16.2: CLI Full Pipeline + JSON Output Writer

#### src/main.rs (modified)

- `--output-dir` changed from required with default `.` to optional, defaults to `<project-root>/.kloc/`
- Added `--quiet` / `-q` flag that suppresses all output except errors
- `verbose` now computed as `args.verbose && !args.quiet`
- **index.json v4.0 output**: metadata version `"4.0"`, tool name `"scip-php"` (matches PHP implementation), `project_root` as `file://` URI with trailing slash, `"language": "PHP"` in each document
- Verbose mode prints output directory path after completion

### Subtask 16.5: Edge Case Hardening

#### src/pipeline.rs (modified)

- **BOM stripping in `collect_types_parallel()`**: Strips `\u{FEFF}` (UTF-8 BOM) from string content before parsing
- **BOM stripping in `index_files_parallel()`**: Strips `0xEF 0xBB 0xBF` byte sequence from raw bytes before UTF-8 conversion
- 6 new edge case tests:
  - `test_empty_file` — empty PHP file produces no occurrences
  - `test_syntax_error_no_panic` — syntax errors don't panic, return Ok
  - `test_bom_handling` — UTF-8 BOM files are parsed correctly
  - `test_php80_features` — nullsafe operator, match expression, nullable types
  - `test_php81_enum` — backed enums with methods
  - `test_php82_readonly_class` — readonly class with promoted constructor params

### Subtasks 16.3, 16.4: Skipped

- Full snapshot test against reference project and scip-php itself require a PHP binary to generate reference output
- No PHP binary is available in the Rust build environment
- These will be validated when the Rust binary is integrated into the kloc pipeline

## Test Results

- 499 unit tests passed (493 from Task 15 + 6 new edge case tests)
- 3 integration tests passed, 1 ignored
- Build: zero warnings, clean compile

## Key Design Decisions

- `--output-dir` defaults to `.kloc/` inside project root (matches kloc pipeline convention)
- Tool name in metadata is `"scip-php"` (not `"scip-php-rust"`) — drop-in replacement for PHP implementation
- BOM handling done at both string level (type collection) and byte level (indexing) since the two phases read files differently
- `--quiet` flag overrides `--verbose` when both specified
