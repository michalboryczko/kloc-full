# Task 07 Summary: Reference Types & Handlers

## What Was Implemented

Ported the reference type classification engine and USED BY handler strategy pattern from kloc-cli to kloc-intelligence. Three new modules under `src/logic/`: reference type inference, edge context/handler system, and graph helper functions.

### S01: Port _infer_reference_type()
- `src/logic/reference_types.py` with `infer_reference_type()` function
- 20+ branch heuristic matching kloc-cli's `_infer_reference_type()` exactly
- Accepts pre-fetched boolean flags (`has_arg_type_hint`, `has_return_type_hint`, `has_class_property_type_hint`) instead of SoTIndex lookups
- Direct edge type mappings: extends, implements, uses_trait
- Uses edge + target kind sub-classification: Method->method_call, Property->property_access, etc.
- Complex Class/Interface target logic: Argument->parameter_type, Property->property_type, Method with hints->parameter_type/return_type/property_type

### S02: Port EdgeContext + EntryBucket
- `EdgeContext` frozen dataclass with ~30 fields carrying pre-fetched data from Cypher queries
- Fields include: start_id, source/target node data, ref_type, containment data, resolved property data, receiver data (on_expr/on_kind), arguments
- `EntryBucket` mutable collector with typed lists per reference category: instantiation, extends, property_type, method_call, property_access_groups, param_return
- Dedup tracking via `seen_instantiation_methods` and `seen_property_type_props` sets

### S03: Port All 7 Handlers
- `InstantiationHandler`: handles `new ClassName()` calls, deduplicates by containing method, appends `()` to method FQNs
- `ExtendsHandler`: handles class inheritance, emits entries to `bucket.extends`
- `ImplementsHandler`: handles interface implementation, also emits to `bucket.extends` (same list)
- `PropertyTypeHandler`: handles typed property declarations, resolves from direct Property source or pre-fetched resolved property data
- `MethodCallHandler`: handles method invocations, suppresses when containing class has property_type injection for target
- `PropertyAccessHandler`: groups by property FQN and method, merges lines for same group
- `ParamReturnHandler`: handles parameter_type, return_type, type_hint edges; method-level FQN for return_type, class-level grouping for parameter_type

### S04: Port Constants & Sorting Logic
- `CHAINABLE_REFERENCE_TYPES = {"method_call", "property_access", "instantiation", "static_call"}`
- `REF_TYPE_PRIORITY` dict with 10 entries: instantiation(0) through type_hint(6)
- `sort_entries_by_priority()` sorts by ref_type priority, then file, then line
- `sort_entries_by_location()` sorts by file then line
- `get_reference_type_from_call_kind()` maps call_kind strings to ref_type strings

### S05: Port Helper Functions
- `src/logic/graph_helpers.py` with Cypher-based helper functions
- `member_display_name(kind, name)`: pure logic -- formats "method()", "$prop", "CONST"
- `resolve_containing_method(runner, node_id)`: traverses CONTAINS upward to find Method/Function
- `is_internal_reference(runner, source_id, target_class_id)`: checks if source is contained within target class
- `get_contains_parent(runner, node_id)` and `get_contains_children(runner, node_id)`: single-hop containment
- `resolve_containing_class(runner, node_id)`: traverses CONTAINS upward to find Class/Interface/Trait/Enum

### S06: Unit Tests
- `tests/test_reference_types.py` with 44 tests across 6 test classes
- `tests/test_handlers.py` with 22 tests across 8 test classes
- All tests are pure unit tests (no Neo4j required) -- handlers tested with in-memory EdgeContext/EntryBucket

## Files Created/Modified
- `kloc-intelligence/src/logic/reference_types.py` (new -- inference engine, constants, sorting)
- `kloc-intelligence/src/logic/handlers.py` (new -- EdgeContext, EntryBucket, 7 handlers, registry)
- `kloc-intelligence/src/logic/graph_helpers.py` (new -- Cypher-based helper functions)
- `kloc-intelligence/src/logic/__init__.py` (updated -- exports new public API)
- `kloc-intelligence/tests/test_reference_types.py` (new -- 44 unit tests)
- `kloc-intelligence/tests/test_handlers.py` (new -- 22 unit tests)

## Acceptance Criteria Status

### S01: Infer Reference Type
- [x] 20+ branch heuristic matches kloc-cli exactly
- [x] Direct edge types: extends, implements, uses_trait
- [x] Uses edge sub-classification by target kind
- [x] Complex Class/Interface target with hint flags
- [x] Fallback to "uses" for unknown edge types

### S02: EdgeContext + EntryBucket
- [x] EdgeContext frozen dataclass with all pre-fetched fields
- [x] EntryBucket mutable collector with typed lists
- [x] Dedup tracking for instantiation and property_type

### S03: All 7 Handlers
- [x] InstantiationHandler with dedup by containing method
- [x] ExtendsHandler and ImplementsHandler (both to extends list)
- [x] PropertyTypeHandler with direct + resolved property paths
- [x] MethodCallHandler with injection suppression
- [x] PropertyAccessHandler with grouping and line merging
- [x] ParamReturnHandler for parameter_type/return_type/type_hint

### S04: Constants & Sorting
- [x] CHAINABLE_REFERENCE_TYPES has 4 types matching kloc-cli
- [x] REF_TYPE_PRIORITY has 10 entries matching kloc-cli
- [x] Sorting by priority and by location work correctly
- [x] get_reference_type_from_call_kind maps all known kinds

### S05: Helper Functions
- [x] member_display_name pure logic ported
- [x] resolve_containing_method with Cypher traversal
- [x] is_internal_reference with Cypher containment check
- [x] get_contains_parent/children helpers
- [x] resolve_containing_class with Cypher traversal

### S06: Unit Tests
- [x] 66 total unit tests for T07 modules (44 reference_types + 22 handlers)
- [x] All tests pass without Neo4j (pure unit tests)
- [x] Handler registry test validates shared ParamReturnHandler instance

## Test Results
- 259 passed, 14 xfailed in 22.66s
- New T07 unit tests: 66 pass (44 reference_types + 22 handlers)
- All existing tests pass (no regressions)
- Snapshot tests: 36 pass, 14 xfail (context only)
- Linter: All checks passed

## Key Design Decisions

### Pre-fetched Data Instead of Index Lookups
The kloc-cli handlers accept a `SoTIndex` and do lookups at handler time. In kloc-intelligence, all needed data is pre-fetched by Cypher queries and packed into `EdgeContext`. This means the orchestrator (T09+) must run appropriate queries before creating EdgeContext instances.

### Dict-based Entries (Not Dataclasses)
Handlers produce `dict` entries rather than `ContextEntry` dataclasses, matching the approach used in usages/deps. This keeps the handler output format simple and avoids premature coupling to the context output model (which will be defined in T08).

### Shared Handler Instance in Registry
The `USED_BY_HANDLERS` registry maps 9 ref_type strings to 7 handler instances. `parameter_type`, `return_type`, and `type_hint` share a single `ParamReturnHandler` instance. This matches kloc-cli's pattern.

### ParamReturnHandler Class-level Grouping Gap
When the source is a Method/Function (not a class-kind node), the handler needs to traverse containment upward to find the class. Currently this path requires `containing_class_id` to be pre-set in EdgeContext (by the orchestrator in T09). If not set, Method/Function sources without class context are skipped.

## Dependencies Satisfied for Downstream Tasks
- T08 can use reference_types module for edge classification
- T09 can use handlers + graph_helpers for context command's USED BY section
- T09/T10/T11 can use USED_BY_HANDLERS registry for processing edges
- All handler business logic is now available for context orchestration
