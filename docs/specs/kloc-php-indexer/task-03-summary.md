# Task 03 Summary: AST Type Definitions (37 Node Types)

**Status:** COMPLETE
**Date:** 2026-03-05

## What Was Implemented

### src/parser/ast.rs (~1760 lines)

**PhpNode<'a> enum** — 32 variants covering all 37 CST node types + Other:
- 6 scope-defining: Namespace, ClassLike, Method, Function, Closure, ArrowFunction
- 5 definition: ClassConst, EnumCase, Param, Property, PropertyItem
- 15 expression/reference: MethodCall, StaticCall, FuncCall, New, PropertyFetch, StaticPropertyFetch, ClassConstFetch, Variable, Assign, Foreach, ArrayDimFetch, Coalesce, Ternary, Match, Name
- 6 type: NullableType, UnionType, IntersectionType, DnfType, NamedType, PrimitiveType
- Other (unrecognized CST nodes)

**classify_node() dispatch** — central match on node.kind() routing to all 37 types. Special case: binary_expression with ?? -> Coalesce, all other binary ops -> Other.

**39 unit tests** across 4 test modules (tests_scope, tests_defs, tests_exprs, tests_classify).

## Deviations from Spec
- Several accessor methods changed from `child_by_field_name()` to `child_by_kind()` or positional access — tree-sitter-php grammar doesn't expose all fields as named fields
- ClassConstFetchNode uses positional children (named_child(0), named_child(1)) instead of field names
- ConstElement uses child_by_kind for name extraction

## Test Results
- 69 unit tests passed (30 from Task 02 + 39 new)
- 2 integration tests passed, 1 ignored
- Build: zero warnings

## Known Issues
- ForeachNode field names need verification against actual tree-sitter-php grammar for PHP foreach statements
- NamespaceNode.is_bracketed() uses "compound_statement" check — may need adjustment for actual grammar
