# Migrating to `symfony-kloc.json` v3

Guide for upgrading existing kloc projects from the v2 (`triggers[]`) Symfony flow format to v3 (`messages[]` + `events[]` + `http_clients[]` first-class node sets, with FlowEnricher dispatch-context expansion).

If you've never imported a `symfony-kloc.json` before, you don't need this doc — generate fresh data with the current `kloc-symfony` and follow [data-setup.md](usage/kloc-intelligence/data-setup.md). This doc is specifically for people whose `symfony-kloc.json` files predate the v3 schema.

## TL;DR

1. **Regenerate** `symfony-kloc.json` with the current `kloc-symfony` exporter — it writes v3 natively. No flag needed.
2. **Re-run** `kloc-intelligence import-flows` against the new file. Idempotent: existing `:Flow.explanation` properties and the `flow_explain_embeddings` Qdrant collection survive.
3. **Re-run** `enrich-flows` only for flows whose explanation needs the new dispatch context to land in the LLM prompt. Old explanations remain valid; new flows added in v3 (e.g. handlers that were missing in v2) are enriched on first encounter.

That's it. There is no `--migrate` flag, no schema-version setting, no data conversion script. The migration is "regenerate + re-import."

## What changes in v3

| Concern | v2 | v3 |
| --- | --- | --- |
| JSON top-level keys | `flows[]` + `triggers[]` | `flows[]` + `messages[]` + `events[]` + `http_clients[]` |
| Field names in dispatch entries | `dispatcher_method_*` | `caller_method_*` |
| Per-message transports | absent | `messages[].transports[]` (e.g. `["sync"]`) |
| Per-event subscriber priority | absent | `events[].targets[].priority` |
| Outbound HTTP integrations | not modeled | `http_clients[]` (with nullable `class_node_id` for vendor classes) |
| Neo4j flow nodes | `:Flow` only | `:Flow`, `:Message`, `:Event`, `:HttpClient` (all first-class with `.id` uniqueness constraints) |
| Neo4j flow edges | `FLOW_ENTRY`, `FLOW_TRIGGERS` | `FLOW_ENTRY`, `FLOW_ENTRY_CLASS`, `EMITS`, `USES_HTTP_CLIENT`, `HANDLED_BY`, `OF_TYPE` (FLOW_TRIGGERS removed) |
| Flow→Flow dispatch | direct edge `FLOW_TRIGGERS` | two-hop `(:Flow)-[:EMITS]->(:Message|:Event)-[:HANDLED_BY]->(:Flow)` |
| Re-import semantics | DETACH DELETE all flows, re-create | MERGE-reconcile per label; `:Flow.explanation` preserved across runs |
| Qdrant on re-import | collection dropped | collection preserved; orphan points pruned by `flow_id` filter |
| FlowEnricher prompt context | flow source + referenced chunks only | adds `emits_messages`, `emits_events`, `http_calls`, `triggered_by_messages`, `triggered_by_events` so the LLM can name dispatched classes and external services explicitly |
| CLI surface | `flows` | `flows`, `messages`, `events`, `http-clients` (each with list + detail + `--json`) |
| MCP tools | `kloc_flows`, `kloc_import_flows` | adds `kloc_messages`, `kloc_message`, `kloc_events`, `kloc_event`, `kloc_http_clients`, `kloc_http_client` |
| Namespace filter | hardcoded `App\` | env-driven `KLOC_FLOW_NAMESPACES` (comma-separated; default `App\`) |

## What happens if you run v3 import-flows on a v2 JSON file

**No error, no exception — but most of the graph silently goes missing.** The v3 parser reads only the v3 top-level keys; `triggers[]` is ignored.

Live test against `kloc-reference-project-php/.kloc/symfony-kloc.json` (v2, 9 flows, 3 triggers):

```
Imported v3 flows in 1.3s
  Flows:        9 upserted, 1 deleted
  Messages:     0 upserted, 2 deleted     ← v2 has no messages[]
  Events:       0 upserted, 2 deleted     ← v2 has no events[]
  HTTP clients: 0 upserted, 1 deleted     ← v2 has no http_clients[]
  FLOW_ENTRY edges:        0              ← node_ids may not align with current sot.json
  FLOW_ENTRY_CLASS edges:  0
  EMITS (Flow→Message):    0              ← v2 triggers[] is ignored entirely
  EMITS (Flow→Event):      0
  EMITS (Call→Message):    0
  EMITS (Call→Event):      0
  USES_HTTP_CLIENT (Flow): 0
  USES_HTTP_CLIENT (Call): 0
  HANDLED_BY (Message):    0
  HANDLED_BY (Event):      0
  OF_TYPE (Message):       0
  OF_TYPE (Event):         0
  OF_TYPE (HttpClient):    0
  Qdrant points deleted:   0
  Legacy FLOW_TRIGGERS removed: 0
```

Symptoms after a v2-on-v3 import:
- `kloc-intelligence flows` lists flows correctly.
- `kloc-intelligence messages` / `events` / `http-clients` return empty lists.
- `kloc-intelligence flows <id>` shows `dispatches_out` / `dispatches_in` as empty objects, even for flows that *do* dispatch messages/events.
- `enrich-flows` succeeds for flows whose entry method can be resolved by FQN fallback, but the resulting prompt has empty dispatch context — explanations no longer name dispatched classes or external services. Semantic queries like "where do we call paypal api?" stop landing on the right flow.

There is **no version check** in the importer today. If you want a hard failure when v2 JSON is passed, the easiest knob is to grep `data.get("version")` in `parse_v3` and raise — but that's currently a deliberate non-decision: re-importing partial data is still better than refusing to import at all.

## End-to-end migration steps

### Step 1 — Regenerate `symfony-kloc.json`

The current `kloc-symfony` exporter writes v3 natively. From the project root:

```bash
cd kloc-symfony/contract-tests
bin/run.sh generate --sot /path/to/your/sot.json
```

Or use the full pipeline (regenerates `sot.json` first):

```bash
./kloc.sh index --project myapp -d /path/to/php-project
# produces data/myapp/sot.json — fresh node IDs

cd kloc-symfony/contract-tests
bin/run.sh generate --sot /Users/.../data/myapp/sot.json
cp output/symfony-kloc.json /Users/.../data/myapp/symfony-kloc.json
```

**Why both:** the v3 exporter cross-references `class_node_id` / `method_node_id` against `sot.json` to populate the `OF_TYPE` edges and the `:Call -[:EMITS]->` mirrors. If your `sot.json` was generated by an older `kloc-mapper`, those node-id columns end up null/unmatched and the importer creates fewer `FLOW_ENTRY` edges. Regenerating both keeps node IDs aligned.

### Step 2 — Import into Neo4j

```bash
cd kloc-intelligence

# Structural graph
uv run kloc-intelligence import /path/to/sot.json

# Flow subgraph (v3)
uv run kloc-intelligence import-flows /path/to/symfony-kloc.json
```

What `import-flows` does on a v3 file:

- MERGE-reconcile across `:Flow`, `:Message`, `:Event`, `:HttpClient` labels. Existing nodes have their **structural** props updated; `:Flow.explanation`, `.explain_model`, `.explain_at` are never overwritten.
- Bulk-replace the six v3 edge types (`FLOW_ENTRY`, `FLOW_ENTRY_CLASS`, `EMITS`, `USES_HTTP_CLIENT`, `HANDLED_BY`, `OF_TYPE`).
- Unconditionally sweep any `FLOW_TRIGGERS` edges left over from v2 imports.
- For orphan flows (in the DB but not in the new JSON): `DETACH DELETE` the `:Flow` node and filter-delete its Qdrant points by `flow_id`. The `flow_explain_embeddings` collection itself is never dropped.

The print-out at the end of the run reports the counts for every operation. If `Messages: 0 upserted` shows up while your v3 JSON has a non-empty `messages[]`, you're still feeding v2 JSON (or the env-driven namespace filter excluded everything — see Step 5).

### Step 3 — Re-enrich (optional, recommended)

```bash
uv run kloc-intelligence enrich-flows         # only enriches flows whose .explanation is NULL
uv run kloc-intelligence enrich-flows --force # re-enriches everything, including v2-era explanations
```

Default behavior **skips** any flow that already has `.explanation` set. That means v2-era flow explanations stay in place and don't pay the LLM cost. New flows added by v3 — or flows whose explanations are blank — get enriched with the new dispatch context.

Use `--force` if you want the v2-era flow explanations rewritten to mention dispatched message/event classes and external HTTP services. This costs one LLM call + one embedding call per flow (about $0.001–$0.005 per flow with the default OpenRouter pricing).

### Step 4 — Verify

```bash
uv run kloc-intelligence flows           # confirm flow count
uv run kloc-intelligence messages        # confirm message node set
uv run kloc-intelligence events
uv run kloc-intelligence http-clients

# Flow detail shows dispatches_in / dispatches_out (NOT triggers_in / triggers_out)
uv run kloc-intelligence flows "OrderController::create" --json
```

Spot-check: after `--force` enrichment, a flow that dispatches a Symfony message should explicitly name the message class in its `.explanation`. From the reference fixture's `OrderController::create`:

> "[…] Dispatches OrderCreatedMessage and OrderCreatedEvent for downstream processing and notification."

And the marquee case — `PaymentController::verify`, which calls PayPal:

> "[…] calling the PayPal API at https://api.paypal.com via paypal.client. Dispatches AuditLogMessage to record the verification action for compliance and auditing."

Then the semantic-search payoff:

```bash
uv run kloc-intelligence search "where do we call paypal api" --collection flows
# top hit: App\Ui\Rest\Controller\PaymentController  (score ~0.64)
```

### Step 5 — Configure namespace filter (optional)

v2 had a hardcoded `App\` namespace filter. v3 makes it an env-driven allow-list:

```ini
# .env
KLOC_FLOW_NAMESPACES=App\,Domain\Orders\,Acme\
```

- Comma-separated FQN prefixes; whitespace around entries is stripped.
- Default / unset / empty → `App\` (backwards compatible).
- Applies to `:Flow` entries only. `:Message`, `:Event`, `:HttpClient` are universal — the PayPal `:HttpClient` always lands in the graph because the filter doesn't apply to them.

If your project ships flows under a non-`App\` root, you'll see `Flows: 0 upserted` until you set this var.

## What about the existing `:Flow.explanation` data?

The whole point of the idempotent re-import is that you don't lose it.

| Operation | Effect on `:Flow.explanation` |
| --- | --- |
| Re-import same JSON | No change. Flow nodes are MERGE'd; preserve_props blocks any SET on enrichment columns. |
| Re-import JSON with a flow removed | That flow is `DETACH DELETE`d AND its Qdrant point in `flow_explain_embeddings` is filter-deleted. Other flows untouched. |
| Re-import JSON with a flow added | Existing flows keep their `.explanation`. The new flow has `.explanation IS NULL` and gets picked up by the next `enrich-flows` run. |
| `enrich-flows` (no `--force`) | Skips flows whose `.explanation` is already set. Only the new/missing ones cost LLM calls. |
| `enrich-flows --force` | Regenerates all explanations from scratch using the new v3 dispatch-context prompt. |
| Drop the Qdrant collection manually | Need to re-embed everything via `enrich-flows --force` — the embeddings live there, not in Neo4j. |

## Common pitfalls

- **Stale `sot.json`.** v3's `OF_TYPE` edges and `:Call -[:EMITS]->` mirrors depend on node IDs matching between `sot.json` and `symfony-kloc.json`. If you regenerated `symfony-kloc.json` against a fresh `sot.json` but Neo4j still holds an older `sot.json`, the new edges won't resolve. Always re-run `kloc-intelligence import /path/to/sot.json` alongside `import-flows`.
- **`KLOC_FLOW_NAMESPACES` typos.** `App\\` (double backslash) in the env file is interpreted literally — you want `App\` (single backslash). If `Flows: 0 upserted` shows up unexpectedly, log out the env var value with `env | grep KLOC_FLOW`.
- **Forgetting `KLOC_PROJECT_ROOT`.** `enrich-flows` needs `KLOC_PROJECT_ROOT` to read the flow's entry source code. If it's unset and the call fails with "Cannot read entry source for flow …", export it.
- **Vendor classes for `:HttpClient`.** When `http_clients[].class_node_id` is null (vendor class, e.g. Symfony's `UriTemplateHttpClient` wrapping PayPal), the `:HttpClient` node is still created with `class_fqn` set; only the `OF_TYPE` edge to `:Class` is skipped. Don't expect `OF_TYPE (HttpClient): 0` to be a bug — it's the vendor-detection working as designed.
- **`FLOW_TRIGGERS` lingering from v2.** Even after a clean v3 import, a `MATCH ()-[r:FLOW_TRIGGERS]->() RETURN count(r)` should return 0 — the v3 importer sweeps this edge type unconditionally. If you see non-zero, the import didn't complete or hit an exception mid-run.

## Rollback

If you need to roll back to v2 for any reason, downgrade `kloc-intelligence` to the pre-v3 release and re-run the old `kloc-symfony` (which still writes v2). The v2 importer will recreate `FLOW_TRIGGERS` edges and ignore the v3 top-level keys (analogous to v3's behavior on v2 input).

There is no automatic "v3 → v2" converter; if you want one, write a JSON-shaping script that flattens `messages[]` + `events[]` back into a `triggers[]` array. The structural information is a strict superset, so the conversion is lossy only in the new fields (`transports[]`, `priority`, `http_clients[]`).
