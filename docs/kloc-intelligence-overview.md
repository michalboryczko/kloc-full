# kloc-intelligence — what it does and why

`kloc-intelligence` is the persistence-and-query layer of the kloc pipeline. It takes a static analysis snapshot of a PHP codebase (`sot.json`), loads it into a graph database, and exposes structured + semantic queries about the code through three surfaces: a CLI, an MCP server (for AI agents), and direct Cypher.

It is the answer to a question that nearly every non-trivial codebase eventually asks: *"how do I navigate, explain, and reason about this code without reading every file?"* It does this by combining three layers of representation:

1. **Structural graph** — every class, method, property, function, and value reference is a node in Neo4j. Containment, calls, type hints, inheritance, overrides, parameter passing, return types — they're all edges. Cypher answers "who uses what" in milliseconds.
2. **Symbolic resolution** — partial names, FQNs, file paths, and method syntax all resolve to the same node via a cascading match strategy. Agents and humans don't need to know the exact spelling.
3. **Semantic embeddings + LLM explanations** — Qdrant stores vector embeddings of source + LLM-authored explanations. Natural-language queries route to the same nodes the structural graph already knows about.

Where `kloc-cli` answers static questions about a single `sot.json` snapshot, `kloc-intelligence` is **stateful and multi-modal**: it persists the graph in Neo4j, supports multi-hop traversals, and overlays AI features on top.

This document is the catalog of capabilities — *what's in the box*. For step-by-step pipelines see [kloc-intelligence-processes.md](kloc-intelligence-processes.md). For day-to-day commands see [usage/kloc-intelligence.md](usage/kloc-intelligence.md).

---

## Capabilities at a glance

| Area | What it answers | Surfaces |
| --- | --- | --- |
| Graph storage and querying | "Where do these symbols live and how are they connected?" | Cypher, CLI, MCP |
| Symbol resolution | "Where is X?" | `resolve` / `owners` |
| Bidirectional traversal | "Who uses X / what does X use?" | `usages` / `deps` / `context` |
| Inheritance and polymorphism | "What implements Y? What overrides Z?" | `inherit` / `overrides` |
| Source code access | "Give me the code for this node" | `source` / `chunks` |
| Flow model (Symfony) | "What HTTP/message/event/CLI flows does this app expose?" | `import-flows` / `flows` |
| AI enrichment | "Explain this code in business / technical terms" | `enrich` / `explain` |
| Semantic search | "Find code that does X" (natural language) | `search` |
| MCP server | All of the above, exposed to AI agents | JSON-RPC stdio |

---

## 1. Graph storage and querying

### What it does

`kloc-intelligence import sot.json` loads the **Source of Truth** produced by `kloc-mapper` into Neo4j as a property graph. Every node is labeled `:Node` with a `kind` property (one of 13 NodeKinds: Class, Interface, Trait, Enum, EnumCase, Method, Function, Property, Const, Argument, Value, File, type_hint). Every edge is one of 13 EdgeTypes (USES, CONTAINS, EXTENDS, IMPLEMENTS, OVERRIDES, USES_TRAIT, ARGUMENT, RECEIVER, ASSIGNED_FROM, TYPE_OF, PRODUCES, FLOW_ENTRY, FLOW_TRIGGERS).

The schema is enforced by uniqueness constraints on `node_id` and `fqn`, plus indexes on `kind` and `name` for fast filtering. Everything else is read-only Cypher you can write directly in the Neo4j Browser, or through `kloc-intelligence`'s CLI/MCP wrappers.

The graph is **idempotent and reproducible**: re-importing the same `sot.json` (with `--clear`, default true) produces an identical graph. No drift between dev and CI. No "first run vs. tenth run" surprises.

### Why it exists

Reading PHP source ad hoc (grep, IDE indexing, cscope-style tools) doesn't scale to "show me every class that depends on this interface, including transitively, but exclude vendor code". You need a graph database to traverse those relationships at depth N efficiently.

Writing the graph yourself from PHP source is a multi-month project (we know — `kloc-indexer-php` is exactly that). `kloc-intelligence` lets you skip that step entirely: feed it the SoT, get a queryable Neo4j graph in seconds.

### When to use it / when not to

Use it whenever you'd otherwise reach for grep, find, or IDE refactoring tools across a >50K-LOC codebase. Don't use it for code transforms (that's not its job — modifying PHP is the host project's responsibility). It's also overkill for codebases where you've memorized the structure; the value compounds with size and unfamiliarity.

---

## 2. Symbol resolution — `resolve` / `owners`

### What it does

`resolve <query>` takes a fuzzy symbol — partial name, FQN, method shorthand (`Class::method`), or file path — and returns the matching `:Node`(s). It cascades through six match strategies in order: exact FQN, case-insensitive FQN, suffix match, name match, name without parens, then a final substring fallback. The first level that produces matches wins.

`owners <query>` walks the structural containment chain upward from a node to its enclosing File. So a Method gets its parent Class, the Class gets its File. This is the answer to "where does this live in the codebase tree?".

Both commands return JSON with `node_id`, `fqn`, `kind`, `file`, and `line`. The `node_id` is stable across runs (deterministic hash) so it can be cached in user code or stored in tickets.

### Why it exists

Anyone who's typed `OrderService` and hit the wrong autocomplete entry knows the problem: codebases have multiple classes with similar names, namespaced and unnamespaced, plus interfaces with the same short name as their concrete implementations. A single-strategy lookup (e.g. "exact FQN only") forces the user to know the spelling. A pure-substring search returns too much noise.

The cascade gives you the best of both: type the FQN and you get exactly one hit; type a partial name and you get a ranked candidate list to pick from. Agents lean on this *constantly* — they often have a class name from a doc string or commit message but no namespace context.

### When to use it / when not to

Use `resolve` whenever a user-supplied symbol enters the system. Skip it if you already have a `node_id` — direct Cypher lookups are faster than the cascade.

---

## 3. Bidirectional traversal — `usages` / `deps` / `context`

### What it does

- `usages <symbol> --depth N --limit L` answers "who uses X?" by walking incoming USES edges to depth N. Returns each caller with its containing class, method, and file/line.
- `deps <symbol> --depth N --limit L` is the mirror image: outgoing USES edges, "what does X use?".
- `context <symbol>` is `usages` + `deps` in one query, plus type info, owners, and (with `--include-impl`) implementations/overrides. It's the "give me everything I need to understand this node before I touch it" command.

All three return tree-structured results: depth-1 children are direct neighbors, depth-2 children are their neighbors, etc. The output is renderable as a Rich tree in the terminal, JSON for tools, or a flat list for grep-friendly piping.

### Why it exists

The two questions that drive 80% of code-archaeology work are *"if I change this, what breaks?"* (`usages`) and *"what does this thing rely on to do its job?"* (`deps`). Without a graph database, you answer both by reading code. With one, you answer both in milliseconds.

`context` deserves its own mention: it's the workhorse command for AI agents preparing to make a code change. The agent fetches the bidirectional context, the type info, and the inheritance tree in a single round-trip; then it has enough information to generate a thoughtful patch or explanation without needing follow-up queries. Most of the agent-side tool-use latency in `kloc-intelligence` is spent on `context`, not anything else.

### When to use it / when not to

Use `usages` for impact analysis before refactoring or deletion. Use `deps` to understand a class you've never seen. Use `context` whenever you'd otherwise need both. None of them are a substitute for actually reading the code at the leaves — they tell you *where to read*, not *what's there*.

---

## 4. Inheritance and polymorphism — `inherit` / `overrides`

### What it does

- `inherit <class> --direction up|down --depth N` walks EXTENDS / IMPLEMENTS edges. `up` returns ancestors (parents and interfaces); `down` returns descendants (everything that extends or implements this class).
- `overrides <method> --direction up|down --depth N` walks OVERRIDES edges. `up` finds the parent method this method overrides; `down` finds every method that overrides this one.

Both are tree-structured and limited by depth + count to keep responses bounded for large hierarchies.

### Why it exists

PHP's `extends` / `implements` chains and abstract method overrides are easy to lose track of in any non-trivial OO codebase. When you're looking at an `OrderProcessor::process` method and you need to know "is this the only implementation, or do six other classes override it?", the answer should be a query, not a 20-minute file-system spelunk.

This matters a lot for AI agents reasoning about Liskov-substitution-style scenarios: an agent that doesn't know an abstract method has six overrides will produce a fix that satisfies one and breaks the others. Surfacing the polymorphism explicitly is the difference between a useful suggestion and a footgun.

### When to use it / when not to

Use `inherit down` when looking at an interface or abstract class to enumerate implementations. Use `overrides down` when modifying a method on a base class. Skip them for classes that aren't part of inheritance hierarchies — most app-code value classes (DTOs, requests) don't extend anything interesting.

---

## 5. Source code access — `source` / `chunks`

### What it does

- `source <symbol>` reads the actual PHP file content for a node, using the node's stored `file` + `start_line` + `end_line` to slice exactly the right region. Returns content + line range + token estimate.
- `chunks <symbol>` returns the same content split into LLM-friendly chunks. For Methods, the entire method is one chunk. For Classes, the file is split by method boundary with a shared "class context preamble" prepended to each chunk so the LLM has class-level types/imports/parents in scope.

Both commands need `KLOC_PROJECT_ROOT` (or `--project-root`) to know where the actual files live; the graph stores file paths relative to that root.

### Why it exists

Once you've used `resolve` and `usages` to find the right node, you usually need to *see the code*. Writing your own "open the file, count lines, slice" code is fine for a one-off but tedious for tools, and gets the line-range-conversion (1-based vs 0-based) wrong half the time. `source` does it once, correctly.

`chunks` exists because LLM context windows aren't infinite, and naive "split the file every N tokens" loses class context (the chunk for `OrderService::createOrder` doesn't know about its constructor-injected dependencies). The class-aware chunker prepends just enough context per chunk that an embedder can produce useful vectors and an LLM can produce useful explanations.

### When to use it / when not to

Use `source` when you have the FQN and want the code. Use `chunks` only when you're feeding the result to an embedder or LLM (it's optimized for that, not for human reading). Both fail gracefully if `KLOC_PROJECT_ROOT` is unset — they'll tell you to set it.

---

## 6. Flow model (Symfony apps) — `import-flows` / `flows`

### What it does

For Symfony applications specifically, `kloc-symfony` produces a `symfony-kloc.json` describing every HTTP route, message handler, event subscriber, and CLI command — plus the dispatch relationships between them. `import-flows` lights up this file as `:Flow` nodes connected by `FLOW_ENTRY` (Flow → entry Method) and `FLOW_TRIGGERS` (Flow → Flow when one dispatches a message/event the other handles) edges in the same Neo4j graph.

`flows` (no argument) lists every flow with its type, route/event/command name, and entry FQN. `flows <flow_id-or-partial>` returns full detail: entry method, file/line, triggers in (who fires this flow), triggers out (what this flow fires). Partial matches return a candidate list so users don't need to know exact flow IDs.

### Why it exists

Modern Symfony apps spend half their complexity in the framework's invisible plumbing: the OrderController doesn't directly call OrderHandler — it dispatches an OrderCreatedMessage, the messenger picks it up off a queue, and the handler runs in a different process. Reading the controller alone tells you nothing about that downstream chain. Reading the YAML config + every event subscriber is a half-day of work.

The flow model condenses all of that into "here are the application's entry points, and here's what each one triggers, transitively". It's the framework-visible answer to "what does a HTTP POST to `/api/orders` actually *do* across the codebase?".

The design is **deliberately minimal**: flows store only entry + triggers. The deep call tree from the entry method into the rest of the graph is reachable via `kloc_context` on the entry method's node — flows tell agents *where to start the investigation*, not the whole investigation. This separation keeps the flow model fast (~1s for hundreds of flows) and the agent-side queries on-demand.

### When to use it / when not to

Use it for any Symfony codebase you're trying to understand. Use it before changing a controller or a handler — knowing what fires upstream and downstream is a strong defense against breaking async paths. Don't use it for non-Symfony PHP projects (no flows would be detected) or to discover *all* the application's behaviors (that requires walking the full call tree, not just the flow boundaries).

---

## 7. AI enrichment — `enrich` / `explain`

### What it does

`enrich` is a batch operation that walks every Class/Method node and produces:
1. A 2–5 sentence human-language **explanation** of what the code does (stored as a node property in Neo4j: `n.explanation`, `n.explain_model`, `n.explain_at`).
2. A vector **embedding** of both the source code and the explanation, written to two Qdrant collections: `code_embeddings` and `explain_embeddings`. Large classes are split by method boundary so each embedding represents one logical unit.

Both steps use OpenAI-compatible providers (configurable per-operation — see [usage/kloc-intelligence.md](usage/kloc-intelligence.md)). Default LLM is `minimax/minimax-m2.7` via OpenRouter; default embeddings are `qwen/qwen3-embedding-8b` 4096-dim. Each can be swapped for Gemini, OpenAI, or any OpenAI-compat endpoint.

`explain <symbol>` runs the same pipeline for a single node on demand (used when the batch hasn't been run yet, or when you want to refresh a specific node).

### Why it exists

Static analysis tells you the *shape* of the code. It doesn't tell you the *purpose*. "OrderService::createOrder takes a CreateOrderInput, calls InventoryChecker, builds an Order, calls OrderRepository::save, dispatches OrderCreatedEvent" is a true description of the method but nobody would ever say it that way to a colleague.

LLM explanations close that gap: they translate the code into the language a senior engineer would use in a code review or PR comment. Stored in the graph, they become permanent first-class metadata you can query, search, and present to agents.

The embeddings layer adds the second axis: instead of scrolling through every explanation, you can ask "show me code that handles failed payments" and the embedding similarity returns the right node even if the word "failed" never appears in the source.

### When to use it / when not to

Run `enrich` once after each significant import, then again whenever you do major refactoring (use `--force` to re-run already-enriched nodes). It's idempotent — re-running on already-enriched nodes is a no-op without `--force`.

The cost is real: every Class/Method node is one LLM call + two embedding calls. For the kloc-reference-project-php (~50 enrichable nodes) this is single-digit cents. For a 5000-method codebase at OpenRouter pricing, expect $5–$30 depending on the chosen model.

Don't enrich vendor code — the value is low and the LLM will hallucinate confident-sounding nonsense about library internals it doesn't really know. The default kind filter (`Class,Method`) skips Properties and Values, which is usually right; override with `--kinds` only if you know what you're doing.

---

## 8. Semantic search — `search`

### What it does

`search "<query>"` embeds the query through the configured embedding model, runs cosine-similarity against both `code_embeddings` and `explain_embeddings` Qdrant collections, dedupes by `node_id` (keeping the highest-scoring match across collections), and returns the top-K results as `(score, kind, fqn, file)` rows.

`--collection code|explain|both` narrows the search; default is `both`. The collection a hit comes from is included in JSON output.

### Why it exists

Grep finds substring matches. `usages`/`deps` find graph neighbors. Neither answers "find code that does X" when you don't know the right substring or the right starting node. That's the question semantic search exists for.

The two-collection setup matters: `code_embeddings` is good for finding code that *looks* like the query (similar structure, similar variable names), while `explain_embeddings` is good for finding code that *behaves* like the query (because the LLM-authored explanation describes behavior in plain language). Merging both gets you both kinds of recall.

### When to use it / when not to

Use it when you're looking for a behavior or capability ("how does this app handle file uploads?"), not a specific symbol (use `resolve` for that). Use it as the first step when onboarding to an unfamiliar codebase — the top hits often reveal the central abstractions.

Skip it if you haven't run `enrich` (the collections will be empty). Skip it if your query is exactly a class name (`resolve` is faster and more precise). Don't expect it to work well on freshly-imported projects without enrichment — code-only embeddings with no explanations are noticeably weaker.

---

## 9. MCP server — `kloc-intelligence mcp-server`

### What it does

Exposes 14 of the capabilities above (everything except CLI-specific commands like `schema reset`) as Model Context Protocol tools over JSON-RPC 2.0 stdio. AI agents that speak MCP — Claude Code, Cursor, custom agent harnesses — can list and call these tools the same way they'd call any tool.

The tools are: `kloc_resolve`, `kloc_usages`, `kloc_deps`, `kloc_context`, `kloc_owners`, `kloc_inherit`, `kloc_overrides`, `kloc_import`, `kloc_explain`, `kloc_search`, `kloc_enrich`, `kloc_import_flows`, `kloc_flows`, `kloc_source`, `kloc_chunks`. Each takes a JSON-Schema-validated input and returns structured JSON output.

The server supports **multiple projects** via a config file mapping project names to Neo4j databases — useful for monorepos or hosted multi-tenant setups. With one project configured (the default mode), `project` is optional in every tool call. With multiple projects, it becomes required to disambiguate.

### Why it exists

Building an agent that understands an unfamiliar PHP codebase from scratch is hard. Building one that can call `kloc_context "OrderService::createOrder"` and get a precise structured answer is dramatically easier. MCP turns the entire feature surface into agent-callable functions — the agent never has to write Cypher, never has to know how Neo4j is configured, never has to deal with auth.

The stateless-per-request design means agents don't hold connections; the server lazily creates Neo4j connections per project on first use and reuses them across calls. This makes it safe to invoke the server as a long-running stdio process for the duration of a session.

### When to use it / when not to

Use it whenever you're integrating kloc-intelligence into an agent (Claude Code, an IDE plugin, a custom orchestrator). For human users at the terminal, the CLI is a better fit — it has Rich output, progress bars, and discoverable subcommands. The MCP tools are the same operations with stricter schemas and machine-readable output.

Don't expose the MCP server over the public internet without a transport wrapper (TLS, auth) — it's stdio JSON-RPC by design, intended for in-process or local-pipe communication.

---

## Limitations, in plain terms

A few things worth knowing before adopting:

1. **Graph quality depends on `sot.json` quality.** If `kloc-mapper` misses an edge — which can happen for PHP dynamic dispatch (`$service->{$method}()`), magic methods, or runtime trait composition — the graph also misses it. Static analysis is a lower bound on truth, never an upper bound.

2. **AI enrichment is async and costs money.** A 5000-node enrichment run takes 30–60 minutes and costs single-digit dollars at OpenRouter list prices. Plan for it; don't run it on every CI build.

3. **Native Gemini embeddings don't work end-to-end.** The Gemini OpenAI-compat endpoint omits the `usage` field, which Haystack's embedder treats as required. Workaround: route gemini-embedding-001 through OpenRouter (which proxies the protocol cleanly) — see the verification doc in `.claude/feature-team-runs/2026-05-11-kloc-intelligence-followups/gemini-verification.md`.

4. **Flows are Symfony-only.** Plain PHP, Laravel, and Yii apps will report 0 flows; the model is tied to Symfony's DI container + Routes + Messenger conventions.

5. **No write surface.** This service reads + writes its own Neo4j and Qdrant state, but it never modifies the original PHP source. If you want refactoring, you want a different tool.

6. **Single-tenant per Neo4j database.** The multi-project mode partitions by Neo4j database name, not by namespacing within one DB. Cross-project queries aren't supported (and probably shouldn't be — the graph cardinality blows up fast).

---

## Architecture in one sketch

```
┌────────────────────────────────────────────────────────────────┐
│  Caller surface                                                │
│  ┌─────────────┐  ┌──────────────┐  ┌─────────────────────┐  │
│  │   Typer CLI │  │  MCP server  │  │  Direct Cypher /    │  │
│  │ (Rich/JSON) │  │ (JSON-RPC)   │  │  Neo4j Browser      │  │
│  └──────┬──────┘  └──────┬───────┘  └──────────┬──────────┘  │
└─────────┼────────────────┼──────────────────────┼─────────────┘
          │                │                      │
┌─────────▼────────────────▼──────────────────────▼─────────────┐
│  Orchestration (Python)                                        │
│  ┌────────────────────────────────────────────────────────┐  │
│  │ resolve / context / usages / deps / inherit / ...     │  │
│  │ Symbol cascade, traversal helpers, output renderers   │  │
│  └─────────┬─────────────────────────────────────┬──────────┘  │
│            │                                     │             │
│  ┌─────────▼──────────┐               ┌──────────▼─────────┐  │
│  │ Graph layer        │               │ AI layer           │  │
│  │ - Cypher queries   │               │ - Haystack pipes   │  │
│  │ - Schema mgmt      │               │ - LLM prompts      │  │
│  │ - Importers        │               │ - Chunker          │  │
│  └─────────┬──────────┘               └──────────┬─────────┘  │
└────────────┼─────────────────────────────────────┼────────────┘
             │                                     │
   ┌─────────▼────────┐                ┌───────────▼──────────┐
   │   Neo4j 5        │                │  Qdrant 1.12         │
   │   :Node + :Flow  │                │  code_embeddings +   │
   │   13 NodeKinds   │                │  explain_embeddings  │
   │   13 EdgeTypes   │                │                      │
   └──────────────────┘                └──────────────────────┘

   ┌──────────────────────────────────────────────────────────┐
   │  Filesystem (read-only)                                   │
   │  PHP source files, accessed via KLOC_PROJECT_ROOT for     │
   │  source/chunks rendering. Never modified by this service. │
   └──────────────────────────────────────────────────────────┘
```

The two storage tiers are independent: Neo4j is the structural source of truth, Qdrant is the semantic index. They share `node_id` as a stable join key, but neither one is canonical for the other. You can drop and rebuild Qdrant (re-run `enrich`) without touching Neo4j; you can drop and rebuild Neo4j (re-run `import`) without touching Qdrant — though embeddings of nodes that no longer exist are dead weight you'd want to clean up.

The PHP source files are referenced from the graph (each `:File`, `:Class`, `:Method` carries a `file` + line range) but are not stored anywhere by `kloc-intelligence` — `source` and `chunks` read them on demand from `KLOC_PROJECT_ROOT`. This keeps the service stateless w.r.t. source content and avoids drift between the graph and the actual files.

---

## How it compares to related tools

| Tool | Lives at | Strengths | What it lacks |
| --- | --- | --- | --- |
| `kloc-cli` | Same kloc pipeline, stateless | Fast `sot.json` reads, no DB, no setup | No multi-hop traversals beyond depth 2-3, no AI |
| `kloc-intelligence` | This service | Graph queries, AI, MCP | Heavier setup, requires Neo4j/Qdrant |
| Sourcegraph / GitHub Code Search | Hosted | Universal language coverage, web UI | Not graph-native, no LLM enrichment by default |
| LSP servers (Intelephense, Phpactor) | IDE-resident | Real-time, in-editor | Per-IDE, no agent integration, no semantic search |
| `grep` / `ripgrep` | Filesystem | Zero deps, fast | Substring only, no structure, no relationships |

The closest comparison is `kloc-cli`: same input data, same FQN resolution semantics, much lighter weight. `kloc-cli` is the right tool for one-off queries against a single SoT snapshot. `kloc-intelligence` is the right tool when you need persistence, multi-hop traversals, AI features, or agent integration.

You can run both side-by-side: `kloc-cli` for shell pipelines and CI checks, `kloc-intelligence` for the agent-driven workflow.

---

## Glossary

A handful of terms recur throughout the codebase:

- **SoT (Source of Truth)** — `sot.json`, the canonical static analysis output produced by `kloc-mapper`. The single input that defines the graph.
- **FQN (Fully Qualified Name)** — PHP namespace + class name, optionally `::method`. Example: `App\Service\OrderService::createOrder`. Stable across runs.
- **node_id** — a deterministic 16-character hash of the node's identity (FQN + kind + file + line). Used as the primary key in Neo4j and Qdrant. Format: `node:abcdef0123456789`.
- **NodeKind** — the type of a node. One of: Class, Interface, Trait, Enum, EnumCase, Method, Function, Property, Const, Argument, Value, File, type_hint.
- **EdgeType** — the type of an edge. One of: USES, CONTAINS, EXTENDS, IMPLEMENTS, OVERRIDES, USES_TRAIT, ARGUMENT, RECEIVER, ASSIGNED_FROM, TYPE_OF, PRODUCES, FLOW_ENTRY, FLOW_TRIGGERS.
- **App namespace** — the convention that application code lives under `App\` and vendor code lives under any other namespace. The flow model and some queries filter by this convention.
- **Enrichment** — the process of generating LLM explanations + Qdrant embeddings for nodes. Distinct from "import" (which only loads the structural graph).
- **Chunk** — a unit of source code passed to the embedder. Methods are always one chunk; classes split by method boundary with a shared class-context preamble.
- **Project** — a logically isolated graph (one Neo4j database). The MCP server can serve multiple projects from one process.

---

## Operational profile

A full kloc-intelligence deployment for a single mid-sized codebase looks like:

- **Disk**: Neo4j data ~50 MB per 100K nodes; Qdrant data ~15 MB per 100K embeddings (4096-dim).
- **Memory**: Neo4j 2–8 GB heap depending on graph size; Qdrant 1–4 GB depending on collection count.
- **CPU**: idle most of the time. Bursts during `import` (Neo4j writes) and `enrich` (LLM/embedder calls, mostly waiting on network).
- **Network**: outbound to `LLM_API_URL` and `EMBEDDING_API_URL` during enrichment + search. Otherwise localhost only.
- **State**: persisted via Docker volumes (`neo4j-data`, `qdrant-storage`). Rebuilds are reproducible from `sot.json` + the source files; backing up the volumes is optional but useful for skipping the enrichment cost on dev machines.

For small projects (a couple thousand nodes) you can run the whole stack on a laptop with no tuning. For large projects (>500K nodes) plan on bumping Neo4j heap and pagecache via the `NEO4J_HEAP_*` / `NEO4J_PAGECACHE` env vars.

---

## Where to go next

- [usage/kloc-intelligence.md](usage/kloc-intelligence.md) — every CLI command and MCP tool with examples
- [kloc-intelligence-processes.md](kloc-intelligence-processes.md) — step-by-step data flows for each major operation
- The `kloc-cli` repo — same data model, lighter touch (no DB, reads `sot.json` directly)
- The `kloc-mapper` repo — produces the `sot.json` this service consumes
