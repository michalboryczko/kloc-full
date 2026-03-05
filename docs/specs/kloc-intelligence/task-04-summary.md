# Task 04 Summary: Graph Query Foundation

## What Was Implemented

Complete query infrastructure for kloc-intelligence: QueryRunner, result mapper, symbol resolver cascade, resolve CLI command, and snapshot test parity for all 7 resolve queries.

### S01: Query Runner
- `src/db/query_runner.py` with `QueryRunner` class
- Methods: `execute()`, `execute_single()`, `execute_value()`, `execute_count()`, `execute_write()`
- All methods log query timing at DEBUG level
- Parameterized queries via `**params` (Cypher `$parameter` syntax)
- 10 unit tests passing

### S02: Query Registry
- `src/db/queries/resolve.py` with Cypher query constants:
  - `RESOLVE_EXACT_FQN`, `RESOLVE_CASE_INSENSITIVE`, `RESOLVE_SUFFIX`, `RESOLVE_CONTAINS`, `RESOLVE_NAME`
- Queries use `$symbol` parameter (not `$query`) to avoid naming conflict with `QueryRunner.execute(query, ...)`
- `SEARCHABLE_KINDS` list excludes internal kinds (Call, Value, Argument)
- Pattern established for all future query modules (T05-T12)

### S03: Result Mapper
- `src/db/result_mapper.py` with mapping functions:
  - `record_to_node()` -- Neo4j Record -> NodeData
  - `records_to_nodes()` -- batch mapping
  - `record_to_flat_node()` -- for flat RETURN queries
- `src/models/node.py` with `NodeData` dataclass:
  - `id` property returns `node_id` (backward compat with kloc-cli)
  - `location_str` property returns `file:line` (1-based)
  - `signature` property extracts method signatures from documentation
  - `display_name` property returns signature for methods, FQN otherwise
- 5 unit tests passing

### S04: Symbol Resolver
- `resolve_symbol()` implements the full cascade matching kloc-cli's `SoTIndex.resolve_symbol()`:
  1. Exact FQN match (with Value/Argument dedup)
  2. Case-insensitive FQN match
  3. Suffix match (ENDS WITH)
  4. Contains match (CONTAINS)
  5. Short name match
  6. Short name without parens
- `_dedup_value_argument()` keeps only Value when both Value and Argument share FQN
- 14 unit tests passing (exact FQN, method, interface, property, const, enum, case-insensitive, suffix, no match, leading backslash, file/line, id property, location_str, searchable kinds)

### S05: Resolve Command
- CLI `resolve` command added to `src/cli.py`
- Supports `--json` flag for JSON output
- Output format matches kloc-cli exactly:
  - Single match: `{id, kind, name, fqn, file, line}`
  - Multiple matches: array of `{id, kind, fqn, file, line}`
  - No match: `{error: "Symbol not found", query: ...}` with exit code 1
- Line numbers: 0-based in Neo4j, +1 for output (1-based)

### S06: Snapshot Tests
- Updated `corpus.yaml` resolve entries to include `{json: true}` option
- Regenerated all 7 resolve golden files with JSON output from kloc-cli
- Updated `test_snapshot.py`:
  - Added "resolve" to `IMPLEMENTED_COMMANDS`
  - Implemented `execute_query()` dispatch for resolve command
  - Tests now execute real queries and compare against golden output
- All 7 resolve snapshot tests PASS (zero diffs against kloc-cli)
- 43 other snapshot tests correctly xfail (commands not yet implemented)

### Additional Fixes
- Fixed `loaded_database` test fixture: uses canary-based detection (checks for known FQN) to detect when import tests clear the database, then reloads uestate data from cached parse
- Cached parsed SoT data (`_parsed_data_cache`) so reloads don't re-parse the JSON file
- Fixed Cypher parameter naming: changed `$query` to `$symbol` in all resolve queries to avoid conflict with `QueryRunner.execute(query: str, ...)` first positional argument

## Files Created/Modified
- `kloc-intelligence/src/db/query_runner.py` (modified - added execute_write)
- `kloc-intelligence/src/db/result_mapper.py` (new)
- `kloc-intelligence/src/db/queries/__init__.py` (modified - exports resolve_symbol)
- `kloc-intelligence/src/db/queries/resolve.py` (new)
- `kloc-intelligence/src/db/__init__.py` (modified - docstring)
- `kloc-intelligence/src/models/node.py` (existing from prior session)
- `kloc-intelligence/src/cli.py` (modified - added resolve command)
- `kloc-intelligence/tests/conftest.py` (modified - loaded_database fixture with canary)
- `kloc-intelligence/tests/test_query_runner.py` (new)
- `kloc-intelligence/tests/test_resolve.py` (new)
- `kloc-intelligence/tests/test_result_mapper.py` (new)
- `kloc-intelligence/tests/test_snapshot.py` (modified - resolve wired, IMPLEMENTED_COMMANDS)
- `kloc-intelligence/tests/snapshots/corpus.yaml` (modified - json:true for resolve)
- `kloc-intelligence/tests/snapshots/golden/resolve-*.json` (7 files regenerated)

## Acceptance Criteria Status

### S01: Query Runner
- [x] execute() runs Cypher and returns list of Records
- [x] execute_single() returns one Record or None
- [x] execute_value() returns single scalar value
- [x] execute_count() returns integer count
- [x] All methods log query timing at DEBUG level
- [x] Parameterized queries work ($param syntax)

### S02: Query Registry
- [x] Query module pattern established
- [x] resolve.py implements all queries as constants
- [x] resolve_symbol() executes cascade search
- [x] $parameter syntax used (not f-strings)
- [x] Pattern replicable for T05-T12

### S03: Result Mapper
- [x] record_to_node() maps all Neo4j properties to NodeData
- [x] NodeData.id returns node_id (backward compat)
- [x] NodeData.location_str returns file:line (1-based)
- [x] NodeData.signature extracts method signatures
- [x] Mapper handles missing optional properties

### S04: Symbol Resolver
- [x] Exact FQN match works
- [x] Case-insensitive match works
- [x] Suffix match works
- [x] Method resolution works
- [x] Short name works
- [x] No match returns empty list
- [x] Value/Argument dedup works
- [x] Cascade matches kloc-cli ordering
- [x] Leading backslash stripped

### S05: Resolve Command
- [x] CLI resolve command returns matching nodes
- [x] --json outputs valid JSON matching kloc-cli format
- [x] Console output shows formatted display
- [x] Line numbers 1-based in output
- [x] "Symbol not found" for unresolvable symbols
- [x] Connection established and closed cleanly

### S06: Snapshot Tests
- [x] All 7 resolve snapshot tests pass (zero diffs)
- [x] 14 resolve unit tests pass
- [x] Other command snapshot tests remain xfail
- [x] Tests run in <10s including data load

## Test Results
- 104 passed, 43 xfailed in 7.0s
- Unit tests: 104 pass (10 query runner + 14 resolve + 5 result mapper + 23 comparator + 52 existing)
- Snapshot tests: 7 pass (resolve), 43 xfail (other commands)
- Linter: All checks passed

## Dependencies Satisfied for Downstream Tasks
- T05 can use QueryRunner + result_mapper + resolve_symbol for usages/deps
- T06 can use same infrastructure for owners/inherit/overrides
- T08+ can use NodeData model and result mapper for context commands
- Snapshot test infrastructure now works end-to-end (proven with resolve)
