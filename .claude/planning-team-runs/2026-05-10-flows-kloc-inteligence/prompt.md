# Planning Team Prompt — flows-kloc-inteligence

Generated: 2026-05-10

To launch this team: copy everything in the fenced block below into a new
Claude Code session, or re-run `/planning-team-run flows-kloc-inteligence --auto`.

---

```
Create an agent team to deeply analyze and validate a feature approach before implementation.

Feature name: flows-kloc-inteligence

Feature description:
Rebuild the flow model in kloc-intelligence using the simple shape that's already
produced by kloc-symfony (`symfony-kloc.json` with `flows[]` containing only
`{id, type, entry}` and a top-level `triggers[]` cross-flow graph). Drop ALL legacy
flow code in kloc-intelligence — no backward compatibility, no deprecation period.
The current Neo4j flow graph carries broken `FLOW_STEP` (Flow→Class) edges from a
previous DI-trace approach that hallucinates participants (e.g. EmailSender shows
under OrderController::get though it's never called there). Three POCs explored
richer call-tree models (call-tree / dataflow-tagged / receiver-grouped) but the
chosen direction is to NOT pre-build a call tree at all: keep `:Flow` nodes minimal
(entry + triggers) and let agents pivot into the existing `kloc_context` /
`kloc_source` / `kloc_chunks` tools to investigate flow internals on demand. Also
remove the over-built flow enrichment layer (multi-type business/technical/search
explanations + 3 Qdrant collections + ASCII diagram generator) — none of it lands
correctly in Neo4j today and all of it is tied to the broken model.

Spec file: none — Analyst will produce a requirements summary from the description.

Context files:
- docs/ideas/_selection.md — POC selection rationale; defines the OrderController::get
  acceptance test (must NOT show EmailSender/InventoryChecker/OrderProcessor) and
  ground-truth call tree.
- docs/ideas/README.md — index for the flow-model idea space.
- docs/ideas/flow-model-1-pure-call-tree.md — POC A spec (Method→Call→Method walk).
- docs/ideas/flow-model-2-dataflow-tagged.md — POC B spec (call tree + main_path tag).
- docs/ideas/flow-model-3-receiver-grouped.md — POC C spec (receiver-centric grouping).
- docs/ideas/flow-model-4-source-order-timeline.md — rejected (needs branch_check data
  not in sot.json).
- docs/ideas/flow-model-5-multi-layer.md — rejected (premature superset).
- docs/ideas/method-flow-model.md — earlier flow exploration.
- kloc-symfony/pocs/call-tree/{README.md, build_flows.py, flows-v2.json} — POC A
  working artifact; passes acceptance test on OrderController::get.
- kloc-symfony/pocs/dataflow/{README.md, build_flows.py, flows-v2.json} — POC B
  working artifact.
- kloc-symfony/pocs/receiver/{README.md, build_flows.py, flows-v2.json} — POC C
  working artifact.
- kloc-reference-project-php/.kloc/symfony-kloc.json — example simple-shape
  symfony-kloc.json (9 flows, 3 triggers, App-only) used as the canonical input.

Code touch points (existing flow code in kloc-intelligence):
- kloc-intelligence/src/db/flow_importer.py (197 LOC) — REPLACE with simpler version
  that imports only Flow nodes + FLOW_ENTRY + FLOW_TRIGGERS edges.
- kloc-intelligence/src/ai/flow_enricher.py (290 LOC) — DELETE entirely.
- kloc-intelligence/src/ai/flow_diagram.py (289 LOC) — DELETE entirely.
- kloc-intelligence/src/cli.py — strip 4 flow commands (flow-diagram, explain-flow,
  enrich-flows, plus _resolve_flow_id helper); keep import-flows but rewire to new
  importer.
- kloc-intelligence/src/server/mcp.py — strip 2 flow tools (kloc_explain_flow,
  kloc_flow_diagram) and their handlers; keep kloc_import_flows with new shape.
- kloc-intelligence/src/ai/pipelines.py — prune flow_business / flow_technical /
  flow_search pipeline functions; keep code/explain pipelines.
- kloc-intelligence/src/db/schema.py — keep flow_id and flow_type indexes.
- Qdrant collections to drop: flow_business_embeddings, flow_technical_embeddings,
  flow_search_embeddings (current vector counts: 8/8/8, all stale).

Key source directories:
- kloc-intelligence/src/ — Python code that consumes sot.json + symfony-kloc.json
  and serves CLI/MCP. Primary work area for this feature.
- kloc-symfony/src/ — PHP tool that produces symfony-kloc.json. Already produces
  the simple shape; verify no PHP changes needed.
- kloc-symfony/pocs/ — three working POCs to be DECOMMISSIONED as feature direction
  (kept on disk for reference but no longer drive code; agent-on-demand investigation
  via context tool replaces them).
- kloc-intelligence/tests/ — has flow tests tied to old code; need triage (delete or
  rewrite).

Relevant repos & current branches:
- kloc-intelligence (branch: main) — primary implementation target.
- kloc-symfony (branch: main) — verification only; consumes its symfony-kloc.json.
- root kloc monorepo (branch: main) — for any cross-cutting docs/specs.

## Team Structure

Spawn these teammates:

### Architect
- **Name**: architect
- **Focus**: Deep codebase exploration and design analysis
- **Agent definition**: Read .claude/teams/planning-team/agents/team-architect.md for role instructions
- **First task**: Systematically explore the codebase to understand patterns, dependencies, and areas affected by flows-kloc-inteligence. Map the dependency graph and identify integration points. Focus areas: (1) what currently consumes :Flow nodes in Neo4j (Cypher queries, MCP handlers, CLI commands, the search pipeline); (2) what Qdrant collections / embedding code references the three flow_* collections being dropped; (3) all import-time vs query-time touchpoints for FLOW_STEP edges; (4) how the new minimal model (Flow + FLOW_ENTRY + FLOW_TRIGGERS) interacts with kloc_context's existing Method→Call→Method traversal so that agent-driven flow investigation is actually feasible.
- **Model**: opus

### POC Developer
- **Name**: poc-dev
- **Focus**: Build isolated proof-of-concepts to validate risky assumptions
- **Agent definition**: Read .claude/teams/planning-team/agents/team-poc-developer.md for role instructions
- **First task**: Read the feature description and explore the technical areas involved. Wait for risk prioritization before building POCs. Likely POC candidates to keep ready: (a) full demolition+rebuild dry-run on a copy of the Neo4j+Qdrant state to confirm no orphan references; (b) sanity check that an MCP agent can answer "what does OrderController::get actually do" using ONLY the new minimal Flow nodes + kloc_context/kloc_source — i.e. prove the agent-on-demand approach is sufficient and we don't accidentally need a pre-computed call tree.
- **Model**: opus

### Analyst
- **Name**: analyst
- **Focus**: Structured criticism — pre-mortem, assumption mapping, FMEA risk scoring
- **Agent definition**: Read .claude/teams/planning-team/agents/team-analyst.md for role instructions
- **First task**: Study the feature description. Produce a requirements summary (core, inferred, ambiguous, out-of-scope). Then prepare challenge questions using pre-mortem and assumption mapping frameworks. Particular pre-mortem scenarios to explore: (1) deleting flow_enricher.py / flow_diagram.py removes capabilities downstream consumers depended on; (2) dropping the three Qdrant flow_* collections breaks kloc_search if any code path still references them; (3) the agent-on-demand model fails when an LLM agent doesn't reliably know to call kloc_context after kloc_flow; (4) hidden coupling between FLOW_STEP edges and the inherit/overrides traversals (cross-flow polymorphic resolution); (5) the hard "no backward compatibility" stance leaves test fixtures or downstream tooling broken without warning.
- **Model**: opus

## Workflow Phases

### Phase 1: Deep Exploration (PARALLEL)
1. Architect explores codebase — patterns, dependencies, data flows
2. POC Dev investigates technical areas (read-only initially)
3. Analyst studies requirements/spec, prepares challenge framework

### Phase 2: Risk Identification (SEQUENTIAL — after Phase 1)
1. Architect presents findings and proposes approach(es)
2. Analyst challenges assumptions using:
   - Pre-mortem: "Imagine this failed. Why?"
   - Assumption mapping: What are we taking for granted?
   - FMEA scoring: Severity × Likelihood × Detectability = RPN
3. Lead synthesizes into prioritized risk list
4. Team agrees on which risks need POC validation (RPN ≥ 200 or severity ≥ 8)

### Phase 3: POC Validation (PARALLEL)
For each high-risk item:
1. POC Dev builds minimal POC in .claude/poc/flows-kloc-inteligence/poc-NN-{description}/
2. Architect advises on approach
3. Analyst defines success criteria
4. Each POC answers ONE specific question

### Phase 4: Synthesis & Report (LEAD)
1. Collect all findings from teammates
2. Produce validated analysis report at .claude/planning-team-runs/2026-05-10-flows-kloc-inteligence/analysis.md
3. Include: risk matrix with evidence, implementation recommendations, go/no-go
4. Shut down teammates, clean up team

## Rules for Lead

1. Use delegate mode — DO NOT explore code or analyze directly
2. Read .claude/teams/planning-team/agents/team-lead.md for full role instructions
3. Follow phases sequentially — wait for ALL Phase 1 completion
4. Only approve POCs for HIGH-risk items (RPN ≥ 200 or severity ≥ 8)
5. POC Dev writes ONLY to .claude/poc/flows-kloc-inteligence/ — enforce strictly
6. No production code changes — this team is analysis-only
7. Final report goes to .claude/planning-team-runs/2026-05-10-flows-kloc-inteligence/
8. Never push to remote
```
