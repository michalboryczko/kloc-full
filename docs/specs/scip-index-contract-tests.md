# Feature Request: SCIP Index Coverage in Contract Tests

## Goal

Extend the contract test framework to validate `index.scip` output alongside `calls.json`, enabling complete coverage of scip-php indexer output for usage-flow-tracking requirements.

## Why

The current contract tests only validate `calls.json` output, which tracks call sites, receiver values, and argument bindings. However, the usage-flow-tracking feature (docs/specs/usage-flow-tracking.md) requires data that only exists in the SCIP index:

| Data Required | calls.json | index.scip | Usage Flow Need |
|--------------|------------|------------|-----------------|
| Type hint references | No | Yes | "via: $gateway property (type_hint)" |
| extends/implements | No | Yes | Inheritance chains |
| Symbol definitions | Strings only | Full metadata | Symbol kind, location |
| All reference occurrences | Call sites only | Every usage | Non-call references |

Without SCIP validation, we cannot verify:
- Property type hints are captured (`private Repository $repo`)
- Parameter type hints are captured (`function save(Order $order)`)
- Return type hints are captured (`function find(): User`)
- Class inheritance is tracked (`class Child extends Parent`)
- Interface implementation is tracked (`class Service implements Contract`)

## Usage examples

### Example 1: Validate Type Hint Reference

**Before** (no SCIP coverage):
```php
// Cannot test that this type hint creates a reference in the index
class OrderService {
    private PaymentGateway $gateway; // Type hint - NOT a call
}
```

**After** (with SCIP coverage):
```php
#[ContractTest(
    name: 'PaymentGateway type hint creates reference',
    description: 'Verifies type hint on property creates occurrence in SCIP index',
    category: 'scip',
)]
public function testPropertyTypeHintCreatesReference(): void
{
    // Query SCIP index for occurrences of PaymentGateway
    $occurrences = $this->scip()
        ->symbol('App\Payment\PaymentGateway')
        ->occurrences()
        ->inFile('src/Service/OrderService.php')
        ->all();

    // Should have at least one reference (the type hint)
    $this->assertNotEmpty($occurrences);
    $this->assertContains('Reference', $occurrences[0]->roles);
}
```

### Example 2: Validate Inheritance Relationship

**After** (with SCIP coverage):
```php
#[ContractTest(
    name: 'AdminController extends BaseController',
    description: 'Verifies extends relationship is captured in SCIP index',
    category: 'scip',
)]
public function testExtendsRelationshipExists(): void
{
    $symbol = $this->scip()
        ->symbol('App\Controller\AdminController')
        ->definition();

    // Symbol should have relationship to parent
    $relationships = $symbol->relationships();
    $extends = array_filter($relationships, fn($r) => $r->isExtends());

    $this->assertCount(1, $extends);
    $this->assertStringContains('BaseController', $extends[0]->targetSymbol);
}
```

### Example 3: Combined SCIP + Calls Validation

**After** (cross-referencing both outputs):
```php
#[ContractTest(
    name: 'Method call has matching SCIP occurrence',
    description: 'Verifies calls.json entries have corresponding SCIP occurrences',
    category: 'integrity',
)]
public function testCallHasMatchingOccurrence(): void
{
    // Get a method call from calls.json
    $call = $this->calls()->kind('method')->first();

    // Verify SCIP has an occurrence at the same location
    $occurrence = $this->scip()
        ->occurrenceAt($call->location->file, $call->location->line)
        ->first();

    $this->assertNotNull($occurrence);
    $this->assertEquals($call->callee, $occurrence->symbol);
}
```

## Detailed behavior

### 1. SCIP Data Loading

Add a new `ScipData` class parallel to `CallsData`:

```php
class ScipData
{
    private Index $index; // Protobuf parsed SCIP index

    public static function load(string $path): self;
    public function symbols(): array;
    public function documents(): array;
    public function occurrences(string $symbol): array;
    public function relationships(string $symbol): array;
}
```

The loader should:
- Parse the protobuf `index.scip` file
- Build lookup indices for symbols, documents, occurrences
- Provide query methods for test assertions

### 2. Query API for SCIP

Extend `CallsContractTestCase` with SCIP query methods:

```php
abstract class CallsContractTestCase extends TestCase
{
    protected static ?CallsData $calls = null;
    protected static ?ScipData $scip = null;  // NEW

    // Existing: values(), calls(), inMethod()

    // NEW: SCIP queries
    protected function scip(): ScipQuery;
    protected function symbols(): SymbolQuery;
    protected function occurrences(): OccurrenceQuery;
}
```

### 3. SCIP-Specific Assertions

Add new assertion classes:

| Assertion | Purpose |
|-----------|---------|
| `assertSymbolExists()` | Verify symbol is defined in index |
| `assertOccurrenceAt()` | Verify reference at specific location |
| `assertRelationship()` | Verify extends/implements/use relationship |
| `assertSymbolKind()` | Verify symbol is class/method/property |
| `assertTypeAnnotation()` | Verify type info on symbol |

### 4. Test Categories

Add new test category `scip` for SCIP-specific validation:

```
Test suites: smoke, integrity, reference, chain, argument, callkind, operator, scip
```

### 5. Required SCIP Validations

Based on usage-flow-tracking spec, validate:

| Validation | SCIP Feature | Test Scenario |
|------------|--------------|---------------|
| Type hint occurrences | Occurrence with role=Reference | Property/param/return type creates reference |
| Symbol definitions | Document.symbols | Every class/method/property has definition |
| Inheritance | Relationship with kind=extends | Child class has extends relationship |
| Interface impl | Relationship with kind=implements | Class has implements relationship |
| Symbol metadata | SymbolInformation | Symbols have kind, documentation |

## Edge cases

| Case | Expected behavior |
|------|-------------------|
| SCIP file missing | Clear error message: "index.scip not found, run bin/run.sh first" |
| SCIP parse failure | Clear error with protobuf details |
| Symbol not found | Return empty result, not exception (for query methods) |
| Multiple occurrences same location | Return all occurrences (array) |
| Cross-file relationships | Relationships work across document boundaries |
| Synthetic symbols (unions) | Handle `scip-php union .` symbols gracefully |

## Dev notes

### Test Directory Structure

Reorganize tests into three main subdirectories:

```
tests/
├── Calls/           # calls.json only validation (existing tests moved here)
│   ├── Reference/
│   ├── Chain/
│   ├── Argument/
│   ├── Integrity/
│   ├── CallKind/
│   └── Operator/
├── Scip/            # index.scip only validation (NEW)
│   ├── TypeHint/
│   ├── Inheritance/
│   ├── Symbol/
│   └── Occurrence/
└── Combined/        # Cross-validation between SCIP and calls.json (NEW)
    ├── Consistency/
    └── UsageFlow/
```

### SCIP JSON Generation

Use the `scip` CLI tool to convert `index.scip` to JSON format inside the Docker container:

```bash
# In bin/run.sh, after generating index.scip:
scip print --json /output/index.scip > /output/index.scip.json
```

This requires installing the `scip` package in the contract-tests Docker image.

### File Changes

- **`contract-tests/Dockerfile`**: Add `scip` CLI installation
- **`contract-tests/bin/run.sh`**: Add SCIP JSON generation step after scip-php indexing
- **`contract-tests/src/ScipData.php`**: New class to load `index.scip.json`
- **`contract-tests/src/Query/ScipQuery.php`**: Query builder for SCIP data
- **`contract-tests/src/Query/SymbolQuery.php`**: Query symbols by name, kind
- **`contract-tests/src/Query/OccurrenceQuery.php`**: Query occurrences by location, role
- **`contract-tests/src/CallsContractTestCase.php:26`**: Add `protected static ?ScipData $scip = null;`
- **`contract-tests/src/CallsContractTestCase.php:30`**: Load SCIP JSON in `setUpBeforeClass()`
- **`contract-tests/config.php`**: Add `scip_json` path (`index.scip.json`)
- **`contract-tests/bootstrap.php`**: Add `SCIP_JSON_PATH` constant

### SCIP CLI Installation in Docker

```dockerfile
# In contract-tests/Dockerfile
# Install scip CLI (Go binary)
RUN curl -L https://github.com/sourcegraph/scip/releases/download/v0.4.0/scip-linux-amd64.tar.gz | tar xz \
    && mv scip /usr/local/bin/
```

### Run Script Changes

```bash
# bin/run.sh additions

# After scip-php generates index.scip:
echo "Converting SCIP to JSON..."
scip print --json "$OUTPUT_DIR/index.scip" > "$OUTPUT_DIR/index.scip.json"
```

## Open questions

- **scip CLI version**: Which version of `scip` CLI should we pin to? Need to verify JSON output format.
- **JSON schema**: Document the expected structure of `scip print --json` output for test assertions.
- **Migration**: Should existing tests in `tests/` be moved to `tests/Calls/` immediately, or in a separate PR?
