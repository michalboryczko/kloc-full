# Task 05 Summary: Simple Commands -- usages & deps

## What Was Implemented

Complete usages and deps commands for kloc-intelligence: Cypher queries with DFS-based member expansion, JSON output formatting, CLI commands, and snapshot test parity for all 14 usages/deps corpus entries.

### S01: Usages Flat Query
- `src/db/queries/usages.py` with `usages_flat()` function
- `USAGES_DIRECT` Cypher query for incoming USES edges
- `CONTAINS_CHILDREN` query for DFS traversal of container members
- `_get_usages_edges()` implements DFS member expansion matching kloc-cli's `get_usages()` ordering exactly

### S02: Usages Tree Query
- `usages_tree()` function with iterative BFS and global visited set
- Reuses `_get_usages_edges()` for member-expanded usages at each depth level
- Visited set prevents re-visiting nodes across depths (matching kloc-cli's DFS-within-BFS behavior)

### S03: Usages Result Builder
- `src/output/json_formatter.py` with `usages_tree_to_dict()` and `deps_tree_to_dict()`
- `_entry_to_dict()` handles 0-based to 1-based line number conversion
- `_count_tree_nodes()` recursive count for `total` field
- Output format: `{target: {fqn, file}, max_depth, total, tree: [{depth, fqn, file, line, children}]}`

### S04: Deps Flat & Tree
- `src/db/queries/deps.py` with `deps_flat()` and `deps_tree()` functions
- `DEPS_DIRECT` Cypher query for outgoing USES edges
- `_get_deps_edges()` implements DFS member expansion for containers
- Key difference from usages: deps line has NO fallback to target `start_line`

### S05: Console Output & CLI Commands
- `usages` and `deps` CLI commands added to `src/cli.py`
- Both support `--depth/-d`, `--limit/-l`, `--json/-j` options
- Symbol resolution with multi-match handling (prints candidates and exits with code 1)
- Console output shows tree with indentation for non-JSON mode

### S06: Snapshot Tests
- Updated `IMPLEMENTED_COMMANDS` to include "usages" and "deps"
- Updated `execute_query()` dispatch for usages and deps
- All 14 usages/deps snapshot tests pass (zero diffs against kloc-cli)

### Critical Fix: Edge Ordering for Behavioral Parity

The most significant implementation challenge was achieving exact ordering parity with kloc-cli. Two issues were discovered and fixed:

1. **Edge insertion order**: kloc-cli processes edges in sot.json insertion order, not sorted by file/line. Fixed by adding `edge_idx` property to all edges during import, preserving their original position in sot.json. Cypher queries ORDER BY `edge_idx` instead of `loc_file, loc_line`.

2. **DFS member expansion ordering**: kloc-cli's `get_usages()`/`get_deps()` for container types collects member edges via DFS traversal of the contains tree, deduplicating by source/target as it goes. A single Cypher query with `OPTIONAL MATCH (target)-[:CONTAINS*]->(member)` + `WITH DISTINCT` could not replicate this ordering because it interleaved edges from different members by their global `edge_idx`. Fixed by replacing the monolithic Cypher query with Python-side DFS traversal: iterate children in contains edge order, collect USES edges per child, deduplicate source/target across the traversal.

### Import Pipeline Enhancement
- `edge_to_props()` now accepts `idx` parameter for edge ordering
- `parse_sot()` passes enumerate index to each edge
- Import query stores `edge_idx` on all relationship types

## Files Created/Modified
- `kloc-intelligence/src/db/queries/usages.py` (rewritten -- DFS member expansion)
- `kloc-intelligence/src/db/queries/deps.py` (rewritten -- DFS member expansion)
- `kloc-intelligence/src/db/queries/__init__.py` (updated -- exports usages/deps functions)
- `kloc-intelligence/src/output/__init__.py` (existing -- docstring only)
- `kloc-intelligence/src/output/json_formatter.py` (new -- JSON formatting)
- `kloc-intelligence/src/cli.py` (updated -- usages and deps commands)
- `kloc-intelligence/src/db/importer.py` (updated -- edge_idx property)
- `kloc-intelligence/tests/test_snapshot.py` (updated -- usages/deps dispatch)
- `kloc-intelligence/tests/test_usages.py` (new -- 9 unit tests)
- `kloc-intelligence/tests/test_deps.py` (new -- 10 unit tests)
- `kloc-intelligence/tests/test_json_formatter.py` (new -- 9 unit tests)

## Acceptance Criteria Status

### S01: Usages Flat Query
- [x] Direct usages query returns incoming USES edges
- [x] Container types expand to include member usages
- [x] Results ordered by sot.json edge insertion order
- [x] Deduplication by source node_id

### S02: Usages Tree Query
- [x] BFS tree building with global visited set
- [x] Visited set prevents revisiting nodes across depths
- [x] Member expansion at each depth level for container types
- [x] Limit parameter caps total results

### S03: Usages Result Builder
- [x] JSON output matches kloc-cli format exactly
- [x] Target contains only {fqn, file} (no id, kind, line)
- [x] Line numbers 1-based in output (0-based internally)
- [x] `total` field counts all nodes recursively

### S04: Deps Flat & Tree
- [x] Outgoing USES edges for direct deps
- [x] Container member expansion via DFS
- [x] No line fallback to target start_line (unlike usages)
- [x] BFS tree with visited set matches kloc-cli

### S05: Console Output
- [x] `--json` flag produces JSON matching kloc-cli
- [x] `--depth` and `--limit` options work
- [x] Symbol not found produces error with exit code 1
- [x] Multiple matches produce candidates with exit code 1

### S06: Snapshot Tests
- [x] All 8 usages snapshot tests pass (flat, depth 2, depth 3, method, interface, property, enum, const)
- [x] All 6 deps snapshot tests pass (flat, depth 2, method, method-depth2, interface, enum)
- [x] Zero JSON diffs between kloc-intelligence and kloc-cli
- [x] Previously passing resolve tests still pass (no regressions)

## Test Results
- 146 passed, 29 xfailed in 23.42s
- Unit tests: 146 pass (9 usages + 10 deps + 9 json_formatter + 10 query runner + 14 resolve + 5 result mapper + 23 comparator + 66 existing)
- Snapshot tests: 21 pass (7 resolve + 8 usages + 6 deps), 29 xfail (other commands)
- Linter: All checks passed

## Dependencies Satisfied for Downstream Tasks
- T06 can use the same DFS member expansion pattern for owners/inherit/overrides
- T06 can use edge_idx ordering for any edge-order-dependent queries
- T09+ can use usages/deps query infrastructure for context command
- JSON formatter pattern established for all tree-based output
