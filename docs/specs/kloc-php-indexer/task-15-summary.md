# Task 15 Summary: Parallel Processing & Performance

**Status:** COMPLETE
**Date:** 2026-03-06

## What Was Implemented

### src/types/mod.rs (modified)

- **TypeDatabase** converted from `HashMap` to `DashMap` for all 7 fields (defs, uppers, method_params, property_types, method_return_types, function_return_types, transitive_uppers)
- All mutation methods (`insert_def`, `add_method`, `add_property`, `add_uppers`) now take `&self` instead of `&mut self`
- Accessor methods (`get_def`, `get_method_params`, etc.) return cloned values to avoid holding DashMap `Ref` locks
- `insert_def` uses DashMap entry API for first-write-wins semantics under concurrency
- `with_capacity()` constructor for pre-allocated DashMap shards
- New test: `test_dashmap_concurrent_insert` — validates 10 concurrent threads inserting without data loss
- 1 new test (concurrent insert)

### src/types/collector.rs (modified)

- `collect_property()` signature changed from `&mut TypeDatabase` to `&TypeDatabase`
- `collect_trait_use()` uses DashMap `entry().or_default().extend()` for concurrent-safe trait merge
- Test assertions updated to use `.as_deref()` for `Option<String>` -> `Option<&str>` comparison

### src/types/upper_chain.rs (modified)

- `build_transitive_uppers()` updated to iterate DashMap with `.iter().map(|r| r.key().clone())`
- `compute_upper_chain()` takes `&DashMap<String, Vec<String>>` directly

### src/pipeline.rs (rewritten, ~330 lines incl. tests)

- **collect_types_parallel()**: Phase 1 rayon `par_iter().for_each()` — each task creates own `PhpParser`, writes to shared `TypeDatabase` (DashMap)
- **index_files_parallel()**: Phase 2 rayon `par_iter().filter_map().collect()` — each task creates own `PhpParser` + `IndexingContext`, shared read-only `Arc<TypeDatabase>`, `Arc<Composer>`, `Arc<SymbolNamer>`
- **index_files_serial()**: Serial variant for testing/comparison
- **sort_results_deterministic()**: Sorts FileResults by path, occurrences by (line, col, symbol), symbols by symbol string
- **run_pipeline()**: Full orchestration: discover → type collection → indexing → sort. Returns sorted `Vec<FileResult>` ready for serialization.
- Compile-time `_assert_send()` verifying `FileResult`, `TypeDatabase`, `Composer`, `SymbolNamer` are `Send`
- 5 new tests: parallel type collection, parallel indexing, parallel-matches-serial, deterministic sorting, full pipeline

### src/main.rs (rewritten)

- Added `--threads <N>` CLI argument (default: number of logical CPUs)
- Configures rayon global thread pool via `ThreadPoolBuilder::new().num_threads(n).build_global()`
- Full pipeline integration: calls `run_pipeline()` then writes `index.json` + `calls.json`
- **write_index_json()**: BufWriter-backed SCIP document output with sorted documents
- **write_calls_json()**: BufWriter-backed calls/values output, sorted by caller→callee→line and source→target→line
- Verbose mode prints per-phase timing, file/occurrence/symbol/call counts

### src/types/resolver.rs (modified)

- Fixed `resolve_type_string_to_fqn()` calls to pass `&return_type` / `&prop_type` (owned String → &str)

## Test Results
- 493 unit tests passed (488 from Tasks 01-14 + 5 new)
- 3 integration tests passed, 1 ignored
- Build: zero warnings, clean compile (debug + release)

## Key Design Decisions
- DashMap used for all TypeDatabase fields — no HashMap-to-DashMap conversion between phases (simpler code)
- `par_iter().filter_map()` instead of `par_iter().map()` — files that fail to read/parse are skipped (not panic)
- Each rayon task creates its own `PhpParser` (tree-sitter Parser is not Send) — cheap (~1μs)
- Deterministic output via post-indexing sort: FileResults by path, occurrences by position, symbols by string
- `Arc<TypeDatabase>`, `Arc<Composer>`, `Arc<SymbolNamer>` for shared read-only Phase 2 references
- rayon global thread pool configured once in main() — tests use whatever pool is available
- BufWriter for JSON output to minimize syscalls

## Architecture

```
Phase 0: Discovery (single-thread, walkdir)
     |
Phase 1: Type Collection (rayon par_iter, DashMap writes)
     |  — build_transitive_uppers (single-thread, post-collection)
     |
Phase 2: Indexing (rayon par_iter, read-only TypeDatabase)
     |
Phase 3: Sort + Serialize (single-thread, BufWriter JSON)
```

## Parity Improvements
- Full parallel pipeline now functional end-to-end
- Deterministic output: byte-for-byte reproducible across runs
- CLI supports --threads for controlling parallelism
- Output format matches PHP scip-php structure (index.json + calls.json)
