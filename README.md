# kloc

Meta-repository for the **KLOC (Knowledge of Code)** project -- a toolkit for extracting and querying rich code context from PHP codebases, designed for AI coding agents.

## Pipeline

```
PHP Source -> [scip-php | kloc-indexer-php] -> index.json -> kloc-mapper -> sot.json -> [kloc-cli | kloc-intelligence] -> output
```

## Components

| Directory | Language | Description |
|-----------|----------|-------------|
| `kloc-cli/` | Python | CLI for querying Source-of-Truth JSON (deps, usages, context, inherit) |
| `kloc-mapper/` | Python | Converts SCIP indexes to Source-of-Truth (SoT) JSON |
| `scip-php/` | PHP | SCIP indexer for PHP projects (Docker-based) |
| `kloc-indexer-php/` | Rust | Drop-in replacement for scip-php, ~200x faster |
| `kloc-intelligence/` | Python | Neo4j-backed graph DB query engine with MCP server |
| `kloc-reference-project-php/` | PHP | Symfony 7.2 reference project for testing |
| `kloc-contracts/` | JSON/Python | Pipeline validation schemas (sot-json, scip-php-output, kloc-cli-context) |

Sub-repos (`kloc-cli`, `kloc-mapper`, `scip-php`, `kloc-indexer-php`, `kloc-reference-project-php`) are managed via `repos.yml` and cloned by `setup.sh`. `kloc-intelligence` and `kloc-contracts` live directly in this repository.

## Prerequisites

- Python 3.11+ with [uv](https://github.com/astral-sh/uv)
- Docker (for scip-php and kloc-intelligence/Neo4j)
- Rust 1.75+ (for kloc-indexer-php, optional)

## Quick Start

```bash
# Clone and set up all sub-repos
git clone https://github.com/michalboryczko/kloc-full.git && cd kloc-full
./setup.sh
./build.sh
```

### Dev Pipeline (reference project)

```bash
# Index the reference project and query it
./kloc-dev.sh context "App\Service\OrderService" --depth 3

# Same, using the Rust indexer
./kloc-dev.sh context "App\Service\OrderService" --depth 3 --rust-indexer
```

### Production Pipeline

```bash
# Index a PHP project
./kloc.sh index --project myapp -d /path/to/php-project

# Index with the Rust indexer
./kloc.sh index --project myapp -d /path/to/php-project --rust-indexer

# Query it
./kloc.sh cli --project myapp context "App\Service\OrderService" --depth 2
```

### kloc-intelligence (Neo4j backend)

```bash
cd kloc-intelligence
docker compose -f docker/docker-compose.yml up -d
uv run kloc-intelligence import artifacts/sot.json
uv run kloc-intelligence context "App\Service\OrderService" --depth 3
```

## Scripts

| Script | Purpose |
|--------|---------|
| `kloc.sh` | Production pipeline: index a PHP project and query it |
| `kloc-dev.sh` | Dev pipeline: indexes the reference project and runs kloc-cli |
| `setup.sh` | Clone or update all sub-repos defined in `repos.yml` |
| `build.sh` | Build binaries from all sub-repos |
| `repos.yml` | Sub-repository URLs and build configuration |

## Directory Layout

```
kloc/
  kloc.sh                  # Production pipeline
  kloc-dev.sh              # Dev pipeline
  setup.sh / build.sh      # Repo management
  repos.yml                # Sub-repo definitions
  kloc-cli/                # Query CLI (sub-repo)
  kloc-mapper/             # SCIP-to-SoT mapper (sub-repo)
  scip-php/                # PHP SCIP indexer (sub-repo)
  kloc-indexer-php/         # Rust SCIP indexer (sub-repo)
  kloc-intelligence/        # Neo4j query engine + MCP server
  kloc-reference-project-php/  # Test fixture (sub-repo)
  kloc-contracts/           # JSON schemas for pipeline validation
  artifacts/                # Pipeline output (index.json, sot.json)
  data/                     # Test datasets
```

## Indexer Comparison

| | scip-php | kloc-indexer-php |
|---|----------|-----------------|
| Language | PHP (Docker) | Rust (native binary) |
| Speed (41 files) | ~2s | ~0.01s |
| Output | index.json + calls.json | index.json + calls.json |
| Status | Production | Drop-in compatible, all parity metrics match |

## License

MIT
