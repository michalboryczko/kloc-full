# Task 13 Summary: CLI Interface & MCP Server

## What Was Implemented

### S01/S02: CLI Framework & Wire Commands

The CLI framework was already in place from earlier tasks (T01-T06). This task updated the `context` command to use the new `ContextOrchestrator` (replacing inline class-only dispatch), and added the `mcp-server` command.

**Updated context command:**
- Now uses `ContextOrchestrator.execute_symbol()` instead of inline kind-switching
- Added `-i` short flag for `--impl`
- Added detailed help text with examples
- Handles `ValueError` from orchestrator for symbol-not-found

**All 9 CLI commands now registered:**
- `import` - Import sot.json into Neo4j
- `resolve` - Resolve a symbol to its definition
- `usages` - Find usages with depth expansion
- `deps` - Find dependencies with depth expansion
- `context` - Bidirectional context (used_by + uses + definition)
- `owners` - Structural containment chain
- `inherit` - Inheritance tree
- `overrides` - Override chain/tree
- `mcp-server` - Start MCP server (stdio)
- `schema` - Manage Neo4j schema (subcommand group)

### S03/S04: MCP Server with Lazy Loading

Full MCP server implementing JSON-RPC 2.0 over stdio, matching kloc-cli's MCP protocol.

**Protocol support:**
- `initialize` - Returns server info and capabilities
- `notifications/initialized` - Acknowledged silently
- `tools/list` - Returns 8 tool definitions with input schemas
- `tools/call` - Dispatches to tool handlers
- `ping` - Health check

**8 MCP tools:**
- `kloc_projects` - List configured projects
- `kloc_resolve` - Resolve symbol
- `kloc_usages` - Find usages
- `kloc_deps` - Find dependencies
- `kloc_context` - Bidirectional context (uses ContextOrchestrator)
- `kloc_owners` - Ownership chain
- `kloc_inherit` - Inheritance tree
- `kloc_overrides` - Override tree

**Multi-project support:**
- Single project: `kloc-intelligence mcp-server` or `kloc-intelligence mcp-server --database myapp`
- Multi-project: `kloc-intelligence mcp-server --config kloc.json`
- Config maps project names to Neo4j database names
- Lazy connection: Neo4j driver created on first query per project

**Connection lifecycle:**
- Connections lazily established on first query
- Reused across subsequent requests (connection pooling)
- Clean shutdown via `atexit` handler
- Neo4j connection errors caught and returned as JSON-RPC errors

### S05: Error Handling

- CLI commands: `Neo4jConnectionError` caught and shown as user-friendly messages
- MCP server: `Neo4jConnectionError` returned as JSON-RPC error code -32000
- Symbol not found: returns `{"error": "Symbol not found: <query>"}`
- Ambiguous symbol: returns candidates list
- All exceptions caught at MCP protocol level (no stack traces to client)

### S06: Help Text

- All commands have detailed docstrings with examples
- Context command: explains --impl, --direct, --with-imports flags
- MCP server: shows config file format and both single/multi-project modes
- Top-level help lists all commands with descriptions

## Files Created/Modified

### New Files
- `kloc-intelligence/src/server/mcp.py` -- MCP server (MCPServer class + run_mcp_server)
- `docs/specs/kloc-intelligence/task-13-summary.md` -- this summary

### Modified Files
- `kloc-intelligence/src/cli.py` -- updated context command (uses ContextOrchestrator), added mcp-server command

## Test Results

- 45 passed, 5 failed (no regression)
- Same 5 known failures as T12 (context-class-d1, context-class-d2, context-interface-d2, context-method-d2, context-file)

## Key Design Decisions

### Neo4j Database Per Project (not sot.json per project)
kloc-cli maps project -> sot.json path. kloc-intelligence maps project -> Neo4j database name. This leverages Neo4j's multi-database support for project isolation.

### Shared Connection via _resolve_symbol
The MCP server's `_resolve_symbol` returns both the resolved node and the QueryRunner, avoiding duplicate connection lookups in tool handlers.

### ContextOrchestrator for MCP Context Tool
The MCP server uses the same `ContextOrchestrator` as the CLI, ensuring identical output from both interfaces.
