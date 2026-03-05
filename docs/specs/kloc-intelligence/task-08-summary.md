# Task 08 Summary: Definition & Context Models

## What Was Implemented

Ported the data models (ContextEntry, MemberRef, ArgumentInfo, DefinitionInfo, ContextResult), the ContextOutput contract model hierarchy, the definition builder logic, and the definition Cypher queries from kloc-cli to kloc-intelligence.

### S01: Data Models
- `src/models/results.py` with all data models:
  - `ContextEntry` (28 fields matching kloc-cli exactly)
  - `MemberRef` (11 fields for member usage references)
  - `ArgumentInfo` (8 fields for argument-to-parameter mappings)
  - `ContextResult` (target, max_depth, used_by, uses, definition)
  - `DefinitionInfo` (17 fields for symbol definition metadata)

### S02: ContextOutput Contract Model
- `src/models/output.py` with full output hierarchy:
  - `ContextOutput.from_result()` -- single conversion point
  - `OutputEntry.from_entry()` -- mode-dependent field suppression (class_level vs method_level)
  - `OutputMemberRef.from_ref()` -- 0-based to 1-based line conversion
  - `OutputArgumentInfo.from_info()` -- preserves all fields
  - `OutputTarget.from_node()` -- target serialization with signature
  - `OutputDefinition.from_info()` -- property type extraction, value fields, declared_in
  - `_shorten_param_key()` -- flat args format key shortening

### S03: Definition Builders
- `src/logic/definition.py` with all definition builder functions:
  - `build_definition()` dispatcher with kind-specific sub-builders
  - `_build_method_definition()` -- typed arguments + return type
  - `_build_class_definition()` -- properties, methods, constructor_deps, inheritance
  - `_build_interface_definition()` -- methods only, extends
  - `_build_property_definition()` -- type, visibility, readonly, static, promoted
  - `_build_argument_definition()` -- type hint resolution
  - `_build_value_definition()` -- value_kind, type_info, source chain
  - `parse_property_doc()` -- SCIP documentation parsing for property metadata
  - `_is_abstract_from_doc()` -- abstract method detection from docs

### S04: Definition Cypher Queries
- `src/db/queries/definition.py` with batched Cypher queries:
  - `NODE_AND_PARENT` -- target node + containing parent
  - `CHILDREN` -- ordered children with override detection
  - `TYPE_HINTS` -- batch type hint resolution for node + children
  - `INHERITANCE` -- extends, implements, uses_trait in one query
  - `CONSTRUCTOR_DEPS` -- promoted constructor parameters
  - `VALUE_SOURCE` -- source chain via assigned_from/produces/calls
  - `VALUE_SCOPE` -- containing method/function scope
  - `PROPERTY_PROMOTED` -- promoted detection via assigned_from
  - `fetch_definition_data()` -- orchestrates all queries into data dict
  - `definition_for_node()` -- main entry point (fetch + build)

### S05: Output Formatters
- Existing `json_formatter.py` already handles simpler commands
- `ContextOutput.to_dict()` handles context command JSON (camelCase: maxDepth, usedBy, refType, onKind, etc.)

### S06: Unit Tests
- `tests/test_definition.py` -- 31 tests across 8 classes
- `tests/test_output.py` -- 52 tests across 8 classes
- All tests are pure unit tests (no Neo4j required)

## Files Created/Modified
- `kloc-intelligence/src/models/results.py` (new -- ContextEntry, MemberRef, ArgumentInfo, etc.)
- `kloc-intelligence/src/models/output.py` (new -- ContextOutput hierarchy)
- `kloc-intelligence/src/models/__init__.py` (updated -- exports all models)
- `kloc-intelligence/src/logic/definition.py` (new -- definition builders)
- `kloc-intelligence/src/logic/__init__.py` (updated -- exports definition functions)
- `kloc-intelligence/src/db/queries/definition.py` (new -- Cypher queries)
- `kloc-intelligence/src/db/queries/__init__.py` (updated -- exports definition functions)
- `kloc-intelligence/tests/test_definition.py` (new -- 31 tests)
- `kloc-intelligence/tests/test_output.py` (new -- 52 tests)

## Test Results
- 342 passed, 14 xfailed in 22.44s
- New T08 unit tests: 83 pass (31 definition + 52 output)
- All existing tests pass (no regressions)
- Linter: All checks passed

## Key Design Decisions

### Pre-fetched Data Dict Pattern
Definition builders accept a pre-fetched `data` dict rather than a `SoTIndex`. The `fetch_definition_data()` function in the Cypher queries module runs all necessary queries and assembles the dict. The builder functions are pure logic that can be tested without Neo4j.

### Signature Extraction from Documentation
Method signatures are extracted from SCIP documentation strings in the Cypher query layer (`_extract_signature_from_doc()`), matching `NodeData.signature` property logic. This is done at query time so child method dictionaries include signature data.

### Mode-Dependent Field Suppression
OutputEntry.from_entry() applies class_level vs method_level rules:
- Signature: class_level suppresses unless override/inherited; method_level shows when no ref_type
- member_ref: only in method_level and only when no ref_type
- Arguments: class_level uses flat dict; method_level uses rich list
- crossed_from: suppressed in class_level at depth < 2

### Property vs Value "type" Field
OutputDefinition.to_dict() handles dual use of "type": for Property it's a string (type_name), for Value it's an object (type_info). The from_info() factory extracts these from different DefinitionInfo fields.

## Dependencies Satisfied for Downstream Tasks
- T09-T11 can use ContextEntry, MemberRef, ArgumentInfo for context tree building
- T09-T11 can use ContextOutput.from_result() for JSON serialization
- T09-T12 can use definition_for_node() for DEFINITION section
- T09-T12 can use build_definition() with pre-fetched data
- All context output models ready for snapshot test comparison
