# kloc-intelligence — CLI reference

Every command accepts `--json` for machine-readable output. For setup see [configuration.md](configuration.md). For the ingestion pipeline see [data-setup.md](data-setup.md). For MCP see [mcp.md](mcp.md).

## Schema management

| Command | What it does |
| --- | --- |
| `schema ensure` | Create constraints + indexes (idempotent). |
| `schema verify` | Show current state. |
| `schema reset` | Drop ALL data and recreate. **Destructive.** |

## Graph ingestion

### `import` — load `sot.json` into Neo4j

```bash
uv run kloc-intelligence import /path/to/sot.json
uv run kloc-intelligence import /path/to/sot.json --no-clear
uv run kloc-intelligence import /path/to/sot.json --no-validate
```

Loads the SoT into Neo4j as `:Node` + per-kind labels with the 13 edge types (USES, CONTAINS, EXTENDS, …). `--clear` (default true) wipes existing data first.

### `import-flows` — load Symfony flows (v3.0)

```bash
uv run kloc-intelligence import-flows /path/to/.kloc/symfony-kloc.json
```

Imports the v3.0 `symfony-kloc.json` shape:
- `:Flow`        — one per HTTP route, message handler, event subscriber, CLI command
- `:Message`     — one per dispatched message class
- `:Event`       — one per dispatched event class
- `:HttpClient`  — one per outbound HTTP integration

Edges written: `FLOW_ENTRY` (Flow→Method), `FLOW_ENTRY_CLASS` (Flow→Class), `EMITS` (Flow/Call→Message/Event), `USES_HTTP_CLIENT` (Flow/Call→HttpClient), `HANDLED_BY` (Message/Event→Flow), `OF_TYPE` (entity→Class).

**Idempotent**: re-running on the same JSON preserves enrichment props on `:Flow` (`explanation`, `explain_model`, `explain_at`). Orphan flows (in DB but not in JSON) have their Qdrant points in `flow_explain_embeddings` removed by `flow_id` filter — the collection itself is never dropped. The legacy v2 `FLOW_TRIGGERS` edge type is cleaned up on every run.

Requires `QDRANT_URL` (and optionally `QDRANT_API_KEY`) in the env for the Qdrant filter-delete; the Neo4j-only path runs fine without them.

### `flows` — list or inspect flows

```bash
uv run kloc-intelligence flows                                         # list all
uv run kloc-intelligence flows --type http                             # filter
uv run kloc-intelligence flows --type http,cli                         # multi-filter
uv run kloc-intelligence flows OrderController                         # candidates
uv run kloc-intelligence flows OrderController::create                 # detail
uv run kloc-intelligence flows --json                                  # machine-readable
```

The JSON response is a discriminated union with `mode`:
- `list` — array of flows (filtered by `--type` if provided)
- `detail` — full info for one flow including entry, **`dispatches_out`** (messages/events/http_clients emitted by the flow) and **`dispatches_in`** (messages/events whose handler is this flow), plus the LLM-authored business-process summary (after `enrich-flows`).
- `candidates` — partial match returned multiple flows

Example detail JSON shape:

```json
{
  "mode": "detail",
  "flow": {
    "flow_id": "flow:http:App\\Ui\\Rest\\Controller\\PaymentController::verify",
    "type": "http",
    "name": "POST /api/payments/{id}/verify",
    "entry": { "fqn": "...", "method_node_id": "...", "file": "...", "start_line": 14, "end_line": 32 },
    "dispatches_out": {
      "messages": [{ "id": "message:App\\...AuditLogMessage", "fqn": "...", "transports": [],
                     "caller_method_fqn": "App\\...PaymentController::verify()",
                     "call_node_id": "node:call:...", "target_flow_ids": [] }],
      "events": [],
      "http_clients": [{ "id": "http_client:paypal.client", "service_id": "paypal.client",
                         "base_uri": "https://api.paypal.com",
                         "class_fqn": "Symfony\\Component\\HttpClient\\UriTemplateHttpClient",
                         "caller_method_fqn": "App\\Service\\PaypalGateway::verify()",
                         "call_node_id": "node:call:..." }]
    },
    "dispatches_in": { "messages": [], "events": [] }
  }
}
```

### `messages` — list or inspect dispatched messages

```bash
uv run kloc-intelligence messages                                      # list all
uv run kloc-intelligence messages OrderCreated                         # candidates
uv run kloc-intelligence messages "message:App\\Ui\\Messenger\\Message\\OrderCreatedMessage"
uv run kloc-intelligence messages OrderCreated --json
```

JSON `detail` mode:

```json
{
  "mode": "detail",
  "message": {
    "id": "message:App\\Ui\\Messenger\\Message\\OrderCreatedMessage",
    "fqn": "App\\Ui\\Messenger\\Message\\OrderCreatedMessage",
    "transports": ["sync"],
    "sources": [{ "flow_id": "...", "caller_method_fqn": "...", "call_node_id": "...",
                  "caller_method_node_id": "..." }],
    "targets": [{ "flow_id": "flow:message:App\\...OrderCreatedHandler::__invoke" }],
    "of_type_class_fqn": "App\\Ui\\Messenger\\Message\\OrderCreatedMessage"
  }
}
```

### `events` — list or inspect dispatched events

```bash
uv run kloc-intelligence events                                        # list all
uv run kloc-intelligence events OrderCreatedEvent                      # detail
uv run kloc-intelligence events OrderCreated --json                    # candidates
```

JSON `detail` mode includes `targets[].priority` (Symfony subscriber priority).

### `http-clients` — list or inspect outbound HTTP integrations

```bash
uv run kloc-intelligence http-clients                                  # list all
uv run kloc-intelligence http-clients paypal                           # candidates
uv run kloc-intelligence http-clients "http_client:paypal.client"      # detail
uv run kloc-intelligence http-clients paypal --json
```

Vendor HTTP clients (e.g. `Symfony\Component\HttpClient\UriTemplateHttpClient`) carry `class_fqn` as a property; `of_type_class_fqn` is `null` because the vendor class is not in the SoT graph.

## Symbol resolution

### `resolve` — find where a symbol is defined

```bash
uv run kloc-intelligence resolve "App\Service\OrderService"
uv run kloc-intelligence resolve "OrderService"                # partial match
uv run kloc-intelligence resolve "OrderService::createOrder()"
```

Six-stage cascade: exact FQN → case-insensitive FQN → suffix → name → name-no-parens → contains. Returns matching `:Node`(s).

### `owners` — containment chain

```bash
uv run kloc-intelligence owners "App\Service\OrderService::createOrder()"
```

Walks upward via `:CONTAINS` edges: Method → Class → File. Answers "where does this live in the codebase?".

## Bidirectional traversal

### `usages` — who uses this?

```bash
uv run kloc-intelligence usages "OrderService::createOrder()" -d 2
```

Incoming `USES` edges to depth `-d`. Default depth 1.

### `deps` — what does this use?

```bash
uv run kloc-intelligence deps "OrderService::createOrder()" -d 2
```

Outgoing `USES` edges. Mirror of `usages`.

### `context` — both directions + types + owners

```bash
uv run kloc-intelligence context "OrderService::createOrder()" -d 3
uv run kloc-intelligence context "OrderRepositoryInterface" --include-impl
```

The richest single query. Returns callers, callees, type-relation neighbors, owners, and (with `--include-impl`) polymorphic implementations in one tree. Most-used command for agent-driven exploration.

## Inheritance and polymorphism

### `inherit` — extends/implements tree

```bash
uv run kloc-intelligence inherit "App\Repository\OrderRepositoryInterface"
uv run kloc-intelligence inherit "App\Service\AbstractOrderProcessor" --direction down
```

`--direction up` (default) returns ancestors; `down` returns descendants.

### `overrides` — concrete implementations of a method

```bash
uv run kloc-intelligence overrides "OrderRepositoryInterface::findById()"
uv run kloc-intelligence overrides "AbstractOrderProcessor::process()" --direction down
```

`up` walks the OVERRIDES edge to the method this one overrides; `down` returns every method that overrides this one.

## Source code access

Both commands need `KLOC_PROJECT_ROOT` (or `--project-root`).

### `source` — raw source for a node

```bash
uv run kloc-intelligence source "OrderService::createOrder()"
uv run kloc-intelligence source "OrderService" --json
```

Reads the file at the node's `file` + `start_line`..`end_line`. Returns content + line range + token estimate.

### `chunks` — token-bounded chunks (same as the embedder uses)

```bash
uv run kloc-intelligence chunks "OrderService"                       # default 8000 tokens
uv run kloc-intelligence chunks "OrderService" --max-tokens 4000     # tighter budget
uv run kloc-intelligence chunks "OrderService" --json
```

Methods are always one chunk. Classes split by method boundary with a shared class-context preamble per chunk.

## AI / enrichment (require `LLM_API_KEY` and/or `EMBEDDING_API_KEY`)

### `enrich` — node-level explanations + embeddings

```bash
uv run kloc-intelligence enrich
uv run kloc-intelligence enrich --kinds Class,Method
uv run kloc-intelligence enrich --force                    # re-enrich already enriched
uv run kloc-intelligence enrich --batch-size 5             # smaller batches if rate-limited
uv run kloc-intelligence enrich --debug                    # verbose
```

Writes LLM-authored explanations to `n.explanation` on Class/Method nodes; embeds source + explanation into Qdrant.

### `enrich-status` — progress check

```bash
uv run kloc-intelligence enrich-status
uv run kloc-intelligence enrich-status --json
```

Shows per-kind enriched / pending counts.

### `enrich-flows` — flow-level business-process summaries

```bash
uv run kloc-intelligence enrich-flows
uv run kloc-intelligence enrich-flows --force              # regenerate existing
```

For each `:Flow`, walks the depth-3 bidirectional context (with implementations) of the entry method, attaches source from referenced nodes, asks the LLM for a structured 3-part summary (surface line + behavior + optional downstream effects). Embedded into `flow_explain_embeddings` so `search` returns flows.

Sample output for `OrderController::create`:

```
API endpoint on path /api/orders method POST

This flow allows customers to place new orders for specific products
by providing their contact information and desired quantities. It
validates the order details and registers the transaction in the
system to begin the fulfillment process.

Triggers inventory availability checks and initiates the order
processing workflow.
```

### `explain` — on-demand explanation for one symbol

```bash
uv run kloc-intelligence explain "OrderService::createOrder()"
uv run kloc-intelligence explain "OrderService" --force            # regenerate
```

### `search` — semantic search

```bash
uv run kloc-intelligence search "validate order before checkout"
uv run kloc-intelligence search "places where we charge a customer" --limit 5
uv run kloc-intelligence search "process customer orders" --collection all
uv run kloc-intelligence search "what triggers OrderPlaced" --collection flows
```

Embeds the query and searches all three collections (`code_embeddings`, `explain_embeddings`, `flow_explain_embeddings`) by cosine similarity. Dedupes by `node_id`, returns top `--limit` (default 10).

`--collection code|explain|flows|all` narrows the search (default `all`).

## MCP server

```bash
uv run kloc-intelligence mcp-server                               # single-project, default db
uv run kloc-intelligence mcp-server --database my_app_db          # single-project, named db
uv run kloc-intelligence mcp-server --config /path/to/config.json # multi-project
```

See [mcp.md](mcp.md) for the full tool list and integration recipes.

## Output flags

- `--json` — emit JSON to stdout (no Rich wrapping). Commands that produce structured data accept it.
- `--debug` — verbose logging to stderr.
- `--project <name>` — disambiguate when the server has multiple projects configured. Not relevant for single-project mode (the default).

## Exit codes

| Code | Meaning |
| --- | --- |
| 0 | Success |
| 1 | User error (symbol not found, missing key, invalid arg) |
| 2 | Typer/CLI parse error |
