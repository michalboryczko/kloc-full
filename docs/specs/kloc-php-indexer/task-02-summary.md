# Task 02 Summary: tree-sitter-php Parser Wrapper & Position Handling

**Status:** COMPLETE
**Date:** 2026-03-05

## What Was Implemented

### Subtask 2.1 — Parser Wrapper (src/parser/mod.rs, ~160 lines)
- `PhpParser` struct wrapping tree_sitter::Parser, configured for PHP
- `PhpParser::new()`, `parse()`, `parse_file()` methods
- `ParsedFile` struct with tree, source bytes, path
- `ParsedFile::root()`, `has_errors()`, `source_str()`
- Rayon thread-local usage pattern documented
- 5 unit tests (valid PHP, syntax error, empty file, PHP 8 features, source roundtrip)

### Subtask 2.2 — CST Helper Functions (src/parser/cst.rs, ~260 lines)
- 13 utility functions: node_text, child_by_kind, children_by_kind, named_children, all_children, find_ancestor, find_ancestor_where, has_ancestor, has_child_text, has_named_child, find_all, first_significant_child, preceding_doc_comment
- #[inline] on hot path functions (node_text, has_named_child, has_ancestor)
- 8 unit tests covering all key functions

### Subtask 2.3 — Position and Range Types (src/parser/position.rs, ~130 lines)
- Position { line, col } — 0-indexed, from_ts_point()
- Range { start, end } — from_node(), to_scip_vec() (always 4-element)
- ByteRange — from_node(), extract_text()
- LineOffsetCache — O(n) build, O(log n) byte_to_position() via binary search
- NodeRange extension trait on tree_sitter::Node
- 7 unit tests

### Subtask 2.4 — File Discovery (src/discovery.rs, ~100 lines)
- DiscoveredFiles { project, vendor } with all() method
- discover_php_files() — walkdir, deterministic sort, vendor/project split
- relative_path() — forward-slash normalization
- Hidden directory exclusion, non-PHP exclusion
- 7 unit tests + 1 integration test against reference project

## Files Created/Modified

| File | Lines | Action |
|------|-------|--------|
| src/parser/mod.rs | 160 | Replaced stub |
| src/parser/cst.rs | 260 | New |
| src/parser/position.rs | 130 | New |
| src/discovery.rs | 100 | New |
| src/lib.rs | +1 | Added discovery module |
| Cargo.toml | +2 | Added tempfile dev-dep |
| tests/integration_test.rs | +20 | Added discovery test |

## Test Results
- 30 unit tests passed
- 2 integration tests passed, 1 ignored
- 2 doc tests passed, 2 ignored (no_run examples)

## Deviations from Spec
- tree-sitter-php's program node has `php_tag` as named_child(0), not class_declaration — tests adjusted
- tempfile pinned to =3.14.0 for Rust 1.81 compatibility (getrandom 0.4.2 needs edition2024)

## Known Issues
- Doc test examples marked no_run/ignored (need full project context)
- main.rs still uses its own stub discover_php_files() — should be wired to discovery::discover_php_files() in Task 9+
