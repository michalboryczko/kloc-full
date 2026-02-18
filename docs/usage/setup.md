# kloc Setup Guide

This guide walks you through setting up kloc and using it to analyze a PHP codebase. By the end, you will be able to query your project's class hierarchy, method calls, and dependencies from the command line.

## Prerequisites

- **Python 3.12+** -- required for kloc-cli and kloc-mapper
- **uv** -- Python package manager ([install guide](https://docs.astral.sh/uv/getting-started/installation/))
- **Docker** -- required for the scip-php indexer
- **A PHP project** to analyze (with Composer dependencies installed)

## Step-by-step Setup

### 1. Clone the monorepo

```bash
git clone https://github.com/michalboryczko/kloc.git
cd kloc
```

### 2. Fetch sub-projects

The monorepo contains three sub-repositories: `kloc-cli`, `kloc-mapper`, and `scip-php`. The setup script clones or updates all of them:

```bash
./setup.sh
```

To set up only a specific component:

```bash
./setup.sh kloc-cli      # CLI query tool only
./setup.sh kloc-mapper    # SCIP-to-SoT mapper only
./setup.sh scip-php       # PHP indexer only
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

### 5. Build the scip-php Docker image

```bash
cd scip-php
./build/build.sh
cd ..
```

This builds a Docker image named `scip-php` that contains the PHP indexer.

## Indexing Your PHP Project

The kloc pipeline has three stages:

```
PHP project  -->  scip-php  -->  kloc-mapper  -->  kloc-cli
               (index.json)     (sot.json)       (queries)
```

### Stage 1: Run scip-php on your PHP project

scip-php analyzes your PHP source code and produces an `index.json` file containing symbol definitions, references, and call graph data.

```bash
./scip-php/bin/scip-php.sh -d /path/to/your/php-project -o /path/to/output
```

Options:
- `-d, --project-dir` -- path to your PHP project root (required)
- `-o, --output` -- output directory for index.json (default: current directory)
- `--experimental` -- include experimental call kinds (function calls, array access, etc.)
- `--internal-all` -- treat all vendor packages as internal (full indexing of dependencies)

Your PHP project should have Composer dependencies installed (`composer install`) before indexing.

Output: `index.json` in the specified output directory.

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

You can build standalone binaries for kloc-cli and kloc-mapper using `build.sh`. These binaries do not require Python or uv to run.

### Build all components

```bash
./build.sh
```

### Build a specific component

```bash
./build.sh kloc-cli       # Build kloc-cli binary only
./build.sh kloc-mapper     # Build kloc-mapper binary only
./build.sh scip-php        # Build scip-php only
```

Binaries are placed in the `bin/` directory at the monorepo root.

### Platform notes

- On **macOS**, kloc-cli and kloc-mapper build natively using PyInstaller
- On **Linux**, the build runs natively as well
- To cross-compile a **Linux binary from macOS**, use the Docker-based build described in the component CLAUDE.md files

### Using the binaries

After building, you can use the binaries directly without `uv run`:

```bash
# Instead of: cd kloc-cli && uv run kloc-cli resolve ...
./bin/kloc-cli resolve "App\Service\OrderService" -s sot.json

# Instead of: cd kloc-mapper && uv run kloc-mapper map ...
./bin/kloc-mapper map index.json -o sot.json
```

## Quick Start Summary

For the impatient, here is the entire flow from zero to querying:

```bash
# 1. Clone and setup
git clone https://github.com/michalboryczko/kloc.git
cd kloc
./setup.sh

# 2. Install tools
cd kloc-cli && uv pip install -e ".[dev]" && cd ..
cd kloc-mapper && uv pip install -e ".[dev]" && cd ..
cd scip-php && ./build/build.sh && cd ..

# 3. Index your PHP project
./scip-php/bin/scip-php.sh -d /path/to/your/project -o ./output

# 4. Map to sot.json
cd kloc-mapper
uv run kloc-mapper map ../output/index.json -o ../output/sot.json
cd ..

# 5. Query
cd kloc-cli
uv run kloc-cli context "App\YourNamespace\YourClass" -s ../output/sot.json -d 2
```
