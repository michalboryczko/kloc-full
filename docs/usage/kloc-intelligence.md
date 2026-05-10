# kloc-intelligence — Usage

kloc-intelligence is a graph-native code intelligence service for PHP codebases. It loads a `sot.json` (Source of Truth) into Neo4j, optionally enriches the graph with LLM-generated explanations and Qdrant vector embeddings, and exposes everything through a CLI and an MCP server for AI agents.

```
PHP source -> kloc-indexer-php -> index.json
              kloc-mapper       -> sot.json
              kloc-symfony      -> symfony-kloc.json
                                   |
                                   v
                          kloc-intelligence (Neo4j + Qdrant)
                                   |
                                   v
                          CLI / MCP server / Cypher
```

Compared to `kloc-cli` (stateless, reads `sot.json` directly), `kloc-intelligence` is **stateful** — it persists the graph in Neo4j, supports multi-hop Cypher traversals, and adds AI features (explanations, semantic search, code chunks) backed by Qdrant.

## Documentation

- [configuration.md](kloc-intelligence/configuration.md) — install, env vars, provider recipes (OpenRouter, Google Gemini, OpenAI, mixed)
- [data-setup.md](kloc-intelligence/data-setup.md) — pipeline order: import → flows → enrich → enrich-flows, plus reset/regenerate recipes
- [cli.md](kloc-intelligence/cli.md) — every CLI command grouped by area
- [mcp.md](kloc-intelligence/mcp.md) — MCP server setup, tool catalog, Claude Code/Cursor integration

## Conceptual docs

- [../kloc-intelligence-overview.md](../kloc-intelligence-overview.md) — what it does and why (capability catalog)
- [../kloc-intelligence-processes.md](../kloc-intelligence-processes.md) — how each major process works (data flows)
- [../specs/flows-kloc-inteligence.md](../specs/flows-kloc-inteligence.md) — flows feature spec
- [../specs/flows-kloc-inteligence-plan.md](../specs/flows-kloc-inteligence-plan.md) — flows implementation plan

## Companion tools

- [setup.md](setup.md) — full kloc pipeline setup
- [cli.md](cli.md) — `kloc-cli` (the stateless, sot.json-only CLI)
- [mcp.md](mcp.md) — `kloc-cli`'s MCP server
- [kloc-symfony.md](kloc-symfony.md) — Symfony framework extractor that produces `symfony-kloc.json`

## Quick start

```bash
cd kloc-intelligence
uv sync --all-extras
docker compose up -d
cp .env.example .env                              # edit LLM_API_KEY + EMBEDDING_API_KEY
uv run kloc-intelligence schema ensure

# Ingest
uv run kloc-intelligence import /path/to/sot.json
uv run kloc-intelligence import-flows /path/to/.kloc/symfony-kloc.json   # Symfony only

# (Optional) enrich
uv run kloc-intelligence enrich
uv run kloc-intelligence enrich-flows

# Query
uv run kloc-intelligence context "OrderService::createOrder" -d 2
uv run kloc-intelligence flows OrderController::create
uv run kloc-intelligence search "create a new customer order"
```

For details on any of these steps, see the linked docs above.
