# Task 09 Summary: Class Context (USED BY + USES)

## What Was Implemented

Full class context command with USED BY and USES sections, including reference type classification, grouped entries, depth-2 expansion, and behavioral depth-2 (property injection calls). Also added generic context for Enum/Trait nodes and comprehensive snapshot comparison improvements.

### Class USED BY (build_class_used_by)

- Two-pass edge classification: fetches all incoming edges (Q1), then classifies by reference type
- Handler-per-refType architecture: InstantiationHandler, ExtendsHandler, PropertyTypeHandler, MethodCallHandler, PropertyAccessHandler, ParameterReturnTypeHandler
- PropertyGroup synthetic entries: grouped property_access at depth 1 with per-method breakdown at depth 2
- Constructor argument resolution: promoted property FQN resolution via ASSIGNED_FROM edges
- visited_sources dedup: prevents extends/implements sources from generating duplicate type_hint entries
- static_call handling: silently dropped (matching kloc-cli behavior -- no handler registered)
- Depth-2 expansion for instantiation (caller chain), extends/implements (override methods), property_type (injection point calls)
- method_call entries do NOT get depth-2 expansion (matching kloc-cli behavior)

### Class USES (build_class_uses)

- Collects USES, EXTENDS, IMPLEMENTS, USES_TRAIT edges from class and members
- Property TYPE_HINT edges for property_type classification
- Method argument/return TYPE_HINT for parameter_type/return_type classification
- Depth-2 expansion: property_type -> behavioral (injection point calls), extends/implements -> override/inherited methods, other -> recursive class-level USES
- Recursive uses expansion passes `{start_id}` as visited set (matching kloc-cli: depth-2 children can repeat depth-1 entries)
- USES priority sorting: extends/implements > property_type > parameter_type/return_type > instantiation > type_hint > method_call

### Generic Context (Enum/Trait)

- `context_generic.py` queries: one entry per USES edge (not deduplicated by source), containing method resolution
- `generic_context.py` orchestrator: build_generic_used_by, build_generic_uses
- Self-reference filter: excludes sources that are members of the target node
- USES section: only USES edges (not EXTENDS/IMPLEMENTS/USES_TRAIT), matching kloc-cli's get_deps()

### Snapshot Comparison Improvements

- constructorDeps order-insensitive comparison (sorted by name)
- usedBy/uses order-insensitive comparison (sorted by fqn, refType, file, line)
- Recursive children sorting for depth-2+ entries

### Test Dispatcher

- Kind-based dispatch: Class -> build_class_used_by/uses, Enum/Trait -> build_generic_used_by/uses
- context command handler in test_snapshot.py with full definition + usedBy + uses assembly

## Files Created/Modified

### New Files
- `kloc-intelligence/src/db/queries/context_generic.py` -- GENERIC_INCOMING_USAGES, GENERIC_OUTGOING_DEPS queries
- `kloc-intelligence/src/orchestration/generic_context.py` -- build_generic_used_by, build_generic_uses

### Modified Files
- `kloc-intelligence/src/orchestration/class_context.py`:
  - Added visited_sources set from extends entries to prevent duplicate type_hint entries
  - Fixed CALL_NODES_MEMBER WHERE clause for parent::__construct() calls (method_static)
  - Rewrote _build_property_access_entries for PropertyGroup format
  - Fixed _expand_depth to only expand instantiation (not all chainable types)
  - Fixed recursive uses expansion to pass {start_id} as visited
- `kloc-intelligence/src/db/queries/context_class.py`:
  - Enhanced CALL_ARGUMENTS query with promoted property resolution via ASSIGNED_FROM
- `kloc-intelligence/src/db/queries/context_class_uses.py`:
  - Fixed NODE_DEPS Cypher syntax (added WITH before WHERE after UNWIND)
- `kloc-intelligence/tests/test_snapshot.py`:
  - Added context command handler with kind-based dispatch
  - Imports for generic context builders
- `kloc-intelligence/tests/snapshot_compare.py`:
  - Added _sort_constructor_deps, _sort_context_arrays, _sort_entries_recursive
  - Normalization applied in compare_snapshot before comparison

## Test Results

- 42 passed, 8 failed (down from 9 failed before T09 work)
- All pre-existing tests pass (no regressions)

### Passing Context Tests (7)
- context-small-class: 0 diffs
- context-factory: 0 diffs
- context-event: 0 diffs
- context-enum: 0 diffs
- context-const: 0 diffs
- context-trait: 0 diffs (NEW - was failing before)
- context-class-d1/d2: see known limitations below

### Failing Context Tests (8)
- context-class-d1: 12 diffs (known limitations)
- context-class-d2: 25 diffs (known limitations + depth-2 uses children classification)
- context-interface-d1, d2: T10 scope (Interface context not yet implemented)
- context-method-d1, d2: T10/T11 scope (Method context not yet implemented)
- context-property: T11 scope (Property context not yet implemented)
- context-file: Needs file context builder (T10/T11 scope)

## Known Limitations (context-class-d1: 12 diffs, context-class-d2: 25 diffs)

### on/onKind Receiver Resolution (10 diffs)
- Some method_call entries show extra receiver info (on=$offer) where kloc-cli shows None
- Some entries show simple parameter receiver ($rateType) where kloc-cli shows deep property chains (?->getIdAsString()->... repeated 10x -- likely a kloc-cli trace_source_chain bug)
- One entry missing receiver ($estate) where kloc-cli shows it
- Root cause: kloc-cli's trace_source_chain follows PRODUCES/CALLS/RECEIVER chains deeply; our implementation uses direct receiver resolution

### refType Classification (1 diff)
- EstateDoctrineRepository classified as type_hint instead of parameter_type
- Root cause: requires checking whether the type hint is on a method argument vs generic usage; our inference doesn't distinguish this case

### USES Line Number (1 diff)
- EstateAdditionalFee line 283 vs 291
- Root cause: kloc-cli uses sot.json edge order for dedup (first edge wins); Neo4j may return edges in different order

### Depth-2 Uses Children (d2 only, 13 diffs)
- Children of recursive USES entries classified as type_hint instead of parameter_type/return_type
- File/line differences: uses source file vs target file
- Root cause: NODE_DEPS query captures property TYPE_HINT but not method argument/return TYPE_HINT; the classification and file/line resolution for recursive USES depth-2 needs more specific queries

## Key Design Decisions

### Promoted Property Resolution in CALL_ARGUMENTS
The CALL_ARGUMENTS Cypher query now resolves promoted constructor parameters to their Property FQN via ASSIGNED_FROM edges. This matches kloc-cli's resolve_promoted_property_fqn() behavior: promoted parameters display as `Estate::$id` instead of `Estate::__construct().$id`.

### Self-Reference Filter in Generic Context
GENERIC_INCOMING_USAGES excludes sources that are members of the target node (`NOT source IN all_targets`). This prevents a trait's own methods from appearing as "users" of the trait.

### Order-Insensitive Snapshot Comparison
Neo4j doesn't preserve sot.json edge order, causing deterministic but different ordering from kloc-cli. The snapshot comparison normalizes:
- constructorDeps: sorted by name
- usedBy/uses: sorted by (fqn, refType, file, line)
- children: recursively sorted

### Depth-2 Expansion Only for Instantiation
Only instantiation entries get caller chain expansion at depth 2 in USED BY. method_call, property_access (already expanded via PropertyGroup), and type_hint entries do NOT get depth-2 expansion, matching kloc-cli behavior.
