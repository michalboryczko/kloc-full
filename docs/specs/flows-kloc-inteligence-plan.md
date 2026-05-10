# Implementation Plan: Flows — kloc-intelligence Rebuild

Spec: `/Users/michal/dev/ai/kloc/docs/specs/flows-kloc-inteligence.md`
Plan author: architect (planning team `flows-kloc-inteligence`)
Plan date: 2026-05-10
Target repo: `/Users/michal/dev/ai/kloc/kloc-intelligence` (branch: `main`)

---

## Overview

### Problem
The current `:Flow` graph in Neo4j carries hallucinated `FLOW_STEP` (Flow→Class) edges from a stale DI-trace approach. Three POCs explored richer call-tree models; the chosen direction is to **NOT pre-build a call tree at all** — keep `:Flow` nodes minimal (entry + triggers) and let agents pivot into existing `kloc_context` / `kloc_source` / `kloc_chunks` tools. Also drop the over-built enrichment layer (multi-type explanations, 3 Qdrant collections, ASCII diagram generator).

### Acceptance criteria source
14 MUST + 2 STRETCH ACs in `flows-kloc-inteligence.md` §Acceptance Criteria.

### Hard constraints (from spec + lead direction)
- No backward compatibility (no deprecation period, no compat shim)
- No `FLOW_STEP` edges anywhere in the new model
- No PHP changes (`kloc-symfony` already produces correct shape)
- No changes to POCs at `kloc-symfony/pocs/`
- Indexes `flow_id` and `flow_type` in `schema.py` STAY UNCHANGED
- New `flow_importer.py` keeps the same module-level public API as today (`load_symfony_kloc`, `parse_flows`, `import_flow_nodes`, `import_flow_edges`, `clear_flows`)
- **One-owner-per-shared-file rule (lead direction, NON-NEGOTIABLE)**: every source file is touched by exactly one developer. No file is shared between Stream A and Stream B. Streams A and B run truly in parallel with zero file overlap.

---

## Codebase Summary (from Phase 1 exploration)

### Verified file inventory

| Path | LOC | Callers | Action |
|------|-----|---------|--------|
| `/Users/michal/dev/ai/kloc/kloc-intelligence/src/db/flow_importer.py` | 197 | `cli.py:684`, `mcp.py:821` | **REPLACE** |
| `/Users/michal/dev/ai/kloc/kloc-intelligence/src/ai/flow_enricher.py` | 290 | `cli.py:765,845`, `mcp.py:753` | **DELETE** |
| `/Users/michal/dev/ai/kloc/kloc-intelligence/src/ai/flow_diagram.py` | 289 | `cli.py:724`, `mcp.py:797` | **DELETE** |
| `/Users/michal/dev/ai/kloc/kloc-intelligence/src/cli.py` | 1086 | (entry point) | **STRIP** flow commands + **REWIRE** import-flows |
| `/Users/michal/dev/ai/kloc/kloc-intelligence/src/server/mcp.py` | 1018 | (entry point) | **STRIP** flow tools + **REWIRE** kloc_import_flows |
| `/Users/michal/dev/ai/kloc/kloc-intelligence/src/ai/pipelines.py` | 403 | `cli.py:515`, `mcp.py:685`, `flow_enricher.py:18-24` | **STRIP** flow-only constants & list entries |
| `/Users/michal/dev/ai/kloc/kloc-intelligence/src/db/qdrant_store.py` | ~190 | `cli.py` (qdrant ops) | **STRIP** 3 collection entries |
| `/Users/michal/dev/ai/kloc/kloc-intelligence/src/db/schema.py` | 100 | (schema bootstrap) | **UNCHANGED** |

### CLI command line ranges (cli.py)
- `import-flows` — lines **674-709** (KEEP, REWIRE — Stream A owns)
- `flow-diagram` — lines **712-746** (DELETE — Stream A owns)
- `explain-flow` — lines **749-828** (DELETE — Stream A owns)
- `enrich-flows` — lines **831-886** (DELETE — Stream A owns)
- `_resolve_flow_id` helper — lines **889-906** (DELETE — Stream A owns)

### MCP tool/handler line ranges (server/mcp.py)
- `kloc_explain_flow` definition — lines **416-438** (DELETE — Stream A owns)
- `kloc_flow_diagram` definition — lines **439-453** (DELETE — Stream A owns)
- `kloc_import_flows` definition — lines **454-468** (KEEP, REWIRE description — Stream A owns)
- Handler dispatch dict entries — lines **536-538** (DELETE 2, KEEP 1 — Stream A owns)
- `_handle_explain_flow` — lines **750-792** (DELETE — Stream A owns)
- `_handle_flow_diagram` — lines **794-818** (DELETE — Stream A owns)
- `_handle_import_flows` — lines **820-835** (KEEP, REWIRE — Stream A owns)

### pipelines.py prunable regions
- Flow-only constants `FLOW_BUSINESS_SYSTEM`/`FLOW_BUSINESS_TEMPLATE`/`FLOW_TECHNICAL_*`/`FLOW_SEARCH_*`/`FLOW_LABELS_*` — lines **32-98** (DELETE — only used by `flow_enricher.py`)
- `ALL_SEARCH_COLLECTIONS` list — lines **369-375** — DELETE 3 flow entries (lines **372-374**)

### qdrant_store.py prunable regions
- `COLLECTIONS` dict — lines **17-23** — DELETE 3 flow entries (lines **20-22**)

### Tests state
- **Zero** test files reference any flow code (`grep flow_importer\|flow_enricher\|flow_diagram\|FLOW_BUSINESS\|kloc_explain_flow tests/` returns nothing)
- No `tests/test_*flow*.py` exists — green field for new tests
- `tests/conftest.py` provides reusable `requires_neo4j`, `neo4j_connection`, `loaded_database` fixtures that the new tests will plug into

### Reference input
- `/Users/michal/dev/ai/kloc/kloc-reference-project-php/.kloc/symfony-kloc.json` — 9 App flows, 3 triggers (2 event, 1 message)
- All 9 entries pass the App-only filter (no framework flows in the reference output)

### New shape vs. old shape (root cause of the broken model)

| | Old (current importer reads this) | New (current symfony-kloc.json produces this) |
|--|----------------------------------|-----------------------------------------------|
| Per-flow chain | `chain[]` with `{node_id, role, position, triggers[]}` per step | **REMOVED** — no chain field |
| Triggers source | nested under each `chain[].triggers[].target_flows[]` | top-level `triggers[]` with `{sources[], targets[]}` |
| Trigger id | implicit | explicit `id` field per trigger |
| Trigger metadata | `type`, `via` strings | `type`, `dispatched_class`, `dispatched_class_node_id`, full source/target metadata |

The current importer's `chain[]` parsing is dead against today's output and is the root of the broken `FLOW_STEP` problem.

---

## Technical Approach

### Chosen approach: parallel streams under strict file ownership

Two developers work fully in parallel with zero file overlap:

- **Stream A (developer-1) — Demolition + Rewire**: owns every file that needs demolition AND the rewire-the-callers work. This is the heavier stream by file count but each file edit is mechanical (delete blocks, swap two import lines, update one print and one return dict). No new module is created here.
- **Stream B (developer-2) — New flow_importer + Cleanup ops**: owns the brand-new `flow_importer.py` rewrite (the only "thinking" file in this PR) and the operational cleanup scripts (Qdrant collection drop, Neo4j orphan-edge sweep). Lower file count, more design work.
- **Stream D (Tests)** runs after Streams A and B converge; either developer can own it (recommended: developer-2 because they have fresh context on the new importer's behavior).

### Why this split (rationale)

- **Eliminates the file-overlap failure mode**: prior plan had Stream B touching `cli.py` lines 674-709 and `mcp.py` lines 454-468 + 820-835 to rewire `import-flows`. Lead direction makes that a non-negotiable failure mode. New split: developer-1 makes those rewire edits in the same pass as the deletion edits — both edits are local, target the same files, and are best done atomically by one owner.
- **Stream B never blocks on Stream A**: the new `flow_importer.py` keeps the SAME module-level public API as today (`parse_flows`, `import_flow_nodes`, `import_flow_edges`, `clear_flows`, `load_symfony_kloc`). Developer-1 can rewire `cli.py`/`mcp.py` purely against the documented interface contract — they don't need to wait for developer-2's implementation to finish. The two streams converge only at smoke-test time.
- **Demolition before rebuild logically**: even within Stream A, deletion of unused files (`flow_enricher.py`, `flow_diagram.py`) and removal of references in `cli.py`/`mcp.py` should land together as one coherent commit so the codebase is never in a state where a deleted module is still imported.
- **No `FLOW_STEP` ever**: the new `flow_importer.py` doesn't emit `flow_step` edges (parser-layer guarantee). Combined with `clear_flows()` at start of each import, AC-4 is structurally satisfied.

### Why alternatives were rejected

- **Option: dev-2 also owns the `import-flows` rewire** — rejected by lead's one-owner-per-shared-file rule. Plus, prior team incidents show concurrent edits to large files like `cli.py` cause merge conflicts.
- **Option: serialize Phase 1 (demolition) then Phase 2 (rebuild)** — rejected because it makes Stream B sit idle. With strict ownership and a stable interface contract, both streams run concurrently from t=0.
- **Option: keep `FLOW_STEP` edge type but never create new ones** — leaves a footgun for future regressions. Spec is explicit: `FLOW_STEP` does not exist in the new model.

### Patterns being followed (from current code)

- **Lazy imports inside command/handler functions** — preserved in the rewired code paths.
- **Module-level functions** in `flow_importer.py` — same as the existing one (no class wrapper), same names.
- **`Neo4jConnection` direct usage with `UNWIND $batch`** — same batch-import pattern as `db/importer.py:217-228`.
- **`ensure_schema(conn)` called from `import-flows` command** — preserved (cli.py:691).
- **`_get_runner(project)` for MCP handlers** — preserved.

---

## Interface Contract (Stream A ↔ Stream B boundary)

This is the contract that Stream A imports from and Stream B implements. Both streams MUST honor it; smoke test verifies adherence.

### `src/db/flow_importer.py` — module-level public API

```python
def load_symfony_kloc(path: str | Path) -> dict
    # Loads and parses a symfony-kloc.json file. Returns the dict as-is.

def parse_flows(data: dict) -> tuple[list[dict], list[dict]]
    # Parses the new-shape symfony-kloc.json into (flow_node_props, flow_edge_props).
    # flow_edge_props contains ONLY dicts with type='flow_entry' or 'flow_triggers'.
    # No 'flow_step' EVER. App-only filter applied.

def import_flow_nodes(connection: Neo4jConnection, nodes: list[dict]) -> int
    # MERGEs :Flow nodes by flow_id. Returns count imported.

def import_flow_edges(connection: Neo4jConnection, edges: list[dict]) -> int
    # MERGEs FLOW_ENTRY (Flow -> Node) and FLOW_TRIGGERS (Flow -> Flow) edges.
    # Skips silently with WARNING log if either endpoint is missing.
    # Returns count imported.

def clear_flows(connection: Neo4jConnection) -> None
    # MATCH (f:Flow) DETACH DELETE f. Removes all :Flow nodes and any
    # FLOW_*-typed edge attached to them (including stale FLOW_STEP).
```

### Edge dict shape (`parse_flows` return)

```python
# flow_entry edge
{"type": "flow_entry",
 "source_flow_id": "<flow_id>",
 "target_node_id": "<method_node_id>"}

# flow_triggers edge
{"type": "flow_triggers",
 "source_flow_id": "<flow_id>",
 "target_flow_id": "<flow_id>",
 "trigger_type": "event" | "message",
 "via": "<dispatched class FQN>"}
```

**`type == "flow_step"` MUST NEVER appear** — this is enforced at the parser, not at import time, so misuse of `import_flow_edges` cannot create one.

### Stream A consumer pattern (cli.py + mcp.py call sites)

Both cli.py:684 and mcp.py:821 import the EXACT same five public names:
```python
from .db.flow_importer import (
    load_symfony_kloc, parse_flows, import_flow_nodes,
    import_flow_edges, clear_flows,
)
```
Imports remain lazy (inside the command/handler function body) for fast CLI startup.

### CLI `import-flows` stdout contract

```
Parsing /path/to/symfony-kloc.json...
  Parsed 9 flow nodes, 12 flow edges
Clearing existing flows...
Importing flow nodes...
Importing flow edges...
Imported 9 flows, 9 FLOW_ENTRY edges, 3 FLOW_TRIGGERS edges in 0.X s
```
(12 = 9 entry + 3 trigger edges in the reference project.)

### MCP `kloc_import_flows` return contract

```json
{
  "status": "ok",
  "flows": 9,
  "flow_entry_edges": 9,
  "flow_triggers_edges": 3
}
```

### Final state of stripped data structures (Stream A's responsibility)

```python
# src/db/qdrant_store.py
COLLECTIONS = {
    "code_embeddings":    "Source code embeddings for Class/Method nodes",
    "explain_embeddings": "Human-language explanation embeddings",
}

# src/ai/pipelines.py
ALL_SEARCH_COLLECTIONS = [
    "code_embeddings",
    "explain_embeddings",
]
```

---

## Phased Implementation

### Stream A — Demolition + Rewire (Developer-1) — runs in parallel with Stream B

Owner: **developer-1** — sole owner of `cli.py`, `mcp.py`, `pipelines.py`, `qdrant_store.py`, `flow_enricher.py`, `flow_diagram.py`.

**Suggested ordering inside Stream A** (each step is one logical commit; landing them in this order keeps the codebase consistent at every point):

- [ ] **Task A1 — Strip flow constants from `src/ai/pipelines.py`**.
  - Delete `FLOW_BUSINESS_SYSTEM`, `FLOW_BUSINESS_TEMPLATE`, `FLOW_TECHNICAL_SYSTEM`, `FLOW_TECHNICAL_TEMPLATE`, `FLOW_SEARCH_SYSTEM`, `FLOW_SEARCH_TEMPLATE`, `FLOW_LABELS_SYSTEM`, `FLOW_LABELS_TEMPLATE` (lines 32-98).
  - Remove the three flow entries from `ALL_SEARCH_COLLECTIONS` (lines 372-374). Final list: `["code_embeddings", "explain_embeddings"]`.
  - Why first: these constants are imported only by `flow_enricher.py`; removing them does not break anything until A4. But it's the smallest, safest first edit.

- [ ] **Task A2 — Strip flow entries from `src/db/qdrant_store.py`**.
  - Remove the three flow entries from `COLLECTIONS` dict (lines 20-22). Final dict: `{"code_embeddings": "...", "explain_embeddings": "..."}`.
  - Independent of A1, A3, A4. Safe at any point.

- [ ] **Task A3 — Strip flow commands and rewire `import-flows` in `src/cli.py`** (single edit pass).
  - **Delete** `flow-diagram` command (lines 712-746).
  - **Delete** `explain-flow` command (lines 749-828).
  - **Delete** `enrich-flows` command (lines 831-886).
  - **Delete** `_resolve_flow_id` helper (lines 889-906).
  - **Rewire** `import-flows` command (lines 674-709): leave the structure intact, but split `edges` by type before the import calls so the summary line can show three counts:
    ```python
    nodes, edges = parse_flows(data)
    entry_edges = [e for e in edges if e["type"] == "flow_entry"]
    trigger_edges = [e for e in edges if e["type"] == "flow_triggers"]
    console.print(f"  Parsed {len(nodes)} flow nodes, {len(edges)} flow edges")
    if clear:
        console.print("Clearing existing flows...")
        clear_flows(conn)
    console.print("Importing flow nodes...")
    import_flow_nodes(conn, nodes)
    console.print("Importing flow edges...")
    import_flow_edges(conn, edges)
    total = time_mod.perf_counter() - start
    console.print(f"\n[green]Imported {len(nodes)} flows, "
                  f"{len(entry_edges)} FLOW_ENTRY edges, "
                  f"{len(trigger_edges)} FLOW_TRIGGERS edges in {total:.1f}s[/green]")
    ```
  - The import statement on line 684 is unchanged (Stream B preserves the same names).
  - Why combined edit: all four affected regions are within a 230-line slab of one file. One developer, one pass, one commit avoids review churn.

- [ ] **Task A4 — Strip flow tools and rewire `kloc_import_flows` in `src/server/mcp.py`** (single edit pass).
  - **Delete** `kloc_explain_flow` tool definition in `list_tools` (lines 416-438).
  - **Delete** `kloc_flow_diagram` tool definition in `list_tools` (lines 439-453).
  - **Delete** dispatch dict entries (lines 536-537):
    ```python
    "kloc_explain_flow": self._handle_explain_flow,
    "kloc_flow_diagram": self._handle_flow_diagram,
    ```
    Keep line 538 (`"kloc_import_flows": self._handle_import_flows`).
  - **Delete** `_handle_explain_flow` method (lines 750-792).
  - **Delete** `_handle_flow_diagram` method (lines 794-818).
  - **Rewire** `kloc_import_flows` tool description (line 456) to:
    > `"Import symfony-kloc.json flows into Neo4j as :Flow nodes with FLOW_ENTRY and FLOW_TRIGGERS edges. Replaces all existing flows on each call."`
  - **Rewire** `_handle_import_flows` (lines 820-835) to break out edge counts in the return dict:
    ```python
    nodes, edges = parse_flows(data)
    entry_edges = [e for e in edges if e["type"] == "flow_entry"]
    trigger_edges = [e for e in edges if e["type"] == "flow_triggers"]
    clear_flows(conn)
    import_flow_nodes(conn, nodes)
    import_flow_edges(conn, edges)
    return {"status": "ok",
            "flows": len(nodes),
            "flow_entry_edges": len(entry_edges),
            "flow_triggers_edges": len(trigger_edges)}
    ```

- [ ] **Task A5 — Delete `src/ai/flow_enricher.py`** (290 LOC, entire file).
  - Safe ONLY after A1, A3, A4 land — those steps remove all import sites.
  - Verify: `git rm src/ai/flow_enricher.py`.

- [ ] **Task A6 — Delete `src/ai/flow_diagram.py`** (289 LOC, entire file).
  - Safe ONLY after A3, A4 land — those steps remove all import sites.
  - Verify: `git rm src/ai/flow_diagram.py`.

- [ ] **Task A7 — Verify Stream A in isolation**.
  - `cd /Users/michal/dev/ai/kloc/kloc-intelligence && python -c "import src.cli; import src.server.mcp; import src.ai.pipelines; import src.db.qdrant_store"` — should succeed (no import errors).
  - `kloc --help` — confirm `flow-diagram`/`explain-flow`/`enrich-flows` are absent and `import-flows` is still listed.
  - Note: `kloc import-flows` would still call OLD-shape `flow_importer.py` if Stream B has not landed yet. Acceptable in isolation since Stream A doesn't run import-flows.

### Stream B — New flow_importer + Cleanup ops (Developer-2) — runs in parallel with Stream A

Owner: **developer-2** — sole owner of `src/db/flow_importer.py`, `kloc-intelligence/scripts/` (new directory), and operational cleanup work.

**Suggested ordering inside Stream B**:

- [ ] **Task B1 — Replace `src/db/flow_importer.py`** entirely.
  - Preserve the module-level public API documented in §Interface Contract above.
  - Drop ALL chain[] parsing — the new shape has no chain field.
  - Read `triggers[]` at the top level of the input dict; iterate `sources[]` × `targets[]` to emit `flow_triggers` edges.
  - Apply App-only filter: `flow["entry"]["fqn"].startswith("App\\")` for flows; verify both `source.flow_id` and `target.flow_id` reference an App flow before emitting a trigger edge.
  - Emit `:Flow` node properties per type:
    - `http`: `route` (string), `http_methods` (string[]), `name = f"{','.join(http_methods)} {route}"` (e.g. `"GET /api/orders/{id}"`)
    - `message`: `message_class` (= `entry.message`), `name` = short class name (after last `\\`)
    - `event`: `event_name` (= `entry.event`), `name` = short class name
    - `cli`: `command_name` (= `entry.command_name`), `name = command_name`
  - Emit `flow_entry` edge dict per flow:
    ```python
    {"type": "flow_entry",
     "source_flow_id": flow["id"],
     "target_node_id": flow["entry"]["method_node_id"]}
    ```
    If `method_node_id` is missing/null, skip with `logger.warning(...)`.
  - Emit `flow_triggers` edge dicts per source×target pair:
    ```python
    {"type": "flow_triggers",
     "source_flow_id": source["flow_id"],
     "target_flow_id": target["flow_id"],
     "trigger_type": trigger.get("type", ""),
     "via": trigger.get("dispatched_class", "")}
    ```
    Skip with WARNING if either flow_id fails the App-only filter (defensive, since spec covers framework targets too).
  - **Cypher for `:Flow` MERGE** (idempotency layered on top of `clear_flows`):
    ```cypher
    UNWIND $batch AS props
    MERGE (f:Flow {flow_id: props.flow_id})
    SET f += props
    ```
  - **Cypher for FLOW_ENTRY** (use double-MATCH so missing target endpoint silently skips with WARNING — counted at the application layer):
    ```cypher
    MATCH (f:Flow {flow_id: $flow_id})
    MATCH (n:Node {node_id: $node_id})
    MERGE (f)-[:FLOW_ENTRY]->(n)
    ```
    If the MATCH on `:Node` fails, the statement reports zero changes — application code logs WARNING and continues.
  - **Cypher for FLOW_TRIGGERS**:
    ```cypher
    MATCH (f1:Flow {flow_id: $source_id})
    MATCH (f2:Flow {flow_id: $target_id})
    MERGE (f1)-[r:FLOW_TRIGGERS]->(f2)
    SET r.trigger_type = $trigger_type, r.via = $via
    ```
  - **`clear_flows`** unchanged from today: `MATCH (f:Flow) DETACH DELETE f` — handles purging old `FLOW_STEP` edges as a side effect.
  - Define module constant `APP_NAMESPACE = "App\\"` at top to avoid duplicating the literal.
  - Estimated LOC: 80-100. Old file was 197.
  - Imports: only `json`, `logging`, `pathlib.Path`, `from .connection import Neo4jConnection`. No flow_diagram, no flow_enricher.

- [ ] **Task B2 — Add cleanup script `kloc-intelligence/scripts/drop_flow_collections.py`**.
  - One-shot operational script to drop the three stale Qdrant collections (AC-11).
    ```python
    """Drop the three stale flow_*_embeddings Qdrant collections — one-time cleanup."""
    from qdrant_client import QdrantClient
    STALE = ["flow_business_embeddings", "flow_technical_embeddings", "flow_search_embeddings"]
    client = QdrantClient(url="http://localhost:6333")
    for name in STALE:
        try:
            client.delete_collection(name)
            print(f"Dropped {name}")
        except Exception as e:
            print(f"Skip {name}: {e}")
    ```
  - Run once during deployment. Verify with `curl -s http://localhost:6333/collections | jq '.result.collections[].name'` — no flow_* names should appear.

- [ ] **Task B3 — Add belt-and-suspenders cleanup script `kloc-intelligence/scripts/sweep_flow_step_edges.py`**.
  - In case a partially deployed environment retains stray `FLOW_STEP` edges from a pre-deployment Neo4j state where someone called `import-flows` after Stream A landed but before Stream B's new importer is live (unlikely with concurrent merge, but cheap to add).
    ```python
    """One-time sweep: delete any FLOW_STEP edges. clear_flows() in the new importer
    will already do this for any :Flow node, but this catches orphan FLOW_STEP edges
    that somehow survived (defensive)."""
    from src.config import Neo4jConfig
    from src.db.connection import Neo4jConnection
    conn = Neo4jConnection(Neo4jConfig.from_env())
    with conn.session() as session:
        result = session.run("MATCH ()-[r:FLOW_STEP]->() WITH r, count(*) AS c DELETE r RETURN c").single()
        print(f"Deleted {result['c'] if result else 0} FLOW_STEP edges")
    conn.close()
    ```
  - Run once after the new importer lands, to be sure AC-4 holds across all environments.

- [ ] **Task B4 — Verify Stream B in isolation**.
  - `python -c "from src.db.flow_importer import load_symfony_kloc, parse_flows, import_flow_nodes, import_flow_edges, clear_flows; print('OK')"` — confirm public API is intact.
  - Run unit-only parsing test against the reference file:
    ```python
    from src.db.flow_importer import load_symfony_kloc, parse_flows
    data = load_symfony_kloc("/Users/michal/dev/ai/kloc/kloc-reference-project-php/.kloc/symfony-kloc.json")
    nodes, edges = parse_flows(data)
    assert len(nodes) == 9
    entry = [e for e in edges if e["type"] == "flow_entry"]
    triggers = [e for e in edges if e["type"] == "flow_triggers"]
    steps = [e for e in edges if e["type"] == "flow_step"]
    assert len(entry) == 9
    assert len(triggers) == 3
    assert len(steps) == 0
    ```

### Stream D — Tests (after A and B converge)

Owner: **developer-2** (recommended — fresh context on importer behavior). Stream D blocks on Stream A and Stream B both completing.

- [ ] **Task D1 — Create `tests/test_flow_importer.py`** with parser unit tests (no Neo4j):
  - `test_parse_flows_reference_project_counts` — load `kloc-reference-project-php/.kloc/symfony-kloc.json`, assert 9 flow nodes, 9 flow_entry edges, 3 flow_triggers edges. (covers AC-2, AC-3 at parse layer)
  - `test_parse_flows_no_flow_step_edges` — assert no edge has `type == "flow_step"` (covers AC-4 at parse layer)
  - `test_parse_flows_filters_non_app` — synthetic fixture with one App flow + one `Symfony\\` flow → only the App one in output
  - `test_parse_flows_http_name_derivation` — verify `name == "GET /api/orders/{id}"`
  - `test_parse_flows_message_name_derivation` — verify `name == "OrderCreatedMessage"`
  - `test_parse_flows_event_name_derivation` — verify `name == "OrderCreatedEvent"`
  - `test_parse_flows_cli_name_derivation` — verify `name == "app:process-orders"`
  - `test_parse_flows_per_type_props` — verify `route`/`http_methods`/`message_class`/`event_name`/`command_name` set per type
  - `test_parse_flows_trigger_via_field` — verify `via` field carries the dispatched class FQN

- [ ] **Task D2 — Add integration tests `class TestFlowImporterNeo4j` to the same file**, gated by `@requires_neo4j`:
  - `test_import_creates_9_flow_nodes` — covers AC-2
  - `test_import_creates_3_flow_triggers_edges` — covers AC-3
  - `test_import_creates_zero_flow_step_edges` — covers AC-4
  - `test_flow_entry_resolves_to_correct_node` — OrderController::get FLOW_ENTRY → `node:779b5ec2e2f2e61b` (covers AC-5)
  - `test_idempotent_double_import` — two consecutive imports → 9 flow nodes, 9 FLOW_ENTRY edges (covers AC-7)
  - `test_missing_method_node_warns_and_continues` — bogus `method_node_id` → flow node still created, FLOW_ENTRY skipped, WARNING logged (covers AC-12)
  - `test_http_flow_node_properties` — verify all expected props present, no `explanation_business`/`_technical`/`_search` (covers AC-14)
  - `test_clear_flows_removes_legacy_flow_step` — pre-seed Neo4j with a fake `:Flow` and `[:FLOW_STEP]` edge, run `clear_flows`, assert no FLOW_STEP edges remain (defensive — covers AC-4 across deployments)

- [ ] **Task D3 — Create `tests/test_cli_flow_commands.py`**:
  - `test_kloc_help_excludes_removed_commands` — invoke `kloc --help` via subprocess; assert `flow-diagram`/`explain-flow`/`enrich-flows` strings absent (covers AC-8)
  - `test_kloc_help_includes_import_flows` — assert `import-flows` is still listed (regression)

- [ ] **Task D4 — Create `tests/test_mcp_flow_tools.py`** (or add to existing `test_handlers.py`):
  - `test_mcp_list_tools_excludes_explain_flow` — instantiate the MCP handler, call `list_tools()`, assert no tool has name `kloc_explain_flow` (covers AC-9)
  - `test_mcp_list_tools_excludes_flow_diagram` — same for `kloc_flow_diagram` (covers AC-9)
  - `test_mcp_list_tools_keeps_kloc_import_flows` — assert `kloc_import_flows` still listed (regression)
  - `test_qdrant_collections_no_flow_entries` — `from src.db.qdrant_store import COLLECTIONS; assert no key startswith "flow_"` (covers AC-10 at unit level)
  - `test_pipelines_search_collections_no_flow_entries` — `from src.ai.pipelines import ALL_SEARCH_COLLECTIONS; assert no entry startswith "flow_"` (covers AC-10 at unit level)

---

## File Manifest

| Action | File Path | Description | Owner | Stream |
|--------|-----------|-------------|-------|--------|
| DELETE | `/Users/michal/dev/ai/kloc/kloc-intelligence/src/ai/flow_enricher.py` | 290 LOC | developer-1 | A |
| DELETE | `/Users/michal/dev/ai/kloc/kloc-intelligence/src/ai/flow_diagram.py` | 289 LOC | developer-1 | A |
| MODIFY | `/Users/michal/dev/ai/kloc/kloc-intelligence/src/cli.py` | Strip 4 commands + helper; rewire `import-flows` summary | developer-1 | A |
| MODIFY | `/Users/michal/dev/ai/kloc/kloc-intelligence/src/server/mcp.py` | Strip 2 tool defs + 2 handlers + 2 dispatch entries; rewire `kloc_import_flows` description + return | developer-1 | A |
| MODIFY | `/Users/michal/dev/ai/kloc/kloc-intelligence/src/ai/pipelines.py` | Remove FLOW_*_SYSTEM/TEMPLATE constants; remove 3 entries from `ALL_SEARCH_COLLECTIONS` | developer-1 | A |
| MODIFY | `/Users/michal/dev/ai/kloc/kloc-intelligence/src/db/qdrant_store.py` | Remove 3 entries from `COLLECTIONS` dict | developer-1 | A |
| REPLACE | `/Users/michal/dev/ai/kloc/kloc-intelligence/src/db/flow_importer.py` | Replace 197 LOC with new ~80-100 LOC parser-importer | developer-2 | B |
| CREATE | `/Users/michal/dev/ai/kloc/kloc-intelligence/scripts/drop_flow_collections.py` | One-time Qdrant cleanup | developer-2 | B |
| CREATE | `/Users/michal/dev/ai/kloc/kloc-intelligence/scripts/sweep_flow_step_edges.py` | Defensive Neo4j FLOW_STEP sweep | developer-2 | B |
| CREATE | `/Users/michal/dev/ai/kloc/kloc-intelligence/tests/test_flow_importer.py` | Parser unit tests + Neo4j integration tests | developer-2 | D |
| CREATE | `/Users/michal/dev/ai/kloc/kloc-intelligence/tests/test_cli_flow_commands.py` | CLI help smoke tests | developer-2 | D |
| CREATE | `/Users/michal/dev/ai/kloc/kloc-intelligence/tests/test_mcp_flow_tools.py` | MCP list_tools + COLLECTIONS unit tests | developer-2 | D |
| UNCHANGED | `/Users/michal/dev/ai/kloc/kloc-intelligence/src/db/schema.py` | `flow_id` and `flow_type` indexes stay | — | — |

---

## File Ownership Suggestion (final, lead-approved split)

| Stream | Developer | Files (full ownership, non-overlapping) | Phase |
|--------|-----------|------------------------------------------|-------|
| A — Demolition + Rewire | developer-1 | `src/cli.py`, `src/server/mcp.py`, `src/ai/pipelines.py`, `src/db/qdrant_store.py`, `src/ai/flow_enricher.py` (DELETE), `src/ai/flow_diagram.py` (DELETE) | 1 |
| B — New importer + Cleanup ops | developer-2 | `src/db/flow_importer.py`, `scripts/drop_flow_collections.py` (CREATE), `scripts/sweep_flow_step_edges.py` (CREATE) | 2 |
| D — Tests | developer-2 | `tests/test_flow_importer.py` (CREATE), `tests/test_cli_flow_commands.py` (CREATE), `tests/test_mcp_flow_tools.py` (CREATE) | 4 |

**Critical: zero file overlap between Stream A and Stream B.** Both streams may run from t=0 in parallel. Stream D blocks on both A and B. The convergence point is the smoke test (§Smoke Test below).

---

## Test Cases (descriptive, mapped to ACs)

### Unit (no Neo4j, no Qdrant)

| Test | Verifies AC | Description |
|------|-------------|-------------|
| `test_parse_flows_reference_project_counts` | AC-2, AC-3 (parse layer) | Loading `symfony-kloc.json` → 9 flow nodes, 9 flow_entry edges, 3 flow_triggers edges |
| `test_parse_flows_no_flow_step_edges` | AC-4 (parse layer) | No edge has `type == "flow_step"` |
| `test_parse_flows_filters_non_app_flows` | regression | Synthetic Symfony\\ flow excluded from output |
| `test_parse_flows_http_name_is_get_route` | AC-14 | `name == "GET /api/orders/{id}"` for OrderController::get |
| `test_parse_flows_per_type_properties_set` | AC-14 | type-specific props on right flows |
| `test_qdrant_collections_no_flow_entries` | AC-10 | `COLLECTIONS` no longer references flow_* |
| `test_pipelines_search_collections_no_flow_entries` | AC-10 | `ALL_SEARCH_COLLECTIONS` no longer references flow_* |

### Integration (`@requires_neo4j`)

| Test | Verifies AC | Description |
|------|-------------|-------------|
| `test_import_creates_9_flow_nodes` | AC-2 | `MATCH (f:Flow) RETURN count(f) == 9` |
| `test_import_creates_3_flow_triggers_edges` | AC-3 | 3 `:FLOW_TRIGGERS` edges, correct source/target |
| `test_import_creates_0_flow_step_edges` | AC-4 | `MATCH ()-[r:FLOW_STEP]->() RETURN count(r) == 0` |
| `test_flow_entry_to_correct_node_id` | AC-5 | `node:779b5ec2e2f2e61b` |
| `test_idempotent_double_import` | AC-7 | Two imports → 9 flow nodes, 9 FLOW_ENTRY edges |
| `test_missing_method_node_warns_skips_edge` | AC-12 | bogus method_node_id → flow created, FLOW_ENTRY skipped, WARNING logged |
| `test_http_flow_no_explanation_props` | AC-14 | http flow has expected props but no explanation_business/_technical/_search |
| `test_clear_flows_removes_legacy_flow_step` | AC-4 (deployment) | Pre-seed FLOW_STEP edge → `clear_flows` removes it |

### CLI / MCP smoke

| Test | Verifies AC | Description |
|------|-------------|-------------|
| `test_kloc_help_excludes_removed_commands` | AC-8 | `kloc --help` excludes `flow-diagram`/`explain-flow`/`enrich-flows` |
| `test_kloc_help_includes_import_flows` | regression | `import-flows` still listed |
| `test_mcp_list_tools_excludes_explain_flow` | AC-9 | MCP list_tools no `kloc_explain_flow` |
| `test_mcp_list_tools_excludes_flow_diagram` | AC-9 | MCP list_tools no `kloc_flow_diagram` |
| `test_mcp_list_tools_keeps_kloc_import_flows` | regression | MCP list_tools still includes `kloc_import_flows` |

### End-to-end (manual / smoke test only — not pytest)

| Test | Verifies AC | Description |
|------|-------------|-------------|
| Reference project full pipeline | AC-1, AC-6, AC-11, AC-13 | See §Smoke Test |

---

## Smoke Test (full end-to-end, AC-1 through AC-14)

Run from `/Users/michal/dev/ai/kloc/kloc-intelligence` after both Streams A and B land. Requires Neo4j + Qdrant running (per `docker-compose.yml`).

```bash
cd /Users/michal/dev/ai/kloc/kloc-intelligence
SYMFONY_KLOC=/Users/michal/dev/ai/kloc/kloc-reference-project-php/.kloc/symfony-kloc.json

# === AC-1: deleted files are gone ===
test ! -e src/ai/flow_enricher.py && echo "OK AC-1a: flow_enricher.py absent" || echo "FAIL"
test ! -e src/ai/flow_diagram.py  && echo "OK AC-1b: flow_diagram.py absent"  || echo "FAIL"

# === AC-8: kloc --help no longer lists removed commands ===
kloc --help 2>&1 | grep -E '(flow-diagram|explain-flow|enrich-flows)' \
    && echo "FAIL AC-8" || echo "OK AC-8: removed commands absent"
kloc --help 2>&1 | grep -q 'import-flows' && echo "OK regression: import-flows kept" || echo "FAIL"

# === AC-2, AC-3, AC-4, AC-7: import + counts ===
kloc import-flows "$SYMFONY_KLOC"
# Expected last stdout line: "Imported 9 flows, 9 FLOW_ENTRY edges, 3 FLOW_TRIGGERS edges in 0.Xs"

cypher-shell -u neo4j -p test "MATCH (f:Flow) RETURN count(f) AS flows;"
# Expected: 9   (AC-2)

cypher-shell -u neo4j -p test "MATCH ()-[r:FLOW_TRIGGERS]->() RETURN count(r) AS triggers;"
# Expected: 3   (AC-3)

cypher-shell -u neo4j -p test "MATCH ()-[r:FLOW_STEP]->() RETURN count(r) AS steps;"
# Expected: 0   (AC-4 — CRITICAL)

cypher-shell -u neo4j -p test "MATCH ()-[r:FLOW_ENTRY]->() RETURN count(r) AS entries;"
# Expected: 9

# === AC-5: FLOW_ENTRY for OrderController::get ===
cypher-shell -u neo4j -p test "
MATCH (f:Flow {flow_id: 'flow:http:App\\\\Ui\\\\Rest\\\\Controller\\\\OrderController::get'})-[:FLOW_ENTRY]->(n:Node)
RETURN n.node_id AS nid;"
# Expected: "node:779b5ec2e2f2e61b"

# === AC-7: idempotency ===
kloc import-flows "$SYMFONY_KLOC"
cypher-shell -u neo4j -p test "MATCH (f:Flow) RETURN count(f);"
# Expected: 9 (no duplicates)
cypher-shell -u neo4j -p test "MATCH ()-[r:FLOW_ENTRY]->() RETURN count(r);"
# Expected: 9 (no duplicates)

# === AC-14: http flow has expected props, no explanations ===
cypher-shell -u neo4j -p test "
MATCH (f:Flow {flow_id: 'flow:http:App\\\\Ui\\\\Rest\\\\Controller\\\\OrderController::get'})
RETURN f.flow_id, f.type, f.entry_fqn, f.entry_method, f.name, f.route, f.http_methods,
       f.explanation_business IS NULL AS no_biz,
       f.explanation_technical IS NULL AS no_tech,
       f.explanation_search IS NULL AS no_search;"
# Expected: name='GET /api/orders/{id}', route='/api/orders/{id}',
#           no_biz=true, no_tech=true, no_search=true

# === AC-6 (manual): agent-on-demand call tree purity ===
# Start kloc mcp-server, call kloc_context with symbol "App\Ui\Rest\Controller\OrderController::get".
# Expected included: OrderService::getOrder, OrderRepositoryInterface::findById,
#                    InMemoryOrderRepository (via overrides), OrderOutput, OrderResponse
# Expected EXCLUDED: EmailSender, InventoryChecker, OrderProcessor

# === AC-9 (manual): MCP server tool list ===
# Inspect MCP server tool list; assert kloc_explain_flow + kloc_flow_diagram are absent.

# === AC-10: COLLECTIONS / ALL_SEARCH_COLLECTIONS unit-level ===
python -c "from src.db.qdrant_store import COLLECTIONS; assert all(not k.startswith('flow_') for k in COLLECTIONS), COLLECTIONS; print('OK AC-10a')"
python -c "from src.ai.pipelines import ALL_SEARCH_COLLECTIONS; assert all(not c.startswith('flow_') for c in ALL_SEARCH_COLLECTIONS); print('OK AC-10b')"

# === AC-11: Qdrant collection cleanup ===
python scripts/drop_flow_collections.py
curl -s http://localhost:6333/collections | jq -r '.result.collections[].name' | grep '^flow_' \
    && echo "FAIL AC-11" || echo "OK AC-11: flow_* collections gone from Qdrant"

# === AC-12: missing method_node_id tolerance (manual) ===
# Run: kloc import-flows /tmp/test-bad.json   where /tmp/test-bad.json has a flow with
# entry.method_node_id pointing to a non-existent node. Expected: WARNING logged, exit 0,
# the :Flow node created, no FLOW_ENTRY edge from it.

# === AC-13: kloc_import_flows MCP tool invokes new importer ===
# Manual: send MCP tool call kloc_import_flows with path; expect return:
#   {"status": "ok", "flows": 9, "flow_entry_edges": 9, "flow_triggers_edges": 3}
# (Old importer would return {"status": "ok", "flows": N, "edges": M} — breaking-change check.)

# === Final: pytest with no regressions ===
python -m pytest tests/ -x
```

### Pass criteria
Every numbered check passes. The CRITICAL one is AC-4 (`MATCH ()-[r:FLOW_STEP]->() RETURN count(r) == 0`) — failure here means `FLOW_STEP` leaked back into the new model.

---

## Verification Commands (lead/QA convenience)

```bash
# Sanity: deleted files are gone
ls /Users/michal/dev/ai/kloc/kloc-intelligence/src/ai/flow_enricher.py 2>/dev/null && echo FAIL || echo OK
ls /Users/michal/dev/ai/kloc/kloc-intelligence/src/ai/flow_diagram.py  2>/dev/null && echo FAIL || echo OK

# Sanity: no remaining grep hits for removed identifiers
grep -rn "FlowEnricher\|FlowDiagramBuilder\|FlowInfo\|FlowStepInfo\|FlowEnrichmentProgress" \
    /Users/michal/dev/ai/kloc/kloc-intelligence/src/ /Users/michal/dev/ai/kloc/kloc-intelligence/tests/
# Expected: no output

grep -rn "FLOW_BUSINESS\|FLOW_TECHNICAL\|FLOW_SEARCH\|FLOW_LABELS" \
    /Users/michal/dev/ai/kloc/kloc-intelligence/src/
# Expected: no output

grep -rn "flow_business_embeddings\|flow_technical_embeddings\|flow_search_embeddings" \
    /Users/michal/dev/ai/kloc/kloc-intelligence/src/
# Expected: no output

grep -rn "flow_step\|FLOW_STEP" /Users/michal/dev/ai/kloc/kloc-intelligence/src/
# Expected: no output (parser-layer guarantee that no flow_step is ever emitted)

# Sanity: kept commands/tools
grep -n "import-flows" /Users/michal/dev/ai/kloc/kloc-intelligence/src/cli.py        # ≥1 hit
grep -n "kloc_import_flows" /Users/michal/dev/ai/kloc/kloc-intelligence/src/server/mcp.py  # ≥3 hits

# Tests pass
cd /Users/michal/dev/ai/kloc/kloc-intelligence && python -m pytest tests/ -x

# Schema indexes still defined
grep -n "flow_id\|flow_type" /Users/michal/dev/ai/kloc/kloc-intelligence/src/db/schema.py
# Expected: 2 hits (lines 55-56)

# Stream B isolation check (Stream A doesn't import the right names)
python -c "from src.db.flow_importer import load_symfony_kloc, parse_flows, import_flow_nodes, import_flow_edges, clear_flows; print('OK')"

# End-to-end smoke (after Qdrant + Neo4j running)
SYMFONY_KLOC=/Users/michal/dev/ai/kloc/kloc-reference-project-php/.kloc/symfony-kloc.json
kloc import-flows "$SYMFONY_KLOC"
```

---

## Risks & Mitigations

| ID | Risk | Likelihood | Impact | Mitigation |
|----|------|-----------|--------|-----------|
| R1 | `kloc_search` runtime error if a Qdrant pipeline is built for a deleted collection | Low | Medium | `search_both_collections` already silences `Exception` on missing collection (`pipelines.py:393-394`). Stream A removes the names from `ALL_SEARCH_COLLECTIONS` so pipelines aren't built in the first place. AC-10 covers. |
| R2 | Hidden import of deleted `flow_enricher` / `flow_diagram` causes `ImportError` at runtime | Low | High | All imports are lazy (inside command/handler functions). Stream A removes the only known import sites in tasks A3, A4 BEFORE deletion in A5, A6. Run `pytest tests/ -x` after Stream A converges; any leftover surfaces immediately. |
| R3 | Stale `FLOW_STEP` edges persist in Neo4j after a partial deploy | Medium | High | New importer always calls `clear_flows()` first (DETACH DELETE removes all flow-attached relationships, including `FLOW_STEP`). New importer never emits `flow_step` type. AC-4 verifies post-import. Belt-and-suspenders: Task B3 sweep script. |
| R4 | Agent-on-demand model insufficient — agents don't reliably chain `kloc_context` after `kloc_import_flows` | Low-Medium | Medium | Out of scope for MVP (no new MCP tool). Mitigation deferred to STRETCH ACs 15-16. The `:Flow` node still has `entry_method` + `FLOW_ENTRY` edge → agents can pivot via Cypher. |
| R5 | Test fixtures import a deleted symbol → entire suite fails to collect | Very Low | High | Phase 1 grep confirmed: tests/ has zero references to flow_enricher/flow_diagram/FLOW_*/kloc_explain_flow. Run `pytest --collect-only` after Stream A. |
| R6 | Stream A and Stream B integration drift — A's call sites import names that B never exports | Medium | High | Mitigation: §Interface Contract is the single source of truth for both streams. Stream B's Task B4 (isolation check) confirms the public API matches before A and B converge. Smoke test catches any drift end-to-end. |
| R7 | New importer reads a shape that kloc-symfony evolves before this lands | Low | High | Spec freezes the input shape against the canonical reference file. Test D1 uses that file as fixture — any kloc-symfony shape drift breaks D1 immediately. |
| R8 | `:Flow` node's `name` is empty for malformed input (no http_methods, no route) | Low | Low | Stream B defaults `name` to `flow_id` short form if per-type derivation is empty. Defensive — canonical input always has fields. |
| R9 | `MERGE (f:Flow {flow_id}) SET f += props` leaves stale properties on re-import if a flow shrinks its prop set | Low | Low | `clear_flows()` runs first on every import — stale state never carries across runs. MERGE+SET serves as in-run idempotency only. Documented in importer module docstring. |
| R10 | One-shot scripts in `scripts/` get called twice and error | Low | Low | `drop_flow_collections.py` already has try/except per collection. `sweep_flow_step_edges.py` is naturally idempotent (DELETE on empty match is a no-op). |

---

## Open Questions (for PM/lead)

These are items the spec does not fully resolve. Defaults proposed; flag for confirmation:

1. **`MERGE` vs `CREATE` for `:Flow` import** — current importer uses `CREATE`. Spec §Idempotency says re-runs must produce identical state, satisfied via `clear_flows()`. **Default: MERGE-by-`flow_id`** (defensive against partial failures within a single import).

2. **Default for `clear` flag on `import-flows`** — current CLI command has `clear: bool = typer.Option(True)`. Spec §Idempotency strongly implies clear-always. **Default: keep flag at `True`** but document that `False` violates AC-7. Alternative: remove the flag entirely.

3. **STRETCH ACs 15-16 (`kloc_flows` + `kloc_flow` MCP tools)** — explicitly out of MVP per spec §Out of Scope. **Default: not in this PR.** Confirm exclusion.

4. **`scripts/drop_flow_collections.py` and `scripts/sweep_flow_step_edges.py` checked in permanently?** — One-time cleanup scripts. **Default: yes, keep in `scripts/` indefinitely** (zero maintenance, useful template). Alternative: delete after first run.

5. **Backward-compat for old-shape `symfony-kloc.json` (with `chain[]`)** — spec §Out of Scope says no. The new importer will silently ignore `chain[]` if present. **Default: graceful ignore (no warning, no error).** Alternative: detect old shape and error.

6. **App-flow filter literal** — both old and new code filter by `fqn.startswith("App\\")`. The reference project uses `App\\` namespace per its CLAUDE.md. **Default: hardcode `App\\`** (matches old behavior; not in this PR's scope to generalize).

7. **`scripts/` directory location** — does `kloc-intelligence/scripts/` already exist? If not, Task B2/B3 create it. Confirm path is acceptable (alternatives: `bin/`, `tools/`).

---

## Definition of Done

- [ ] All 14 must-have ACs (1-14) verified via smoke test or pytest.
- [ ] Stretch ACs 15-16 explicitly excluded (out of scope per spec).
- [ ] Both deleted files absent from repo (`git status` shows them as deleted).
- [ ] `pytest tests/ -x` passes with zero errors and only the existing `requires_neo4j`-related skips.
- [ ] Smoke test §AC-1 through AC-14 all pass.
- [ ] Code review approves: no `FLOW_STEP` references in any source file; new `flow_importer.py` ≤ 120 LOC; module-level API matches §Interface Contract.
- [ ] Stream A and Stream B each landed by their sole owner (no cross-stream commits to a file).
- [ ] Verification commands §Verification block run cleanly by lead/QA.
