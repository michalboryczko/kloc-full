# kloc-intelligence — Configuration

How to install kloc-intelligence and configure its providers. For data ingestion + enrichment workflow see [data-setup.md](data-setup.md). For commands see [cli.md](cli.md) and [mcp.md](mcp.md).

## Prerequisites

- Python 3.11+
- [uv](https://github.com/astral-sh/uv) package manager
- Docker for Neo4j 5.x and Qdrant 1.12+
- A `sot.json` produced by the kloc pipeline (`kloc-indexer-php` → `kloc-mapper`)
- (optional) `symfony-kloc.json` for Symfony projects — needed for the flows feature
- (optional) An OpenAI-compatible LLM API key (OpenRouter, Google Gemini OpenAI-compat, OpenAI, or any equivalent) for AI features (`enrich`, `explain`, `search`, `enrich-flows`)

## Install

```bash
cd kloc-intelligence
uv sync --all-extras                 # installs the `ai` extras (haystack, qdrant-client)
docker compose up -d                 # Neo4j on 7474/7687, Qdrant on 6333/6334
cp .env.example .env                 # edit ports + LLM_API_KEY + EMBEDDING_API_KEY
uv run kloc-intelligence schema ensure
```

The `--all-extras` flag installs the AI dependencies. Without it, `import`, `flows`, and the symbol/traversal commands still work; `enrich`, `explain`, `search`, and `enrich-flows` raise a "missing ai extras" error and tell you to install.

## Environment variables

LLM and embedding providers are configured **independently** — each has its own URL/key/model. They may share a key (both pointed at OpenRouter, default) or split (Gemini for chat + OpenRouter for embeddings).

### Neo4j

| Variable | Default | What it controls |
| --- | --- | --- |
| `NEO4J_URI` | `bolt://localhost:7687` | Neo4j Bolt endpoint |
| `NEO4J_USERNAME` | `neo4j` | Neo4j username |
| `NEO4J_PASSWORD` | `kloc-intelligence` | Neo4j password |
| `NEO4J_DATABASE` | `neo4j` | Neo4j database name |

### Qdrant

| Variable | Default | What it controls |
| --- | --- | --- |
| `QDRANT_URL` | `http://localhost:6333` | Qdrant HTTP endpoint |
| `QDRANT_API_KEY` | unset | Qdrant API key (managed instances only) |

### LLM (used by `explain`, `enrich`, `enrich-flows`)

| Variable | Default | What it controls |
| --- | --- | --- |
| `LLM_API_URL` | `https://openrouter.ai/api/v1` | Base URL for chat completions |
| `LLM_API_KEY` | unset | API key for the LLM endpoint |
| `LLM_MODEL` | `minimax/minimax-m2.7` | Chat model identifier |

### Embeddings (used by `enrich`, `search`, `enrich-flows`)

| Variable | Default | What it controls |
| --- | --- | --- |
| `EMBEDDING_API_URL` | `https://openrouter.ai/api/v1` | Base URL for embeddings |
| `EMBEDDING_API_KEY` | unset | API key for the embedding endpoint |
| `EMBEDDING_MODEL` | `qwen/qwen3-embedding-8b` | Embedding model identifier |
| `EMBEDDING_DIMENSION` | `4096` | Vector dimension — must match the model |

### Project metadata

| Variable | Default | What it controls |
| --- | --- | --- |
| `KLOC_PROJECT_ROOT` | unset | Default project root for `source` / `chunks` / `enrich` |
| `KLOC_PROJECT_NAME` | `default` | Project label written to embedding metadata |

`explain` / `enrich` / `enrich-flows` need `LLM_API_KEY`. `search` and the embedding side of `enrich` / `enrich-flows` need `EMBEDDING_API_KEY`. They are validated independently — `search` will not fail if `LLM_API_KEY` is missing, and vice versa.

### Symfony flow filtering

| Variable | Default | What it controls |
| --- | --- | --- |
| `KLOC_FLOW_NAMESPACES` | `App\` | Comma-separated FQN-prefix allow-list applied to `:Flow` entry classes during `import-flows`. Whitespace around list entries is stripped. Empty / unset falls back to the default. |

Only `:Flow` entries (HTTP routes, message handlers, event subscribers, CLI commands) are namespace-filtered. `:Message`, `:Event`, and `:HttpClient` nodes are always imported regardless of the originating namespace — so the PayPal `:HttpClient` lands in the graph even though its class lives under `Symfony\Component\HttpClient`.

Examples:

```ini
# Default — only keep flows whose entry class starts with App\
KLOC_FLOW_NAMESPACES=App\

# Keep multiple project namespaces
KLOC_FLOW_NAMESPACES=App\,Domain\Orders\,Acme\
```

Empty / unset falls back to `App\`. To keep flows from every namespace, list each desired prefix explicitly — there is no "match all" wildcard.

## Provider recipes

### OpenRouter (single key, both surfaces)

```ini
LLM_API_URL=https://openrouter.ai/api/v1
LLM_API_KEY=sk-or-v1-...
LLM_MODEL=minimax/minimax-m2.7

EMBEDDING_API_URL=https://openrouter.ai/api/v1
EMBEDDING_API_KEY=sk-or-v1-...
EMBEDDING_MODEL=qwen/qwen3-embedding-8b
EMBEDDING_DIMENSION=4096
```

### Google Gemini (native, both surfaces)

```ini
LLM_API_URL=https://generativelanguage.googleapis.com/v1beta/openai/
LLM_API_KEY=<gemini-key>
LLM_MODEL=gemini-3-flash-preview

EMBEDDING_API_URL=https://generativelanguage.googleapis.com/v1beta/openai/
EMBEDDING_API_KEY=<gemini-key>
EMBEDDING_MODEL=gemini-embedding-001
EMBEDDING_DIMENSION=3072
```

Gemini's OpenAI-compat embeddings endpoint omits the `usage` field that Haystack expects. kloc-intelligence ships a compat shim (`src/ai/_haystack_compat.py`) that handles this, so native Gemini works end-to-end.

### Mixed: Gemini for chat, OpenRouter for embeddings

```ini
LLM_API_URL=https://generativelanguage.googleapis.com/v1beta/openai/
LLM_API_KEY=<gemini-key>
LLM_MODEL=gemini-3-flash-preview

EMBEDDING_API_URL=https://openrouter.ai/api/v1
EMBEDDING_API_KEY=<openrouter-key>
EMBEDDING_MODEL=google/gemini-embedding-001
EMBEDDING_DIMENSION=3072
```

Useful when your Gemini quota is generous on chat but you'd rather not spend it on embeddings, or to mix-and-match the best model per operation.

### OpenAI

```ini
LLM_API_URL=https://api.openai.com/v1
LLM_API_KEY=sk-...
LLM_MODEL=gpt-4o-mini

EMBEDDING_API_URL=https://api.openai.com/v1
EMBEDDING_API_KEY=sk-...
EMBEDDING_MODEL=text-embedding-3-large
EMBEDDING_DIMENSION=3072
```

## Changing the embedding dimension

The dimension is fixed at Qdrant collection creation time. If you change `EMBEDDING_MODEL` to one with a different vector size, you must drop and re-create the collections:

```bash
uv run python -c "
from qdrant_client import QdrantClient
c = QdrantClient(url='http://localhost:6333')
for n in ('code_embeddings','explain_embeddings','flow_explain_embeddings'):
    try: c.delete_collection(n)
    except: pass
"
uv run kloc-intelligence enrich            # re-run enrichment with the new dimension
uv run kloc-intelligence enrich-flows
```

## Connection verification

```bash
docker compose ps                          # Neo4j + Qdrant containers should be healthy
uv run kloc-intelligence schema verify     # confirms Neo4j is reachable + schema is in place
```

If Neo4j is unreachable: check `NEO4J_URI` and that the Docker container is running. If Qdrant is unreachable, the AI commands will tell you; the structural commands work fine without it.
