# kloc-intelligence follow-ups — implementation summary

Date: 2026-05-10
Session: single dev (no team)
Working repo: `/Users/michal/dev/ai/kloc/kloc-intelligence` (commits below baseline `fa78d2a`)
Doc repo: `/Users/michal/dev/ai/kloc` (parent monorepo)

## Tasks delivered

### Task 1 — `flows` command (CLI + MCP)
Single command/tool with optional positional arg producing a discriminated-union response (mode: list / detail / candidates).

- New file: `src/db/queries/flows.py` — three pure-Cypher helpers (list_flows, find_flow, get_flow_detail).
- Extended `src/cli.py` with the `flows` Typer command (Rich + JSON modes, `--type http,message,event,cli` filter).
- Extended `src/server/mcp.py` with `kloc_flows` tool and `_handle_flows` handler.
- Tests: `tests/test_flows_command.py` (15 new tests covering query module + CLI integration), `tests/test_mcp_tools.py` (3 new MCP tests).

Verified against the reference project: lists 9 flows, `--type http` returns 4, `OrderController::create` detail returns the 2 expected `triggers_out` (OrderCreatedEvent + OrderCreatedMessage).

### Task 2 — Per-operation LLM provider config
Generic env var names, independent providers per operation.

- Rewrote `src/ai/config.py` into nested `LLMProviderConfig` + `EmbeddingProviderConfig` dataclasses on `AIConfig`.
- New env vars: `LLM_API_URL/KEY/MODEL` + `EMBEDDING_API_URL/KEY/MODEL/DIMENSION`. `OPENROUTER_*` and `KLOC_LLM_*` removed.
- `validate()` now takes `require_llm` / `require_embedding` flags so `search` doesn't fail when only embedding key is set.
- Updated `src/ai/pipelines.py`, `src/ai/enricher.py`, `src/cli.py` for the new field paths.
- Rewrote `tests/test_ai_config.py` (12 new tests) for the new shape.
- Updated `.env.example`.

`git grep OPENROUTER src/ .env.example` returns nothing (acceptance criterion met).

#### Gemini verification
Both providers smoke-tested:
- ✅ OpenRouter (default): explain + embedding work end-to-end with the new env-var shape.
- ✅ Gemini chat (`gemini-3-flash-preview` via `https://generativelanguage.googleapis.com/v1beta/openai/`): works.
- ❌ Gemini native embeddings (`gemini-embedding-001`): Haystack's `OpenAIDocumentEmbedder` crashes because Gemini's response omits the `usage` field. Workaround: route gemini-embedding-001 through OpenRouter (which proxies the protocol cleanly).

Full findings + cost notes + mixed-provider config example: [gemini-verification.md](gemini-verification.md).

### Task 3 — Documentation
Two new long-form docs at `docs/` (parent monorepo):

- [docs/kloc-intelligence-overview.md](../../docs/kloc-intelligence-overview.md) (354 LOC): functionality catalog covering 9 capability areas + architecture sketch + comparison with related tools + glossary + operational profile.
- [docs/kloc-intelligence-processes.md](../../docs/kloc-intelligence-processes.md) (556 LOC): step-by-step data flows for 6 major processes (indexing, flow ingestion, enrichment, context, search, MCP lifecycle) with ASCII sequence diagrams.

Also updated `docs/usage/kloc-intelligence.md` to:
- Add cross-links to the two new docs at the top.
- Document the new `flows` command (Task 1).
- Document the new env vars + mixed-provider example (Task 2).
- Update MCP tool count to 15.

## Commits

In `kloc-intelligence/` (own git repo, baseline `fa78d2a`):

1. `6834057` flows: add list/detail flows command (CLI + MCP) — Task 1
2. `39ff55c` ai: provider-agnostic LLM and embedding config — Task 2

In parent `kloc/` repo:

3. `f8b66f5` docs: add kloc-intelligence overview and processes guides — Task 3 (force-added to bypass `docs/` gitignore, matching prior precedent)

Plus this summary commit (also force-added since `.claude/` is gitignored).

## Test results

Focused suite for the changed surfaces:

```
$ uv run pytest tests/ -k "ai_config or flows or flow_importer or mcp_tools or cli_flows or handlers"
97 passed in 1.62s
```

The 37 pre-existing `test_snapshot.py::TestContextSnapshots` failures are unrelated to this work — they predate the baseline commit (verified via `git stash` + retest). They appear to be expected-snapshot drift from an earlier indexer change.

## Files touched

```
kloc-intelligence/
  .env.example                         (rewritten)
  src/ai/config.py                     (rewritten)
  src/ai/enricher.py                   (1-line update)
  src/ai/pipelines.py                  (3 callsites updated)
  src/cli.py                           (+125 LOC: flows command, ai_config refs)
  src/db/queries/flows.py              (NEW, 196 LOC)
  src/server/mcp.py                    (+82 LOC: kloc_flows tool + handler)
  tests/test_ai_config.py              (rewritten, 12 tests)
  tests/test_flows_command.py          (NEW, 15 tests)
  tests/test_mcp_tools.py              (+3 tests for kloc_flows)

kloc/ (parent)
  docs/kloc-intelligence-overview.md            (NEW, 354 LOC)
  docs/kloc-intelligence-processes.md           (NEW, 556 LOC)
  docs/usage/kloc-intelligence.md               (cross-links + flows + env vars)
  .claude/feature-team-runs/2026-05-11-kloc-intelligence-followups/
    gemini-verification.md                       (NEW)
    summary.md                                   (this file)
```

## Out-of-scope items NOT done (per prompt guardrails)

- No Haystack rewrite. The Gemini embedding incompatibility is documented as a limitation, not patched.
- No new Qdrant collections, no Neo4j schema changes.
- No fix for the pre-existing snapshot test failures.

## Suggested next step

Run `git push origin main` from both repos (`kloc-intelligence/` and `kloc/`) when ready. Both push targets are `main`, both are 1+ commits ahead of origin.
