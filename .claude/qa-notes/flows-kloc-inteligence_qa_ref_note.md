# QA Notes: flows-kloc-inteligence (spec v2)

**Spec:** `/Users/michal/dev/ai/kloc/docs/specs/flows-kloc-inteligence.md` (20 ACs: 18 MVP + 2 STRETCH)
**Canonical input:** `/Users/michal/dev/ai/kloc/kloc-reference-project-php/.kloc/symfony-kloc.json` (9 flows, 3 triggers, App-only)
**CLI binary:** `kloc-intelligence` (entry point in `pyproject.toml`)
**Stack:** Neo4j (`bolt://localhost:7687`, user `neo4j`, pw `kloc-intelligence`) + Qdrant (`http://localhost:6333`)

---

## Reference oracle (from spec §Reference Oracle, lines 19–47)

After a clean `import-flows` against the canonical input, Neo4j must hold:

| Metric | Expected |
|--------|----------|
| `:Flow` nodes | **9** |
| `FLOW_ENTRY` edges | **9** |
| `FLOW_TRIGGERS` edges | **3** |
| `FLOW_STEP` edges | **0** |

The 9 `flow_id` values:

```
flow:http:App\Ui\Rest\Controller\CustomerController::get
flow:http:App\Ui\Rest\Controller\CustomerController::summary
flow:http:App\Ui\Rest\Controller\OrderController::get
flow:http:App\Ui\Rest\Controller\OrderController::create
flow:message:App\Ui\Messenger\Handler\OrderCreatedHandler::__invoke
flow:event:App\Ui\EventSubscriber\OrderEventSubscriber::onOrderCreated[OrderCreatedEvent]
flow:event:App\Ui\EventSubscriber\ReportEventSubscriber::onReportGenerated[ReportGeneratedEvent]
flow:cli:App\Ui\Console\ProcessOrdersCommand::execute
flow:cli:App\Ui\Console\ProcessReportsCommand::execute
```

The 3 `FLOW_TRIGGERS` edges (source → target, via):

| source | target | trigger_type | via |
|--------|--------|--------------|-----|
| `flow:http:...OrderController::create` | `flow:event:...OrderEventSubscriber::onOrderCreated[...]` | `event` | `App\Event\OrderCreatedEvent` |
| `flow:cli:...ProcessReportsCommand::execute` | `flow:event:...ReportEventSubscriber::onReportGenerated[...]` | `event` | `App\Event\ReportGeneratedEvent` |
| `flow:http:...OrderController::create` | `flow:message:...OrderCreatedHandler::__invoke` | `message` | `App\Ui\Messenger\Message\OrderCreatedMessage` |

---

## Pre-flight: environment setup

These commands MUST succeed before any AC validation runs.

```bash
# 1. Bring up Neo4j + Qdrant
cd /Users/michal/dev/ai/kloc/kloc-intelligence
docker compose up -d
docker compose ps   # both kloc-neo4j and kloc-qdrant must be 'healthy'

# 2. Confirm CLI is installed and resolvable
which kloc-intelligence
kloc-intelligence --help   # exit 0

# 3. Initialize fresh schema (creates indexes, including flow_id and flow_type)
kloc-intelligence schema reset

# 4. Confirm reference fixture exists
test -f /Users/michal/dev/ai/kloc/kloc-reference-project-php/.kloc/symfony-kloc.json && echo OK
```

**Pre-flight pass:** all four commands exit 0; both containers healthy.

---

## Validation per acceptance criterion

> All counts assume a clean import of the reference file *immediately preceding* the assertion. Per D5, `import-flows` always calls `clear_flows()` first, so no manual reset is required between runs.

### AC-1: 9 :Flow nodes

```bash
kloc-intelligence import-flows /Users/michal/dev/ai/kloc/kloc-reference-project-php/.kloc/symfony-kloc.json
docker compose exec -T neo4j cypher-shell -u neo4j -p kloc-intelligence \
  "MATCH (f:Flow) RETURN count(f) AS n"
# expect: n = 9
```

Belt-and-braces — verify the 9 `flow_id` values exactly:

```bash
docker compose exec -T neo4j cypher-shell -u neo4j -p kloc-intelligence \
  "MATCH (f:Flow) RETURN f.flow_id AS id ORDER BY id"
# expect: 9 lines matching the oracle list above
```

**PASS:** `n = 9` AND all 9 oracle `flow_id` strings present, no extras.

### AC-2: 9 FLOW_ENTRY edges

```bash
docker compose exec -T neo4j cypher-shell -u neo4j -p kloc-intelligence \
  "MATCH ()-[r:FLOW_ENTRY]->() RETURN count(r) AS n"
# expect: n = 9
```

**PASS:** `n = 9`.

### AC-3: 3 FLOW_TRIGGERS edges

```bash
docker compose exec -T neo4j cypher-shell -u neo4j -p kloc-intelligence \
  "MATCH ()-[r:FLOW_TRIGGERS]->() RETURN count(r) AS n"
# expect: n = 3
```

Detail check — verify the 3 expected (source, target, via) tuples:

```bash
docker compose exec -T neo4j cypher-shell -u neo4j -p kloc-intelligence \
"MATCH (s:Flow)-[r:FLOW_TRIGGERS]->(t:Flow)
 RETURN s.flow_id AS source, t.flow_id AS target, r.trigger_type AS ttype, r.via AS via
 ORDER BY r.via"
```

Cross-check against oracle table above. Per spec line 343 the general formula is `Σ |sources_i| × |targets_i|`; the reference project's per-trigger 1×1 fan-out yields exactly 3.

**PASS:** `n = 3` AND all 3 oracle rows present.

### AC-4: 0 FLOW_STEP edges (demolition signal)

```bash
docker compose exec -T neo4j cypher-shell -u neo4j -p kloc-intelligence \
  "MATCH ()-[r:FLOW_STEP]->() RETURN count(r) AS n"
# expect: n = 0
```

**PASS:** `n = 0`. Critical regression — non-zero means demolition is incomplete.

### AC-5: D7 canonical regression — OrderController::get only has FLOW_ENTRY

This is the structural regression test per spec D7 (line 114). The Cypher form is firmer than a `kloc-intelligence context` walk because it asserts on the actual graph state:

```bash
docker compose exec -T neo4j cypher-shell -u neo4j -p kloc-intelligence \
"MATCH (:Flow {flow_id: 'flow:http:App\\\\Ui\\\\Rest\\\\Controller\\\\OrderController::get'})-[r]->()
 RETURN type(r) AS rel_type, count(r) AS cnt"
```

**Expected output (exactly one row):**

| rel_type     | cnt |
|--------------|-----|
| `FLOW_ENTRY` | 1   |

No `FLOW_STEP` row exists. No other relationship types appear.

**PASS:** exactly one row; `rel_type = "FLOW_ENTRY"`; `cnt = 1`.

### AC-6: OrderController::get FLOW_ENTRY target has correct node_id

```bash
docker compose exec -T neo4j cypher-shell -u neo4j -p kloc-intelligence \
"MATCH (:Flow {flow_id: 'flow:http:App\\\\Ui\\\\Rest\\\\Controller\\\\OrderController::get'})-[:FLOW_ENTRY]->(n:Node)
 RETURN n.node_id AS nid"
# expect: nid = 'node:779b5ec2e2f2e61b'
```

**PASS:** nid equals `node:779b5ec2e2f2e61b`.

**Optional smoke** (not part of AC, but useful evidence of agent-on-demand pivot):

```bash
kloc-intelligence context node:779b5ec2e2f2e61b --depth 4 > /tmp/order_get_context.txt
grep -E "EmailSender|InventoryChecker|OrderProcessor[^I]" /tmp/order_get_context.txt \
  && echo "WARN: forbidden class found via context walk" \
  || echo "OK: forbidden classes absent in context walk"
```

### AC-7: Idempotency — double import yields same counts

Per D5, `import-flows` always clears first. The `--no-clear` flag from the v1 spec no longer exists.

```bash
kloc-intelligence import-flows /Users/michal/dev/ai/kloc/kloc-reference-project-php/.kloc/symfony-kloc.json
kloc-intelligence import-flows /Users/michal/dev/ai/kloc/kloc-reference-project-php/.kloc/symfony-kloc.json

docker compose exec -T neo4j cypher-shell -u neo4j -p kloc-intelligence \
"MATCH (f:Flow) WITH count(f) AS flows
 MATCH ()-[e:FLOW_ENTRY]->() WITH flows, count(e) AS entries
 MATCH ()-[t:FLOW_TRIGGERS]->() RETURN flows, entries, count(t) AS triggers"
# expect: flows=9, entries=9, triggers=3
```

**PASS:** counts match oracle after second run.

### AC-8: FLOW_TRIGGERS edge properties — required keys present, forbidden keys absent

```bash
docker compose exec -T neo4j cypher-shell -u neo4j -p kloc-intelligence \
"MATCH ()-[r:FLOW_TRIGGERS]->() RETURN keys(r) AS k ORDER BY r.via"
# expect: every row's keys list = ['trigger_type', 'via'] (set comparison)
```

```bash
# Negative — call-tree leftover properties must be absent
docker compose exec -T neo4j cypher-shell -u neo4j -p kloc-intelligence \
"MATCH ()-[r:FLOW_TRIGGERS]->()
 WHERE 'dispatcher_method_node_id' IN keys(r) OR 'call_node_id' IN keys(r) OR 'flow_entry_node_id' IN keys(r)
 RETURN count(r) AS n"
# expect: n = 0
```

**PASS:** every edge has exactly `{trigger_type, via}`; n = 0 for forbidden keys.

### AC-9: :Flow node properties — OrderController::get http example

```bash
docker compose exec -T neo4j cypher-shell -u neo4j -p kloc-intelligence \
"MATCH (f:Flow {flow_id: 'flow:http:App\\\\Ui\\\\Rest\\\\Controller\\\\OrderController::get'})
 RETURN keys(f) AS keys, f.type AS type, f.entry_fqn AS fqn, f.entry_method AS method,
        f.name AS name, f.route AS route, f.http_methods AS methods"
```

**Expected:**
- `type = "http"`
- `fqn = "App\\Ui\\Rest\\Controller\\OrderController"`
- `method = "get"`
- `name = "GET /api/orders/{id}"`
- `route = "/api/orders/{id}"`
- `methods = ["GET"]`
- `keys` ⊇ `{flow_id, type, entry_fqn, entry_method, name, route, http_methods}`

```bash
# Negative — no enrichment properties on any :Flow node
docker compose exec -T neo4j cypher-shell -u neo4j -p kloc-intelligence \
"MATCH (f:Flow)
 WHERE 'explanation_business' IN keys(f) OR 'explanation_technical' IN keys(f) OR 'explanation_search' IN keys(f)
 RETURN count(f) AS n"
# expect: n = 0
```

Per D3, type-specific keys must be present only on the right flow types. Spot check `message` flows:

```bash
docker compose exec -T neo4j cypher-shell -u neo4j -p kloc-intelligence \
"MATCH (f:Flow {type: 'message'})
 RETURN f.flow_id AS id, f.message_class AS msg, exists(f.route) AS has_route,
        exists(f.event_name) AS has_event, exists(f.command_name) AS has_cmd"
# expect: msg non-null; has_route=false; has_event=false; has_cmd=false
```

**PASS:** http example values match exactly; n = 0 for forbidden keys; type-specific properties present only where required.

### AC-10: Deletion of flow_enricher / flow_diagram does not break imports / pytest

```bash
test ! -f /Users/michal/dev/ai/kloc/kloc-intelligence/src/ai/flow_enricher.py && echo "PASS: enricher absent"
test ! -f /Users/michal/dev/ai/kloc/kloc-intelligence/src/ai/flow_diagram.py  && echo "PASS: diagram absent"

# No remaining references in src/
grep -rn "flow_enricher\|FlowEnricher\|flow_diagram\|FlowDiagramBuilder" /Users/michal/dev/ai/kloc/kloc-intelligence/src/
echo "exit=$?"  # expect 1 (no matches)

# pytest must succeed (no ModuleNotFoundError)
cd /Users/michal/dev/ai/kloc/kloc-intelligence
python -m pytest tests/ --collect-only 2>&1 | tail -20
# expect: no ModuleNotFoundError; collection succeeds

# CLI help must succeed
kloc-intelligence --help; echo "exit=$?"
# expect: exit 0
```

**PASS:** both files absent; grep no matches; pytest collection clean; `--help` exit 0.

### AC-11: CLI help — removed commands absent, kept commands present

```bash
kloc-intelligence --help > /tmp/cli_help.txt

for cmd in flow-diagram explain-flow enrich-flows; do
  if grep -qw "$cmd" /tmp/cli_help.txt; then
    echo "FAIL: $cmd still listed"
  else
    echo "PASS: $cmd absent"
  fi
done

grep -qw "import-flows" /tmp/cli_help.txt && echo "PASS: import-flows present"
```

**PASS:** all 3 removed cmds absent; `import-flows` present.

### AC-12: MCP tool list — removed tools absent, kept tools present

```bash
python -c "
from src.server.mcp import MCPServer
import json
s = MCPServer()
tools = [t['name'] for t in s.list_tools()]
print(json.dumps(tools, indent=2))
" > /tmp/mcp_tools.txt

for tool in kloc_explain_flow kloc_flow_diagram; do
  grep -qw "$tool" /tmp/mcp_tools.txt && echo "FAIL: $tool still listed" || echo "PASS: $tool absent"
done

grep -qw "kloc_import_flows" /tmp/mcp_tools.txt && echo "PASS: kloc_import_flows present"
```

**PASS:** both removed tools absent; `kloc_import_flows` present.

### AC-13: Qdrant flow_* collections deleted from running instance after import

Per D4, `import-flows` actively deletes the three flow collections before `clear_flows()`. After running it, `GET /collections` must show their absence.

```bash
# Plant the collections so deletion has something to act on
for col in flow_business_embeddings flow_technical_embeddings flow_search_embeddings; do
  curl -s -X PUT "http://localhost:6333/collections/$col" \
    -H 'Content-Type: application/json' \
    -d '{"vectors":{"size":4,"distance":"Cosine"}}'
done

# Run import — must delete them
kloc-intelligence import-flows /Users/michal/dev/ai/kloc/kloc-reference-project-php/.kloc/symfony-kloc.json

# Verify absence
curl -s http://localhost:6333/collections | python -m json.tool > /tmp/qdrant_collections.json

for col in flow_business_embeddings flow_technical_embeddings flow_search_embeddings; do
  if grep -q "\"$col\"" /tmp/qdrant_collections.json; then
    echo "FAIL: $col still present in Qdrant"
  else
    echo "PASS: $col absent"
  fi
done
```

Idempotency on cold instance — re-run when collections are already absent:

```bash
kloc-intelligence import-flows /Users/michal/dev/ai/kloc/kloc-reference-project-php/.kloc/symfony-kloc.json; echo "exit=$?"
# expect: exit 0; no error from missing-collection delete
```

**PASS:** all 3 flow_* collections absent post-import; second run with collections already absent exits 0.

### AC-14: Registry constants pruned

```bash
# qdrant_store.py — COLLECTIONS dict
python -c "from src.db.qdrant_store import COLLECTIONS; print(list(COLLECTIONS))"
# expect: NO 'flow_business_embeddings', 'flow_technical_embeddings', 'flow_search_embeddings'

# pipelines.py — ALL_SEARCH_COLLECTIONS
python -c "from src.ai.pipelines import ALL_SEARCH_COLLECTIONS; print(ALL_SEARCH_COLLECTIONS)"
# expect: NO flow_* entries; per D4 only code_embeddings, explain_embeddings

# Static grep — no literals remain
grep -n "flow_business_embeddings\|flow_technical_embeddings\|flow_search_embeddings" \
  /Users/michal/dev/ai/kloc/kloc-intelligence/src/db/qdrant_store.py \
  /Users/michal/dev/ai/kloc/kloc-intelligence/src/ai/pipelines.py
echo "exit=$?"   # expect 1
```

**PASS:** introspection shows no flow_* entries; grep no matches.

### AC-15: kloc_search succeeds after flow_* collections are deleted (D6 silent skip)

After AC-13 has run (collections gone), `kloc_search` must still return results from `code_embeddings` + `explain_embeddings` without raising.

```bash
python -c "
from src.ai.pipelines import search_both_collections
results = search_both_collections('OrderService', top_k=5)
assert len(results) > 0, 'no results returned'
collections_seen = {r.get('collection') for r in results}
assert 'flow_business_embeddings' not in collections_seen
assert 'flow_technical_embeddings' not in collections_seen
assert 'flow_search_embeddings' not in collections_seen
assert collections_seen.issubset({'code_embeddings', 'explain_embeddings'})
print('PASS: kloc_search OK, only keeper collections returned')
"
```

If `search_both_collections` is private/lazy, fall back to invoking the MCP `kloc_search` tool via stdio.

**PASS:** call succeeds; results sourced only from `code_embeddings` / `explain_embeddings`; no exception.

### AC-16: Performance ceiling — under 2 seconds (D12)

Two measurement paths — run both, they cross-validate.

**Path A — wall clock via shell `time`** (Linux uses GNU `/usr/bin/time -f`; macOS bash uses `TIMEFORMAT`):

```bash
docker compose exec -T neo4j cypher-shell -u neo4j -p kloc-intelligence "MATCH (f:Flow) DETACH DELETE f"

# Linux:
#   /usr/bin/time -f "run %e seconds" kloc-intelligence import-flows ...
# macOS bash equivalent:
for i in 1 2 3; do
  TIMEFORMAT="run $i: %R seconds"
  time kloc-intelligence import-flows \
    /Users/michal/dev/ai/kloc/kloc-reference-project-php/.kloc/symfony-kloc.json \
    > /dev/null
done 2>&1 | grep "^run"
# expect: every run < 2.00s
```

**Path B — parse CLI's own `"in 0.Xs"` line from stdout** (PM-flagged smoke; survives any host's `time` quirks, mirrors what dev-1's unit test asserts via `assert elapsed < 2.0`):

```bash
for i in 1 2 3; do
  out=$(kloc-intelligence import-flows \
    /Users/michal/dev/ai/kloc/kloc-reference-project-php/.kloc/symfony-kloc.json 2>&1)
  elapsed=$(printf '%s\n' "$out" | grep -oE 'in [0-9]+\.[0-9]+s' | head -1 | grep -oE '[0-9]+\.[0-9]+')
  python -c "
e = float('$elapsed')
assert e < 2.0, f'FAIL: run $i = {e}s exceeds 2.0s ceiling'
print(f'run $i: {e}s — PASS')
"
done
```

If Path B finds no `"in 0.Xs"` line (CLI hasn't emitted timing yet), Path A is decisive.

**PASS:** all three measured runs complete in < 2.0s by Path A AND Path B (when Path B is available). (Reference project is tiny; this is mostly a regression alarm against accidental N+1 Cypher or per-flow embedding leftovers.)

**FAIL action:** if any run ≥ 2s, capture `EXPLAIN PROFILE` for the import Cypher and report to dev for investigation.

### AC-17: Missing-node tolerance for entry.method_node_id

Smoke-level supplement to dev-2's unit coverage in `tests/test_flow_importer.py`. The unit test exercises the parse path with a mock connection; this smoke test exercises the live import path against a real Neo4j with a synthetic bogus `method_node_id`.

```bash
python <<'PY' > /tmp/bad_method_node.json
import json
src = "/Users/michal/dev/ai/kloc/kloc-reference-project-php/.kloc/symfony-kloc.json"
data = json.load(open(src))
data["flows"][0]["entry"]["method_node_id"] = "node:doesnotexist"
print(json.dumps(data, indent=2))
PY

kloc-intelligence import-flows /tmp/bad_method_node.json 2> /tmp/import_stderr.log
echo "exit=$?"   # expect 0

grep -i "warning" /tmp/import_stderr.log   # expect at least one WARNING

docker compose exec -T neo4j cypher-shell -u neo4j -p kloc-intelligence \
"MATCH (f:Flow) WITH count(f) AS flows
 MATCH ()-[e:FLOW_ENTRY]->() RETURN flows, count(e) AS entries"
# expect: flows=9, entries=8
```

**PASS:** exit 0; WARNING logged; 9 :Flow nodes; 8 FLOW_ENTRY edges.

### AC-18: Indexes survive — flow_id and flow_type both exist on :Flow

```bash
docker compose exec -T neo4j cypher-shell -u neo4j -p kloc-intelligence "SHOW INDEXES" > /tmp/indexes.txt

grep -E "flow_id\b" /tmp/indexes.txt && echo "PASS: flow_id index present"
grep -E "flow_type\b" /tmp/indexes.txt && echo "PASS: flow_type index present"

# Verify they target Flow label
grep -E "flow_id\b.*Flow" /tmp/indexes.txt && echo "PASS: flow_id targets :Flow"
grep -E "flow_type\b.*Flow" /tmp/indexes.txt && echo "PASS: flow_type targets :Flow"
```

**PASS:** both indexes present and target `:Flow`.

### AC-19 (STRETCH): kloc_flows MCP tool

If implemented, validate:

```bash
echo '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"kloc_flows","arguments":{}}}' \
  | python -m src.server.mcp \
  | python -c "
import json,sys
r = json.load(sys.stdin)['result']
assert isinstance(r, list) and len(r) == 9, f'expected 9, got {len(r)}'
for f in r:
    for k in ('flow_id','type','name','entry_fqn','entry_method'):
        assert k in f, f'missing {k}: {f}'
print('PASS: kloc_flows OK')
"
```

**PASS / SKIP:** PASS if implemented and shape matches; SKIP acceptable per spec line 425.

### AC-20 (STRETCH): kloc_flow MCP tool

```bash
echo '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"kloc_flow","arguments":{"flow_id":"flow:http:App\\\\Ui\\\\Rest\\\\Controller\\\\OrderController::create"}}}' \
  | python -m src.server.mcp \
  | python -c "
import json,sys
r = json.load(sys.stdin)['result']
assert r['flow_id'].endswith('OrderController::create'), r
assert r.get('method_node_id'), 'missing method_node_id'
trg = r.get('triggers', [])
assert len(trg) == 2, f'expected 2 triggers, got {len(trg)}'
# Spec line 382 requires a hint about kloc_context as next call
assert 'kloc_context' in json.dumps(r), 'missing kloc_context hint'
print('PASS: kloc_flow OK')
"
```

**PASS / SKIP:** PASS if implemented; SKIP acceptable.

---

## Smoke test — full pipeline (PHP source -> indexer -> mapper -> kloc-symfony -> kloc-intelligence)

End-to-end validation. Confirms upstream consumers still work after the demolition.

```bash
set -euxo pipefail

# 1. Indexer
cd /Users/michal/dev/ai/kloc/kloc-indexer-php
./run.sh /Users/michal/dev/ai/kloc/kloc-reference-project-php /tmp/scip-out

# 2. Mapper
cd /Users/michal/dev/ai/kloc/kloc-mapper
python -m src /tmp/scip-out/index.json -o /tmp/sot.json

# 3. kloc-symfony (Docker)
docker run --rm \
  -v /Users/michal/dev/ai/kloc/kloc-reference-project-php:/input \
  -v /tmp/sot.json:/sot.json \
  -v /tmp:/output \
  kloc-symfony --sot /sot.json -o /output/symfony-kloc.json

# 4. Imports into kloc-intelligence
cd /Users/michal/dev/ai/kloc/kloc-intelligence
kloc-intelligence schema reset
kloc-intelligence import /tmp/sot.json
kloc-intelligence import-flows /tmp/symfony-kloc.json

# 5. Counts must match oracle
docker compose exec -T neo4j cypher-shell -u neo4j -p kloc-intelligence \
"MATCH (f:Flow) WITH count(f) AS flows
 MATCH ()-[s:FLOW_STEP]->() WITH flows, count(s) AS steps
 MATCH ()-[t:FLOW_TRIGGERS]->() RETURN flows, steps, count(t) AS triggers"
# expect: flows=9, steps=0, triggers=3

# 6. D7 regression query
docker compose exec -T neo4j cypher-shell -u neo4j -p kloc-intelligence \
"MATCH (:Flow {flow_id: 'flow:http:App\\\\Ui\\\\Rest\\\\Controller\\\\OrderController::get'})-[r]->()
 RETURN type(r) AS rel_type, count(r) AS cnt"
# expect: single row, rel_type=FLOW_ENTRY, cnt=1
```

**PASS:** all steps exit 0; final counts match oracle; D7 returns the canonical single row.

---

## Negative tests

### N1 — Malformed JSON

```bash
echo "{ not json" > /tmp/bad.json
kloc-intelligence import-flows /tmp/bad.json; echo "exit=$?"
# expect: non-zero exit, JSON decode error in stderr
```

Verify state can recover via a clean run:

```bash
kloc-intelligence import-flows /Users/michal/dev/ai/kloc/kloc-reference-project-php/.kloc/symfony-kloc.json
docker compose exec -T neo4j cypher-shell -u neo4j -p kloc-intelligence "MATCH (f:Flow) RETURN count(f)"
# expect: 9 (clean state)
```

**PASS:** non-zero exit on malformed JSON; subsequent clean import yields oracle counts.

### N2 — Missing `flows` key

```bash
echo "{}" > /tmp/empty.json
kloc-intelligence import-flows /tmp/empty.json; echo "exit=$?"
# expect: exit 0; "Imported 0 flows" in output
docker compose exec -T neo4j cypher-shell -u neo4j -p kloc-intelligence "MATCH (f:Flow) RETURN count(f)"
# expect: 0 (clear_flows ran; nothing to import)
```

**PASS:** exit 0; 0 flows imported.

### N3 — Unknown trigger flow_id (target not in :Flow set)

```bash
python <<'PY' > /tmp/bad_trigger.json
import json
src = "/Users/michal/dev/ai/kloc/kloc-reference-project-php/.kloc/symfony-kloc.json"
data = json.load(open(src))
data["triggers"][0]["targets"][0]["flow_id"] = "flow:event:Symfony\\Bundle\\Doesnotexist"
print(json.dumps(data, indent=2))
PY

kloc-intelligence import-flows /tmp/bad_trigger.json 2> /tmp/trig_stderr.log; echo "exit=$?"
grep -i "warning" /tmp/trig_stderr.log

docker compose exec -T neo4j cypher-shell -u neo4j -p kloc-intelligence \
  "MATCH ()-[r:FLOW_TRIGGERS]->() RETURN count(r) AS n"
# expect: n = 2 (one trigger edge silently skipped, others survive)
```

**PASS:** exit 0; WARNING logged; FLOW_TRIGGERS count = 2.

### N4 — Cold Neo4j (no :Node entries to attach FLOW_ENTRY to)

```bash
docker compose exec -T neo4j cypher-shell -u neo4j -p kloc-intelligence "MATCH (n) DETACH DELETE n"
kloc-intelligence import-flows /Users/michal/dev/ai/kloc/kloc-reference-project-php/.kloc/symfony-kloc.json 2> /tmp/cold.log

docker compose exec -T neo4j cypher-shell -u neo4j -p kloc-intelligence \
"MATCH (f:Flow) WITH count(f) AS flows
 OPTIONAL MATCH ()-[e:FLOW_ENTRY]->() RETURN flows, count(e) AS entries"
# expect: flows=9, entries=0
grep -ci warning /tmp/cold.log   # expect: ≥9 (one per flow)
```

**PASS:** import survives; 9 :Flow nodes; 0 FLOW_ENTRY edges; warnings emitted.

After N4, restore the real graph with the smoke pipeline before continuing other ACs.

### N5 — Double import idempotency

Captured under AC-7. No separate command set needed.

---

## Performance baseline

Captured by AC-16. Reference numbers to record in the QA report:

| Metric | Pre-change (old importer) | Post-change (new) | Ceiling (D12) |
|--------|---------------------------|-------------------|---------------|
| Import wall time (warm) | < 1 s (per architect's measurement) | < 2 s | 2 s |
| `:Flow` nodes | 9 (with FLOW_STEP pollution) | 9 | 9 |
| `FLOW_ENTRY` edges | 9 | 9 | 9 |
| `FLOW_TRIGGERS` edges | varies (chain-derived) | 3 | 3 |
| `FLOW_STEP` edges | non-zero | **0** | **0** |
| Qdrant flow_* collections | 3 stale | absent | absent |

The most important regression signal is `FLOW_STEP = 0`. If a future change accidentally re-introduces chain parsing, this number will jump.

---

## Cross-cutting checks (from Phase 1 bonus risks)

### XC-1 — CLI and MCP `import-flows` are atomically updated

Both code paths must use the same new importer; no path can sneak in the old behavior.

```bash
# Both should import from src.db.flow_importer (no side modules)
grep -n "from .db.flow_importer\|from ..db.flow_importer\|from src.db.flow_importer" \
  /Users/michal/dev/ai/kloc/kloc-intelligence/src/cli.py \
  /Users/michal/dev/ai/kloc/kloc-intelligence/src/server/mcp.py

# Old API surface (chain, FLOW_STEP) must not appear anywhere in src/
grep -rn "FLOW_STEP\|flow_step\|chain\[" /Users/michal/dev/ai/kloc/kloc-intelligence/src/
echo "exit=$?"   # expect 1 (no matches)
```

**PASS:** both files import from the new module; no `chain[]` / `FLOW_STEP` leftovers in src/.

### XC-2 — No stale flow properties on non-Flow nodes

If any historic bug placed `flow_id` / `flow_type` properties on non-`:Flow` nodes, surface them now.

```bash
docker compose exec -T neo4j cypher-shell -u neo4j -p kloc-intelligence \
"MATCH (n) WHERE NOT n:Flow AND (exists(n.flow_id) OR exists(n.flow_type))
 RETURN labels(n) AS labels, count(*) AS n"
# expect: empty result set
```

**PASS:** no non-:Flow nodes carry flow_* properties.

### XC-3 — Docker-compose validation env stable across all ACs

Already encoded in the pre-flight section. Each AC re-uses `docker compose exec -T neo4j cypher-shell ...` — if any AC fails with a connection error, the validation env regressed and pre-flight must be re-run.

---

## Test data / fixtures

| Fixture | Use | Build |
|---------|-----|-------|
| `kloc-reference-project-php/.kloc/symfony-kloc.json` | Canonical input | already on disk |
| `/tmp/bad_method_node.json` | AC-17 missing-node tolerance | inline Python |
| `/tmp/bad.json` | N1 malformed JSON | `echo "{ not json"` |
| `/tmp/empty.json` | N2 missing `flows` key | `echo "{}"` |
| `/tmp/bad_trigger.json` | N3 unknown trigger target | inline Python |

All fixtures are throw-away (`/tmp/...`). Nothing needs committing.

---

## Pass / fail summary table

| AC  | Verifier | PASS condition |
|-----|----------|----------------|
| 1   | Cypher count | `count(:Flow) = 9` and 9 oracle flow_ids present |
| 2   | Cypher count | `count(FLOW_ENTRY) = 9` |
| 3   | Cypher count + rows | `count(FLOW_TRIGGERS) = 3`; 3 oracle rows match |
| 4   | Cypher count | `count(FLOW_STEP) = 0` |
| 5   | D7 Cypher | One row, `FLOW_ENTRY`, cnt=1 |
| 6   | Cypher MATCH | `node_id = 'node:779b5ec2e2f2e61b'` |
| 7   | Double import + count | flows=9, entries=9, triggers=3 unchanged |
| 8   | Cypher keys | Each FLOW_TRIGGERS has only `{trigger_type, via}` |
| 9   | Cypher props | http example all-keys correct; no enrichment props anywhere |
| 10  | `pytest --collect-only` + `--help` | No ModuleNotFoundError; exit 0 |
| 11  | `--help` parse | 3 removed cmds absent; import-flows present |
| 12  | MCP tools list | 2 removed tools absent; kloc_import_flows present |
| 13  | Qdrant `GET /collections` | 3 flow_* absent; idempotent on cold |
| 14  | Python introspection + grep | No flow_* in `COLLECTIONS` / `ALL_SEARCH_COLLECTIONS` |
| 15  | `search_both_collections` call | Returns from keepers only, no exception |
| 16  | `/usr/bin/time` ×3 | Every run < 2.0s |
| 17  | Bad fixture import | exit 0; WARNING; flows=9, entries=8 |
| 18  | `SHOW INDEXES` | flow_id + flow_type both target :Flow |
| 19* | MCP tools/call | 9-item list with required fields |
| 20* | MCP tools/call | OrderController::create returns 2 triggers + kloc_context hint |

\* STRETCH — SKIP allowed per spec line 425.

**Overall PASS:** all of AC-1..18 PASS, plus XC-1/2/3, plus N1..N4. AC-19/20 SKIPPED is acceptable.
**Overall FAIL:** any AC-1..18 fails OR pre-flight env not green OR XC fails.

---

## What QA does NOT test (out of scope per spec §Out of Scope)

- Per-flow business / technical / search explanations
- Qdrant `flow_*` embedding pipelines (beyond their absence)
- ASCII flow diagram generation
- Call-tree pre-computation in any form
- Backward compat with `chain[]`
- Changes to `kloc-symfony` PHP code
- POC scripts in `kloc-symfony/pocs/`
- LLM evaluation of agent-on-demand pivot (D8 explicitly out of scope)

---

## Reporting verdict to lead

Final QA report MUST include:

1. PASS/FAIL per AC (table above) with evidence — Cypher output, command exit codes, log snippets
2. Performance baseline numbers (3 timed runs, exact seconds)
3. Verification-checkpoints sign-off summary path
4. Any deviations (e.g., AC-19/20 skipped) with justification
5. Cross-cutting check results (XC-1/2/3)
