# QA Notes: scip-php DocIndexer Decoupling Refactor

## Feature Summary

Pure refactoring of `scip-php/src/DocIndexer.php` (2,669 lines, 45 methods) into 7 focused single-responsibility services. **No behavioral changes** — all existing output must remain byte-identical.

## Baselines Captured (2026-02-17)

| Baseline | File/Location | Checksum/Count |
|----------|---------------|----------------|
| Snapshot | `tests/snapshot-1702262229.json` | 42/42 cases |
| Index (standard) | `/tmp/index-before.json` | MD5: a11ad970363e0dd124263d3fa25c5929 |
| Index (experimental) | `/tmp/index-before-exp.json` | MD5: 2256de564b7ac1234de9978ddcaef4e3 |
| Contract tests | 289 total, 241 run, 48 skipped (experimental/internal) | 0 failures |
| Unit tests | 92 tests, 8550 assertions | 1 skipped (root chmod), 0 failures |
| PHPStan | 66 pre-existing errors | Baseline — no new errors allowed |
| PHPCS | ~48 pre-existing violations | Baseline — no new violations allowed |

## 4 Test Gates (per phase)

### Gate 1: Snapshot Tests
- **Command:** `./tests/snapshot.sh verify tests/snapshot-1702262229.json`
- **Requirement:** 42/42 PASS, 0 FAIL, 0 diff
- **What it validates:** Full pipeline output (scip-php -> kloc-mapper -> kloc-cli) identical

### Gate 2: Contract Tests
- **Command:** `cd kloc-reference-project-php/contract-tests && bin/run.sh test`
- **Requirement:** 241 pass, 48 skipped, 0 failures (289 total)
- **What it validates:** scip-php JSON output structure, calls, values, argument binding, chaining

### Gate 3: Index Binary Comparison
- **Standard command:**
  ```bash
  ./scip-php/bin/scip-php.sh -d ./kloc-reference-project-php -o /tmp/scip-after
  diff /tmp/index-before.json /tmp/scip-after/index.json
  ```
- **Experimental command:**
  ```bash
  ./scip-php/bin/scip-php.sh -d ./kloc-reference-project-php -o /tmp/scip-after-exp --experimental
  diff /tmp/index-before-exp.json /tmp/scip-after-exp/index.json
  ```
- **Requirement:** Both diffs produce no output (byte-identical)
- **IMPORTANT:** Must rebuild scip-php image first: `cd scip-php && ./build/build.sh && cd ..`

### Gate 4: Unit Tests + Static Analysis
- **Unit tests:** `docker run --rm -v $(pwd):/app scip-php-dev vendor/bin/phpunit --no-coverage`
  - Requirement: 92 tests pass, 0 failures (1 skipped is OK — root chmod issue)
- **PHPStan:** `docker run --rm -v $(pwd):/app scip-php-dev vendor/bin/phpstan --memory-limit=2G`
  - Requirement: No NEW errors beyond the 66 pre-existing ones
- **PHPCS:** `docker run --rm -v $(pwd):/app scip-php-dev vendor/bin/phpcs`
  - Requirement: No NEW violations beyond the ~48 pre-existing ones

## Test Scenarios

### Regression (Primary Concern)
- GIVEN the refactored code WHEN running full pipeline THEN output is byte-identical to baseline
- GIVEN any extracted service WHEN called from DocIndexer THEN behavior is unchanged
- GIVEN the reference project WHEN indexed in standard mode THEN index.json matches baseline MD5
- GIVEN the reference project WHEN indexed in experimental mode THEN index.json matches baseline MD5

### Phase-Specific Risks

| Phase | Issue | Risk | Key Concern |
|-------|-------|------|-------------|
| 0 | ISSUE-A: IndexingContext | LOW | State bag must preserve exact mutation order |
| 1 | ISSUE-B: TypeResolver | LOW | Stateless methods — safe extraction |
| 2 | ISSUE-C: CallRecordBuilder | MEDIUM | Writes to ctx.calls/values/expressionIds — mutation order matters |
| 3 | ISSUE-D: ExpressionTracker | MEDIUM | 14 track methods with complex state dependencies |
| 4 | ISSUE-E: LocalVariableTracker | HIGH | Dual output (SCIP + values), 6 state properties, most coupled |
| 5 | ISSUE-F: ScipEmitters | LOW | def/ref methods relatively independent |

### Edge Cases to Watch
- Shared mutable state (`expressionIds`, `localSymbols`, `localCallsSymbols`) accessed by multiple extracted services
- `resetLocals()` must clear ALL state correctly across services
- Constructor parameter order and dependency injection wiring
- `isExperimentalKind()` gating must work identically after extraction

## Automated Test Expectations
- No NEW contract tests needed (pure refactoring)
- NEW unit tests required per phase (see README.md Phase table)
- All 4 gates must pass after EVERY phase

## Manual Testing Steps
1. After each phase: Run all 4 gates in sequence
2. For Gate 3: Always rebuild Docker image before comparing
3. Compare PHPStan/PHPCS error counts against baseline (no increase allowed)

## Pre-Existing Issues (Not Blockers)
- PHPStan: 66 errors (mostly type strictness, unused methods, empty() usage)
- PHPCS: ~48 violations (mostly style — empty(), line length, naming)
- These are pre-existing and NOT caused by this feature
- Developers must not introduce NEW violations but are not required to fix existing ones
