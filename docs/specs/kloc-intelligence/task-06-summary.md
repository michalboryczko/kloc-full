# Task 06 Summary: Simple Commands -- owners, inherit, overrides

## What Was Implemented

Complete owners, inherit, and overrides commands for kloc-intelligence: Cypher queries with BFS traversal, JSON output formatting, CLI commands, and snapshot test parity for all 15 owners/inherit/overrides corpus entries.

### S01: Owners Query
- `src/db/queries/owners.py` with `owners_chain()` function
- `CONTAINS_PARENT` Cypher query for traversing containment chain upward
- Simple while loop: target -> parent -> grandparent -> ... -> File root
- Returns `{"chain": [NodeData, ...]}` matching kloc-cli's OwnersQuery.execute()

### S02: Inherit Queries
- `src/db/queries/inherit.py` with `inherit_tree()` function
- BFS traversal using `deque` with `(node_id, depth, parent_entry)` tuples
- Up direction: follows outgoing EXTENDS + IMPLEMENTS edges (ancestors)
- Down direction: follows incoming EXTENDS + IMPLEMENTS edges (descendants)
- Validates node kind in {Class, Interface, Trait, Enum}
- `_get_all_parents()` and `_get_all_children()` helper functions
- Global visited set prevents re-visiting nodes across depths

### S03: Overrides Queries
- `src/db/queries/overrides.py` with `overrides_tree()` function
- BFS traversal using `deque` matching kloc-cli's OverridesQuery
- Up direction: follows outgoing OVERRIDES edge (single chain typically)
- Down direction: follows incoming OVERRIDES edges (fan-out)
- Validates node kind == "Method"

### S04: Result Builders (JSON Formatters)
- `owners_chain_to_dict()`: chain format `{"chain": [{"kind", "fqn", "file", "line"}]}`
- `inherit_tree_to_dict()`: tree format with `root`, `direction`, `max_depth`, `total`, `tree` (entries include `kind`)
- `overrides_tree_to_dict()`: same tree format but entries do NOT include `kind` (matching kloc-cli)
- All line numbers converted from 0-based to 1-based at the formatter boundary

### S05: CLI Commands
- `owners` command: resolves symbol, calls owners_chain, JSON or console output
- `inherit` command: `--direction/-r`, `--depth/-d`, `--limit/-l`, `--json/-j` options
- `overrides` command: same options as inherit, validates Method kind
- All three handle multi-match (print candidates, exit 1) and symbol-not-found errors

### S06: Snapshot Tests
- Updated `IMPLEMENTED_COMMANDS` to include "owners", "inherit", "overrides"
- Updated `execute_query()` dispatch for all three commands
- All 15 owners/inherit/overrides snapshot tests pass (zero diffs against kloc-cli)

## Files Created/Modified
- `kloc-intelligence/src/db/queries/owners.py` (new -- containment chain query)
- `kloc-intelligence/src/db/queries/inherit.py` (new -- BFS inherit tree)
- `kloc-intelligence/src/db/queries/overrides.py` (new -- BFS override tree)
- `kloc-intelligence/src/db/queries/__init__.py` (updated -- exports new functions)
- `kloc-intelligence/src/output/json_formatter.py` (updated -- 3 new formatters)
- `kloc-intelligence/src/cli.py` (updated -- 3 new commands)
- `kloc-intelligence/tests/test_snapshot.py` (updated -- owners/inherit/overrides dispatch)
- `kloc-intelligence/tests/test_owners.py` (new -- 6 unit tests)
- `kloc-intelligence/tests/test_inherit.py` (new -- 11 unit tests)
- `kloc-intelligence/tests/test_overrides.py` (new -- 8 unit tests)
- `kloc-intelligence/tests/test_json_formatter.py` (updated -- 8 new tests for formatters)

## Acceptance Criteria Status

### S01: Owners Query
- [x] Containment chain traversal via CONTAINS edges
- [x] Chain starts with target, ends with File root
- [x] Handles Method, Property, Class, Const, EnumCase targets
- [x] File node at root has no parent (single-element chain)

### S02: Inherit Queries
- [x] BFS ancestors via outgoing EXTENDS + IMPLEMENTS
- [x] BFS descendants via incoming EXTENDS + IMPLEMENTS
- [x] Validates kind in {Class, Interface, Trait, Enum}
- [x] Depth and limit parameters work correctly
- [x] Global visited set prevents re-visiting nodes

### S03: Overrides Queries
- [x] BFS up via outgoing OVERRIDES edge
- [x] BFS down via incoming OVERRIDES edges
- [x] Validates kind == "Method"
- [x] Depth and limit parameters work correctly

### S04: Result Builders
- [x] owners_chain_to_dict matches kloc-cli format (kind, fqn, file, line)
- [x] inherit_tree_to_dict includes kind in entries
- [x] overrides_tree_to_dict does NOT include kind in entries
- [x] All line numbers 1-based in output

### S05: CLI Commands
- [x] All three commands registered and functional
- [x] Symbol resolution with multi-match handling
- [x] JSON and console output modes

### S06: Snapshot Tests
- [x] 5 owners snapshot tests pass (method, property, class, const, enum-case)
- [x] 6 inherit snapshot tests pass (class-up, class-down, interface-down, interface-up, enum-up, class-depth2)
- [x] 4 overrides snapshot tests pass (method-up, method-down, interface-method, no-match)
- [x] Zero JSON diffs between kloc-intelligence and kloc-cli
- [x] Previously passing resolve/usages/deps tests still pass (no regressions)

## Test Results
- 193 passed, 14 xfailed in 14.35s
- Unit tests: 193 pass (6 owners + 11 inherit + 8 overrides + 17 json_formatter + 9 usages + 10 deps + 10 query_runner + 14 resolve + 5 result_mapper + 23 comparator + 66 existing + 14 snapshot-new)
- Snapshot tests: 36 pass (7 resolve + 8 usages + 6 deps + 5 owners + 6 inherit + 4 overrides), 14 xfail (context)
- Linter: All checks passed

## Key Design Decisions

### Cypher Record Key Mapping
The inherit and overrides Cypher queries RETURN nodes as `parent`, `child`, etc. rather than the default `n`. Used `record_to_node(rec, key="parent")` and `record_to_node(rec, key="child")` to map correctly, leveraging the existing `key` parameter in `result_mapper.py`.

### Edge Ordering for Inherit/Overrides
For downward (incoming) queries, edge ordering uses `ORDER BY e.edge_idx` to match kloc-cli's insertion-order processing. For upward (outgoing) queries, ordering is by `node_id` since kloc-cli processes these in sot.json edge insertion order (typically just one extends parent anyway).

### No Container Member Expansion
Unlike usages/deps which require DFS member expansion for container types, owners/inherit/overrides operate on individual nodes only. Owners walks the containment chain; inherit works on Class/Interface/Trait/Enum; overrides works on Method. This made implementation straightforward.

## Dependencies Satisfied for Downstream Tasks
- T07+ can use owners_chain for context command's containment information
- T07+ can use inherit_tree for context command's inheritance section
- T07+ can use overrides_tree for context command's override resolution
- All six simple commands (resolve, usages, deps, owners, inherit, overrides) now complete
