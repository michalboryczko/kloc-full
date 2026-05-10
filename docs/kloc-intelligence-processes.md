# kloc-intelligence — how the processes work

This document walks through each of the major processes inside `kloc-intelligence` at high abstraction. The goal is for someone integrating, debugging, or operating the service to understand the data flow without reading the source. For *what each capability does* see [kloc-intelligence-overview.md](kloc-intelligence-overview.md). For day-to-day commands see [usage/kloc-intelligence.md](usage/kloc-intelligence.md).

Six processes are documented here:

1. **Indexing pipeline** — PHP source → Neo4j graph
2. **Flow ingestion** — Symfony DI container → `:Flow` nodes
3. **Enrichment** — graph → LLM explanations + Qdrant embeddings
4. **Query path: `context`** — the workhorse query for code-archaeology
5. **Query path: `search`** — semantic search
6. **MCP server lifecycle** — JSON-RPC stdio for agent integration

---

## 1. Indexing pipeline (PHP → Neo4j)

### Data flow

```
┌─────────────────────────┐
│   PHP source files      │
│   (src/, vendor/, ...)  │
└────────────┬────────────┘
             │
             ▼
┌─────────────────────────┐    Static analysis: parse PHP CST,
│   kloc-indexer-php      │    extract symbols and relationships.
│   (Rust + tree-sitter)  │    Output: SCIP-ish JSON.
└────────────┬────────────┘
             │
             ▼
       index.json + calls.json
             │
             ▼
┌─────────────────────────┐    Normalize cross-references,
│   kloc-mapper           │    build canonical FQNs, split
│   (Python)              │    Argument vs Value nodes.
└────────────┬────────────┘
             │
             ▼
        sot.json (Source of Truth)
             │
             ▼
┌─────────────────────────┐    Parse SoT, validate against
│   kloc-intelligence     │    schema, write to Neo4j as
│   import sot.json       │    :Node + edges (UNWIND batches).
└────────────┬────────────┘
             │
             ▼
       :Node graph in Neo4j
       (~13 NodeKinds × 13 EdgeTypes)
```

### Step-by-step

1. **Parse and validate the SoT.** `parse_sot()` reads the JSON, asserts the schema (top-level `nodes[]` and `edges[]` arrays, type-tagged entries), and converts each entry into a typed dataclass (`NodeRecord` / `EdgeRecord`). Bad records fail fast with a line-pointer.

2. **Optional `--clear`.** If passed, runs `MATCH (n) DETACH DELETE n` to wipe the database. This is the default behavior because import is the canonical "set the graph state" operation; partial deltas are not supported (run a full re-import instead).

3. **Ensure schema.** Creates uniqueness constraints on `(:Node {node_id})` and `(:Node {fqn})` plus secondary indexes on `kind` and `name`. Idempotent — safe to call repeatedly.

4. **Batch-write nodes.** `UNWIND $batch AS props MERGE (n:Node {node_id: props.node_id}) SET n += props` in 1000-record batches. The MERGE makes the import idempotent: re-importing a node with the same `node_id` updates its properties rather than duplicating.

5. **Batch-write edges.** Each EdgeType has its own UNWIND batch with the appropriate Cypher `MATCH ... MATCH ... MERGE` pattern. Edges with unresolvable endpoints (target node never imported) are logged and skipped — usually a sign of vendor code referenced from app code, where the vendor wasn't included in the SoT.

6. **Validate (optional).** `--validate` (default true) runs a post-import counts check: every NodeKind in the SoT should have ≥1 instance in the graph, and edge counts should match the SoT. Mismatches are logged as warnings, not errors — they usually indicate a `kloc-mapper` bug worth filing, but don't necessarily mean the import is broken.

### Performance characteristics

- **Reference project** (~50 nodes, ~100 edges): <1 second.
- **Mid-sized real app** (~10K nodes, ~30K edges): 2–4 seconds.
- **Internal mono-repo** (~700K nodes, ~1.6M edges, 552 MB sot.json): 12 seconds end-to-end. Bottleneck is Neo4j write throughput, not parsing.

The Cypher `UNWIND ... MERGE` pattern is how this stays fast. Per-record `CREATE` queries would be 10–100× slower at this scale.

### Error modes

- **`Schema mismatch: unknown NodeKind 'Foo'`** → SoT contains a NodeKind newer than this version of `kloc-intelligence` understands. Fix: update kloc-intelligence; importer is forward-compatible only at the cost of dropping unknown kinds, which is usually wrong.
- **`Edge skipped: target node not found`** (warning) → benign for vendor cross-references, problematic if it accumulates. Run `kloc-mapper --include-vendor` if you want them resolvable.
- **Neo4j out of memory** → bump `NEO4J_HEAP_MAX` and `NEO4J_PAGECACHE` in `docker-compose.yml` or your env. The default 2 GB heap handles up to ~1M nodes; bigger graphs need 4–8 GB.

### Reproducibility

The same `sot.json` produces the same graph byte-for-byte (modulo Neo4j-internal IDs, which we don't depend on). `node_id` is a hash of the FQN + kind + file + line, so it's stable across machines as long as the source files are identical. Good enough for caching and CI use.

---

## 2. Flow ingestion (Symfony app → `:Flow` nodes)

### Data flow

```
┌──────────────────────────────────────────┐
│  Symfony DI container (compiled)         │
│  + Routing definitions                   │
│  + Messenger handler registrations       │
│  + Event subscriber tags                 │
│  + Console command registrations         │
└────────────────┬─────────────────────────┘
                 │
                 ▼
┌──────────────────────────────────────────┐
│  kloc-symfony (PHP, runs Symfony kernel) │
│  Reads container XML dump + Router state.│
│  Cross-references against sot.json for   │
│  FQN → node_id mapping.                  │
└────────────────┬─────────────────────────┘
                 │
                 ▼
            symfony-kloc.json
            ├─ flows[]: HTTP / message / event / cli
            └─ triggers[]: dispatched_class + sources + targets
                 │
                 ▼
┌──────────────────────────────────────────┐
│  kloc-intelligence import-flows          │
│  ├─ parse_flows(): drop non-App, derive  │
│  │  per-type display names               │
│  ├─ clear_flows(): MATCH (f:Flow) DELETE │
│  ├─ import_flow_nodes(): UNWIND MERGE    │
│  ├─ import_flow_edges():                 │
│  │   FLOW_ENTRY  (Flow → Method node)    │
│  │   FLOW_TRIGGERS (Flow → Flow per pair)│
│  └─ Side effect: Qdrant cleanup —        │
│     drops 3 stale flow_*_embeddings      │
│     collections from the legacy design   │
└────────────────┬─────────────────────────┘
                 │
                 ▼
   :Flow nodes + FLOW_ENTRY + FLOW_TRIGGERS in Neo4j
   (joined to the existing :Node graph via FLOW_ENTRY)
```

### Step-by-step

1. **App-namespace filter.** Only flows where the entry FQN starts with `App\` are imported. Vendor and framework code don't get `:Flow` representations — they're noise for this view of the codebase. Filtered-out flows are logged at WARNING level.

2. **Name derivation.** Each flow type gets a human-readable `name` per the spec D3a rules:
   - HTTP: `"GET /api/orders/{id}"` from `http_methods` + `route`
   - Message: short class name from `App\Message\OrderCreatedMessage` → `"OrderCreatedMessage"`
   - Event: same convention as message
   - CLI: `command_name` (e.g. `"app:process-orders"`)

3. **Idempotent clear-then-write.** Per spec D5, `import-flows` always clears existing flows before importing. There is no `--no-clear` flag — flows are deterministic from the SoT, partial updates are not supported.

4. **FLOW_ENTRY edge.** For each flow with a resolvable `entry.method_node_id`, creates a single edge `(:Flow)-[:FLOW_ENTRY]->(:Node {node_id: $method_node_id})`. If the target node doesn't exist (e.g. you imported flows before importing the SoT), the edge is skipped and a warning is logged. This is a safe degradation: the flow node still exists, just without a back-link to its method.

5. **FLOW_TRIGGERS cartesian.** For each `trigger` entry in `symfony-kloc.json`, the `sources[]` × `targets[]` cartesian product becomes one edge per pair (per spec D2). So a single message dispatched from one place to one handler produces one edge; dispatched from three controllers to two handlers, six edges. Edge properties carry the `trigger_type` (event / message) and `via` (the dispatched class FQN).

6. **Qdrant cleanup.** Three legacy collections from the v1 flow design (`flow_business_embeddings`, `flow_technical_embeddings`, `flow_search_embeddings`) are unconditionally deleted. They were retained from the prior design and are never repopulated under the new model — the deletion is non-destructive (they're empty if you're starting fresh) and idempotent.

### Performance

For the kloc-reference-project-php (9 flows, 3 triggers): **~0.4 seconds end-to-end**, well under the 2-second performance ceiling specified in D12.

### What this *doesn't* do

The flow model deliberately stops at "entry + triggers". It does **not** import the deep call tree from the entry method into the rest of the graph as additional `:Flow` step edges. The legacy `FLOW_STEP` edge was removed by design — the rationale is that:

1. The call tree from each entry method is already in the `:Node` graph (USES edges from `kloc-mapper`'s analysis). Re-encoding it as `:Flow` step edges is duplicate state.
2. Agents that want the deep call tree should use `kloc_context` on the entry method's node — that's the well-tested path that handles polymorphism, type info, and depth limits correctly.

This is why the flow model is so lean: it tells you *where the application's entry points are* and *how they trigger each other through async dispatch*. The structural graph already knows everything else.

### Common confusion

- **"My controller doesn't show up in flows."** Check that its FQN is in the `App\` namespace. Vendor controllers aren't imported.
- **"FLOW_ENTRY edge missing."** Either the SoT wasn't imported first (no target node to point to), or the entry method's `node_id` in `symfony-kloc.json` doesn't match what's in the SoT. The latter is a `kloc-symfony` bug — file an issue.
- **"FLOW_TRIGGERS count seems wrong."** Remember it's cartesian: 1 dispatch site × 1 handler = 1 edge, but 3 × 2 = 6 edges. The trigger *count* in `symfony-kloc.json` is dispatched-class count, not edge count.

---

## 3. Enrichment (graph → LLM explanations + Qdrant embeddings)

### Data flow per node

```
:Class or :Method node selected by kind filter
    │
    ▼
┌────────────────────────────────────────────┐
│  Has explanation already?                  │
│  └─ yes + not --force → SKIP               │
│  └─ no, or --force → continue              │
└────────────┬───────────────────────────────┘
             │
             ▼
┌────────────────────────────────────────────┐
│  SourceReader.read_node_source(node)       │
│  reads file at start_line..end_line.       │
│  Empty result → skip + log.                │
└────────────┬───────────────────────────────┘
             │
             ▼
┌────────────────────────────────────────────┐
│  Gather context (kind-dependent):          │
│  Method → arg/return type bodies (one hop) │
│  Class  → parent extends/implements bodies │
│         + first-level dependent code       │
└────────────┬───────────────────────────────┘
             │
             ▼
┌────────────────────────────────────────────┐
│  LLM call (build_explain_pipeline):        │
│  ChatPromptBuilder fills Jinja2 template,  │
│  OpenAIChatGenerator hits LLM_API_URL.     │
└────────────┬───────────────────────────────┘
             │
             ▼
       Explanation text (2-5 sentences)
             │
             ├─────► Neo4j: SET n.explanation,
             │       n.explain_model, n.explain_at
             │
             ▼
┌────────────────────────────────────────────┐
│  CodeChunker.chunk_node(node, src):        │
│  Method → 1 chunk                          │
│  Class  → split by method boundary +       │
│           shared class-context preamble    │
└────────────┬───────────────────────────────┘
             │
             ▼
┌────────────────────────────────────────────┐
│  build_embed_pipeline x2:                  │
│  - 'code_embeddings' for source chunks     │
│  - 'explain_embeddings' for explanation    │
│  Both call EMBEDDING_API_URL.              │
└────────────┬───────────────────────────────┘
             │
             ▼
   Qdrant collections updated
   (one point per node + chunk_index)
```

### Step-by-step

1. **Selection.** `enrich_all()` queries Neo4j for every node matching `--kinds` (default `Class,Method`). Without `--force`, nodes that already have `n.explanation` are filtered out at the query level — so re-running on a fully enriched graph is essentially a no-op cost-wise (one Neo4j read, no LLM calls).

2. **Source read.** `SourceReader.read_node_source(node)` opens the file at `node.file` and slices the byte range corresponding to `node.start_line` to `node.end_line` (1-based, inclusive). If `KLOC_PROJECT_ROOT` isn't set or the file doesn't exist, the node is skipped with a warning — the rest of the batch continues.

3. **Context gathering** (kind-dependent):
   - For a **Method**, fetches the bodies of every type referenced in its signature (parameter types, return type) up to one hop. This gives the LLM the data shapes the method works with.
   - For a **Class**, fetches: (a) every class/interface it extends or implements (parent context), (b) the bodies of the first-level callers — classes/methods that USE this class, capped at a small number. Provides upstream + downstream context.

   This kind-aware context selection is why explanations are markedly better than just feeding raw source — the LLM sees not just *what the code is* but *how it fits in*.

4. **LLM call.** A Haystack pipeline (`build_explain_pipeline`) chains `ChatPromptBuilder` → `OpenAIChatGenerator`. The system prompt asks for a 2–5 sentence functional description; the user prompt is a Jinja2 template that interleaves the code with the gathered context. The provider is whatever `LLM_API_URL` + `LLM_API_KEY` + `LLM_MODEL` point at — OpenRouter by default, swappable to Gemini, OpenAI, or any OpenAI-compat endpoint.

5. **Persist explanation.** Single Neo4j write: `MATCH (n:Node {node_id: $nid}) SET n.explanation = $text, n.explain_model = $model, n.explain_at = datetime()`. Idempotent — re-running with `--force` overwrites prior explanations cleanly.

6. **Chunking.** `CodeChunker.chunk_node(node, src)` produces 1+ chunks:
   - Methods are always one chunk regardless of length (typically <8K tokens; oversized methods go through anyway and rely on the LLM/embedding model's context window).
   - Classes split at method boundaries, with the **class header** (declaration + use statements + property declarations) prepended to each chunk so each one carries enough context to be useful in isolation.

7. **Embed code chunks.** `build_embed_pipeline(config, "code_embeddings")` chains `OpenAIDocumentEmbedder` → Qdrant `DocumentWriter`. Each chunk produces one vector with metadata: `node_id`, `kind`, `fqn`, `name`, `file`, `chunk_index`, `total_chunks`, `project`. Multi-chunk classes contribute multiple points, all keyed by `(node_id, chunk_index)`.

8. **Embed explanation.** Same pipeline pointed at `explain_embeddings`, embedding the LLM-authored explanation text rather than the source. One point per node (explanations are short enough to fit in one chunk).

### Performance and cost

- **Per-node latency**: 2–5 seconds (LLM is the bottleneck; embedding is <500 ms).
- **Throughput**: limited to the LLM's rate limit. OpenRouter free tier ≈ 1 req/s per model; paid tiers higher.
- **Cost per node**: dependent on model + token count. For minimax-m2.7 + qwen3-embedding-8b on a typical method (~1K tokens prompt, ~150 tokens response, ~1K tokens embedding input): ~$0.0005. A 5000-method codebase costs ~$2.50. Larger / pricier models scale linearly.

### Failure handling

- **LLM timeout / 429** → the failing node is recorded in `progress.failed_nodes`, the batch continues. Re-run with `--force` and the same kinds filter to retry just the failures (or filter manually by `n.explanation IS NULL`).
- **Embedding API error** → same: skip + log + continue.
- **Source not readable** → skip + log; rare in practice (usually means `KLOC_PROJECT_ROOT` is misconfigured).

The orchestrator never aborts the whole run on a single-node failure — too expensive after thousands of successful calls.

### Idempotency

Without `--force`: skips any node where `n.explanation` is non-null. Re-running is safe and cheap.

With `--force`: re-runs every node, overwriting the prior explanation and re-writing all embeddings. Use this when changing the LLM model or the prompt — but expect a full cost re-spend.

---

## 4. Query path: `context` (the workhorse query)

### Data flow

```
User invocation
    kloc-intelligence context "OrderService::createOrder()"
    │
    ▼
┌────────────────────────────────────────────┐
│  resolve_symbol(query)                     │
│  cascade: exact FQN → CI FQN → suffix →    │
│  short name → no-parens → CONTAINS         │
│  Returns 1+ candidate :Node                │
└────────────┬───────────────────────────────┘
             │
             ▼  (take first candidate; if N>1 prompt user via --json or pick)
┌────────────────────────────────────────────┐
│  context_method (or context_class, ...)    │
│  Cypher: MATCH neighbors at depth N        │
│  for both incoming USES and outgoing USES  │
│  + parent type info (TYPE_OF, ARGUMENT,    │
│  RECEIVER, ASSIGNED_FROM, PRODUCES)        │
│  + owners chain (CONTAINS upward)          │
│  + (optional) implementations / overrides  │
└────────────┬───────────────────────────────┘
             │
             ▼
┌────────────────────────────────────────────┐
│  Assemble bidirectional ContextResult:     │
│  - node: the resolved subject              │
│  - usedBy[]: depth-N callers               │
│  - uses[]: depth-N callees                 │
│  - types[]: type-relation neighbors        │
│  - owners[]: containment chain to file     │
│  - implementations[] (if include_impl)     │
└────────────┬───────────────────────────────┘
             │
             ▼
┌────────────────────────────────────────────┐
│  Render: Rich tree (default)               │
│        | JSON (for tools, --json)          │
│        | flat list (for grep, --flat)      │
└────────────────────────────────────────────┘
```

### Why it's three Cypher rounds, not one

Looks like overkill but isn't. The reason: each "direction" has its own performance and depth profile.

- **Caller traversal** (incoming USES) at depth N can blow up on hub nodes — a base utility class might be used by thousands of others. We cap with `LIMIT` early in the Cypher to avoid hauling all of them back. Different cap for different node kinds.
- **Callee traversal** (outgoing USES) is usually small (a method calls a few dozen things at most), so we can be more permissive.
- **Type info** is a separate set of edges (TYPE_OF, ARGUMENT, RECEIVER, ASSIGNED_FROM, PRODUCES) and intersects USES only partially. Mixing it into the USES query produces complex paths and hard-to-debug results.

Three focused queries are easier to reason about, easier to optimize independently, and produce cleaner intermediate results for the assembly step.

### Output rendering

Default rendering is a **Rich tree** (`rich.tree.Tree`) with the subject node at the root, children grouped by direction (`usedBy`, `uses`, `types`, etc.), and color-coding by kind. Optimized for terminal reading.

`--json` swaps the renderer for `print(json.dumps(...))` — bypassing Rich entirely to avoid its line-wrapping artifacts on long FQNs (a known Rich JSON bug). Tools should always use `--json`; humans should use the default tree.

`--flat` produces a one-line-per-node grep-friendly format with no indentation. Useful for piping into `awk` / `cut` / `grep`.

### Performance

- **Reference project** (~50 nodes): <50 ms total for depth-1 context.
- **Mid-sized real app** at depth 2: 100–300 ms.
- **Hub-node case** (e.g. `LoggerInterface` at depth 1): 500 ms with default `--limit 50`. The limit is the safety valve — uncap it and a hub query can return tens of thousands of rows and take many seconds.

### When to widen the scope

- `--depth 2` or `3` is justifiable for "explain this method's neighborhood" use cases. Most useful patterns top out at depth 2.
- `--include-impl` is needed when looking at an interface or abstract class — without it the context shows only direct USES, not the polymorphic implementations. Adds a separate Cypher call so factor it into latency.
- `--limit` higher than 100 is rarely useful for human reading; agents that *need* the full neighborhood should use `usages`/`deps` directly with explicit pagination.

---

## 5. Query path: `search` (semantic)

### Data flow

```
User invocation
    kloc-intelligence search "validate order before checkout"
    │
    ▼
┌────────────────────────────────────────────┐
│  build_search_pipeline(config, collection) │
│  Constructs Haystack pipeline:             │
│    OpenAITextEmbedder                      │
│      → QdrantEmbeddingRetriever            │
│  Pointed at EMBEDDING_API_URL.             │
└────────────┬───────────────────────────────┘
             │
             ▼
┌────────────────────────────────────────────┐
│  Embed query: single text → single vector  │
│  Dimension must match collection dim (set  │
│  at enrich time). Mismatch = error.        │
└────────────┬───────────────────────────────┘
             │
             ▼
┌────────────────────────────────────────────┐
│  Qdrant cosine similarity:                 │
│    code_embeddings (top_k results)         │
│    explain_embeddings (top_k results)      │
│  Each hit returns: score + metadata        │
│  (node_id, kind, fqn, file, chunk_index)   │
└────────────┬───────────────────────────────┘
             │
             ▼
┌────────────────────────────────────────────┐
│  Merge + dedupe by node_id                 │
│  (keep highest-scoring duplicate)          │
│  Sort by score descending                  │
│  Return top `limit` results                │
└────────────┬───────────────────────────────┘
             │
             ▼
   Render: Rich table (default) | JSON (--json)
```

### Step-by-step

1. **Pipeline construction.** Two pipelines are built — one per collection. Each has an `OpenAITextEmbedder` (calls the embedding API for the single query text) chained to a `QdrantEmbeddingRetriever` (runs the cosine search against one collection). Building takes <100 ms; running the embed + retrieve takes ~200–500 ms.

2. **Query embedding.** The query text is sent to the embedding endpoint exactly as typed (no preprocessing). The endpoint must be configured with the same model that produced the collection's vectors — mismatched models produce useless results (different vector spaces).

3. **Per-collection retrieval.** Each collection independently returns its top-K hits (`top_k=limit`). Results have a similarity score in `[0, 1]` (higher is more similar) plus the node metadata captured at enrich time.

4. **Merge + dedupe.** A node may appear in both collections (its source matched well *and* its explanation matched well). Merging by `node_id` and keeping the highest score is the right behavior — it removes duplicates while preserving the strongest signal.

5. **Cap to `limit`.** After merging, sort by score descending and return the top `limit` (default 10).

6. **Render.** Rich table for terminal display (columns: rank, score, kind, FQN, file). `--json` swaps to plain JSON for tools.

### Why two collections, not one

The two collections capture different signal:

- `code_embeddings` excels at **structural / textual** matches. "find methods that build SQL queries" → high-score hits on methods that contain `SELECT`, `INSERT`, `WHERE`, `query()`, etc.
- `explain_embeddings` excels at **semantic / behavioral** matches. "find code that handles failed payments" → high-score hits on methods whose LLM-authored explanations describe error handling for payment flows, even when the source code doesn't contain the words "failed" or "payment".

Querying both and merging gives you both kinds of recall. The cost: 2× embedding + 2× retrieval per query. For latency-critical applications you can pick `--collection code` or `--collection explain` based on the query style.

### When it returns nothing

If the query produces no hits at all, possibilities are:

- **Collection is empty.** Run `enrich` first. `kloc-intelligence enrich-status` shows the count.
- **Query is far from the corpus.** Embedding similarity is bounded — a query about a topic the codebase genuinely doesn't address will produce only low-score hits, and we don't filter by absolute score (we always return the top `limit`). A "no results" UX is a deliberate design choice not to do here; the caller gets the top-K regardless of how poor they are. Filtering by score is a frontend concern.
- **Embedding model mismatch.** If you re-enriched with a different `EMBEDDING_MODEL`, collections may have inconsistent vectors. Drop and re-enrich.

### Performance

- **Single query**: 200–500 ms (mostly embedding API latency).
- **Throughput**: limited by the embedding API. Search is read-only against Qdrant, which is fast (<50 ms per retrieval at this scale).

---

## 6. MCP server lifecycle

### Data flow

```
Process startup:
    kloc-intelligence mcp-server [--database NAME | --config path/to/config.json]
    │
    ▼
┌────────────────────────────────────────────┐
│  MCPServer.__init__:                       │
│  - Load projects map (single or multi)     │
│  - Don't connect to Neo4j yet (lazy)       │
└────────────┬───────────────────────────────┘
             │
             ▼
┌────────────────────────────────────────────┐
│  for line in sys.stdin:                    │
│    request = json.loads(line)              │
│    method = request["method"]              │
│    dispatch:                               │
│      "initialize" → server info            │
│      "tools/list" → list[tool]             │
│      "tools/call" → call_tool(name, args)  │
│      "shutdown"   → break                  │
│    response = {"jsonrpc": "2.0", ...}      │
│    print(json.dumps(response), flush=True) │
└────────────┬───────────────────────────────┘
             │
             ▼
   On tools/call:
   ┌──────────────────────────────────────┐
   │ resolve project → get_runner(proj)   │
   │ (lazy: connect on first use)         │
   ├──────────────────────────────────────┤
   │ dispatch handler by tool name        │
   │ (kloc_resolve, kloc_context, ...)    │
   ├──────────────────────────────────────┤
   │ handler: build query, run Cypher,    │
   │ format result as dict                │
   ├──────────────────────────────────────┤
   │ return JSON-RPC result envelope      │
   └──────────────────────────────────────┘

Process shutdown (SIGTERM/SIGINT):
    server.close() → close all Neo4j connections
    sys.exit(0)
```

### Step-by-step

1. **Process startup.** The CLI's `mcp-server` subcommand runs `run_mcp_server(database, config_path)`. No connections are made yet — the project map is built (from `--config` JSON or single `--database` arg), and that's it.

2. **JSON-RPC loop.** stdin is read line-by-line; each line should be a JSON-RPC 2.0 request. Methods supported: `initialize`, `tools/list`, `tools/call`, `shutdown`, plus `notifications/initialized` (no-op ack). Requests with unknown methods return a JSON-RPC error.

3. **`tools/list`** returns the static catalog of 14 tools (built once at server-init time, not per-request). Each tool entry has a name, a one-paragraph description, and a JSONSchema for inputs. Agents use this to know which tools are available and how to call them.

4. **`tools/call`** is the workhorse:
   - Looks up the handler by tool name in the dispatch table (`_handle_resolve`, `_handle_context`, etc.). Unknown names return an error.
   - Resolves the project (single-project mode picks the only entry; multi-project mode requires the `project` arg).
   - Lazily creates the Neo4j connection + `QueryRunner` for that project on first call. Connections are cached and reused for subsequent calls within the session.
   - Calls the handler with the parsed args. Handlers raise `ValueError` for input errors (returned as `error.message` in the JSON-RPC envelope) or return a plain dict (returned as `result`).

5. **Lazy connections.** A server in multi-project mode might have 5 projects configured but only ever query one in a given session — connecting to all 5 at startup is wasted work. Connections are created only when first needed and persist for the rest of the process lifetime.

6. **Shutdown.** SIGINT / SIGTERM trigger `server.close()` which closes all open Neo4j connections, then `sys.exit(0)`. The JSON-RPC `shutdown` method does the same. Half-closed connections from sudden process kills are tolerated (Neo4j's driver cleans up server-side after a timeout).

### Multi-project disambiguation

The `--config` mode accepts a JSON like:

```json
{
  "projects": {
    "my-app": "my_app_db",
    "payments": "payments_db"
  }
}
```

Each tool call must include `"project": "my-app"` to disambiguate. With one project configured, `project` is optional in tool args (the server picks the only entry). With multiple, it's required — calls without it return a `ValueError` listing the available projects.

### Error envelope

All errors are returned as JSON-RPC 2.0 error objects:

```json
{"jsonrpc": "2.0", "id": <req-id>, "error": {"code": -32000, "message": "Symbol not found: Foo"}}
```

The error code is always `-32000` (server-defined). The message is the exception text. Agents that need finer-grained error classification should check the message text — this is acceptable because the error vocabulary is small (resolve failures, missing-project errors, validation errors).

### Why stdio, not HTTP

stdio is the MCP protocol's standard transport. It's:

- **In-process or local-pipe only.** No exposure to the network. Auth is whatever your shell process has access to.
- **Trivial to integrate.** Spawning the server as a child process and piping JSON-RPC over stdin/stdout is one syscall.
- **Correctly buffered.** `print(..., flush=True)` ensures every response lands on the wire immediately; no half-buffered partial JSON.

If you need network access, wrap the server in a transport layer (gRPC, WebSocket, ssh tunnel). That's intentionally out of scope for the core service.

### Performance

- **Startup**: <500 ms (importing Python deps is the bottleneck; no actual work).
- **Per-call latency**: dominated by the underlying Cypher query (5–500 ms) plus tiny JSON-RPC envelope overhead (<1 ms).
- **Concurrent calls**: the server serializes calls through the stdin loop. For higher concurrency, run multiple server processes and route between them at the agent layer.

### Operational notes

- **Logs go to stderr** (not stdout — stdout is the JSON-RPC channel). Set `KLOC_DEBUG=1` for verbose logs.
- **Crashes are visible** to the client (stdout closes, JSON-RPC fails with no response). Restart the server process and reissue the call.
- **One process per session** is the recommended model. Spinning up a new server per call works but adds the 500 ms startup latency to every request.
