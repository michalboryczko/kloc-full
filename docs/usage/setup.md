# kloc Setup Guide

This guide walks you through setting up kloc and using it to analyze a PHP codebase. By the end, you will be able to query your project's class hierarchy, method calls, and dependencies from the command line.

The core pipeline is **kloc-indexer-php → kloc-mapper → kloc-cli**. Two optional components extend it:

- **kloc-symfony** — extracts Symfony framework architecture (routes, DI, message handlers, event listeners, console commands, flows) as `symfony-kloc.json`. Only relevant if your project is a Symfony app. See [kloc-symfony.md](kloc-symfony.md).
- **kloc-intelligence** — a graph-native code-intelligence service backed by Neo4j (plus Qdrant for AI features). Imports `sot.json` (and optionally `symfony-kloc.json`) once, then answers the same queries as kloc-cli plus multi-hop Cypher traversals, flow analysis, and semantic search. See [kloc-intelligence.md](kloc-intelligence.md).

## Prerequisites

- **Python 3.12+** -- required for kloc-cli and kloc-mapper (kloc-intelligence needs 3.11+)
- **uv** -- Python package manager ([install guide](https://docs.astral.sh/uv/getting-started/installation/))
- **Rust toolchain** -- required to build `kloc-indexer-php` (the SCIP indexer); `cargo build --release`, or build via `./build.sh`
- **Docker** -- required for kloc-symfony and the kloc-intelligence backing services (Neo4j + Qdrant)
- **A PHP project** to analyze (with Composer dependencies installed)
- *(kloc-symfony only)* the project must be a **Symfony 6.x or 7.x** app with `vendor/autoload.php` present
- *(kloc-intelligence AI features only)* an OpenAI-compatible **LLM + embedding API key** (OpenRouter, Gemini, OpenAI, …)

## Step-by-step Setup

### 1. Clone the monorepo

```bash
git clone https://github.com/michalboryczko/kloc.git
cd kloc
```

### 2. Fetch sub-projects

The monorepo is composed of several sub-repositories (listed in `repos.yml`). The setup script clones or updates all of them:

```bash
./setup.sh
```

| Component | Role | Required? |
|-----------|------|-----------|
| `kloc-cli` | Stateless CLI that queries `sot.json` | yes |
| `kloc-mapper` | Maps the SCIP index → `sot.json` | yes |
| `kloc-indexer-php` | Rust SCIP indexer for PHP projects → `index.json` | yes |
| `kloc-symfony` | Symfony framework extractor → `symfony-kloc.json` | optional |
| `kloc-intelligence` | Neo4j/Qdrant-backed code-intelligence service | optional |
| `kloc-reference-project-php` | PHP reference project for tests / experimentation | optional |

To set up only a specific component:

```bash
./setup.sh kloc-cli            # CLI query tool only
./setup.sh kloc-mapper         # SCIP-to-SoT mapper only
./setup.sh kloc-indexer-php    # PHP indexer only
./setup.sh kloc-symfony        # Symfony framework extractor only
./setup.sh kloc-intelligence   # Graph code-intelligence service only
```

### 3. Install kloc-cli

```bash
cd kloc-cli
uv pip install -e ".[dev]"
cd ..
```

Verify the installation:

```bash
cd kloc-cli && uv run kloc-cli --help && cd ..
```

### 4. Install kloc-mapper

```bash
cd kloc-mapper
uv pip install -e ".[dev]"
cd ..
```

Verify the installation:

```bash
cd kloc-mapper && uv run kloc-mapper --help && cd ..
```

### 5. Build the kloc-indexer-php indexer

```bash
cd kloc-indexer-php
cargo build --release
cd ..
```

This produces the indexer binary at `kloc-indexer-php/target/release/kloc-indexer-php`. Alternatively, `./build.sh kloc-indexer-php` builds it and copies the binary to `bin/kloc-indexer-php`.

### 6. (Optional) Build the kloc-symfony Docker image

Only needed if you want to extract Symfony framework architecture (`symfony-kloc.json`). The tool ships as a Docker image — no local PHP install required.

```bash
cd kloc-symfony
docker build -t kloc-symfony .
cd ..
```

This builds a Docker image named `kloc-symfony` (base `php:8.4-cli`, with `composer install --no-dev` baked in).

### 7. (Optional) Set up kloc-intelligence

Only needed if you want graph-backed queries, flow analysis, or AI features (explanations + semantic search). It needs Neo4j (and Qdrant for AI features), both provided via Docker Compose.

```bash
cd kloc-intelligence
uv sync --all-extras                  # omit --all-extras to skip the AI deps
cp .env.example .env                  # then edit LLM_API_KEY + EMBEDDING_API_KEY (only for enrich/search)
docker compose up -d                  # starts Neo4j + Qdrant
uv run kloc-intelligence schema ensure
cd ..
```

Verify the installation:

```bash
cd kloc-intelligence && uv run kloc-intelligence --help && cd ..
```

See [kloc-intelligence.md](kloc-intelligence.md) for the full command reference.

## Indexing Your PHP Project

The core kloc pipeline has three stages:

```
PHP project  -->  kloc-indexer-php  -->  kloc-mapper  -->  kloc-cli
                    (index.json)         (sot.json)       (queries)
```

Two optional branches plug in after the mapper:

```
                              sot.json
                                 |
                +----------------+-----------------+
                |                                  |
                v                                  v
   (Symfony apps only)                   (graph + AI, persistent)
   kloc-symfony                          kloc-intelligence
   --> symfony-kloc.json  ------------>  (Neo4j + Qdrant)
                                         --> CLI / MCP / Cypher
```

### Stage 1: Run kloc-indexer-php on your PHP project

kloc-indexer-php analyzes your PHP source code and produces an `index.json` file containing symbol definitions, references, and call graph data.

```bash
./kloc-indexer-php/target/release/kloc-indexer-php -d /path/to/your/php-project -o /path/to/output/index.json
# or, if you ran ./build.sh:  ./bin/kloc-indexer-php -d /path/to/your/php-project -o /path/to/output/index.json
```

Options:
- `-d, --project-root` -- path to your PHP project root (required)
- `-o, --output` -- output path for `index.json` (default: `<project>/.kloc/index.json`)
- `--experimental` -- include experimental call kinds (function calls, array access, etc.)
- `--internal-all` -- treat all vendor packages as internal (full indexing of dependencies)
- `--threads N`, `--php-version VERSION` -- see `kloc-indexer-php --help`

Your PHP project should have Composer dependencies installed (`composer install`) before indexing.

Output: `index.json` at the specified path.

### Stage 2: Run kloc-mapper to produce sot.json

kloc-mapper transforms the raw index into a structured Source-of-Truth JSON file that kloc-cli can query.

```bash
cd kloc-mapper
uv run kloc-mapper map /path/to/output/index.json -o /path/to/output/sot.json
cd ..
```

Options:
- First argument -- path to the index.json file (required)
- `--out, -o` -- output path for sot.json (required)
- `--pretty, -p` -- pretty-print the JSON output (useful for inspection)

Output: `sot.json` at the specified path.

### Stage 2b (optional): Extract Symfony architecture with kloc-symfony

If your project is a Symfony app, run kloc-symfony to produce `symfony-kloc.json` (routes, message handlers, event listeners, console commands, and cross-flow triggers). It boots the target's Symfony kernel and reads the compiled DI container, so it needs the target's `vendor/` to be installed.

Using the wrapper script (builds the Docker image on first run):

```bash
./kloc-symfony/bin/kloc-symfony.sh /path/to/symfony-app /path/to/output /path/to/output/sot.json
```

Arguments:
- `<project-dir>` -- path to the Symfony project root (required)
- `[output-dir]` -- directory for `symfony-kloc.json` (default: `<project-dir>/.kloc`)
- `[sot-path]` -- optional `sot.json` for `node_id` cross-referencing + trigger detection

Or invoke the image directly:

```bash
docker run --rm \
  -v /path/to/symfony-app:/input:ro \
  -v /path/to/output:/output \
  -v /path/to/output/sot.json:/sot.json:ro \
  kloc-symfony extract /input -o /output/symfony-kloc.json --sot /sot.json --pretty
```

Output: `symfony-kloc.json`. It is consumed by kloc-intelligence (`import-flows`); kloc-cli does not read it.

See [kloc-symfony.md](kloc-symfony.md) for details.

### Stage 3: Query with kloc-cli

Now you can query your codebase:

```bash
cd kloc-cli

# Find where a class is defined
uv run kloc-cli resolve "App\Service\OrderService" -s /path/to/output/sot.json

# Get the full picture of a class
uv run kloc-cli context "App\Service\OrderService" -s /path/to/output/sot.json -d 2

# Find all usages of a method
uv run kloc-cli usages "App\Service\OrderService::createOrder()" -s /path/to/output/sot.json
```

See [cli.md](cli.md) for the full command reference.

### Stage 3 (alternative): Query with kloc-intelligence

For large codebases, persistent analysis, flow queries, or AI workflows, import `sot.json` into Neo4j once and query many times:

```bash
cd kloc-intelligence

# Import the graph (clears the DB by default)
uv run kloc-intelligence import /path/to/output/sot.json

# Import Symfony flows (optional — only if you produced symfony-kloc.json)
uv run kloc-intelligence import-flows /path/to/output/symfony-kloc.json

# (Optional) enrich with LLM explanations + embeddings — needs API keys in .env
uv run kloc-intelligence enrich
uv run kloc-intelligence enrich-flows

# Query
uv run kloc-intelligence context "App\Service\OrderService::createOrder()" -d 2
uv run kloc-intelligence flows "App\Controller\OrderController::create"
uv run kloc-intelligence search "create a new customer order"
cd ..
```

kloc-intelligence emits the same JSON contract as kloc-cli for the shared commands (`resolve`, `usages`, `deps`, `context`, `owners`, `inherit`, `overrides`). See [kloc-intelligence.md](kloc-intelligence.md) for the full command reference and MCP server setup.

## Configuration File

For convenience, you can create a `kloc.json` configuration file that stores paths to your sot.json files. This is especially useful when working with multiple projects or when using the MCP server.

```json
{
  "projects": [
    {
      "name": "my-app",
      "sot": "/absolute/path/to/my-app/sot.json"
    }
  ]
}
```

For multiple projects:

```json
{
  "projects": [
    {
      "name": "my-app",
      "sot": "/path/to/my-app/sot.json"
    },
    {
      "name": "payments",
      "sot": "/path/to/payments-service/sot.json"
    },
    {
      "name": "auth",
      "sot": "/path/to/auth-service/sot.json"
    }
  ]
}
```

Use with the MCP server:

```bash
cd kloc-cli
uv run kloc-cli mcp-server --config /path/to/kloc.json
```

## Using kloc-dev.sh (Development Mode)

`kloc-dev.sh` is a pipeline wrapper that combines all three stages (index, map, query) into a single command. It uses the bundled `kloc-reference-project-php` as the target project and caches artifacts so repeated runs are fast.

### Basic usage

```bash
# Run the full pipeline and query OrderService
./kloc-dev.sh context "App\Service\OrderService" --depth 2

# Include implementations/overrides
./kloc-dev.sh context "App\Service\OrderService" --depth 2 --impl

# Resolve a symbol
./kloc-dev.sh resolve "App\Entity\Order"
```

### Named runs with --id

By default, each run gets a timestamped ID. You can assign a name with `--id` to reuse cached artifacts:

```bash
# First run: indexes and maps (slow)
./kloc-dev.sh context "App\Service\OrderService" --id=my-test --depth 2

# Second run: skips index/map, goes straight to query (fast)
./kloc-dev.sh resolve "App\Entity\Order" --id=my-test
```

Artifacts are stored in `artifacts/kloc-dev/{id}/` and contain:
- `index.json` -- the SCIP index output
- `sot.json` -- the mapped Source-of-Truth file

### Full vendor indexing

To index all vendor packages as internal (not just your project code):

```bash
./kloc-dev.sh context "App\Service\OrderService" --internal-all
```

### Generate artifacts only

Run without a kloc-cli command to just produce the index and sot.json:

```bash
./kloc-dev.sh
# Artifacts ready at: artifacts/kloc-dev/<timestamp>/
```

## Building Standalone Binaries

You can build standalone binaries for kloc-cli, kloc-mapper, and kloc-indexer-php using `build.sh` — they are copied into the `bin/` directory at the monorepo root. The kloc-cli/kloc-mapper binaries do not require Python or uv to run.

### Build all components

```bash
./build.sh
```

### Build a specific component

```bash
./build.sh kloc-cli           # Build kloc-cli binary only
./build.sh kloc-mapper        # Build kloc-mapper binary only
./build.sh kloc-indexer-php   # Build the Rust indexer only
```

Binaries are placed in the `bin/` directory at the monorepo root.

### Platform notes

- On **macOS**, kloc-cli and kloc-mapper build natively using PyInstaller
- On **Linux**, the build runs natively as well
- To cross-compile a **Linux binary from macOS**, use the Docker-based build described in the component CLAUDE.md files
- `kloc-symfony` and `kloc-intelligence` are registered in `repos.yml` but have no binary build step — `build.sh` reports them as "(no build needed)". Set them up as described in steps 6–7 above (Docker image / `uv sync`).

### Using the binaries

After building, you can use the binaries directly without `uv run`:

```bash
# Instead of: cd kloc-cli && uv run kloc-cli resolve ...
./bin/kloc-cli resolve "App\Service\OrderService" -s sot.json

# Instead of: cd kloc-mapper && uv run kloc-mapper map ...
./bin/kloc-mapper map index.json -o sot.json
```

## Quick Start Summary

For the impatient, here is the entire core flow from zero to querying:

```bash
# 1. Clone and setup
git clone https://github.com/michalboryczko/kloc.git
cd kloc
./setup.sh

# 2. Install / build tools
cd kloc-cli && uv pip install -e ".[dev]" && cd ..
cd kloc-mapper && uv pip install -e ".[dev]" && cd ..
cd kloc-indexer-php && cargo build --release && cd ..

# 3. Index your PHP project
./kloc-indexer-php/target/release/kloc-indexer-php -d /path/to/your/project -o ./output/index.json

# 4. Map to sot.json
cd kloc-mapper
uv run kloc-mapper map ../output/index.json -o ../output/sot.json
cd ..

# 5. Query
cd kloc-cli
uv run kloc-cli context "App\YourNamespace\YourClass" -s ../output/sot.json -d 2
```

### Optional add-ons

```bash
# Symfony architecture (only for Symfony apps) -> symfony-kloc.json
cd kloc-symfony && docker build -t kloc-symfony . && cd ..
./kloc-symfony/bin/kloc-symfony.sh /path/to/your/project ./output ./output/sot.json

# Graph + AI code intelligence (Neo4j + Qdrant)
cd kloc-intelligence
uv sync --all-extras
cp .env.example .env            # edit LLM_API_KEY + EMBEDDING_API_KEY for enrich/search
docker compose up -d
uv run kloc-intelligence schema ensure
uv run kloc-intelligence import ../output/sot.json
uv run kloc-intelligence import-flows ../output/symfony-kloc.json   # if you ran kloc-symfony
uv run kloc-intelligence context "App\YourNamespace\YourClass" -d 2
cd ..
```
