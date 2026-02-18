# kloc MCP Server

kloc exposes its code intelligence as an MCP (Model Context Protocol) server. Connect it to Claude Code, Cursor, Windsurf, or any MCP-compatible client to give your AI assistant structural understanding of your codebase -- symbol resolution, usage tracking, dependency graphs, inheritance trees, and data-flow context.

The server runs over stdio using JSON-RPC 2.0. It supports single-project and multi-project configurations.

## Setup

### Prerequisites

- Python 3.12+
- `uv` package manager
- A generated `sot.json` file for your project (produced by the kloc pipeline: PHP source -> scip-php -> kloc-mapper -> sot.json)

### Single project

Add to your MCP client configuration (e.g., `.claude/settings.json`, `.cursor/mcp.json`):

```json
{
  "mcpServers": {
    "kloc": {
      "command": "uv",
      "args": ["run", "kloc-cli", "mcp-server", "--sot", "/path/to/sot.json"],
      "cwd": "/path/to/kloc-cli"
    }
  }
}
```

### Multi-project

Create a config file (e.g., `kloc.json`):

```json
{
  "projects": [
    {"name": "my-app", "sot": "/path/to/my-app/sot.json"},
    {"name": "payments", "sot": "/path/to/payments/sot.json"}
  ]
}
```

Then point the MCP server at it:

```json
{
  "mcpServers": {
    "kloc": {
      "command": "uv",
      "args": ["run", "kloc-cli", "mcp-server", "--config", "/path/to/kloc.json"],
      "cwd": "/path/to/kloc-cli"
    }
  }
}
```

With multiple projects configured, every tool call accepts an optional `project` parameter. If omitted with a single project, it defaults automatically. With multiple projects, `project` is required -- call `kloc_projects` first to list available names.

## Available Tools

### kloc_projects

List all configured projects.

**When to use:** Discover which projects are available before running queries, especially in multi-project setups.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| *(none)* | | | |

**Returns:** Array of `{name, sot}` objects.

---

### kloc_resolve

Resolve a symbol to its definition location. Supports fully-qualified names, partial matches, and method syntax (`ClassName::methodName()`).

**When to use:** "Where is this symbol defined?" -- find the file, line number, and signature for any class, method, property, interface, or trait.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `symbol` | string | yes | Symbol to resolve (FQN, partial, or `Class::method()`) |
| `project` | string | no | Project name (required if multiple projects configured) |

**Returns:** `{id, kind, name, fqn, file, line, signature}`.

**Example questions:**
- "Where is `OrderService` defined?"
- "What file contains `App\Repository\UserRepository::findByEmail()`?"

---

### kloc_usages

Find all usages of a symbol with breadth-first depth expansion.

**When to use:** "What calls this method?" or "What references this class?" -- trace inbound references to understand who depends on a symbol.

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `symbol` | string | yes | | Symbol to find usages for |
| `depth` | integer | no | 1 | BFS depth (1 = direct callers, 2 = callers of callers, etc.) |
| `limit` | integer | no | 50 | Maximum results |
| `project` | string | no | | Project name |

**Returns:** Tree of usages with `{depth, fqn, file, line, children}` per entry.

**Example questions:**
- "What calls `UserRepository::save()`?"
- "Show me everything that uses `OrderService` up to 3 levels deep"

---

### kloc_deps

Find all dependencies of a symbol with breadth-first depth expansion.

**When to use:** "What does this class depend on?" or "What does this method call?" -- trace outbound references to understand what a symbol relies on.

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `symbol` | string | yes | | Symbol to find dependencies for |
| `depth` | integer | no | 1 | BFS depth |
| `limit` | integer | no | 50 | Maximum results |
| `project` | string | no | | Project name |

**Returns:** Tree of dependencies with `{depth, fqn, file, line, children}` per entry.

**Example questions:**
- "What does `OrderService::placeOrder()` call?"
- "What are the transitive dependencies of `PaymentGateway`?"

---

### kloc_context

Bidirectional context: combines usages (who calls it) and dependencies (what it calls) in a single query. Optionally includes implementations and overrides for polymorphic analysis.

**When to use:** "Show me the full picture" -- understand both how a symbol is used and what it uses. This is the most powerful query for impact analysis and refactoring.

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `symbol` | string | yes | | Symbol to get context for |
| `depth` | integer | no | 1 | BFS depth |
| `limit` | integer | no | 50 | Maximum results per direction |
| `include_impl` | boolean | no | false | Include implementations/overrides for polymorphic analysis |
| `project` | string | no | | Project name |

**Returns:** `{target, definition, usedBy, uses}` where:
- `target` -- the resolved symbol with file and FQN
- `definition` -- full definition including signature, return type, declared-in class, properties, methods, extends/implements
- `usedBy` -- tree of inbound references (with reference types, access chains, arguments, result variables)
- `uses` -- tree of outbound dependencies (with the same detail)

With `include_impl: true`:
- **USES direction:** includes implementations of interfaces and overriding methods
- **USED BY direction:** includes callers that reference an interface method which this concrete method implements

**Example questions:**
- "Show me the full picture of `OrderService` -- who uses it and what it depends on"
- "Trace the data flow through `PaymentProcessor::charge()` with implementations"

---

### kloc_owners

Structural containment chain: shows what class a method belongs to, and what file that class lives in.

**When to use:** "What class does this method belong to?" or "What file is this in?" -- navigate the structural hierarchy.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `symbol` | string | yes | Symbol to find ownership for |
| `project` | string | no | Project name |

**Returns:** `{chain}` -- array of `{kind, fqn, file}` from leaf to root (e.g., Method -> Class -> File).

**Example questions:**
- "What class does `processPayment()` belong to?"
- "Where in the file hierarchy is `App\Service\OrderService::validate()`?"

---

### kloc_inherit

Inheritance tree for a class, interface, trait, or enum.

**When to use:** "What classes extend this?" or "What does this class inherit from?" -- explore the type hierarchy.

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `symbol` | string | yes | | Class/interface to show inheritance for |
| `direction` | string | no | `"up"` | `up` = ancestors (parents, interfaces), `down` = descendants (subclasses, implementors) |
| `depth` | integer | no | 1 | BFS depth |
| `limit` | integer | no | 100 | Maximum results |
| `project` | string | no | | Project name |

**Returns:** `{root, direction, tree}` with nested `{depth, kind, fqn, file, line, children}`.

**Example questions:**
- "What classes implement `PaymentGatewayInterface`?" -> `direction: "down"`
- "What does `PremiumUser` extend?" -> `direction: "up"`
- "Show me the full inheritance tree of `AbstractController` down 3 levels" -> `direction: "down", depth: 3`

---

### kloc_overrides

Method override tree: find which methods override a given method, or what method a given method overrides.

**When to use:** "Does this method override something?" or "What methods override this one?" -- trace method polymorphism.

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `symbol` | string | yes | | Method to show overrides for |
| `direction` | string | no | `"up"` | `up` = methods this overrides, `down` = methods that override this |
| `depth` | integer | no | 1 | BFS depth |
| `limit` | integer | no | 100 | Maximum results |
| `project` | string | no | | Project name |

**Returns:** `{root, direction, tree}` with nested `{depth, fqn, file, line, children}`.

**Example questions:**
- "Does `UserRepository::findById()` override a parent method?" -> `direction: "up"`
- "What concrete methods override `RepositoryInterface::save()`?" -> `direction: "down"`

## Example Workflows

### 1. Impact analysis before refactoring

You want to refactor `OrderService` and need to know what will break.

1. **Get the full picture:** `kloc_context` with `symbol: "OrderService"`, `depth: 2`
   - `usedBy` shows every caller up to 2 levels -- controllers, other services, commands
   - `uses` shows every dependency -- repositories, value objects, external services
2. **Check implementations:** `kloc_context` with `symbol: "OrderService"`, `include_impl: true`
   - If `OrderService` implements an interface, this reveals callers that reference the interface rather than the concrete class
3. **Check subclasses:** `kloc_inherit` with `symbol: "OrderService"`, `direction: "down"`
   - Shows any subclasses that would be affected by changes to the base class

### 2. Understanding an interface's implementation landscape

You see `PaymentGatewayInterface` in the code and want to know who implements it and where it is used.

1. **Find implementations:** `kloc_inherit` with `symbol: "PaymentGatewayInterface"`, `direction: "down"`
   - Lists all classes that implement the interface
2. **Find usages with polymorphism:** `kloc_context` with `symbol: "PaymentGatewayInterface::charge()"`, `include_impl: true`
   - `usedBy` shows every call site, including calls through the interface
   - `uses` shows what each implementation depends on
3. **Check override chain:** `kloc_overrides` with `symbol: "StripeGateway::charge()"`, `direction: "up"`
   - Confirms which interface method it implements

### 3. Tracing data flow through a method

You want to understand the call chain around `InvoiceService::generate()`.

1. **Bidirectional context:** `kloc_context` with `symbol: "InvoiceService::generate()"`, `depth: 2`, `include_impl: true`
   - `usedBy` at depth 2 traces: who calls `generate()`, and who calls *those* callers
   - `uses` shows what `generate()` calls internally -- repositories, formatters, mailers
   - With v2.0 sot.json, entries include `arguments` (what values are passed), `result_var` (what variable captures the return), and `access_chain` (how the method is accessed, e.g., `$this->invoiceService->generate()`)
2. **Locate the method:** `kloc_resolve` with `symbol: "InvoiceService::generate()"`
   - Get the exact file and line to read the source code

## Symbol Resolution

All tools accept flexible symbol syntax:

| Syntax | Example | Matches |
|--------|---------|---------|
| Fully-qualified name | `App\Service\OrderService` | Exact FQN match |
| Short class name | `OrderService` | Partial match (suffix) |
| Method syntax | `OrderService::placeOrder()` | Class method |
| Method name only | `placeOrder` | Any method with that name (may be ambiguous) |

If a symbol is ambiguous (multiple matches), the server returns all candidates with their IDs, kinds, and FQNs so you can refine the query.
