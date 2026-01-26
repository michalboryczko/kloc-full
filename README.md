# kloc-full

Meta repository for the **KLOC (Knowledge of Code)** project - a toolkit for extracting and querying rich code context for AI coding agents.

## Overview

KLOC provides a pipeline to transform source code into a queryable graph representation:

```
Source Code → SCIP Index → SoT JSON → Queries/MCP
```

## Components

| Repository | Description |
|------------|-------------|
| [kloc-mapper](https://github.com/michalboryczko/kloc-mapper) | Converts SCIP indexes to Source-of-Truth (SoT) JSON |
| [kloc-cli](https://github.com/michalboryczko/kloc-cli) | CLI for querying SoT JSON (deps, usages, context, inherit) |
| [scip-php](https://github.com/michalboryczko/scip-php) | PHP SCIP indexer |

## Quick Start

```bash
# Clone this meta repo
git clone git@github.com:michalboryczko/kloc-full.git
cd kloc-full

# Fetch all component repos
./setup.sh

# Build all binaries
./build.sh

# Binaries are now in bin/
./bin/kloc-mapper --help
./bin/kloc-cli --help
```

## Usage Pipeline

```bash
# 1. Index your PHP project (using scip-php or other SCIP indexer)
scip-php index --output artifacts/scip.index

# 2. Convert to SoT JSON
./bin/kloc-mapper map -s artifacts/scip.index -o artifacts/sot.json

# 3. Query the codebase
./bin/kloc-cli context "UserService::createUser" --sot artifacts/sot.json --impl
./bin/kloc-cli deps "UserController" --sot artifacts/sot.json --depth 2
./bin/kloc-cli usages "User" --sot artifacts/sot.json

# 4. Or use MCP server for AI integration
./bin/kloc-cli mcp-server --sot artifacts/sot.json
```

## Directory Structure

```
kloc-full/
├── setup.sh          # Fetch all repos
├── build.sh          # Build all binaries
├── repos.yml         # Repository definitions
├── bin/              # Built binaries (after build.sh)
├── kloc-cli/         # Cloned repo (after setup.sh)
├── kloc-mapper/      # Cloned repo (after setup.sh)
└── scip-php/         # Cloned repo (after setup.sh)
```

## Development

To work on individual components:

```bash
cd kloc-cli
source venv/bin/activate  # or: source .venv/bin/activate
pip install -e ".[dev]"
pytest
```

## License

MIT
