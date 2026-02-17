# Feature: scip-php DocIndexer Refactoring

**Feature Name**: scip-php-refactor
**Status**: Implementation Ready
**Version**: 1.0
**Date**: 2026-02-17

---

## Goal

`scip-php/src/DocIndexer.php` is a 2,669-line monolithic AST visitor with 45 methods and 14 mutable properties. It currently handles 7 distinct responsibilities in a single class: node dispatch, SCIP definitions, SCIP references, local variable management, call record building, expression tracking, and type resolution.

This refactoring extracts each responsibility into a focused single-responsibility service class while preserving **identical output behavior**. No output format changes. No downstream impact (kloc-mapper, kloc-cli, sot.json format all unchanged).

**Who benefits:**
- Developers maintaining or extending scip-php: each service is independently testable and understandable
- Future contributors: adding a new PHP node type requires touching one focused file, not searching 2,669 lines
- CI/QA: new unit test coverage per extracted service reduces regression risk

---

## Usage Examples

This is a pure internal refactoring — no CLI interface changes, no output format changes. Before and after refactoring, scip-php produces bit-identical output for all inputs.

### Before: DocIndexer as a monolith

```
scip-php/src/
└── DocIndexer.php    (2,669 lines — 45 methods, 14 properties, 7 concerns)
```

```php
// Indexer.php creates one DocIndexer per file
$docIndexer = new DocIndexer($composer, $namer, $types, $relativePath, $experimental);
$parser->traverse($ast, fn($n) => $docIndexer->index($pos, $n));

// Results read from public properties
$symbols     = $docIndexer->symbols;
$occurrences = $docIndexer->occurrences;
$calls       = $docIndexer->calls;
$values      = $docIndexer->values;
```

### After: DocIndexer as a dispatcher (Phase 5 final state)

```
scip-php/src/
├── DocIndexer.php                   (~150 lines — dispatcher + wiring)
└── Indexing/
    ├── IndexingContext.php           (~80 lines — shared state bag)
    ├── TypeResolver.php              (~200 lines — stateless type/kind resolution)
    ├── CallRecordBuilder.php         (~350 lines — call + result value building)
    ├── ExpressionTracker.php         (~450 lines — recursive expression tracking)
    ├── LocalVariableTracker.php      (~500 lines — local/param/foreach tracking)
    ├── ScipDefinitionEmitter.php     (~250 lines — SCIP definitions + relationships)
    └── ScipReferenceEmitter.php      (~80 lines — SCIP references)
```

```php
// Indexer.php — same call, results now from IndexingContext
$docIndexer = new DocIndexer($composer, $namer, $types, $relativePath, $experimental);
$parser->traverse($ast, fn($n) => $docIndexer->index($pos, $n));

// Results read from IndexingContext (via getContext())
$ctx         = $docIndexer->getContext();
$symbols     = $ctx->symbols;
$occurrences = $ctx->occurrences;
$calls       = $ctx->calls;
$values      = $ctx->values;
```

---

## Detailed Behavior

### Current State: 7 Identified Responsibilities

| # | Concern | Methods | Lines |
|---|---------|---------|-------|
| R1 | Node Dispatch | `index()` | 206–431 (~225) |
| R2 | SCIP Definitions | `def()`, `docDef()`, `extract*Relationships()` | 433–843 (~410) |
| R3 | SCIP References | `ref()` | 847–873 (~27) |
| R4 | Local Variable Mgmt | `handleLocal*()`, `handleForeach*()`, params | 915–1522 (~600) |
| R5 | Call Record Building | `buildCallRecord()`, `buildAccess*()` | 1533–1757 (~225) |
| R6 | Expression Tracking | 14 `track*()` methods | 1818–2324 (~500) |
| R7 | Type Resolution | `resolve*()`, `format*()`, `findEnclosingScope()` | 884–907, 2333–2669 (~300) |

### Dependency Chain

```
R1 (Dispatch) ──→ R2 (Definitions)
       │──→ R3 (References)
       │──→ R4 (Local Vars)  ──→ R6 (Expr Tracking) ──→ R5 (Call Building)
       │──→ R5 (Call Building) ──→ R7 (Type Resolution)
       │──→ R6 (Expr Tracking)
       └──→ R7 (Type Resolution)
```

### Shared Mutable State: 14 Properties

All 14 properties move into `IndexingContext` (Phase 0):

| Property | Category | Written By | Read By |
|----------|----------|------------|---------|
| `symbols` | output | R2, R4 | Indexer |
| `extSymbols` | output | R3 | Indexer |
| `occurrences` | output | R2, R3, R4 | Indexer |
| `values` | output | R4, R5, R6 | Indexer |
| `calls` | output | R5, R6 | Indexer |
| `syntheticTypeSymbols` | output | R2 (disabled) | Indexer |
| `localCounter` | tracking | R4 | R4 |
| `localSymbols` | tracking | R4 | R4, R6 |
| `localCallsSymbols` | tracking | R4 | R4, R6 |
| `localAssignmentLines` | tracking | R4 | R4 |
| `expressionIds` | tracking | R4, R5, R6 | R6 |
| `localValueIds` | tracking | R4 | R4, R6 |
| `parameterValueIds` | tracking | R4 | R4, R6 |

---

### Phase 0 — IndexingContext (ISSUE-A)

**Goal:** Extract all 14 mutable properties into a shared state bag. Prerequisite for all subsequent phases.

**New class:** `scip-php/src/Indexing/IndexingContext.php`

```php
final class IndexingContext
{
    // Output collections (append-only)
    public array $symbols = [];
    public array $extSymbols = [];
    public array $occurrences = [];
    public array $values = [];
    public array $calls = [];
    public array $syntheticTypeSymbols = [];

    // Tracking state (read/write, reset per file)
    public int $localCounter = 0;
    public array $localSymbols = [];
    public array $localAssignmentLines = [];
    public array $localCallsSymbols = [];
    public array $expressionIds = [];
    public array $localValueIds = [];
    public array $parameterValueIds = [];

    public function __construct(
        public readonly string $relativePath,
        public readonly bool $experimental,
    ) {}

    public function resetLocals(): void { /* clears per-file tracking + calls/values */ }
}
```

**DocIndexer changes:**
- All 14 properties removed from DocIndexer
- DocIndexer holds `private readonly IndexingContext $ctx`
- All `$this->symbols` → `$this->ctx->symbols` throughout
- `resetLocals()` delegates to `$this->ctx->resetLocals()`
- New method: `getContext(): IndexingContext`

**Indexer.php changes:**
- `$docIndexer->symbols` → `$docIndexer->getContext()->symbols` etc.

**New tests:** `scip-php/tests/Indexing/IndexingContextTest.php`
- `resetLocals()` clears tracking state but not output symbols
- `relativePath` and `experimental` are immutable after construction
- Output arrays are appendable

---

### Phase 1 — TypeResolver (ISSUE-B)

**Goal:** Extract 9 stateless type/kind resolution methods. Zero state impact — pure functions on Types + SymbolNamer.

**New class:** `scip-php/src/Indexing/TypeResolver.php`

```php
final class TypeResolver
{
    public function __construct(
        private readonly SymbolNamer $namer,
        private readonly Types $types,
    ) {}

    public function formatTypeForDoc(?Type $type): string { ... }      // moved from DocIndexer:884
    public function formatTypeSymbol(?Type $type): ?string { ... }     // moved from DocIndexer:1311
    public function resolveExpressionReturnType(Node\Expr $expr): ?string { ... } // from :2333
    public function resolveAccessReturnType(Node\Expr $expr, ?string $symbol): ?string { ... } // from :2386
    public function resolveCallKind(Node $callNode): string { ... }    // from :2446
    public function resolveKindType(string $kind): string { ... }      // from :2530
    public function resolveReturnType(Node $callNode, string $calleeSymbol): ?string { ... } // from :2555
    public function resolveValueType(Node\Expr $expr): ?string { ... } // from :2615
    public function findEnclosingScope(Node $n): ?string { ... }       // from :2640

    private function applyNullsafeUnion(array $flat, bool $isNullsafe): ?string { ... } // NEW: deduplicates 3 copies
}
```

**Key improvement:** The nullsafe union logic (currently duplicated 3 times across DocIndexer) is consolidated into a single private `applyNullsafeUnion()` helper.

**DocIndexer changes:**
- 9 methods removed
- All call sites updated: `$this->resolveCallKind()` → `$this->typeResolver->resolveCallKind()`

**New tests:** `scip-php/tests/Indexing/TypeResolverTest.php`
- `resolveCallKind()` for each of 13 node types
- `resolveKindType()` for each of 4 kind strings
- `formatTypeSymbol()` with null, single type, union type
- `findEnclosingScope()` walking parent nodes

---

### Phase 2 — CallRecordBuilder (ISSUE-C)

**Goal:** Extract 3 call record building methods. Writes to `ctx->calls`, `ctx->values`, `ctx->expressionIds`.

**New class:** `scip-php/src/Indexing/CallRecordBuilder.php`

```php
final class CallRecordBuilder
{
    public function __construct(
        private readonly IndexingContext $ctx,
        private readonly TypeResolver $typeResolver,
        private readonly SymbolNamer $namer,
        private readonly Types $types,
    ) {}

    public function buildCallRecord(PosResolver $pos, Node $callNode, string $calleeSymbol, array $args): ?CallRecord { ... }
    public function addCallWithResultValue(CallRecord $callRecord): void { ... }
    public function buildAccessOrOperatorCallRecord(PosResolver $pos, Node\Expr $exprNode, ?string $symbol, ...): ?CallRecord { ... }
}
```

**DocIndexer changes:**
- 3 methods removed
- All sites: `$this->buildCallRecord()` → `$this->callBuilder->buildCallRecord()`
- `$this->relativePath` references replaced with `$this->ctx->relativePath`

**New tests:** `scip-php/tests/Indexing/CallRecordBuilderTest.php`
- Method call record creation
- Constructor call record
- Property access record
- Operator call record
- Argument binding (named + positional)
- Receiver tracking

---

### Phase 3 — ExpressionTracker (ISSUE-D)

**Goal:** Extract 16 expression tracking methods (14 `track*()` private methods + `getExpressionId()` + `isCallExpression()`). Manages recursive expression tree walking that links values to calls.

**New class:** `scip-php/src/Indexing/ExpressionTracker.php`

```php
final class ExpressionTracker
{
    public function __construct(
        private readonly IndexingContext $ctx,
        private readonly CallRecordBuilder $callBuilder,
        private readonly TypeResolver $typeResolver,
        private readonly SymbolNamer $namer,
        private readonly Types $types,
    ) {}

    public function track(PosResolver $pos, Node\Expr $expr): ?string { ... } // renamed from trackExpression
    public function getExpressionId(Node\Expr $expr): ?string { ... }
    public function isCallExpression(Node\Expr $expr): bool { ... }

    // 13 private trackers (trackVariableExpression, trackPropertyFetchExpression, etc.)
}
```

**Key API change:** `trackExpression()` is renamed to `track()` for a cleaner public API.

**State access:** reads/writes `ctx->expressionIds`; reads `ctx->localCallsSymbols`, `ctx->localValueIds`, `ctx->parameterValueIds`; writes `ctx->values`.

**DocIndexer changes:**
- 16 methods removed
- `$this->trackExpression()` → `$this->exprTracker->track()`
- `$this->isCallExpression()` → `$this->exprTracker->isCallExpression()`

**New tests:** `scip-php/tests/Indexing/ExpressionTrackerTest.php`
- Variable lookup (local, parameter)
- Literal tracking
- Property fetch chaining
- Cached expression (second track() returns same ID)
- Experimental gating (`--experimental=false` skips operator calls)

---

### Phase 4 — LocalVariableTracker (ISSUE-E) [Highest Risk]

**Goal:** Extract 7 local variable tracking methods. Produces dual output: both SCIP symbols/occurrences AND calls.json ValueRecords. Highest risk due to state complexity.

**New class:** `scip-php/src/Indexing/LocalVariableTracker.php`

```php
final class LocalVariableTracker
{
    public function __construct(
        private readonly IndexingContext $ctx,
        private readonly ExpressionTracker $exprTracker,
        private readonly TypeResolver $typeResolver,
        private readonly SymbolNamer $namer,
        private readonly Types $types,
        private readonly DocGenerator $docGenerator,
    ) {}

    public function registerParameterType(Param $n): void { ... }
    public function createParameterValueRecord(PosResolver $pos, Param $n, ?string $promo): void { ... }
    public function handleAssignment(PosResolver $pos, Assign $n): void { ... }    // was handleLocalVariable
    public function handleReference(PosResolver $pos, Variable $n): void { ... }   // was handleLocalVariableRef
    public function handleForeach(PosResolver $pos, Foreach_ $n): void { ... }     // was handleForeachVariable
    public function ensureForeachVarRegistered(PosResolver $pos, Variable $n, string $name, string $scope): void { ... }

    private function registerForeachVar(...): void { ... }
}
```

**State access:** all 6 tracking properties live on `IndexingContext` (`localCounter`, `localSymbols`, `localCallsSymbols`, `localAssignmentLines`, `localValueIds`, `parameterValueIds`). LocalVariableTracker reads/writes these through `$this->ctx`.

**Critical invariant preserved:** "One Value Per Declaration Rule" — a variable that shadows a parameter must not create a second ValueRecord for the same declaration line.

**DocIndexer changes:**
- 7 methods removed
- Dispatch changes: `$this->handleLocalVariable($pos, $n)` → `$this->localTracker->handleAssignment($pos, $n)`

**New tests:** `scip-php/tests/Indexing/LocalVariableTrackerTest.php`
- Assignment creates both SCIP occurrence and ValueRecord
- Parameter shadowing (local overwrites param in `localSymbols`)
- Foreach key+value variables (separate symbols + ValueRecords for each)
- Parameter value record (with and without constructor promotion)
- One Value Per Declaration Rule enforced

---

### Phase 5 — ScipDefinitionEmitter + ScipReferenceEmitter (ISSUE-F)

**Goal:** Extract 10 SCIP emission methods into two focused emitter classes. After this phase, DocIndexer is ~150 lines.

**New class 1:** `scip-php/src/Indexing/ScipDefinitionEmitter.php`

```php
final class ScipDefinitionEmitter
{
    public function __construct(
        private readonly IndexingContext $ctx,
        private readonly SymbolNamer $namer,
        private readonly Types $types,
        private readonly DocGenerator $docGenerator,
        private readonly DocCommentParser $docCommentParser,
    ) {}

    public function emitDefinition(PosResolver $pos, ...$n, Node $posNode, int $kind): void { ... }
    public function emitDocDefinition(?Doc $doc, string $tagName, PhpDocTagValueNode $node, string $name, string $symbol): void { ... }

    // Private: extractRelationships, extractMethodRelationships, extractReturnTypeRelationships,
    //          extractTypeRelationships, collectTypeSymbols, resolveNameToSymbol, registerSyntheticType
}
```

**New class 2:** `scip-php/src/Indexing/ScipReferenceEmitter.php`

```php
final class ScipReferenceEmitter
{
    public function __construct(
        private readonly IndexingContext $ctx,
        private readonly SymbolNamer $namer,
        private readonly Composer $composer,
    ) {}

    public function emitReference(PosResolver $pos, string $symbol, Node $posNode, int $kind, int $role): void { ... }
}
```

**DocIndexer final state (~150 lines):**

```php
final class DocIndexer
{
    private readonly IndexingContext $ctx;
    private readonly TypeResolver $typeResolver;
    private readonly CallRecordBuilder $callBuilder;
    private readonly ExpressionTracker $exprTracker;
    private readonly LocalVariableTracker $localTracker;
    private readonly ScipDefinitionEmitter $scipEmitter;
    private readonly ScipReferenceEmitter $scipRefEmitter;

    public function __construct(Composer $composer, SymbolNamer $namer, Types $types, string $relativePath = '', bool $experimental = false)
    {
        $this->ctx = new IndexingContext($relativePath, $experimental);
        $this->typeResolver = new TypeResolver($namer, $types);
        // ... wire all services ...
    }

    public function index(PosResolver $pos, Node $n): void { /* dispatch only */ }
    public function getContext(): IndexingContext { return $this->ctx; }
    public function resetLocals(): void { $this->ctx->resetLocals(); }
}
```

**New tests:** `scip-php/tests/Indexing/ScipDefinitionEmitterTest.php` and `ScipReferenceEmitterTest.php`
- Class/method/property definition emission
- Relationship extraction (implements, extends)
- Doc-comment definition emission
- External reference tracking

---

## Edge Cases

| Phase | Case | Expected Handling |
|-------|------|-------------------|
| All | Multiple files processed sequentially | `resetLocals()` on context clears per-file tracking; output collections (`symbols`, `extSymbols`) persist |
| All | Experimental flag off | `ctx->experimental = false`; operators skipped in ExpressionTracker unchanged |
| Phase 0 | Empty file (no AST nodes) | Context created, no state written — valid empty output |
| Phase 1 | `resolveCallKind()` receives unknown node type | Returns `'method'` fallback (unchanged) |
| Phase 1 | `findEnclosingScope()` called on top-level node | Returns `null` (unchanged) |
| Phase 1 | `formatTypeForDoc()` receives null | Returns `'mixed'` (unchanged) |
| Phase 1 | Nullsafe with unresolved type | Returns `'scip-php php builtin . null#'` (unchanged) |
| Phase 2 | Caller scope is null (top-level code) | Returns `null` — no call record (unchanged) |
| Phase 2 | Named arguments (PHP 8) | Parameter position resolved via `array_search` (unchanged) |
| Phase 2 | Static call receiver | `receiverValueId` set to `null` (unchanged) |
| Phase 3 | Already tracked expression | Returns cached ID from `expressionIds` (unchanged) |
| Phase 3 | `$this` variable in tracker | Returns `null` — skipped explicitly (unchanged) |
| Phase 3 | Unknown expression type | Returns `null` via `default => null` (unchanged) |
| Phase 4 | Constructor property promotion | ScipDefinitionEmitter emits SCIP definition; LocalVariableTracker creates ValueRecord — both outputs preserved |
| Phase 4 | Variable shadows parameter | `localSymbols[key]` overwrites; subsequent refs use local symbol (unchanged) |
| Phase 4 | Foreach body visited before `Foreach_` node | `ensureForeachVarRegistered()` walks parent nodes (unchanged) |
| Phase 4 | Foreach with key + value variables | Both tracked with separate symbols and ValueRecords (unchanged) |
| Phase 4 | Variable in nested scope (closure) | `findEnclosingScope()` finds innermost `ClassMethod`/`Function_` (unchanged) |
| Phase 5 | `def()` for ClassLike with `@property`/`@method` | ScipDefinitionEmitter calls `emitDocDefinition()` internally for each parsed tag |
| Phase 5 | `ref()` for external dependency | ScipReferenceEmitter checks `composer->isDependency()` and writes to `ctx->extSymbols` |
| Phase 5 | Invalid UTF-8 symbol | Early return in both emitters (unchanged) |
| Phase 5 | `registerSyntheticType` (currently disabled) | Method moves to ScipDefinitionEmitter still commented out |

---

## Dev Notes

### Method-to-Component Mapping (all 45 methods)

| Method | Current Lines | Target Component |
|--------|--------------|-----------------|
| `__construct` | 165–180 | DocIndexer |
| `index` | 206–431 | DocIndexer (dispatcher) |
| `resetLocals` | 2657–2668 | DocIndexer → `ctx.resetLocals()` |
| `def` | 433–504 | ScipDefinitionEmitter |
| `docDef` | 810–844 | ScipDefinitionEmitter |
| `extractRelationships` | 516–576 | ScipDefinitionEmitter (private) |
| `extractMethodRelationships` | 584–616 | ScipDefinitionEmitter (private) |
| `extractReturnTypeRelationships` | 623–630 | ScipDefinitionEmitter (private) |
| `extractTypeRelationships` | 642–701 | ScipDefinitionEmitter (private) |
| `collectTypeSymbols` | 709–742 | ScipDefinitionEmitter (private) |
| `resolveNameToSymbol` | 749–767 | ScipDefinitionEmitter (private) |
| `registerSyntheticType` | 780–803 | ScipDefinitionEmitter (private) |
| `ref` | 847–873 | ScipReferenceEmitter |
| `registerParameterType` | 915–935 | LocalVariableTracker |
| `createParameterValueRecord` | 948–1010 | LocalVariableTracker |
| `handleLocalVariable` | 1018–1119 | LocalVariableTracker |
| `handleLocalVariableRef` | 1332–1376 | LocalVariableTracker |
| `handleForeachVariable` | 1146–1306 | LocalVariableTracker |
| `ensureForeachVarRegistered` | 1388–1419 | LocalVariableTracker |
| `registerForeachVar` | 1428–1522 | LocalVariableTracker (private) |
| `buildCallRecord` | 1533–1629 | CallRecordBuilder |
| `addCallWithResultValue` | 1639–1656 | CallRecordBuilder |
| `buildAccessOrOperatorCallRecord` | 1679–1757 | CallRecordBuilder |
| `buildValueRecord` | 1772–1810 | ExpressionTracker |
| `getExpressionId` | 1818–1821 | ExpressionTracker |
| `trackExpression` | 1834–1868 | ExpressionTracker (`track()`) |
| `trackVariableExpression` | 1884–1923 | ExpressionTracker (private) |
| `trackPropertyFetchExpression` | 1929–1956 | ExpressionTracker (private) |
| `trackNullsafePropertyFetchExpression` | 1962–1989 | ExpressionTracker (private) |
| `trackStaticPropertyFetchExpression` | 1995–2018 | ExpressionTracker (private) |
| `trackClassConstFetchExpression` | 2024–2052 | ExpressionTracker (private) |
| `trackConstFetchExpression` | 2058–2084 | ExpressionTracker (private) |
| `trackArrayDimFetchExpression` | 2092–2128 | ExpressionTracker (private) |
| `trackCoalesceExpression` | 2136–2182 | ExpressionTracker (private) |
| `trackBinaryOpExpression` | 2195–2200 | ExpressionTracker (private) |
| `trackTernaryExpression` | 2208–2256 | ExpressionTracker (private) |
| `trackMatchExpression` | 2264–2303 | ExpressionTracker (private) |
| `trackLiteralExpression` | 2309–2324 | ExpressionTracker (private) |
| `isCallExpression` | 1124–1138 | ExpressionTracker |
| `isExperimentalKind` | 201–204 | TypeResolver |
| `formatTypeForDoc` | 884–907 | TypeResolver |
| `formatTypeSymbol` | 1311–1327 | TypeResolver |
| `resolveExpressionReturnType` | 2333–2373 | TypeResolver |
| `resolveAccessReturnType` | 2386–2434 | TypeResolver |
| `resolveCallKind` | 2446–2517 | TypeResolver |
| `resolveKindType` | 2530–2542 | TypeResolver |
| `resolveReturnType` | 2555–2606 | TypeResolver |
| `resolveValueType` | 2615–2633 | TypeResolver |
| `findEnclosingScope` | 2640–2652 | TypeResolver |

### Implementation Order and Rationale

Each phase must complete all 4 test gates before the next begins. The order is determined by the dependency graph:

```
Phase 0 (IndexingContext) ← no dependencies, prerequisite for all
Phase 1 (TypeResolver)    ← depends on Phase 0 (context is passed in)
Phase 2 (CallRecordBuilder) ← depends on Phase 0 + 1
Phase 3 (ExpressionTracker) ← depends on Phase 0 + 1 + 2
Phase 4 (LocalVariableTracker) ← depends on Phase 0 + 1 + 3
Phase 5 (ScipEmitters)    ← depends on Phase 0 (can run independently of 1-4)
```

### Baselines (must be captured BEFORE starting)

```bash
# Gate 1 baseline
./tests/snapshot.sh capture
# Note the snapshot filename

# Gate 3 baseline
./bin/scip-php.sh -d ./kloc-reference-project-php -o /tmp/scip-baseline
./bin/scip-php.sh -d ./kloc-reference-project-php -o /tmp/scip-baseline-exp --experimental
cp /tmp/scip-baseline/index.json /tmp/index-before.json
cp /tmp/scip-baseline-exp/index.json /tmp/index-before-exp.json
```

### Cross-Component Impact

All issues are scip-php internal. No changes to kloc-mapper, kloc-cli, sot.json format, or contract test schemas.

| Issue | scip-php/src/ | scip-php/tests/ | contract-tests/ | kloc-mapper | kloc-cli |
|-------|---------------|-----------------|-----------------|-------------|----------|
| A–F | New files + refactor | Add new Indexing/ test files | No change | No change | No change |

---

## Acceptance Criteria

### Global (all phases)

1. GIVEN any PHP file indexed WHEN using refactored DocIndexer THEN output is byte-identical to the pre-refactoring baseline
2. GIVEN 35 snapshot test cases WHEN `./tests/snapshot.sh verify <baseline>` THEN 35/35 PASS, 0 FAIL, 0 diff
3. GIVEN 235 contract tests WHEN `cd kloc-reference-project-php/contract-tests && bin/run.sh test` THEN 235/235 PASS, 0 FAIL
4. GIVEN baseline index.json WHEN re-indexed after refactoring THEN `diff` produces no output (byte-identical)
5. GIVEN DocIndexer exists WHEN PHPStan runs at current strictness level THEN 0 errors
6. GIVEN DocIndexer exists WHEN PHPCS runs THEN 0 violations

### Phase 0 (ISSUE-A): IndexingContext

7. GIVEN phase 0 complete WHEN checking `scip-php/src/Indexing/IndexingContext.php` THEN file exists with 6 output collections, 7 tracking properties, `relativePath` + `experimental` readonly, `resetLocals()` method
8. GIVEN phase 0 complete WHEN checking DocIndexer THEN none of the 14 original state properties declared directly on DocIndexer
9. GIVEN phase 0 complete WHEN `Indexer.php` reads results THEN it calls `getContext()` on DocIndexer to access output collections
10. GIVEN `resetLocals()` called WHEN checking context state THEN `localCounter=0`, empty tracking arrays, empty `calls`/`values` arrays; `symbols`/`extSymbols`/`occurrences` unchanged
11. GIVEN `tests/Indexing/IndexingContextTest.php` WHEN running phpunit THEN all test cases pass

### Phase 1 (ISSUE-B): TypeResolver

12. GIVEN phase 1 complete WHEN checking `scip-php/src/Indexing/TypeResolver.php` THEN file exists with all 9 extracted methods
13. GIVEN phase 1 complete WHEN searching DocIndexer THEN none of `resolveCallKind`, `resolveKindType`, `resolveReturnType`, `resolveValueType`, `resolveExpressionReturnType`, `resolveAccessReturnType`, `formatTypeForDoc`, `formatTypeSymbol`, `findEnclosingScope` defined on DocIndexer
14. GIVEN TypeResolver WHEN checking for nullsafe union logic THEN appears exactly once (in `applyNullsafeUnion` private helper), not 3 times
15. GIVEN `tests/Indexing/TypeResolverTest.php` WHEN running phpunit THEN all test cases pass

### Phase 2 (ISSUE-C): CallRecordBuilder

16. GIVEN phase 2 complete WHEN checking `scip-php/src/Indexing/CallRecordBuilder.php` THEN file exists with `buildCallRecord`, `addCallWithResultValue`, `buildAccessOrOperatorCallRecord`
17. GIVEN phase 2 complete WHEN searching DocIndexer THEN none of the 3 call-building methods defined on DocIndexer
18. GIVEN CallRecordBuilder WHEN building a call record THEN writes to `ctx->calls`, `ctx->values`, `ctx->expressionIds`
19. GIVEN `tests/Indexing/CallRecordBuilderTest.php` WHEN running phpunit THEN all test cases pass

### Phase 3 (ISSUE-D): ExpressionTracker

20. GIVEN phase 3 complete WHEN checking `scip-php/src/Indexing/ExpressionTracker.php` THEN file exists with public methods `track()`, `getExpressionId()`, `isCallExpression()` and 13 private `track*()` methods
21. GIVEN phase 3 complete WHEN searching DocIndexer THEN `trackExpression` not defined on DocIndexer
22. GIVEN an already-tracked expression WHEN `track()` called again THEN returns same cached ID without creating duplicate record
23. GIVEN `tests/Indexing/ExpressionTrackerTest.php` WHEN running phpunit THEN all test cases pass

### Phase 4 (ISSUE-E): LocalVariableTracker

24. GIVEN phase 4 complete WHEN checking `scip-php/src/Indexing/LocalVariableTracker.php` THEN file exists with 6 public methods
25. GIVEN phase 4 complete WHEN searching DocIndexer THEN `handleLocalVariable`, `handleLocalVariableRef`, `handleForeachVariable`, `registerParameterType`, `createParameterValueRecord`, `ensureForeachVarRegistered` not defined on DocIndexer
26. GIVEN constructor-promoted property WHEN indexing THEN both SCIP definition AND ValueRecord created (dual output preserved)
27. GIVEN variable shadowing parameter in same scope WHEN indexing reference THEN uses local variable symbol (not parameter symbol)
28. GIVEN `tests/Indexing/LocalVariableTrackerTest.php` WHEN running phpunit THEN all test cases pass
29. GIVEN existing `tests/Indexer/DocIndexerTest.php` (6 tests) WHEN running phpunit THEN all 6 still pass

### Phase 5 (ISSUE-F): ScipEmitters

30. GIVEN phase 5 complete WHEN checking `scip-php/src/Indexing/ScipDefinitionEmitter.php` THEN file exists with `emitDefinition()` and `emitDocDefinition()` public methods
31. GIVEN phase 5 complete WHEN checking `scip-php/src/Indexing/ScipReferenceEmitter.php` THEN file exists with `emitReference()` public method
32. GIVEN phase 5 complete WHEN measuring DocIndexer line count THEN approximately 150 lines (dispatch + wiring only)
33. GIVEN relationship extraction methods WHEN searching ScipDefinitionEmitter THEN `extractRelationships`, `extractMethodRelationships`, `extractReturnTypeRelationships`, `extractTypeRelationships`, `collectTypeSymbols`, `resolveNameToSymbol`, `registerSyntheticType` all private
34. GIVEN `tests/Indexing/ScipDefinitionEmitterTest.php` and `tests/Indexing/ScipReferenceEmitterTest.php` WHEN running phpunit THEN all test cases pass

### New Unit Test Coverage (Gate 4 requirement)

| Phase | New Test File | Min Test Cases |
|-------|--------------|----------------|
| Phase 0 | `tests/Indexing/IndexingContextTest.php` | reset clears state, relativePath/experimental immutable, output arrays appendable |
| Phase 1 | `tests/Indexing/TypeResolverTest.php` | resolveCallKind for each node type (13 cases), resolveKindType (4 cases), formatTypeSymbol null/single/union, findEnclosingScope walk |
| Phase 2 | `tests/Indexing/CallRecordBuilderTest.php` | method call, constructor, property access, operator call, argument binding, receiver tracking |
| Phase 3 | `tests/Indexing/ExpressionTrackerTest.php` | variable lookup, literal tracking, property fetch chaining, cached expression, experimental gating |
| Phase 4 | `tests/Indexing/LocalVariableTrackerTest.php` | assignment creates SCIP + value, shadowing, foreach key+value, parameter value record, One Value Per Declaration |
| Phase 5 | `tests/Indexing/ScipDefinitionEmitterTest.php`, `tests/Indexing/ScipReferenceEmitterTest.php` | class/method/property def, relationships, doc-comment def, external ref tracking |

### Existing Tests That Must Continue to Pass

| Test File | Cases |
|-----------|-------|
| `tests/Indexer/DocIndexerTest.php` | 6 tests (parameter refs, foreach, local shadowing) |
| `tests/Indexer/IndexerTest.php` | Smoke tests |
| `tests/Indexer/CallsTrackingTest.php` | Call record validation |
| `tests/Indexer/ExpressionChainsTest.php` | Chaining validation |
| `tests/Indexer/SyntheticTypesTest.php` | Synthetic type tests |
| `tests/Calls/CallRecordTest.php` | Record serialization |
| `tests/Calls/CallsSchemaValidationTest.php` | Schema compliance |
| `tests/SymbolNamerTest.php` | Symbol naming |
| `tests/Types/TypesTest.php` | Type resolution |
| `tests/Parser/ParserTest.php` | Parser tests |
| `tests/Parser/PosResolverTest.php` | Position tests |
| `tests/File/ReaderTest.php` | File reader |
| `tests/Composer/ComposerTest.php` | Composer tests |
| `tests/Types/Internal/NamedTypeTest.php` | Type internals |
| `tests/Types/Internal/CompositeTypeTest.php` | Type internals |
