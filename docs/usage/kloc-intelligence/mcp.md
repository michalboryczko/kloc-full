# kloc-intelligence — MCP server

Exposes the kloc-intelligence feature surface as Model Context Protocol tools over JSON-RPC 2.0 stdio. For setup see [configuration.md](configuration.md). For the CLI equivalent see [cli.md](cli.md).

## Start the server

```bash
uv run kloc-intelligence mcp-server                                    # single-project, default db
uv run kloc-intelligence mcp-server --database my_app_db               # single-project, named db
uv run kloc-intelligence mcp-server --config /path/to/config.json      # multi-project
```

The server runs over stdio — no TCP port. Spawn it as a child process and read/write JSON-RPC messages on stdin/stdout.

### Multi-project config

```json
{
  "projects": {
    "my-app": "my_app_db",
    "payments": "payments_db"
  }
}
```

With multiple projects configured, every tool call must include `"project": "my-app"` to disambiguate. With one project, `project` is optional (the server picks the only entry).

## Tool catalog (16 tools)

| Tool | Equivalent CLI | One-line purpose |
| --- | --- | --- |
| `kloc_resolve` | `resolve` | Resolve a symbol to its `:Node`. |
| `kloc_owners` | `owners` | Walk containment chain (Method → Class → File). |
| `kloc_usages` | `usages` | Who uses this? (incoming USES, depth N) |
| `kloc_deps` | `deps` | What does this use? (outgoing USES, depth N) |
| `kloc_context` | `context` | Bidirectional + types + owners + impl in one tree. |
| `kloc_inherit` | `inherit` | Walk EXTENDS / IMPLEMENTS. |
| `kloc_overrides` | `overrides` | Walk OVERRIDES. |
| `kloc_import` | `import` | Load `sot.json`. |
| `kloc_import_flows` | `import-flows` | Load `symfony-kloc.json`. |
| `kloc_flows` | `flows` | List / inspect `:Flow` nodes. |
| `kloc_enrich` | `enrich` | Batch enrichment of Class/Method nodes. |
| `kloc_enrich_flows` | `enrich-flows` | Generate business-process summaries for flows. |
| `kloc_explain` | `explain` | LLM explanation for one node. |
| `kloc_search` | `search` | Semantic search across all 3 collections. |
| `kloc_source` | `source` | Read raw source for a node. |
| `kloc_chunks` | `chunks` | Token-bounded chunks (same as the embedder uses). |

## Wire into Claude Code

`~/.claude.json` or project-local config:

```json
{
  "mcpServers": {
    "kloc-intelligence": {
      "command": "uv",
      "args": ["run", "kloc-intelligence", "mcp-server"],
      "cwd": "/path/to/kloc-intelligence",
      "env": {
        "KLOC_PROJECT_ROOT": "/path/to/php-project"
      }
    }
  }
}
```

Then the tools become callable from Claude Code as `mcp__kloc-intelligence__kloc_context`, etc. — the agent picks the right tool for its task.

### Multi-project version

```json
{
  "mcpServers": {
    "kloc-intelligence": {
      "command": "uv",
      "args": [
        "run", "kloc-intelligence", "mcp-server",
        "--config", "/path/to/projects.json"
      ],
      "cwd": "/path/to/kloc-intelligence"
    }
  }
}
```

## Wire into Cursor / other MCP clients

Any MCP-aware client that supports stdio-spawned servers can use the same `command + args + env` pattern. The protocol is standard JSON-RPC 2.0.

## Tool input/output shapes

### kloc_resolve

```json
// Input
{"symbol": "OrderService::createOrder"}

// Output
{"id": "node:...", "kind": "Method", "name": "createOrder",
 "fqn": "App\\Service\\OrderService::createOrder()",
 "file": "src/Service/OrderService.php", "line": 24}
```

### kloc_context

```json
// Input
{"symbol": "OrderService::createOrder", "depth": 2, "limit": 50, "include_impl": false}

// Output: tree-structured ContextResult (target + usedBy[] + uses[] + types + owners)
```

### kloc_flows

```json
// List mode (no flow_id)
{"type": "http"}  // optional filter

// Detail mode
{"flow_id": "flow:http:App\\...::create"}

// Output: discriminated union — {mode: "list"|"detail"|"candidates", ...}
```

### kloc_enrich_flows

```json
// Input
{"force": false}

// Output
{"total": 9, "processed": 9, "skipped": 0, "failed": 0, "failed_flows": []}
```

### kloc_search

```json
// Input
{"query": "validate order before checkout", "limit": 10, "collection": "all"}

// Output
{"query": "...", "hits": [{"score": 0.87, "kind": "Flow"|"Class"|"Method",
                          "fqn": "...", "file": "...", "node_id": "...",
                          "collection": "code_embeddings"|"explain_embeddings"|"flow_explain_embeddings"}, ...]}
```

`collection` accepts `code`, `explain`, `flows`, or `all` (default). `all` queries every Qdrant collection and merges results — flows enriched via `enrich-flows` appear in the top hits for business-process queries. Use `flows` to scope to flow summaries only.

### kloc_source / kloc_chunks

```json
{"symbol": "OrderService::createOrder", "project_root": "/optional/override"}
```

Returns the file content and line range. `kloc_chunks` returns the same chunks the embedder uses.

## Error envelope

JSON-RPC 2.0 error:

```json
{"jsonrpc": "2.0", "id": <req-id>,
 "error": {"code": -32000, "message": "Symbol not found: Foo"}}
```

Error code is always `-32000` (server-defined). The message is the human-readable cause — common ones: `Symbol not found: X`, `Multiple projects configured. Specify 'project' parameter. Available: [...]`, `EMBEDDING_API_KEY is required for search`.

## Agent recipe: explore an unfamiliar flow

```
1. kloc_import (if not loaded yet)
2. kloc_import_flows (if Symfony app and not loaded yet)
3. kloc_flows  → list, pick a flow_id
4. kloc_flows flow_id=<id>  → detail with entry method_node_id
5. kloc_context symbol=<entry_fqn> depth=3 include_impl=true  → call tree
6. kloc_source / kloc_chunks for any node you want to read
7. (Optional) kloc_search "what triggers this" → semantic recall of related flows
```

Most agent sessions stop at step 5 — `context` gives enough to answer most questions.

## Operational notes

- **Logs go to stderr** (not stdout — stdout is the JSON-RPC channel). Set the standard Python `KLOC_DEBUG=1` for verbose logs.
- **Stateless per request, connections lazy** — Neo4j connections are created on first use per project and reused across calls.
- **One process per session** is the recommended model — startup cost (~500 ms) amortizes across calls.
- **No network exposure** — stdio only. Wrap in TLS / auth if you need remote access.
