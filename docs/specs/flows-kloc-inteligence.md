# Feature: Flows — kloc-intelligence Rebuild

## Goal

Replace the broken Neo4j flow model (which carries hallucinated `FLOW_STEP` edges from a stale DI-trace approach) with a minimal, correct model: `:Flow` nodes linked only to their entry-point method via `FLOW_ENTRY` and to sibling flows via `FLOW_TRIGGERS`. Agents investigate flow internals on demand using the existing `kloc_context` / `kloc_source` / `kloc_chunks` tools rather than pre-computed call trees.

## Non-Goals

- No pre-computed call tree (no `FLOW_STEP` edges — not now, not hidden under a different name)
- No backward compatibility with the old flow shape — hard cut
- No flow enrichment layer (business / technical / search explanations)
- No ASCII diagram generation
- No per-flow Qdrant embeddings (the three `flow_*` collections are actively deleted)
- No PHP changes to `kloc-symfony` — it already produces the correct simple shape
- No changes to POC scripts in `kloc-symfony/pocs/` — kept as historical reference, not drivers of code

---

## Reference Oracle

**Canonical input**: `/Users/michal/dev/ai/kloc/kloc-reference-project-php/.kloc/symfony-kloc.json`
(196 lines, 9 flows, 3 triggers, all `App\`-namespaced)

After a clean import of this file, Neo4j must contain exactly:

| Metric | Expected |
|--------|----------|
| `:Flow` nodes | **9** |
| `FLOW_ENTRY` edges | **9** (every flow has `method_node_id`) |
| `FLOW_TRIGGERS` edges | **3** (one per source×target pair across all triggers) |
| `FLOW_STEP` edges | **0** (never written) |

The 9 flows by `flow_id`:
1. `flow:http:App\Ui\Rest\Controller\CustomerController::get`
2. `flow:http:App\Ui\Rest\Controller\CustomerController::summary`
3. `flow:http:App\Ui\Rest\Controller\OrderController::get`
4. `flow:http:App\Ui\Rest\Controller\OrderController::create`
5. `flow:message:App\Ui\Messenger\Handler\OrderCreatedHandler::__invoke`
6. `flow:event:App\Ui\EventSubscriber\OrderEventSubscriber::onOrderCreated[OrderCreatedEvent]`
7. `flow:event:App\Ui\EventSubscriber\ReportEventSubscriber::onReportGenerated[ReportGeneratedEvent]`
8. `flow:cli:App\Ui\Console\ProcessOrdersCommand::execute`
9. `flow:cli:App\Ui\Console\ProcessReportsCommand::execute`

The 3 `FLOW_TRIGGERS` edges (source → target, via):
- `flow:http:...OrderController::create` → `flow:event:...OrderEventSubscriber::onOrderCreated[...]` via `App\Event\OrderCreatedEvent`
- `flow:cli:...ProcessReportsCommand::execute` → `flow:event:...ReportEventSubscriber::onReportGenerated[...]` via `App\Event\ReportGeneratedEvent`
- `flow:http:...OrderController::create` → `flow:message:...OrderCreatedHandler::__invoke` via `App\Ui\Messenger\Message\OrderCreatedMessage`

---

## Decisions

These are binding decisions, not options. They resolve QA blockers B1–B12.

### D1 — App\ filter: KEEP

`_is_app_flow` filtering by `entry.fqn.startswith("App\\")` is retained. The reference project has no non-App flows so it cannot break the oracle counts. If a future project adds framework-internal flows, they are silently skipped and logged.

### D2 — FLOW_TRIGGERS fan-out: one edge per (source_flow, target_flow) pair

For each trigger object, iterate `sources × targets` (cartesian product). Each resulting pair produces one `FLOW_TRIGGERS` edge from the source `:Flow` to the target `:Flow`. In the reference project all triggers are 1×1, yielding exactly 3 edges.

Edge properties on `FLOW_TRIGGERS`:
- `trigger_type` — the trigger's `type` field (e.g., `event`, `message`)
- `via` — `dispatched_class` FQN (e.g., `App\Event\OrderCreatedEvent`)

Properties **NOT** stored: `dispatcher_method_node_id`, `call_node_id`, `flow_entry_node_id`. These are call-tree data irrelevant to the minimal model.

### D3 — :Flow node property contract (complete list)

All properties are entry-derived. None come from `chain[]` (that array is dropped entirely from parsing).

| Property | Type | Required | Source | Notes |
|----------|------|----------|--------|-------|
| `flow_id` | string | always | `flows[].id` | Unique key; indexed |
| `type` | string | always | `flows[].type` | `http`/`message`/`event`/`cli` |
| `entry_fqn` | string | always | `entry.fqn` | Owning class FQN |
| `entry_method` | string | always | `entry.method` | Method name |
| `name` | string | always | derived (see D3a) | Human-readable label |
| `route` | string | http only | `entry.route` | Absent on non-http flows |
| `http_methods` | string[] | http only | `entry.http_methods` | Absent on non-http flows |
| `message_class` | string | message only | `entry.message` | Absent on non-message flows |
| `event_name` | string | event only | `entry.event` | Absent on non-event flows |
| `command_name` | string | cli only | `entry.command_name` | Absent on non-cli flows |

**D3a — `name` derivation:**
- `http` → `"GET /api/orders/{id}"` (space-joined `http_methods` + route)
- `message` → short class name after last `\` (e.g. `OrderCreatedMessage`)
- `event` → short class name after last `\` (e.g. `OrderCreatedEvent`)
- `cli` → `command_name` value verbatim (e.g. `app:process-orders`)

Properties **NOT** written: `explanation_business`, `explanation_technical`, `explanation_search`, or any enrichment-derived property.

### D4 — Qdrant flow_* collection cleanup: active deletion at import time

`import-flows` (both CLI and MCP handler) actively deletes all three flow Qdrant collections before importing nodes:
- `flow_business_embeddings`
- `flow_technical_embeddings`
- `flow_search_embeddings`

Deletion is idempotent — if a collection does not exist, the call is a no-op (Qdrant returns 200 for non-existent collection deletion).

Registry entries in `src/db/qdrant_store.py:20-22` for these three collections are removed.
`ALL_SEARCH_COLLECTIONS` in `src/ai/pipelines.py:369-375` is pruned to contain only `code_embeddings` and `explain_embeddings`.

### D5 — Idempotency: always clear before import

`import-flows` always calls `clear_flows()` first (no `--no-clear` flag). Running it twice in succession produces identical counts.

### D6 — kloc_search with missing flow_* collections: silent skip

`search_both_collections` in `pipelines.py` already wraps each collection search in `try/except` and skips on error (lines 392–394). This behavior is preserved. After cleanup, the two remaining collections (`code_embeddings`, `explain_embeddings`) are searched normally.

### D7 — Regression test query for OrderController::get

The canonical regression query is:

```cypher
MATCH (:Flow {flow_id: 'flow:http:App\\Ui\\Rest\\Controller\\OrderController::get'})-[r]->()
RETURN type(r) AS rel_type, count(r) AS cnt
```

Expected result: exactly one row — `rel_type = "FLOW_ENTRY"`, `cnt = 1`. No `FLOW_STEP` row appears (because it is never written). This is structurally testable without an LLM.

### D8 — Agent-on-demand pivot testability: structural test only, LLM eval out of scope

The spec does not require an LLM evaluation. The structural guarantee is: the `FLOW_ENTRY` edge supplies `method_node_id` = `node:779b5ec2e2f2e61b`, which is the exact argument an agent passes to `kloc_context`. QA verifies this via Neo4j query, not by running an agent. LLM eval is explicitly out of scope.

### D9 — Test file triage

- `tests/test_method_context.py` — KEEP (its `TestBuildExecutionFlow` at line 503 tests `kloc_context` method traversal, not flow import)
- Any test that directly imports `flow_enricher` or `flow_diagram` — DELETE (those modules are removed). Confirmed by grep: no test file currently imports them — only `cli.py` and `mcp.py` do, lazily.

### D10 — Schema indexes: unchanged

`flow_id` and `flow_type` indexes in `src/db/schema.py:55-56` survive unchanged.

### D11 — Shared file ownership (one dev per file)

Files touched by both demolition and rebuild must have exactly one developer owner in Phase 3:
- `src/cli.py` — one owner
- `src/server/mcp.py` — one owner
- `src/ai/pipelines.py` — one owner
- `src/db/qdrant_store.py` — one owner

No concurrent edits to the same file.

### D12 — Performance baseline

Import of the 9-flow reference file must complete in under 2 seconds.

---

## Input Shape: `symfony-kloc.json`

Source: `/Users/michal/dev/ai/kloc/kloc-reference-project-php/.kloc/symfony-kloc.json` (196 lines)

Two top-level arrays: `flows[]` and `triggers[]`.

### `flows[]` — one object per application entry point

```json
{
  "id":    "flow:http:App\\Ui\\Rest\\Controller\\OrderController::get",
  "type":  "http",
  "entry": {
    "fqn":              "App\\Ui\\Rest\\Controller\\OrderController",
    "node_id":          "node:06b437e3103ea90c",
    "method":           "get",
    "method_node_id":   "node:779b5ec2e2f2e61b",
    "route":            "/api/orders/{id}",
    "http_methods":     ["GET"]
  }
}
```

Type-specific `entry` fields: `route`/`http_methods` (http), `message`/`message_node_id` (message), `event` (event), `command_name` (cli). The `chain[]` field from the old shape is absent in the new shape and must be ignored if encountered.

### `triggers[]` — cross-flow dispatch relationships

```json
{
  "id":                      "trigger:event:App\\Event\\OrderCreatedEvent",
  "type":                    "event",
  "dispatched_class":        "App\\Event\\OrderCreatedEvent",
  "sources": [{ "flow_id": "flow:http:...OrderController::create", ... }],
  "targets": [{ "flow_id": "flow:event:...OrderEventSubscriber::onOrderCreated[...]", ... }]
}
```

---

## New Flow Model (Neo4j)

### `:Flow` Node Properties

See D3 above for the complete property contract.

### Edges

**`FLOW_ENTRY`** — `(:Flow)-[:FLOW_ENTRY]->(:Node)` — points to the entry method node (`entry.method_node_id`). No properties on the edge.

**`FLOW_TRIGGERS`** — `(:Flow)-[:FLOW_TRIGGERS {trigger_type, via}]->(:Flow)` — one per (source_flow, target_flow) cartesian pair. See D2 for fan-out semantics and property list.

### What this model does NOT carry

- No `FLOW_STEP` edges
- No `chain[]` parsing
- No enrichment properties on `:Flow` nodes

### Existing indexes (unchanged, per D10)

```python
"flow_id":   "CREATE INDEX flow_id IF NOT EXISTS FOR (n:Flow) ON (n.flow_id)"
"flow_type": "CREATE INDEX flow_type IF NOT EXISTS FOR (n:Flow) ON (n.type)"
```

---

## Demolition Checklist

### Files to DELETE entirely

| File | LOC | Action |
|------|-----|--------|
| `/Users/michal/dev/ai/kloc/kloc-intelligence/src/ai/flow_enricher.py` | 290 | DELETE |
| `/Users/michal/dev/ai/kloc/kloc-intelligence/src/ai/flow_diagram.py` | 289 | DELETE |

### Files to REPLACE (rewrite in-place)

| File | Current LOC | Action |
|------|-------------|--------|
| `/Users/michal/dev/ai/kloc/kloc-intelligence/src/db/flow_importer.py` | 197 | REPLACE — strip `chain[]`/`FLOW_STEP` parsing; add top-level `triggers[]` parsing (cartesian fan-out); add Qdrant collection deletion call |

### Files to STRIP (targeted removals)

| File | Current LOC | What to remove |
|------|-------------|----------------|
| `/Users/michal/dev/ai/kloc/kloc-intelligence/src/cli.py` | 1086 | Commands `flow-diagram`, `explain-flow`, `enrich-flows` + `_resolve_flow_id` helper. Rewire `import-flows` to new importer. |
| `/Users/michal/dev/ai/kloc/kloc-intelligence/src/server/mcp.py` | 1018 | Tool definitions + handlers for `kloc_explain_flow` and `kloc_flow_diagram`. Rewire `kloc_import_flows` to new importer. |
| `/Users/michal/dev/ai/kloc/kloc-intelligence/src/ai/pipelines.py` | 403 | Remove `flow_business_embeddings`, `flow_technical_embeddings`, `flow_search_embeddings` from `ALL_SEARCH_COLLECTIONS` (lines 372–374). |
| `/Users/michal/dev/ai/kloc/kloc-intelligence/src/db/qdrant_store.py` | ~60 | Remove 3 entries from `COLLECTIONS` dict (lines 20–22). |

### Files to KEEP UNCHANGED

- `/Users/michal/dev/ai/kloc/kloc-intelligence/src/db/schema.py` — 100 LOC; `flow_id` and `flow_type` indexes are correct (D10).

### Qdrant Collections to Delete (active, inside import-flows, per D4)

1. `flow_business_embeddings` (currently 8 vectors — stale)
2. `flow_technical_embeddings` (currently 8 vectors — stale)
3. `flow_search_embeddings` (currently 8 vectors — stale)

Deletion is idempotent and runs before `clear_flows()`.

### Neo4j State Reset (inside import-flows, per D5)

```cypher
MATCH (f:Flow) DETACH DELETE f
```

Called via `clear_flows()` as first Neo4j operation, every time.

---

## New `flow_importer.py` Behavioural Contract

### Input

A parsed `symfony-kloc.json` dict (or path for `load_symfony_kloc`), plus a live `Neo4jConnection`.

### Output Graph State (reference file)

1. Exactly 9 `:Flow` nodes
2. Exactly 9 `FLOW_ENTRY` edges
3. Exactly 3 `FLOW_TRIGGERS` edges
4. Zero `FLOW_STEP` edges

### Triggers Parsing (new — top-level, not chain-embedded)

```python
for trigger in data.get("triggers", []):
    trigger_type = trigger["type"]
    via = trigger["dispatched_class"]
    for source in trigger["sources"]:
        for target in trigger["targets"]:
            edges.append({
                "type": "flow_triggers",
                "source_flow_id": source["flow_id"],
                "target_flow_id": target["flow_id"],
                "trigger_type": trigger_type,
                "via": via,
            })
```

### Idempotency

Always calls `clear_flows()` before importing. Running twice yields equal counts (D5).

### Missing-node Tolerance

- `entry.method_node_id` absent or not in Neo4j → skip `FLOW_ENTRY`, log `WARNING`, `:Flow` node still created.
- `source.flow_id` or `target.flow_id` not matched to an imported `:Flow` → skip `FLOW_TRIGGERS` edge, log `WARNING`.

### Public API (unchanged signatures)

```python
def load_symfony_kloc(path: str | Path) -> dict
def parse_flows(data: dict) -> tuple[list[dict], list[dict]]
def import_flow_nodes(connection: Neo4jConnection, nodes: list[dict]) -> int
def import_flow_edges(connection: Neo4jConnection, edges: list[dict]) -> int
def clear_flows(connection: Neo4jConnection) -> None
```

`parse_flows` returns `(flow_nodes, flow_edges)` where `flow_edges` contains only `flow_entry` and `flow_triggers` typed dicts — no `flow_step`.

---

## CLI `import-flows` Command

```
kloc import-flows <path-to-symfony-kloc.json>
```

Steps:
1. Delete Qdrant collections `flow_business_embeddings`, `flow_technical_embeddings`, `flow_search_embeddings` (idempotent, per D4)
2. Call `clear_flows()` — DETACH DELETE all `:Flow` nodes (per D5)
3. Parse and import flow nodes and edges
4. Print: `Imported N flows, M FLOW_ENTRY edges, K FLOW_TRIGGERS edges`

Removed commands: `flow-diagram`, `explain-flow`, `enrich-flows` (and `_resolve_flow_id` helper).

---

## Acceptance Criteria

**Reference oracle**: assertions referencing "the reference file" use `/Users/michal/dev/ai/kloc/kloc-reference-project-php/.kloc/symfony-kloc.json`.

1. **GIVEN** a clean Neo4j state **WHEN** `kloc import-flows <reference-file>` runs **THEN** `MATCH (f:Flow) RETURN count(f)` returns **9**.

2. **GIVEN** the same import **WHEN** `MATCH ()-[r:FLOW_ENTRY]->() RETURN count(r)` is run **THEN** result is **9**.

3. **GIVEN** the same import **WHEN** `MATCH ()-[r:FLOW_TRIGGERS]->() RETURN count(r)` is run **THEN** result is **3**. (The reference project has 3 triggers each with 1 source and 1 target. The general formula is `Σ |sources_i| × |targets_i|` over all trigger objects — future fixtures with N×M fan-out must adjust the expected count accordingly.)

4. **GIVEN** the same import **WHEN** `MATCH ()-[r:FLOW_STEP]->() RETURN count(r)` is run **THEN** result is **0**.

5. **GIVEN** the `OrderController::get` flow **WHEN** the Cypher query
   ```cypher
   MATCH (:Flow {flow_id: 'flow:http:App\\Ui\\Rest\\Controller\\OrderController::get'})-[r]->()
   RETURN type(r) AS rel_type, count(r) AS cnt
   ```
   is run **THEN** exactly one row is returned: `rel_type = "FLOW_ENTRY"`, `cnt = 1` — no `FLOW_STEP` row exists.

6. **GIVEN** the `OrderController::get` `FLOW_ENTRY` edge **WHEN** the target `:Node` is fetched **THEN** its `node_id` = `"node:779b5ec2e2f2e61b"` (confirming an agent calling `kloc_context("node:779b5ec2e2f2e61b")` traverses only the actual call tree — `EmailSender`, `InventoryChecker`, and `OrderProcessor` are unreachable from that entry point via any graph path from this flow node).

7. **GIVEN** `import-flows` is run twice in succession **WHEN** the second run completes **THEN** Neo4j contains exactly 9 `:Flow` nodes, 9 `FLOW_ENTRY` edges, 3 `FLOW_TRIGGERS` edges (no duplicates).

8. **GIVEN** the 3 `FLOW_TRIGGERS` edges **WHEN** each edge's properties are read **THEN** each has `trigger_type` and `via` and does NOT have `dispatcher_method_node_id` or `call_node_id`.

9. **GIVEN** the `OrderController::get` `:Flow` node **WHEN** its properties are read **THEN** it has `flow_id`, `type="http"`, `entry_fqn`, `entry_method`, `name="GET /api/orders/{id}"`, `route="/api/orders/{id}"`, `http_methods=["GET"]`; and does NOT have `explanation_business`, `explanation_technical`, or `explanation_search`.

10. **GIVEN** `flow_enricher.py` and `flow_diagram.py` are deleted **WHEN** `pytest tests/` runs **THEN** there is no `ModuleNotFoundError`; `kloc --help` succeeds.

11. **GIVEN** the refactored `cli.py` **WHEN** `kloc --help` is run **THEN** `flow-diagram`, `explain-flow`, and `enrich-flows` are NOT listed; `import-flows` IS listed.

12. **GIVEN** the refactored `mcp.py` **WHEN** the MCP server starts **THEN** `kloc_explain_flow` and `kloc_flow_diagram` are NOT in the tool list; `kloc_import_flows` IS present.

13. **GIVEN** `import-flows` runs against the reference file **WHEN** Qdrant is queried via `GET /collections` **THEN** `flow_business_embeddings`, `flow_technical_embeddings`, and `flow_search_embeddings` do NOT appear in the collection list.

14. **GIVEN** the updated `qdrant_store.py` and `pipelines.py` **WHEN** `COLLECTIONS` dict and `ALL_SEARCH_COLLECTIONS` list are inspected **THEN** neither contains any of the three dropped collection names.

15. **GIVEN** `kloc_search` is called after the three flow collections are deleted **WHEN** `search_both_collections` runs **THEN** results are returned from `code_embeddings` and `explain_embeddings` without error (per D6 silent-skip behavior).

16. **GIVEN** the import of the reference file **WHEN** wall-clock time is measured **THEN** import completes in under 2 seconds (D12).

17. **GIVEN** a `symfony-kloc.json` where one flow's `entry.method_node_id` is absent from Neo4j **WHEN** `import-flows` runs **THEN** the `:Flow` node is created, the `FLOW_ENTRY` edge is skipped with a `WARNING` log, and the command exits with code 0.

18. **GIVEN** `SHOW INDEXES` is run in Neo4j after import **THEN** both `flow_id` and `flow_type` indexes exist on `:Flow` nodes (D10).

19. **STRETCH — GIVEN** a new `kloc_flows` MCP tool **WHEN** called **THEN** it returns all 9 `:Flow` nodes with `flow_id`, `type`, `name`, `entry_fqn`, `entry_method`.

20. **STRETCH — GIVEN** a new `kloc_flow` MCP tool **WHEN** called with `flow_id = "flow:http:...OrderController::get"` **THEN** it returns the `:Flow` properties plus the `method_node_id` from `FLOW_ENTRY` and any `FLOW_TRIGGERS` targets, with a note that `kloc_context(method_node_id)` is the next call to investigate the flow.

---

## Risks and Mitigations

### R1 — kloc_search breaks if flow_* collections are still referenced

**Likelihood**: Low after D4/D6. `ALL_SEARCH_COLLECTIONS` is pruned; collections are actively deleted at import; `search_both_collections` already swallows per-collection errors.

**Mitigation**: AC-14 and AC-15 cover registry pruning and runtime behavior.

### R2 — Hidden consumers of flow_enricher or flow_diagram

**Likelihood**: Low. Both are imported lazily; grep confirms only `cli.py` and `mcp.py` import them. AC-10 catches any missed reference via import error on test run.

### R3 — Stale FLOW_STEP edges persist if clear_flows is not called

**Likelihood**: High if forgotten. Mitigated structurally: `clear_flows()` is the first Neo4j call, mandatory.

**Mitigation**: AC-7 (idempotency) catches this directly.

### R4 — Agent-on-demand model is insufficient

**Likelihood**: Low-Medium. The `FLOW_ENTRY` edge provides `method_node_id` directly. AC-6 gives the structural guarantee without LLM eval.

### R5 — Concurrent edits to shared files break builds

**Likelihood**: High in multi-developer team if unmanaged. D11 mandates single ownership per shared file.

**Mitigation**: Architect enforces file ownership in the implementation plan before coding starts.

---

## Out of Scope

- Per-flow business / technical / search explanations
- Qdrant `flow_*` embedding pipelines
- ASCII flow diagram generation
- Call-tree pre-computation (`FLOW_STEP` or equivalent)
- Backward compatibility with old `symfony-kloc.json` `chain[]` shape
- Changes to `kloc-symfony` PHP code
- POC scripts in `kloc-symfony/pocs/` (kept on disk, not decommissioned)
- `kloc_flows` and `kloc_flow` MCP tools (STRETCH, not MVP)
- LLM evaluation of agent-on-demand flow investigation
