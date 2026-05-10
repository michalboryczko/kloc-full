# kloc

Meta-repository for the **KLOC (Knowledge of Code)** project -- a toolkit for extracting and querying rich code context from PHP codebases, designed for AI coding agents.

## Pipeline

```
PHP Source -> kloc-indexer-php -> index.json -> kloc-mapper -> sot.json -> [kloc-cli | kloc-intelligence] -> output
                                                                 |
                          kloc-symfony -> symfony-kloc.json  (Symfony apps; consumed by kloc-intelligence)
```

The core pipeline is **kloc-indexer-php -> kloc-mapper -> kloc-cli**. `kloc-symfony` (Symfony framework extractor) and `kloc-intelligence` (graph/AI service) are optional add-ons.

## Components

| Directory | Language | Description |
|-----------|----------|-------------|
| `kloc-cli/` | Python | CLI for querying Source-of-Truth JSON (deps, usages, context, inherit) |
| `kloc-mapper/` | Python | Converts SCIP indexes to Source-of-Truth (SoT) JSON |
| `kloc-indexer-php/` | Rust | SCIP indexer for PHP projects (native binary) |
| `kloc-symfony/` | PHP | Symfony framework extractor — routes, DI, message handlers, event listeners, console commands, flows → `symfony-kloc.json` (Docker-based, optional) |
| `kloc-intelligence/` | Python | Neo4j/Qdrant-backed graph query engine with flow analysis, semantic search, and MCP server (optional) |
| `kloc-reference-project-php/` | PHP | Symfony 7.2 reference project for testing |
| `kloc-contracts/` | JSON/Python | Pipeline validation schemas (sot-json, scip-php-output, kloc-cli-context) |

Sub-repos (`kloc-cli`, `kloc-mapper`, `kloc-indexer-php`, `kloc-symfony`, `kloc-intelligence`, `kloc-reference-project-php`) are managed via `repos.yml` and cloned by `setup.sh`. `kloc-contracts` lives directly in this repository.

> The legacy `scip-php` PHP indexer is deprecated and replaced by `kloc-indexer-php`. It is no longer listed in `repos.yml` (so `setup.sh` won't fetch it) or built by `build.sh`. `kloc.sh` / `kloc-dev.sh` still accept `--scip-php` as an escape hatch if you clone `scip-php/` manually, but it is otherwise unsupported and undocumented.

## Prerequisites

- Python 3.11+ with [uv](https://github.com/astral-sh/uv)
- Docker (for kloc-symfony, and kloc-intelligence's Neo4j + Qdrant)
- Rust 1.75+ (for building kloc-indexer-php)
- An OpenAI-compatible LLM + embedding API key (only for kloc-intelligence's `enrich` / `search` features)

## Quick Start

```bash
# Clone and set up all sub-repos
git clone https://github.com/michalboryczko/kloc-full.git && cd kloc-full
./setup.sh
./build.sh
```

### Dev Pipeline (reference project)

```bash
# Index the reference project (kloc-indexer-php) and query it
./kloc-dev.sh context "App\Service\OrderService" --depth 3
```

### Production Pipeline

```bash
# Index a PHP project (uses kloc-indexer-php)
./kloc.sh index --project myapp -d /path/to/php-project

# Query it
./kloc.sh cli --project myapp context "App\Service\OrderService" --depth 2
```

## Optional Components

Both add-ons consume artifacts produced by the core pipeline and are independent of each other — set up only what you need.

### kloc-symfony — Symfony framework extractor

For Symfony 6.x / 7.x apps. Boots the target's Symfony kernel and reads its compiled DI container to emit `symfony-kloc.json`: every framework entry point (HTTP routes, message handlers, event subscribers/listeners, console commands) plus a `triggers[]` collection of cross-flow links (which message/event class is dispatched, by which flow, at which call site, and which flow handles it). It can optionally take a `sot.json` to attach `node_id` to every entry and walk the call graph for trigger detection. Ships as a Docker image (`php:8.4-cli`) — no local PHP install needed. `kloc-cli` does not read its output; `kloc-intelligence` does (`import-flows`).

```bash
cd kloc-symfony && docker build -t kloc-symfony . && cd ..

# extract routes, handlers, listeners, commands, flows -> symfony-kloc.json
# args: <project-dir> [output-dir=<project>/.kloc] [sot-path]
./kloc-symfony/bin/kloc-symfony.sh /path/to/symfony-app ./artifacts ./artifacts/sot.json
```

See [docs/usage/kloc-symfony.md](docs/usage/kloc-symfony.md) for details.

### kloc-intelligence — graph & AI code-intelligence service

A stateful counterpart to `kloc-cli`. Imports `sot.json` (and optionally `symfony-kloc.json`) into Neo4j once, then answers the same query commands (`resolve`, `usages`, `deps`, `context`, `owners`, `inherit`, `overrides` — identical JSON contract to `kloc-cli`) via Cypher, plus framework `flows` analysis. With the AI extras it adds LLM-generated explanations (`enrich`) and Qdrant-backed semantic `search`. Exposes an MCP server (stdio, multi-project). Best for large codebases, persistent analysis, CI, and AI-agent workflows with many queries.

```bash
cd kloc-intelligence
uv sync --all-extras                       # omit --all-extras to skip the AI deps (Qdrant/Haystack)
cp .env.example .env                       # set LLM_API_KEY + EMBEDDING_API_KEY for enrich/search
docker compose up -d                       # starts Neo4j + Qdrant
uv run kloc-intelligence schema ensure
uv run kloc-intelligence import ../artifacts/sot.json
uv run kloc-intelligence import-flows ../artifacts/symfony-kloc.json   # optional, if you ran kloc-symfony
uv run kloc-intelligence enrich            # optional — needs API keys in .env
uv run kloc-intelligence context "App\Service\OrderService" --depth 3
uv run kloc-intelligence search "create a new customer order"
```

See [docs/usage/kloc-intelligence.md](docs/usage/kloc-intelligence.md) for the full command reference and MCP setup.

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
  kloc-indexer-php/         # Rust SCIP indexer (sub-repo)
  kloc-symfony/             # Symfony framework extractor (sub-repo, optional)
  kloc-intelligence/        # Neo4j/Qdrant query engine + MCP server (sub-repo, optional)
  kloc-reference-project-php/  # Test fixture (sub-repo)
  kloc-contracts/           # JSON schemas for pipeline validation
  artifacts/                # Pipeline output (index.json, sot.json)
  data/                     # Test datasets
```

## Documentation

- [docs/usage/setup.md](docs/usage/setup.md) — full setup guide (all components)
- [docs/usage/cli.md](docs/usage/cli.md) — `kloc-cli` command reference
- [docs/usage/mcp.md](docs/usage/mcp.md) — MCP server
- [docs/usage/kloc-symfony.md](docs/usage/kloc-symfony.md) — Symfony framework extractor
- [docs/usage/kloc-intelligence.md](docs/usage/kloc-intelligence.md) — graph/AI code-intelligence service

## License

MIT
