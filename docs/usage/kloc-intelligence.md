# kloc-intelligence Usage Guide

## Other docs

- [kloc-intelligence-overview.md](../kloc-intelligence-overview.md) — what it does and why (capability catalog)
- [kloc-intelligence-processes.md](../kloc-intelligence-processes.md) — how each major process works
- [setup.md](setup.md), [cli.md](cli.md), [mcp.md](mcp.md) — pipeline + companion tools (kloc-cli)

---

kloc-intelligence is a graph-native code intelligence service for PHP codebases. It loads a `sot.json` (Source of Truth) into Neo4j, optionally enriches the graph with LLM-generated explanations and Qdrant vector embeddings, and exposes everything through a CLI and an MCP server for AI agents.

It sits at the end of the kloc pipeline:

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

Compared to `kloc-cli` (which is stateless and reads `sot.json` directly), `kloc-intelligence` is **stateful** — it persists the graph in Neo4j, supports multi-hop Cypher traversals, and adds AI features (explanations, semantic search, code chunks) backed by Qdrant.

## Setup

### Prerequisites

- Python 3.11+
- [uv](https://github.com/astral-sh/uv) package manager
- Docker for Neo4j 5.x and Qdrant 1.12+
- A `sot.json` produced by the kloc pipeline
- (optional) `symfony-kloc.json` if your project is a Symfony app and you want the flows feature
- (optional) An OpenAI-compatible LLM API key (OpenRouter, Google Gemini via the OpenAI-compat endpoint, OpenAI, or any equivalent) for `enrich` / `explain` / semantic `search`

### Install and bring up infra

```bash
cd kloc-intelligence
uv sync --all-extras
docker compose up -d                 # Neo4j on 7474/7687, Qdrant on 6333/6334
cp .env.example .env                 # edit ports / fill in LLM_API_KEY + EMBEDDING_API_KEY
uv run kloc-intelligence schema ensure
```

### Environment variables

LLM and embedding providers are configured **independently** — each has its own URL/key/model. They may share a key (e.g. both pointed at OpenRouter) or split (e.g. Gemini for chat + OpenRouter for embeddings).

| Variable | Default | What it controls |
|----------|---------|------------------|
| `NEO4J_URI` | `bolt://localhost:7687` | Neo4j Bolt endpoint |
| `NEO4J_USERNAME` | `neo4j` | Neo4j username |
| `NEO4J_PASSWORD` | `kloc-intelligence` | Neo4j password |
| `NEO4J_DATABASE` | `neo4j` | Neo4j database name |
| `QDRANT_URL` | `http://localhost:6333` | Qdrant HTTP endpoint |
| `QDRANT_API_KEY` | unset | Qdrant API key (managed instances) |
| `LLM_API_URL` | `https://openrouter.ai/api/v1` | Base URL for chat completions (`enrich`, `explain`) |
| `LLM_API_KEY` | unset | API key for the LLM endpoint |
| `LLM_MODEL` | `minimax/minimax-m2.7` | Chat model identifier |
| `EMBEDDING_API_URL` | `https://openrouter.ai/api/v1` | Base URL for embeddings (`enrich`, `search`) |
| `EMBEDDING_API_KEY` | unset | API key for the embedding endpoint |
| `EMBEDDING_MODEL` | `qwen/qwen3-embedding-8b` | Embedding model identifier |
| `EMBEDDING_DIMENSION` | `4096` | Vector dimension — must match the model |
| `KLOC_PROJECT_ROOT` | unset | Default project root for `source` / `chunks` |

The MCP server reads the same `.env`. `explain` / `enrich` need `LLM_API_KEY`; `search` needs `EMBEDDING_API_KEY`. They are independent.

#### Mixed-provider example (Gemini chat + OpenRouter embeddings)

```ini
LLM_API_URL=https://generativelanguage.googleapis.com/v1beta/openai/
LLM_API_KEY=<gemini-key>
LLM_MODEL=gemini-3-flash-preview

EMBEDDING_API_URL=https://openrouter.ai/api/v1
EMBEDDING_API_KEY=<openrouter-key>
EMBEDDING_MODEL=google/gemini-embedding-001
EMBEDDING_DIMENSION=3072
```

Note: Gemini's *native* OpenAI-compat embeddings endpoint omits the `usage` field that Haystack expects, so the embedding step crashes when `EMBEDDING_API_URL` points directly at Gemini. Routing through OpenRouter (which fills in `usage`) is the workaround. Chat completions against native Gemini work fine.

When you change `EMBEDDING_DIMENSION`, drop the existing Qdrant collections and re-run `enrich` — vector dimension is fixed at collection-create time.

## Pipeline at a glance

1. Generate `sot.json` for your PHP project (via `kloc-indexer-php` + `kloc-mapper`)
2. **Import the graph** — `kloc-intelligence import sot.json`
3. (optional) **Enrich with LLM explanations + embeddings** — `kloc-intelligence enrich`
4. (optional, Symfony only) **Generate `symfony-kloc.json`** via `kloc-symfony`
5. (optional, Symfony only) **Import flows** — `kloc-intelligence import-flows symfony-kloc.json`
6. **Query** via CLI, MCP, or Cypher in the Neo4j browser

Steps 1–2 are mandatory. Steps 3–5 are independent and additive.

## CLI commands

All commands accept `--json` for machine-readable output and `--project <name>` to disambiguate when multiple projects are loaded.

### Graph ingestion

#### import — load `sot.json` into Neo4j

**Question it answers:** "Get the graph in place."

```bash
uv run kloc-intelligence import /path/to/sot.json --project-root /path/to/php-project
```

Wipes existing nodes for the project and imports fresh. `--project-root` is stored as `:Node.project_root` so `source` / `chunks` can read files later.

#### import-flows — load Symfony flows

**Question it answers:** "What HTTP / message / event / CLI flows does this app expose, and how do they trigger each other?"

```bash
uv run kloc-intelligence import-flows /path/to/.kloc/symfony-kloc.json
```

Imports `:Flow` nodes (one per HTTP route, message handler, event listener, CLI command) plus `FLOW_ENTRY` (Flow → entry method `:Node`) and `FLOW_TRIGGERS` (Flow → Flow, when one flow dispatches a message/event another flow handles).

The model is **deliberately minimal** — a flow knows its entry point and its dispatch relations, nothing more. Use `kloc_context` / `source` / `chunks` to investigate what a flow actually does. See "Flows in practice" below.

Each call also drops the three stale Qdrant `flow_*_embeddings` collections from a previous design (idempotent).

#### flows — list or inspect imported flows

**Question:** "What flows did the import produce, and what does flow X look like?"

```bash
uv run kloc-intelligence flows                                         # list all
uv run kloc-intelligence flows --type http                             # filter by type
uv run kloc-intelligence flows --type http,cli                         # multi-filter
uv run kloc-intelligence flows OrderController                         # candidate match
uv run kloc-intelligence flows OrderController::create                 # detail (single match)
uv run kloc-intelligence flows --json                                  # JSON output
```

The response is a discriminated union with `mode`:
- `list` — array of flows (with `--type` filter optionally applied)
- `detail` — full info for one flow including entry method + triggers in/out
- `candidates` — when the query partially matches multiple flows; a list of `(flow_id, type, name)` for the user to disambiguate

Use detail mode to find an entry method's `node_id`, then feed that to `kloc_context` for the deep call-tree investigation. Flows know boundaries; `context` walks the tree.

#### enrich-flows — generate business-process summaries for flows

**Question:** "What business process does each flow drive, in plain language?"

```bash
uv run kloc-intelligence enrich-flows
uv run kloc-intelligence enrich-flows --force      # re-enrich already enriched
```

For each `:Flow`, walks the depth-3 bidirectional context (with implementations) of the entry method, attaches source snippets from the referenced nodes, and asks the LLM for a 1–3 sentence abstract description optimized for business-vocabulary search queries. The result is stored as `f.explanation` on the flow node and embedded into the `flow_explain_embeddings` Qdrant collection so `search` returns flows alongside other code.

The summaries are deliberately abstract: business vocabulary (orders, customers, payments) over implementation vocabulary (controllers, dispatchers). The goal is recall on phrases like "process customer orders" or "send notification when delivery fails", not implementation accuracy.

### Symbol resolution

#### resolve — find where a symbol is defined

```bash
uv run kloc-intelligence resolve "App\Service\OrderService"
uv run kloc-intelligence resolve "OrderService"             # partial match
uv run kloc-intelligence resolve "OrderService::createOrder()"
```

#### owners — containment chain

**Question:** "Where does `createOrder` live? File / class / namespace."

```bash
uv run kloc-intelligence owners "App\Service\OrderService::createOrder()"
```

### Bidirectional traversal

#### usages — who uses this?

```bash
uv run kloc-intelligence usages "App\Service\OrderService::createOrder()" -d 2
```

Walks incoming `USES` edges. `-d` controls BFS depth.

#### deps — what does this use?

```bash
uv run kloc-intelligence deps "App\Service\OrderService::createOrder()" -d 2
```

Walks outgoing `USES` edges.

#### context — both directions, plus inheritance

```bash
uv run kloc-intelligence context "App\Service\OrderService::createOrder()" -d 3
```

The richest single query. Returns callers, callees, types referenced, owners, and overrides in one tree. Best starting point for an AI agent investigating an unfamiliar method.

### Inheritance and polymorphism

#### inherit — extends/implements tree

```bash
uv run kloc-intelligence inherit "App\Repository\OrderRepositoryInterface"
```

#### overrides — concrete implementations of an interface method

```bash
uv run kloc-intelligence overrides "App\Repository\OrderRepositoryInterface::findById()"
```

### AI / enrichment features (require `LLM_API_KEY` and/or `EMBEDDING_API_KEY`)

#### enrich — generate explanations and embeddings

```bash
uv run kloc-intelligence enrich
uv run kloc-intelligence enrich-status      # progress
```

Writes business-language and technical explanations to `:Class` and `:Method` nodes, then embeds source code and explanations into Qdrant collections `code_embeddings` and `explain_embeddings`.

#### explain — show or generate explanation for one symbol

```bash
uv run kloc-intelligence explain "App\Service\OrderService::createOrder()"
uv run kloc-intelligence explain "App\Service\OrderService" --force   # regenerate
```

#### search — semantic search across code + explanations

```bash
uv run kloc-intelligence search "validate order before checkout"
uv run kloc-intelligence search "places where we charge a customer" --limit 5
```

### Source code access

#### source — raw source for a node

```bash
uv run kloc-intelligence source "App\Service\OrderService::createOrder()"
uv run kloc-intelligence source node:779b5ec2e2f2e61b --json
```

Reads the file referenced by the node's `file` property (relative to `--project-root` or `KLOC_PROJECT_ROOT`).

#### chunks — token-bounded chunks (the same chunks used for embeddings)

```bash
uv run kloc-intelligence chunks "App\Service\OrderService" --max-tokens 4000
```

For long classes the output is split by method boundary, with class-context preamble repeated in each chunk. Useful when feeding code to an LLM with a context-window budget.

### Schema management

```bash
uv run kloc-intelligence schema ensure       # create indexes/constraints
uv run kloc-intelligence schema info         # show current schema
```

## Flows in practice

After `import-flows`, querying is just Cypher:

```cypher
// All flows
MATCH (f:Flow) RETURN f.flow_id, f.type, f.name, f.route ORDER BY f.flow_id

// Entry method for one flow
MATCH (f:Flow {flow_id: $flow_id})-[:FLOW_ENTRY]->(m:Node)
RETURN f.name, m.fqn, m.node_id, m.start_line, m.end_line

// Trigger graph (which flow dispatches which)
MATCH (src:Flow)-[r:FLOW_TRIGGERS]->(dst:Flow)
RETURN src.flow_id, r.trigger_type, r.via, dst.flow_id
```

To dig into what a flow *actually does*, take its entry `node_id` and pivot into the regular tools:

```bash
# 1. Find the flow's entry node
uv run kloc-intelligence resolve "App\Ui\Rest\Controller\OrderController::get()"

# 2. Walk the real call tree from there
uv run kloc-intelligence context node:779b5ec2e2f2e61b -d 4

# 3. Read the entry method's source if you need details
uv run kloc-intelligence source node:779b5ec2e2f2e61b
```

This is the design: the `:Flow` graph stays small and correct; the agent does the deep work on demand instead of trusting a pre-built (and historically buggy) call tree.

## MCP server

Start the server over stdio:

```bash
uv run kloc-intelligence mcp-server
```

Tools exposed (16 in this build):

| Tool | Equivalent CLI |
|------|----------------|
| `kloc_resolve` | `resolve` |
| `kloc_usages` | `usages` |
| `kloc_deps` | `deps` |
| `kloc_context` | `context` |
| `kloc_owners` | `owners` |
| `kloc_inherit` | `inherit` |
| `kloc_overrides` | `overrides` |
| `kloc_import` | `import` |
| `kloc_explain` | `explain` |
| `kloc_search` | `search` |
| `kloc_enrich` | `enrich` |
| `kloc_import_flows` | `import-flows` |
| `kloc_enrich_flows` | `enrich-flows` |
| `kloc_flows` | `flows` |
| `kloc_source` | `source` |
| `kloc_chunks` | `chunks` |

Wire it into a Claude Code, Cursor, or other MCP client config:

```json
{
  "mcpServers": {
    "kloc-intelligence": {
      "command": "uv",
      "args": ["run", "kloc-intelligence", "mcp-server"],
      "cwd": "/path/to/kloc-intelligence",
      "env": {
        "KLOC_PROJECT_ROOT": "/path/to/php-project"
      }
    }
  }
}
```

`kloc_import_flows` returns `{status, flows, flow_entry_edges, flow_triggers_edges}` so the agent can confirm the import succeeded and counts match expectations.

## Recipes

### Index a fresh project end-to-end

```bash
# Generate sot.json (run from the kloc monorepo root)
./kloc.sh /path/to/php-project

# Load into Neo4j
cd kloc-intelligence
uv run kloc-intelligence import /path/to/php-project/.kloc/sot.json \
  --project-root /path/to/php-project

# Enrich (optional, costs LLM tokens)
uv run kloc-intelligence enrich

# Symfony app? Add flows
uv run kloc-intelligence import-flows /path/to/php-project/.kloc/symfony-kloc.json
```

### Reset everything (Neo4j + Qdrant)

```bash
# Stop and wipe Docker volumes
docker compose down -v
docker compose up -d
uv run kloc-intelligence schema ensure

# (or, surgical: only flow data)
echo 'MATCH (f:Flow) DETACH DELETE f' | cypher-shell -u neo4j -p kloc-intelligence
uv run python bin/drop-flow-collections.py
```

### Verify a flows import

```bash
# Counts (should match the source symfony-kloc.json's flows[] / triggers[] cardinality)
echo 'MATCH (f:Flow) RETURN count(f) AS flows' | cypher-shell -u neo4j -p kloc-intelligence
echo 'MATCH ()-[r:FLOW_ENTRY]->() RETURN count(r) AS entries' | cypher-shell -u neo4j -p kloc-intelligence
echo 'MATCH ()-[r:FLOW_TRIGGERS]->() RETURN count(r) AS triggers' | cypher-shell -u neo4j -p kloc-intelligence

# Should always be zero
echo 'MATCH ()-[r:FLOW_STEP]->() RETURN count(r)' | cypher-shell -u neo4j -p kloc-intelligence
```

### Explore a single flow with an LLM agent

In Claude Code (with the MCP server connected) or any agent that speaks the same tools:

```
1. kloc_import_flows path=/path/to/symfony-kloc.json
2. Cypher (via your DB tool of choice): list flows -> pick one
3. kloc_context node_id=<entry_node_id> depth=4
4. kloc_source / kloc_chunks for any node you want to read
```

## Troubleshooting

- **`Connection refused` on Neo4j** — `docker compose ps`; restart with `docker compose up -d`.
- **`OPENROUTER_KEY not set`** on `enrich` / `search` / `explain` — these features need an OpenRouter API key. Other commands (resolve / usages / deps / context / owners / inherit / overrides / import / import-flows / source / chunks) work without it.
- **`Could not resolve …`** — the symbol isn't in `sot.json`, or it's in the indexer's vendor-skip set. Re-run the indexer with the relevant directories included.
- **Many `FLOW_ENTRY skipped: no :Node with node_id=…` warnings during `import-flows`** — your loaded `sot.json` doesn't match the project that produced `symfony-kloc.json`. Re-run `import` with the right `sot.json` first.
- **`kloc_search` returns no results** — embeddings haven't been generated yet. Run `enrich`, or check `enrich-status`.

## See also

- [setup.md](setup.md) — full pipeline setup (indexer, mapper, kloc-symfony)
- [cli.md](cli.md) — `kloc-cli` (the stateless, sot.json-only CLI)
- [mcp.md](mcp.md) — `kloc-cli`'s MCP server (counterpart to the one documented here)
- [../specs/flows-kloc-inteligence.md](../specs/flows-kloc-inteligence.md) — flows feature spec (decisions D1–D12, all 20 ACs)
- [../specs/flows-kloc-inteligence-plan.md](../specs/flows-kloc-inteligence-plan.md) — flows implementation plan and interface contracts
