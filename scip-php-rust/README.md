# scip-php-rust

A high-performance PHP SCIP indexer written in Rust. Drop-in replacement for the PHP `scip-php` implementation, producing identical `index.json` and `calls.json` output.

## Features

- **Fast**: Parallel indexing via rayon — 10-50x faster than the PHP implementation
- **Single binary**: No PHP runtime, Composer, or extensions required
- **Full PHP support**: PHP 8.0-8.3 syntax including enums, readonly classes, named arguments, match expressions, nullsafe operator, intersection/union types, DNF types, typed class constants (PHP 8.4 property hooks not yet supported — requires tree-sitter-php 0.24+)
- **SCIP v4.0 output**: Compatible with the kloc pipeline (`index.json` + `calls.json`)
- **Robust**: Graceful handling of syntax errors, BOM markers, invalid UTF-8, empty files

## Usage

```bash
# Index a PHP project (output to .kloc/ by default)
scip-php-rust --project-root /path/to/php-project

# Custom output directory
scip-php-rust --project-root /path/to/php-project --output-dir /path/to/output

# Verbose mode with thread control
scip-php-rust --project-root /path/to/php-project --verbose --threads 8

# Quiet mode (errors only)
scip-php-rust --project-root /path/to/php-project --quiet
```

### CLI Options

| Option | Description | Default |
|--------|-------------|---------|
| `--project-root <PATH>` | Root directory of the PHP project | (required) |
| `--output-dir <PATH>` | Output directory for index.json and calls.json | `.kloc/` in project root |
| `--threads <N>` | Number of indexing threads | Number of logical CPUs |
| `--verbose`, `-v` | Enable verbose logging with timing | off |
| `--quiet`, `-q` | Suppress all output except errors | off |

## Building

```bash
cargo build --release
# Binary at target/release/scip-php-rust
```

## Architecture

The indexer runs in 4 phases:

```
Phase 0: Discovery        — walkdir finds .php files, separates project/vendor
Phase 1: Type Collection   — parallel CST parse → collect classes, methods, properties into DashMap TypeDatabase
Phase 2: Indexing          — parallel per-file indexing with shared read-only TypeDatabase, Composer, SymbolNamer
Phase 3: Output            — deterministic sort → write index.json + calls.json
```

### Module Structure

```
src/
├── main.rs              — CLI entry point
├── lib.rs               — Library root
├── pipeline.rs          — 4-phase parallel pipeline orchestration
├── discovery.rs         — PHP file discovery (walkdir, .gitignore aware)
├── parser/              — tree-sitter-php CST wrapper
│   ├── mod.rs           — PhpParser (parse files/strings)
│   ├── ast.rs           — AST node classification (PhpNode enum)
│   ├── cst.rs           — CST helper functions
│   └── position.rs      — Line/column position types
├── names/               — Name resolution
│   ├── mod.rs           — FileNameResolver (per-file namespace + imports)
│   ├── resolver.rs      — FQN resolution (class, function, constant)
│   └── traversal.rs     — CST traversal for use statements
├── composer/            — Composer integration
│   ├── mod.rs           — Composer loader
│   ├── psr4.rs          — PSR-4 autoload resolution
│   ├── classmap.rs      — Classmap autoload
│   ├── installed.rs     — composer.lock / installed.json parsing
│   ├── stubs.rs         — PHPStorm stubs (built-in PHP classes)
│   ├── discovery.rs     — Vendor package discovery
│   ├── types.rs         — Composer data types
│   └── php_array_parser.rs — PHP array literal parser (for classmap files)
├── types/               — Type system
│   ├── mod.rs           — TypeDatabase (DashMap-based, concurrent)
│   ├── collector.rs     — Phase 1 type collection from CST
│   ├── resolver.rs      — Type resolution (method returns, property types)
│   ├── upper_chain.rs   — Transitive inheritance chain builder
│   ├── phpdoc.rs        — PHPDoc @param/@return/@var parser
│   └── debug_dump.rs    — Debug output for TypeDatabase
├── symbol/              — SCIP symbol naming
│   ├── mod.rs           — Symbol types
│   ├── namer.rs         — SymbolNamer (FQN → SCIP symbol string)
│   └── scope.rs         — Scope chain construction
├── indexing/            — Per-file indexing (Phase 2)
│   ├── mod.rs           — Module root
│   ├── context.rs       — IndexingContext (per-file state) + FileResult
│   ├── definitions.rs   — Definition emission (classes, methods, properties, etc.)
│   ├── references.rs    — Reference emission (class refs, method calls, property access)
│   ├── expression_tracker.rs — Call tracking + value extraction
│   ├── locals.rs        — Local variable tracking
│   └── calls.rs         — CallRecord / ValueRecord types
└── output/              — Output serialization
    ├── mod.rs           — Module root
    ├── scip.rs          — SCIP JSON format
    └── calls.rs         — Calls JSON format
```

## Output Format

### index.json

SCIP v4.0 compatible index with metadata, documents (one per PHP file), and symbol information:

```json
{
  "metadata": {
    "version": "4.0",
    "tool_info": { "name": "scip-php", "version": "0.1.0" },
    "project_root": "file:///path/to/project/",
    "text_document_encoding": "UTF8"
  },
  "documents": [
    {
      "language": "PHP",
      "relative_path": "src/MyClass.php",
      "occurrences": [...],
      "symbols": [...]
    }
  ]
}
```

### calls.json

Call graph and value flow records:

```json
{
  "calls": [
    { "caller": "App\\Service#process().", "callee": "App\\Repo#find().", "line": 42 }
  ],
  "values": [
    { "source": "App\\Service#process().", "target": "App\\Repo#find().($id)", "line": 42 }
  ]
}
```

## Testing

```bash
# Run all tests (499 unit + 3 integration + 2 doc tests)
cargo test

# Run only unit tests
cargo test --lib

# Run integration tests
cargo test --test integration_test

# Run a specific test
cargo test test_bom_handling
```

## Stats

- ~19,900 lines of Rust (including ~8,000 lines of tests)
- 37 source files
- 499 unit tests, 3 integration tests
- Implements 16 task specifications from `docs/specs/kloc-php-indexer/`
