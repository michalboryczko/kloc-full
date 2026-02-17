# Feature: scip-php DocIndexer Refactoring — Implementation Plan

**Feature Name**: scip-php-refactor
**Spec**: [docs/specs/scip-php-refactor.md](./scip-php-refactor.md)
**Analysis**: [docs/doc-indexer-decoupling/README.md](../doc-indexer-decoupling/README.md)
**Date**: 2026-02-17
**Author**: Architect Agent

---

## Overview

### Problem Statement

`scip-php/src/DocIndexer.php` is a 2,669-line monolithic AST visitor with 45 methods and 14 mutable properties handling 7 distinct responsibilities. It is difficult to test in isolation, hard to navigate, and risky to extend.

### Acceptance Criteria (from spec)

- Output byte-identical before and after refactoring (SCIP index + calls/values)
- 35/35 snapshot tests pass, 235/235 contract tests pass, index binary comparison passes
- PHPStan level MAX passes, PHPCS passes
- Each extracted service has dedicated unit tests
- All 15 existing test files continue to pass unchanged
- DocIndexer reduced to ~150 lines (dispatcher + wiring) after Phase 5

### Constraints

- **Pure refactoring** — zero behavioral changes
- **Sequential phases** — all phases modify `DocIndexer.php`, preventing parallel development on different phases
- **4 test gates** after every phase (snapshot, contract, binary diff, unit+lint)
- **Strict code style** — PSR-12 + Slevomat: `final` classes, constructor property promotion, trailing commas, explicit `use function` imports, no `stdClass`
- **PHPStan level MAX** — strict types, treats phpDoc types as uncertain

---

## Codebase Summary

### Key Patterns Discovered

1. **Post-order traversal**: `Parser::traverse()` uses `leaveNode` — loop body variables are visited before `Foreach_` nodes. This is why `ensureForeachVarRegistered()` exists and must be preserved exactly.

2. **Parent connecting**: `ParentConnectingVisitor` sets `parent` attribute on all nodes. `findEnclosingScope()` walks parents to find `ClassMethod`/`Function_`. All services that call this get it from TypeResolver.

3. **Dual symbol system**: SCIP uses `local N` format; calls.json uses descriptive `{scope}local${name}@{line}` format. Both must be preserved in LocalVariableTracker.

4. **One Value Per Declaration Rule**: Parameters and locals each get exactly one ValueRecord at declaration. Subsequent usages reference that single value ID. This invariant is critical in LocalVariableTracker and ExpressionTracker.

5. **Nullsafe union pattern**: Currently duplicated 3 times (lines 2333-2373, 2386-2434, 2555-2606). TypeResolver consolidates into one `applyNullsafeUnion()` helper.

6. **`EXPERIMENTAL_KINDS` gating**: `isExperimentalKind()` and direct `$this->experimental` checks gate operator expressions. After refactoring, `ctx->experimental` is the single source of truth.

### Architecture Notes

- `Indexer.php` creates one `DocIndexer` per file (line 91), reads 6 public properties after traversal
- `Indexer.php` interface MUST NOT change (constructor signature, property access pattern)
- After Phase 0, `Indexer.php` reads via `getContext()` instead of direct properties — this is the only change to Indexer.php
- PSR-4 autoloading: `ScipPhp\\` -> `src/`, so `ScipPhp\Indexing\` -> `src/Indexing/`
- Test namespace: `Tests\\` -> `tests/`, so `Tests\Indexing\` -> `tests/Indexing/`

---

## Technical Approach

### Chosen Approach: Extract to Collaborating Services via Shared State Bag

Each responsibility is extracted into a `final` service class under `ScipPhp\Indexing\`. All services share a single `IndexingContext` instance that holds the 14 mutable properties. Services are wired in DocIndexer's constructor and called via delegation.

### Why This Approach

- **Minimal coupling change**: Services access shared state through IndexingContext rather than passing dozens of parameters
- **No interface change**: DocIndexer's constructor signature stays the same. Indexer.php only changes property access to `getContext()`
- **Incremental extraction**: Each phase removes methods from DocIndexer one service at a time, always maintaining byte-identical output
- **Testable**: Each service can be unit-tested with a mock IndexingContext

### Alternatives Rejected

1. **Event-based architecture** — Over-engineering for a single-file-at-a-time traversal. Adds indirection without benefit.
2. **Separate state per service** — Would require complex state merging. The existing 14 properties are inherently cross-cutting.
3. **Parallel extraction (multiple services at once)** — Too risky. A single broken method reference causes cascading failures.

---

## Phased Implementation

### Pre-Phase: Capture Baselines

Before any code changes:

```bash
# Gate 1 baseline (snapshot tests)
./tests/snapshot.sh capture
# Note filename: tests/snapshot-XXXXXXXXXX.json

# Gate 3 baseline (binary comparison)
./bin/scip-php.sh -d ./kloc-reference-project-php -o /tmp/scip-baseline
./bin/scip-php.sh -d ./kloc-reference-project-php -o /tmp/scip-baseline-exp --experimental
cp /tmp/scip-baseline/index.json /tmp/index-before.json
cp /tmp/scip-baseline-exp/index.json /tmp/index-before-exp.json
```

---

### Phase 0: IndexingContext (ISSUE-A) — Foundation

**Risk: LOW | Effort: S | Dependencies: None**

- [ ] Create directory `scip-php/src/Indexing/`
- [ ] Create `scip-php/src/Indexing/IndexingContext.php` with:
  - 6 output collection properties (`symbols`, `extSymbols`, `occurrences`, `values`, `calls`, `syntheticTypeSymbols`)
  - 7 tracking state properties (`localCounter`, `localSymbols`, `localCallsSymbols`, `localAssignmentLines`, `expressionIds`, `localValueIds`, `parameterValueIds`)
  - 2 readonly config properties (`relativePath`, `experimental`)
  - Constant `EXPERIMENTAL_KINDS` (moved from DocIndexer)
  - Method `isExperimentalKind(string $kind): bool`
  - Method `resetLocals(): void` — clears tracking state + calls/values but NOT symbols/extSymbols/occurrences
- [ ] Modify `scip-php/src/DocIndexer.php`:
  - Remove all 14 property declarations
  - Remove `EXPERIMENTAL_KINDS` constant and `isExperimentalKind()` method
  - Add `private readonly IndexingContext $ctx` property
  - Create IndexingContext in constructor: `$this->ctx = new IndexingContext($relativePath, $experimental)`
  - Replace all `$this->symbols` with `$this->ctx->symbols` (and similarly for all 14 properties)
  - Replace `$this->experimental` reads with `$this->ctx->experimental`
  - Replace `$this->relativePath` reads with `$this->ctx->relativePath`
  - Replace `$this->isExperimentalKind()` with `$this->ctx->isExperimentalKind()`
  - Add `public function getContext(): IndexingContext`
  - `resetLocals()` delegates to `$this->ctx->resetLocals()`
- [ ] Modify `scip-php/src/Indexer.php`:
  - Replace `$docIndexer->symbols` with `$docIndexer->getContext()->symbols`
  - Replace `$docIndexer->extSymbols` with `$docIndexer->getContext()->extSymbols`
  - Replace `$docIndexer->occurrences` with `$docIndexer->getContext()->occurrences`
  - Replace `$docIndexer->syntheticTypeSymbols` with `$docIndexer->getContext()->syntheticTypeSymbols`
  - Replace `$docIndexer->values` with `$docIndexer->getContext()->values`
  - Replace `$docIndexer->calls` with `$docIndexer->getContext()->calls`
- [ ] Create directory `scip-php/tests/Indexing/`
- [ ] Create `scip-php/tests/Indexing/IndexingContextTest.php`
- [ ] Run all 4 test gates

---

### Phase 1: TypeResolver (ISSUE-B) — Stateless Extraction

**Risk: LOW | Effort: S | Dependencies: Phase 0**

- [ ] Create `scip-php/src/Indexing/TypeResolver.php` with:
  - Constructor: `(SymbolNamer $namer, Types $types)`
  - Public methods (9 moved from DocIndexer):
    - `formatTypeForDoc(?Type $type): string`
    - `formatTypeSymbol(?Type $type): ?string`
    - `resolveExpressionReturnType(Node\Expr $expr): ?string`
    - `resolveAccessReturnType(Node\Expr $expr, ?string $symbol): ?string`
    - `resolveCallKind(Node $callNode): string`
    - `resolveKindType(string $kind): string`
    - `resolveReturnType(Node $callNode, string $calleeSymbol): ?string`
    - `resolveValueType(Node\Expr $expr): ?string`
    - `findEnclosingScope(Node $n): ?string`
  - Private helper (NEW): `applyNullsafeUnion(list<string> $flat, bool $isNullsafe): ?string` — consolidates 3 duplicated nullsafe union patterns
- [ ] Modify `scip-php/src/DocIndexer.php`:
  - Add `private readonly TypeResolver $typeResolver` property
  - Create in constructor: `$this->typeResolver = new TypeResolver($namer, $types)`
  - Remove 9 methods from DocIndexer
  - Replace all call sites: `$this->resolveCallKind(...)` -> `$this->typeResolver->resolveCallKind(...)` etc.
  - Replace `$this->findEnclosingScope(...)` -> `$this->typeResolver->findEnclosingScope(...)`
  - Replace `$this->formatTypeForDoc(...)` -> `$this->typeResolver->formatTypeForDoc(...)`
  - Replace `$this->formatTypeSymbol(...)` -> `$this->typeResolver->formatTypeSymbol(...)`
- [ ] Create `scip-php/tests/Indexing/TypeResolverTest.php`
- [ ] Run all 4 test gates

---

### Phase 2: CallRecordBuilder (ISSUE-C) — Call Construction

**Risk: MEDIUM | Effort: M | Dependencies: Phase 0, Phase 1**

- [ ] Create `scip-php/src/Indexing/CallRecordBuilder.php` with:
  - Constructor: `(IndexingContext $ctx, TypeResolver $typeResolver, SymbolNamer $namer, Types $types)`
  - Holds `PrettyPrinter $prettyPrinter` (created internally, moved from DocIndexer)
  - Public methods (3 moved from DocIndexer):
    - `buildCallRecord(PosResolver $pos, Node $callNode, string $calleeSymbol, array $args): ?CallRecord`
    - `addCallWithResultValue(CallRecord $callRecord): void`
    - `buildAccessOrOperatorCallRecord(PosResolver $pos, Node\Expr $exprNode, ?string $symbol, ...): ?CallRecord`
- [ ] Modify `scip-php/src/DocIndexer.php`:
  - Add `private readonly CallRecordBuilder $callBuilder` property
  - Remove `PrettyPrinter $prettyPrinter` property (moved to CallRecordBuilder)
  - Create in constructor: `$this->callBuilder = new CallRecordBuilder($this->ctx, $this->typeResolver, $namer, $types)`
  - Remove 3 methods from DocIndexer
  - Replace all call sites: `$this->buildCallRecord(...)` -> `$this->callBuilder->buildCallRecord(...)` etc.
- [ ] Create `scip-php/tests/Indexing/CallRecordBuilderTest.php`
- [ ] Run all 4 test gates

---

### Phase 3: ExpressionTracker (ISSUE-D) — Expression Tree Walking

**Risk: MEDIUM | Effort: M | Dependencies: Phase 0, Phase 1, Phase 2**

- [ ] Create `scip-php/src/Indexing/ExpressionTracker.php` with:
  - Constructor: `(IndexingContext $ctx, CallRecordBuilder $callBuilder, TypeResolver $typeResolver, SymbolNamer $namer, Types $types)`
  - Public methods (3):
    - `track(PosResolver $pos, Node\Expr $expr): ?string` (renamed from `trackExpression`)
    - `getExpressionId(Node\Expr $expr): ?string`
    - `isCallExpression(Node\Expr $expr): bool`
  - Public method: `buildValueRecord(PosResolver $pos, Node\Expr $exprNode, string $kind, ?string $symbol, ?string $sourceCallId, ?string $sourceValueId): ?ValueRecord`
  - Private methods (13 track* methods moved from DocIndexer):
    - `trackVariableExpression`, `trackPropertyFetchExpression`, `trackNullsafePropertyFetchExpression`
    - `trackStaticPropertyFetchExpression`, `trackClassConstFetchExpression`, `trackConstFetchExpression`
    - `trackArrayDimFetchExpression`, `trackCoalesceExpression`, `trackBinaryOpExpression`
    - `trackTernaryExpression`, `trackMatchExpression`, `trackLiteralExpression`
- [ ] Modify `scip-php/src/DocIndexer.php`:
  - Add `private readonly ExpressionTracker $exprTracker` property
  - Create in constructor: `$this->exprTracker = new ExpressionTracker($this->ctx, $this->callBuilder, $this->typeResolver, $namer, $types)`
  - Remove 16 methods from DocIndexer
  - Replace call sites: `$this->trackExpression(...)` -> `$this->exprTracker->track(...)` etc.
- [ ] Create `scip-php/tests/Indexing/ExpressionTrackerTest.php`
- [ ] Run all 4 test gates

---

### Phase 4: LocalVariableTracker (ISSUE-E) — Variable Management [Highest Risk]

**Risk: HIGH | Effort: L | Dependencies: Phase 0, Phase 1, Phase 3**

- [ ] Create `scip-php/src/Indexing/LocalVariableTracker.php` with:
  - Constructor: `(IndexingContext $ctx, ExpressionTracker $exprTracker, TypeResolver $typeResolver, SymbolNamer $namer, Types $types)`
  - Public methods (6 moved from DocIndexer, some renamed):
    - `registerParameterType(Param $n): void`
    - `createParameterValueRecord(PosResolver $pos, Param $n, ?string $promotedPropertySymbol): void`
    - `handleAssignment(PosResolver $pos, Assign $n): void` (was `handleLocalVariable`)
    - `handleReference(PosResolver $pos, Variable $n): void` (was `handleLocalVariableRef`)
    - `handleForeach(PosResolver $pos, Foreach_ $n): void` (was `handleForeachVariable`)
    - `ensureForeachVarRegistered(PosResolver $pos, Variable $n, string $varName, string $scope): void`
  - Private method: `registerForeachVar(...)` (moved from DocIndexer)
- [ ] Modify `scip-php/src/DocIndexer.php`:
  - Add `private readonly LocalVariableTracker $localTracker` property
  - Remove `DocGenerator $docGenerator` property (moves to LocalVariableTracker — but also needed by ScipDefinitionEmitter in Phase 5, so create a shared instance)
  - Create in constructor: `$this->localTracker = new LocalVariableTracker($this->ctx, $this->exprTracker, $this->typeResolver, $namer, $types)`
  - Remove 7 methods from DocIndexer
  - Replace dispatch calls in `index()`:
    - `$this->registerParameterType($n)` -> `$this->localTracker->registerParameterType($n)`
    - `$this->createParameterValueRecord(...)` -> `$this->localTracker->createParameterValueRecord(...)`
    - `$this->handleLocalVariable($pos, $n)` -> `$this->localTracker->handleAssignment($pos, $n)`
    - `$this->handleLocalVariableRef($pos, $n)` -> `$this->localTracker->handleReference($pos, $n)`
    - `$this->handleForeachVariable($pos, $n)` -> `$this->localTracker->handleForeach($pos, $n)`
    - `$this->ensureForeachVarRegistered(...)` -> `$this->localTracker->ensureForeachVarRegistered(...)`
- [ ] Create `scip-php/tests/Indexing/LocalVariableTrackerTest.php`
- [ ] Run all 4 test gates

**Note on DocGenerator**: `DocGenerator` is currently created in DocIndexer's constructor and used by:
- `def()` in DocIndexer (R2 — moves to ScipDefinitionEmitter in Phase 5)
- `handleLocalVariable()` and `handleForeachVariable()` (R4 — moves to LocalVariableTracker in Phase 4)

In Phase 4, LocalVariableTracker needs DocGenerator for building variable documentation strings (e.g., `['```php', '$varName: Type', '```']`). However, LocalVariableTracker only uses the formatting pattern directly — it does NOT call `$this->docGenerator->create()`. It builds doc arrays inline. So LocalVariableTracker does NOT need DocGenerator.

---

### Phase 5: ScipDefinitionEmitter + ScipReferenceEmitter (ISSUE-F) — SCIP Emission

**Risk: LOW | Effort: M | Dependencies: Phase 0**

- [ ] Create `scip-php/src/Indexing/ScipDefinitionEmitter.php` with:
  - Constructor: `(IndexingContext $ctx, SymbolNamer $namer, Types $types, DocGenerator $docGenerator, DocCommentParser $docCommentParser)`
  - Public methods (2):
    - `emitDefinition(PosResolver $pos, Const_|ClassLike|ClassMethod|EnumCase|Function_|Param|PropertyItem $n, Node $posNode, int $kind): void` (was `def()`)
    - `emitDocDefinition(?Doc $doc, string $tagName, PhpDocTagValueNode $node, string $name, string $symbol, int $kind): void` (was `docDef()`)
  - Private methods (7 moved from DocIndexer):
    - `extractRelationships(ClassLike $n): array`
    - `extractMethodRelationships(ClassMethod $n, string $methodSymbol): array`
    - `extractReturnTypeRelationships(ClassMethod|Function_ $n): array`
    - `extractTypeRelationships(Node $type): array`
    - `collectTypeSymbols(Node $type): array`
    - `resolveNameToSymbol(Name $name): ?string`
    - `registerSyntheticType(string $symbol, array $constituents, bool $isIntersection): void`
- [ ] Create `scip-php/src/Indexing/ScipReferenceEmitter.php` with:
  - Constructor: `(IndexingContext $ctx, SymbolNamer $namer, Composer $composer)`
  - Public method:
    - `emitReference(PosResolver $pos, string $symbol, Node $posNode, int $kind, int $role): void` (was `ref()`)
- [ ] Modify `scip-php/src/DocIndexer.php`:
  - Add `private readonly ScipDefinitionEmitter $defEmitter` and `private readonly ScipReferenceEmitter $refEmitter`
  - Remove `DocGenerator`, `DocCommentParser` properties (moved to ScipDefinitionEmitter)
  - Remove 10 methods from DocIndexer (`def`, `docDef`, `ref`, 7 private relationship/symbol methods)
  - Replace dispatch calls in `index()`:
    - `$this->def($pos, ...)` -> `$this->defEmitter->emitDefinition($pos, ...)`
    - `$this->docDef(...)` -> `$this->defEmitter->emitDocDefinition(...)`
    - `$this->ref($pos, ...)` -> `$this->refEmitter->emitReference($pos, ...)`
  - DocIndexer is now ~150 lines: constructor wiring + `index()` dispatch + `getContext()` + `resetLocals()`
- [ ] Create `scip-php/tests/Indexing/ScipDefinitionEmitterTest.php`
- [ ] Create `scip-php/tests/Indexing/ScipReferenceEmitterTest.php`
- [ ] Run all 4 test gates

---

## File Manifest

| Action | File Path | Phase | Description |
|--------|-----------|-------|-------------|
| CREATE | `scip-php/src/Indexing/IndexingContext.php` | 0 | Shared state bag (14 properties + resetLocals) |
| CREATE | `scip-php/src/Indexing/TypeResolver.php` | 1 | Stateless type/kind resolution (9 methods) |
| CREATE | `scip-php/src/Indexing/CallRecordBuilder.php` | 2 | Call record construction (3 methods) |
| CREATE | `scip-php/src/Indexing/ExpressionTracker.php` | 3 | Recursive expression tracking (16 methods) |
| CREATE | `scip-php/src/Indexing/LocalVariableTracker.php` | 4 | Variable management (7 methods) |
| CREATE | `scip-php/src/Indexing/ScipDefinitionEmitter.php` | 5 | SCIP definitions + relationships (9 methods) |
| CREATE | `scip-php/src/Indexing/ScipReferenceEmitter.php` | 5 | SCIP references (1 method) |
| MODIFY | `scip-php/src/DocIndexer.php` | 0-5 | Progressively reduced from 2,669 to ~150 lines |
| MODIFY | `scip-php/src/Indexer.php` | 0 | Property access via `getContext()` |
| CREATE | `scip-php/tests/Indexing/IndexingContextTest.php` | 0 | Unit tests for IndexingContext |
| CREATE | `scip-php/tests/Indexing/TypeResolverTest.php` | 1 | Unit tests for TypeResolver |
| CREATE | `scip-php/tests/Indexing/CallRecordBuilderTest.php` | 2 | Unit tests for CallRecordBuilder |
| CREATE | `scip-php/tests/Indexing/ExpressionTrackerTest.php` | 3 | Unit tests for ExpressionTracker |
| CREATE | `scip-php/tests/Indexing/LocalVariableTrackerTest.php` | 4 | Unit tests for LocalVariableTracker |
| CREATE | `scip-php/tests/Indexing/ScipDefinitionEmitterTest.php` | 5 | Unit tests for ScipDefinitionEmitter |
| CREATE | `scip-php/tests/Indexing/ScipReferenceEmitterTest.php` | 5 | Unit tests for ScipReferenceEmitter |

**Total: 7 new source files, 2 modified source files, 7 new test files**

---

## File Ownership Suggestion

All phases modify `DocIndexer.php`, so work MUST be sequential. The handoff point is after Phase 2.

| Developer | Phases | Files Owned | Rationale |
|-----------|--------|-------------|-----------|
| developer-1 | 0, 1, 2 | `IndexingContext.php`, `TypeResolver.php`, `CallRecordBuilder.php`, `DocIndexer.php` (phases 0-2), `Indexer.php` | Foundation chain: context -> type resolver -> call builder. Lowest risk phases. Establishes patterns for Dev-2. |
| developer-2 | 3, 4, 5 | `ExpressionTracker.php`, `LocalVariableTracker.php`, `ScipDefinitionEmitter.php`, `ScipReferenceEmitter.php`, `DocIndexer.php` (phases 3-5) | Higher-complexity phases. ExpressionTracker + LocalVariableTracker have the deepest state coupling. Takes over DocIndexer.php after Dev-1 commits Phase 2. |

### Handoff Protocol

1. Developer-1 completes Phase 2 and passes all 4 test gates
2. Developer-1 commits Phase 2 changes
3. Developer-2 pulls Phase 2 commit as their starting point
4. Developer-2 begins Phase 3 on that commit

**No parallel development on different phases is safe.** Each phase changes DocIndexer.php.

---

## Interface Contracts

### IndexingContext (shared by all services)

```php
namespace ScipPhp\Indexing;

final class IndexingContext
{
    // Immutable config (set in constructor, never changed)
    public readonly string $relativePath;
    public readonly bool $experimental;

    // Output collections (append-only, read by Indexer.php)
    /** @var array<non-empty-string, SymbolInformation> */
    public array $symbols = [];
    /** @var array<non-empty-string, SymbolInformation> */
    public array $extSymbols = [];
    /** @var list<Occurrence> */
    public array $occurrences = [];
    /** @var list<ValueRecord> */
    public array $values = [];
    /** @var list<CallRecord> */
    public array $calls = [];
    /** @var array<non-empty-string, SymbolInformation> */
    public array $syntheticTypeSymbols = [];

    // Tracking state (read/write by services, reset per scope)
    public int $localCounter = 0;
    /** @var array<string, string> */
    public array $localSymbols = [];
    /** @var array<string, int> */
    public array $localAssignmentLines = [];
    /** @var array<string, string> */
    public array $localCallsSymbols = [];
    /** @var array<int, string> */
    public array $expressionIds = [];
    /** @var array<string, string> */
    public array $localValueIds = [];
    /** @var array<string, string> */
    public array $parameterValueIds = [];

    // Experimental kinds constant (moved from DocIndexer)
    private const EXPERIMENTAL_KINDS = [...];

    public function isExperimentalKind(string $kind): bool;
    public function resetLocals(): void;  // Clears tracking + calls/values, preserves symbols/extSymbols/occurrences
}
```

### Service Dependencies (constructor injection)

```
IndexingContext ← standalone (no service dependencies)
TypeResolver(SymbolNamer, Types) ← standalone
CallRecordBuilder(IndexingContext, TypeResolver, SymbolNamer, Types) ← depends on ctx + typeResolver
ExpressionTracker(IndexingContext, CallRecordBuilder, TypeResolver, SymbolNamer, Types) ← depends on ctx + callBuilder + typeResolver
LocalVariableTracker(IndexingContext, ExpressionTracker, TypeResolver, SymbolNamer, Types) ← depends on ctx + exprTracker + typeResolver
ScipDefinitionEmitter(IndexingContext, SymbolNamer, Types, DocGenerator, DocCommentParser) ← depends on ctx only
ScipReferenceEmitter(IndexingContext, SymbolNamer, Composer) ← depends on ctx only
```

### DocIndexer Constructor Wiring (final state)

```php
public function __construct(
    Composer $composer,
    SymbolNamer $namer,
    Types $types,
    string $relativePath = '',
    bool $experimental = false,
) {
    $this->ctx = new IndexingContext($relativePath, $experimental);
    $this->typeResolver = new TypeResolver($namer, $types);
    $this->callBuilder = new CallRecordBuilder($this->ctx, $this->typeResolver, $namer, $types);
    $this->exprTracker = new ExpressionTracker($this->ctx, $this->callBuilder, $this->typeResolver, $namer, $types);
    $this->localTracker = new LocalVariableTracker($this->ctx, $this->exprTracker, $this->typeResolver, $namer, $types);
    $docGenerator = new DocGenerator();
    $docCommentParser = new DocCommentParser();
    $this->defEmitter = new ScipDefinitionEmitter($this->ctx, $namer, $types, $docGenerator, $docCommentParser);
    $this->refEmitter = new ScipReferenceEmitter($this->ctx, $namer, $composer);
}
```

### Cross-Service State Access Rules

| Service | Reads from ctx | Writes to ctx |
|---------|---------------|---------------|
| TypeResolver | (none — stateless) | (none) |
| CallRecordBuilder | `relativePath`, `experimental` | `calls`, `values`, `expressionIds` |
| ExpressionTracker | `expressionIds`, `localCallsSymbols`, `localValueIds`, `parameterValueIds`, `experimental` | `values`, `expressionIds` |
| LocalVariableTracker | `localSymbols`, `localCallsSymbols`, `localAssignmentLines`, `localValueIds`, `parameterValueIds`, `localCounter`, `relativePath` | `symbols`, `occurrences`, `values`, `localCounter`, `localSymbols`, `localCallsSymbols`, `localAssignmentLines`, `localValueIds`, `parameterValueIds`, `expressionIds` |
| ScipDefinitionEmitter | (none directly) | `symbols`, `occurrences`, `syntheticTypeSymbols` |
| ScipReferenceEmitter | (none directly) | `extSymbols`, `occurrences` |

---

## Test Cases

### Phase 0: IndexingContextTest

| Test Case | Description | Assertion |
|-----------|-------------|-----------|
| `testResetLocalsClearsTrackingState` | Call `resetLocals()` after adding tracking data | `localCounter === 0`, all tracking arrays empty, `calls` and `values` empty |
| `testResetLocalsPreservesOutputSymbols` | Add symbols/occurrences, then `resetLocals()` | `symbols`, `extSymbols`, `occurrences` preserved |
| `testRelativePathIsImmutable` | Access `$ctx->relativePath` after construction | Matches constructor argument |
| `testExperimentalIsImmutable` | Access `$ctx->experimental` after construction | Matches constructor argument |
| `testOutputArraysAreAppendable` | Append to `$ctx->symbols`, `$ctx->values` etc. | Arrays contain appended items |
| `testIsExperimentalKind` | Check `isExperimentalKind('function')` = true, `isExperimentalKind('method')` = false | Correct gating |

### Phase 1: TypeResolverTest

| Test Case | Description | Assertion |
|-----------|-------------|-----------|
| `testResolveCallKindForMethodCall` | Pass `MethodCall` node | Returns `'method'` |
| `testResolveCallKindForStaticCall` | Pass `StaticCall` node | Returns `'method_static'` |
| `testResolveCallKindForNew` | Pass `New_` node | Returns `'constructor'` |
| `testResolveCallKindForFuncCall` | Pass `FuncCall` node | Returns `'function'` |
| `testResolveCallKindForPropertyFetch` | Pass `PropertyFetch` node | Returns `'access'` |
| `testResolveCallKindForNullsafePropertyFetch` | Pass `NullsafePropertyFetch` node | Returns `'access'` |
| `testResolveCallKindForStaticPropertyFetch` | Pass `StaticPropertyFetch` node | Returns `'access_static'` |
| `testResolveCallKindForArrayDimFetch` | Pass `ArrayDimFetch` node | Returns `'access_array'` |
| `testResolveCallKindForCoalesce` | Pass `Coalesce` node | Returns `'coalesce'` |
| `testResolveCallKindForTernaryElvis` | Pass `Ternary` with null if | Returns `'ternary'` |
| `testResolveCallKindForTernaryFull` | Pass `Ternary` with non-null if | Returns `'ternary_full'` |
| `testResolveCallKindForMatch` | Pass `Match_` node | Returns `'match'` |
| `testResolveCallKindFallback` | Pass unexpected node type | Returns `'method'` |
| `testResolveKindTypeInvocation` | Pass `'method'`, `'constructor'`, `'function'` | Returns `'invocation'` |
| `testResolveKindTypeAccess` | Pass `'access'`, `'access_static'`, `'access_array'` | Returns `'access'` |
| `testResolveKindTypeOperator` | Pass `'coalesce'`, `'ternary'`, `'match'` | Returns `'operator'` |
| `testResolveKindTypeDefault` | Pass unknown kind string | Returns `'invocation'` |
| `testFormatTypeSymbolNull` | Pass `null` | Returns `null` |
| `testFormatTypeSymbolSingleType` | Pass Type with one symbol | Returns that symbol string |
| `testFormatTypeSymbolUnionType` | Pass Type with multiple symbols | Returns union symbol via namer |
| `testFindEnclosingScopeInMethod` | Variable inside ClassMethod | Returns method symbol |
| `testFindEnclosingScopeInFunction` | Variable inside Function_ | Returns function symbol |
| `testFindEnclosingScopeTopLevel` | Variable at top level | Returns `null` |

### Phase 2: CallRecordBuilderTest

| Test Case | Description | Assertion |
|-----------|-------------|-----------|
| `testBuildMethodCallRecord` | MethodCall with 2 positional args | Returns CallRecord with kind='method', correct callee, 2 arguments |
| `testBuildConstructorCallRecord` | New_ expression | Returns CallRecord with kind='constructor' |
| `testBuildPropertyAccessRecord` | PropertyFetch expression | Returns CallRecord via `buildAccessOrOperatorCallRecord` with kind='access' |
| `testBuildOperatorCallRecord` | Coalesce expression | Returns CallRecord with kind='coalesce', leftValueId/rightValueId set |
| `testArgumentBindingPositional` | 3 positional args | Arguments have position 0, 1, 2 |
| `testArgumentBindingNamed` | Named argument `flush: true` | Position matches callee's param index |
| `testReceiverTracking` | MethodCall with receiver | `receiverValueId` is non-null |
| `testAddCallWithResultValue` | Add a CallRecord | `ctx->calls` has record; `ctx->values` has result ValueRecord with kind='result' |
| `testNullCallerReturnsNull` | Top-level call (no enclosing scope) | Returns `null` |
| `testExpressionIdTracked` | Build a call record | `ctx->expressionIds[spl_object_id(node)]` is set |

### Phase 3: ExpressionTrackerTest

| Test Case | Description | Assertion |
|-----------|-------------|-----------|
| `testTrackVariableLookupLocal` | Variable `$x` with known local | Returns existing value ID from `localValueIds` |
| `testTrackVariableLookupParameter` | Variable `$param` with known param | Returns existing value ID from `parameterValueIds` |
| `testTrackVariableThisSkipped` | Variable `$this` | Returns `null` |
| `testTrackLiteralExpression` | Scalar string | Returns value ID, creates ValueRecord with kind='literal' |
| `testTrackPropertyFetchChaining` | `$obj->foo->bar` | Two CallRecords created, second has receiverValueId pointing to first |
| `testCachedExpressionReturnsSameId` | Call `track()` twice on same node | Second call returns same ID without creating duplicate |
| `testExperimentalGatingOff` | Track ArrayDimFetch with experimental=false | Returns `null` |
| `testExperimentalGatingOn` | Track ArrayDimFetch with experimental=true | Returns call ID |
| `testIsCallExpressionTrue` | MethodCall, PropertyFetch, Coalesce | Returns `true` |
| `testIsCallExpressionFalse` | Variable, Scalar | Returns `false` (these are value expressions) |

### Phase 4: LocalVariableTrackerTest

| Test Case | Description | Assertion |
|-----------|-------------|-----------|
| `testAssignmentCreatesScipAndValue` | `$x = expr` | SCIP SymbolInformation + Occurrence created; ValueRecord with kind='local' created |
| `testParameterShadowing` | `$x = expr` after param `$x` | `ctx->localSymbols` has local symbol, not param |
| `testForeachKeyAndValue` | `foreach ($arr as $k => $v)` | Two local symbols, two ValueRecords created |
| `testParameterValueRecord` | `function foo(int $x)` | ValueRecord with kind='parameter' at param position |
| `testPromotedPropertyValueRecord` | Constructor promotion `private int $x` | ValueRecord with `promotedPropertySymbol` set |
| `testOneValuePerDeclaration` | `createParameterValueRecord` called twice | Second call is no-op (early return) |
| `testReassignmentUsesWriteAccess` | `$x = 1; $x = 2;` | First occurrence has Definition role; second has WriteAccess role |
| `testHandleReferenceResolvesLocal` | Read `$x` after assignment | Occurrence with local symbol, UnspecifiedSymbolRole |
| `testHandleReferenceResolvesParameter` | Read `$param` (no local shadow) | Occurrence with parameter symbol |
| `testEnsureForeachVarRegisteredEarlyRegistration` | Reference to `$v` inside foreach body | Variable registered from ancestor Foreach_ node |

### Phase 5: ScipDefinitionEmitterTest + ScipReferenceEmitterTest

| Test Case | Description | Assertion |
|-----------|-------------|-----------|
| `testEmitClassDefinition` | ClassLike node | `ctx->symbols` has entry with Definition role |
| `testEmitMethodDefinition` | ClassMethod node | `ctx->symbols` + occurrence with enclosing_range |
| `testEmitPropertyDefinition` | PropertyItem node | `ctx->symbols` + occurrence |
| `testExtractsImplementsRelationship` | Class implements Interface | Relationship with `is_implementation: true` |
| `testExtractsExtendsRelationship` | Class extends Parent | Relationship with `is_reference: true` |
| `testExtractsTraitUseRelationship` | Class uses Trait | Relationship with both flags true |
| `testEmitDocDefinition` | `@property` in docblock | Occurrence for doc-comment definition |
| `testInvalidUtf8SkippedDef` | Symbol with invalid UTF-8 | Early return, nothing emitted |
| `testEmitReferenceForInternal` | Reference to project symbol | Occurrence created, NOT in `extSymbols` |
| `testEmitReferenceForExternal` | Reference to dependency symbol | Occurrence + entry in `ctx->extSymbols` |
| `testInvalidUtf8SkippedRef` | Symbol with invalid UTF-8 | Early return, nothing emitted |

---

## Risks & Mitigations

| Risk | Phase | Impact | Probability | Mitigation |
|------|-------|--------|-------------|------------|
| State property access pattern changes produce subtle output differences | 0 | HIGH | LOW | 4 test gates catch any difference. Phase 0 is purely mechanical ($this->prop -> $this->ctx->prop). |
| Nullsafe union consolidation in TypeResolver introduces logic change | 1 | HIGH | LOW | Extract exact code from all 3 locations, diff them, then consolidate. Unit test each case. |
| PrettyPrinter move to CallRecordBuilder changes serialization behavior | 2 | MEDIUM | LOW | PrettyPrinter is stateless and created fresh — moving it has no effect. |
| Expression tracking order changes due to method extraction | 3 | HIGH | MEDIUM | ExpressionTracker must preserve exact recursion order: check cache first, then match expression type, then recurse. Binary comparison (Gate 3) catches any ordering difference. |
| Foreach early registration broken by LocalVariableTracker extraction | 4 | HIGH | MEDIUM | `ensureForeachVarRegistered()` calls `registerForeachVar()` which accesses 6 state properties. All must be on ctx. Extensive unit testing of the foreach-before-body pattern. |
| Dual symbol system (SCIP local N + calls descriptive) broken | 4 | HIGH | LOW | Both symbol systems go through SymbolNamer. LocalVariableTracker uses the same namer instance. Unit test both outputs for each variable type. |
| ScipDefinitionEmitter misses a relationship extraction method | 5 | MEDIUM | LOW | File manifest lists all 7 private methods. Code search for `$this->namer->name` in DocIndexer after extraction confirms all moved. |
| Docker build caching hides stale code | All | MEDIUM | MEDIUM | Always run `cd scip-php && ./build/build.sh` before Gate 2 and Gate 3. Use `--no-cache` if behavior is suspicious. |
| PHPCS violations in new files due to strict Slevomat rules | All | LOW | HIGH | Run `phpcs` early during development, not just at gate. Pay attention to: trailing commas, `use function` imports, single-line/multi-line signature thresholds, `final` class requirement. |
| PHPStan fails due to property type narrowing changes | 0 | MEDIUM | MEDIUM | IndexingContext properties must have identical PHPDoc types as the original DocIndexer properties. Copy type annotations verbatim. |

---

## Per-Phase Verification Checklist

Run this after EVERY phase:

```bash
#!/bin/bash
set -e

echo "=== GATE 1: Snapshot Tests ==="
./tests/snapshot.sh verify tests/snapshot-BASELINE.json
echo "GATE 1: PASS"

echo "=== GATE 2: Contract Tests ==="
cd scip-php && ./build/build.sh && cd ..
cd kloc-reference-project-php/contract-tests && bin/run.sh test
echo "GATE 2: PASS"
cd ../..

echo "=== GATE 3: Index Binary Comparison ==="
./bin/scip-php.sh -d ./kloc-reference-project-php -o /tmp/scip-verify
diff /tmp/index-before.json /tmp/scip-verify/index.json
./bin/scip-php.sh -d ./kloc-reference-project-php -o /tmp/scip-verify-exp --experimental
diff /tmp/index-before-exp.json /tmp/scip-verify-exp/index.json
echo "GATE 3: PASS"

echo "=== GATE 4: Unit Tests + Static Analysis ==="
cd scip-php
docker run --rm -v $(pwd):/app scip-php-dev vendor/bin/phpunit --no-coverage
docker run --rm -v $(pwd):/app scip-php-dev vendor/bin/phpstan --memory-limit=2G
docker run --rm -v $(pwd):/app scip-php-dev vendor/bin/phpcs
cd ..
echo "GATE 4: PASS"

echo ""
echo "ALL 4 GATES PASSED"
```

**Note:** Replace `tests/snapshot-BASELINE.json` with the actual snapshot filename captured in the pre-phase step.
