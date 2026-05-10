# kloc-intelligence — Data setup

How to load and enrich the graph in order. Each step is independent and can be re-run idempotently.

For provider/env-var setup see [configuration.md](configuration.md). For the full CLI surface see [cli.md](cli.md).

## Pipeline order

```
sot.json                  ─→  schema ensure → import           (mandatory)
symfony-kloc.json (opt)   ─→  import-flows                     (Symfony projects only)
existing graph            ─→  enrich                           (LLM explanations + embeddings)
existing graph + flows    ─→  enrich-flows                     (flow business-process summaries)
```

Mandatory: `schema ensure` + `import`. Everything else is additive.

## Step 1 — Schema

```bash
uv run kloc-intelligence schema ensure        # idempotent: constraints + indexes
uv run kloc-intelligence schema verify        # show current state
uv run kloc-intelligence schema reset         # drop ALL data and recreate (destructive)
```

`schema ensure` is run once per environment. `schema reset` is destructive — only useful when you want a clean baseline before a fresh import.

## Step 2 — Import sot.json

```bash
uv run kloc-intelligence import /path/to/sot.json
uv run kloc-intelligence import /path/to/sot.json --no-clear      # keep existing data
uv run kloc-intelligence import /path/to/sot.json --no-validate   # skip post-import counts check
```

Loads the SoT into Neo4j as `:Node` + `:Class` / `:Method` / etc. labels with the 13 edge types (USES, CONTAINS, EXTENDS, …). `--clear` (default true) wipes everything first; pass `--no-clear` if you really want to layer onto an existing graph (rare — usually you want a full re-import).

Performance: ~1 second per 100K nodes on a default Neo4j heap. For graphs >500K nodes bump `NEO4J_HEAP_MAX` in `docker-compose.yml`.

## Step 3 — Import flows (Symfony projects only)

```bash
uv run kloc-intelligence import-flows /path/to/.kloc/symfony-kloc.json
```

Adds `:Flow` nodes (one per HTTP route, message handler, event subscriber, CLI command) joined to the structural graph via `FLOW_ENTRY` (Flow → entry Method) and `FLOW_TRIGGERS` (Flow → Flow, when one dispatches a message/event another handles).

Behaviors:
- Re-imports always clear existing `:Flow` nodes first. There is no `--no-clear` because flows are deterministic from `symfony-kloc.json`.
- App-namespace filter: only flows with `App\` entry FQNs are imported. Vendor/framework flows are dropped.
- Each call also deletes three legacy Qdrant collections (`flow_business_embeddings`, `flow_technical_embeddings`, `flow_search_embeddings`) — leftovers from a defunct design, idempotent on fresh installs.

After flow import, run `kloc-intelligence flows` to confirm the count.

## Step 4 — Enrich nodes (optional, AI)

```bash
uv run kloc-intelligence enrich                          # all Class + Method nodes
uv run kloc-intelligence enrich --kinds Class            # restrict to classes
uv run kloc-intelligence enrich --force                  # re-enrich already enriched
uv run kloc-intelligence enrich --batch-size 5           # smaller batches if rate-limited
uv run kloc-intelligence enrich --debug                  # verbose logging
```

For each Class/Method node, walks one hop of context (parent classes, argument types, first-level usages), calls the LLM for a 2–5 sentence functional description, writes it as `n.explanation`, then embeds both the source and the explanation into Qdrant (`code_embeddings` + `explain_embeddings`).

Idempotent without `--force`: already-enriched nodes are skipped at query time.

Cost dimension: 1 LLM call + 2 embedding calls per node. The reference project (165 enrichable nodes) costs single-digit cents at OpenRouter list prices. A 5000-method codebase ≈ $1–$5 depending on model.

```bash
uv run kloc-intelligence enrich-status                   # see how many are done
```

## Step 5 — Enrich flows (optional, Symfony only, AI)

```bash
uv run kloc-intelligence enrich-flows
uv run kloc-intelligence enrich-flows --force            # regenerate existing
```

For each `:Flow`, walks depth-3 bidirectional context (with implementations) from the entry method, attaches source from the referenced nodes (callers, callees, types, impls), and asks the LLM for a structured 3-part summary:

```
<Surface line — "API endpoint on path X method Y" / "Message handler for Z" / etc.>

<2-3 sentence behavior in business vocabulary>

<Optional: downstream effects line>
```

Stored as `f.explanation` on the Flow and embedded into `flow_explain_embeddings`. The third collection joins `code_embeddings` and `explain_embeddings` in `search` results — business-process queries like "process customer orders" or "send notification when order placed" return flows as top matches.

## Step 6 — Source code access

Required for `source`, `chunks`, and the embedding side of `enrich` / `enrich-flows`.

```bash
export KLOC_PROJECT_ROOT=/path/to/php-project
# OR pass per-invocation:
uv run kloc-intelligence source "OrderService::createOrder" --project-root /path/to/project
```

The graph stores file paths relative to `KLOC_PROJECT_ROOT`; you point that var at the actual checkout. If it's unset and you run a command that needs source, you'll get a clear error.

## Common recipes

### Index a fresh project end-to-end

```bash
# Assuming sot.json + symfony-kloc.json are already produced
export KLOC_PROJECT_ROOT=/path/to/php-project

uv run kloc-intelligence schema reset
uv run kloc-intelligence import /path/to/sot.json
uv run kloc-intelligence import-flows /path/to/.kloc/symfony-kloc.json
uv run kloc-intelligence enrich
uv run kloc-intelligence enrich-flows
```

The reference project completes the whole sequence in ~15 minutes (the LLM time dominates).

### Reset everything (Neo4j + Qdrant) and re-do

```bash
uv run kloc-intelligence schema reset
uv run python -c "
from qdrant_client import QdrantClient
c = QdrantClient(url='http://localhost:6333')
for n in ('code_embeddings','explain_embeddings','flow_explain_embeddings'):
    try: c.delete_collection(n)
    except: pass
"
```

Then re-run the pipeline above.

### Verify a flows import

```bash
uv run kloc-intelligence flows                       # list all
uv run kloc-intelligence flows --type http           # filter by type
uv run kloc-intelligence flows OrderController       # candidates
uv run kloc-intelligence flows OrderController::create  # detail
```

### Inspect chunking before enriching

```bash
uv run kloc-intelligence chunks "OrderService::createOrder"           # human view
uv run kloc-intelligence chunks "OrderService::createOrder" --json    # JSON, same chunks the embedder uses
uv run kloc-intelligence chunks "OrderService" --max-tokens 4000      # tighter chunk budget
```

Methods are always one chunk. Classes split by method boundary with a shared class-context preamble per chunk.

## Resetting / re-running selectively

| Scenario | Command |
| --- | --- |
| Re-import the whole graph from scratch | `schema reset` then `import` |
| Regenerate one node's explanation | `explain <fqn> --force` |
| Regenerate all explanations + embeddings | `enrich --force` |
| Regenerate all flow summaries | `enrich-flows --force` |
| Drop just the AI data, keep the graph | manually delete Qdrant collections (`code_embeddings`, `explain_embeddings`, `flow_explain_embeddings`) |
| Switch embedding model with different dimension | drop Qdrant collections (see above) then re-run `enrich` + `enrich-flows` |

## Troubleshooting

- **Schema mismatch: unknown NodeKind 'Foo'** — your SoT has a newer NodeKind than this version of kloc-intelligence understands. Update kloc-intelligence.
- **Edge skipped: target node not found** (warning) — benign for vendor cross-references. Run `kloc-mapper --include-vendor` if you want them resolvable.
- **Neo4j OOM** — bump `NEO4J_HEAP_MAX` and `NEO4J_PAGECACHE`. 2 GB heap handles ~1M nodes; bigger graphs need 4–8 GB.
- **LLM 429 / timeout** — already-failed nodes are recorded but the batch continues. Re-run with `--force` to retry just the failures.
- **Embedding dimension mismatch** — drop collections (see Configuration → "Changing the embedding dimension") and re-run.
