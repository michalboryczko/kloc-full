# kloc-cli Usage Guide

kloc-cli is a command-line tool for querying PHP codebase structure from a Source-of-Truth JSON file (sot.json). It answers questions like "who calls this?", "what does this depend on?", and "what implements this interface?" -- all without reading source code directly.

## Commands

### resolve -- Find where a symbol is defined

**Question it answers:** "Where is `OrderService` defined?"

Looks up a symbol by its fully-qualified name (FQN), partial name, or method syntax and returns its definition location (file, line number, kind).

```bash
uv run kloc-cli resolve "App\Service\OrderService" -s sot.json
uv run kloc-cli resolve "OrderService" -s sot.json
uv run kloc-cli resolve "OrderService::createOrder()" -s sot.json
```

If the symbol is ambiguous (multiple matches), kloc-cli lists all candidates so you can refine your query.

### usages -- Who uses this?

**Question it answers:** "What calls `OrderService::createOrder()`?"

Finds all places that reference a symbol, with BFS depth expansion. At depth 1, you see direct callers. At depth 2, you also see who calls those callers, and so on.

```bash
# Direct usages only
uv run kloc-cli usages "App\Service\OrderService" -s sot.json

# Two levels deep: who uses it, and who uses those
uv run kloc-cli usages "App\Service\OrderService::createOrder()" -s sot.json -d 2
```

### deps -- What does this depend on?

**Question it answers:** "What does `OrderService` use internally?"

Finds everything a symbol depends on: classes it instantiates, methods it calls, types it references. Depth expansion follows the dependency chain outward.

```bash
# What does createOrder() call directly?
uv run kloc-cli deps "App\Service\OrderService::createOrder()" -s sot.json

# Two levels: what it calls, and what those call
uv run kloc-cli deps "App\Service\OrderService::createOrder()" -s sot.json -d 2
```

### context -- Full bidirectional picture

**Question it answers:** "Give me the complete picture of `OrderService`."

Combines USAGES (who calls it) + DEPS (what it uses) + DEFINITION (signature, properties, methods) into a single view. This is the most powerful command for understanding a symbol in its full context.

```bash
# Complete picture of OrderService
uv run kloc-cli context "App\Service\OrderService" -s sot.json

# Deeper exploration
uv run kloc-cli context "App\Service\OrderService" -s sot.json -d 2

# Include interface implementations and polymorphic callers
uv run kloc-cli context "App\Service\OrderService::createOrder()" -s sot.json --impl

# Show only direct references (no member usages)
uv run kloc-cli context "App\Service\OrderService" -s sot.json --direct

# Include PHP import/use statements in USED BY output
uv run kloc-cli context "App\Service\OrderService" -s sot.json --with-imports
```

With `--impl` (polymorphic analysis):
- **USES direction:** includes implementations of interfaces and overriding methods
- **USED BY direction:** includes usages of interface methods that concrete methods implement

With `--direct`:
- **USED BY direction:** shows only direct references to the symbol itself (extends, implements, type hints), excluding usages that only reference its members

When sot.json is v2.0 format (with Value and Call nodes), context also provides:
- Accurate reference types (method_call vs type_hint vs instantiation)
- Access chains showing how a method is accessed (e.g., `$this->repository->save()`)
- Argument values and data flow information

### owners -- Containment chain

**Question it answers:** "What class/file contains this method?"

Walks the structural containment chain upward: Method -> Class -> File. Useful for locating where a symbol lives in the codebase hierarchy.

```bash
uv run kloc-cli owners "App\Service\OrderService::createOrder()" -s sot.json
# Output: createOrder() -> OrderService (Class) -> src/Service/OrderService.php (File)
```

### inherit -- Inheritance tree

**Question it answers:** "What extends `AbstractProcessor`?" or "What does `OrderController` extend?"

Shows the inheritance tree for a class, interface, trait, or enum. Use `--direction up` to find ancestors (what does it extend/implement?) or `--direction down` to find descendants (what extends/implements it?).

```bash
# What does OrderController extend? (ancestors)
uv run kloc-cli inherit "App\Controller\OrderController" -s sot.json --direction up

# What implements OrderRepositoryInterface? (descendants)
uv run kloc-cli inherit "App\Repository\OrderRepositoryInterface" -s sot.json --direction down

# Deep inheritance tree
uv run kloc-cli inherit "App\Repository\OrderRepositoryInterface" -s sot.json --direction down -d 3
```

### overrides -- Method override tree

**Question it answers:** "What implementations exist for `process()`?"

Shows which methods override a given method, or which method a given method overrides. Works with `--direction up` (find the method being overridden) or `--direction down` (find overriding methods).

```bash
# What methods override process()?
uv run kloc-cli overrides "App\Processor\AbstractProcessor::process()" -s sot.json --direction down

# What does this method override?
uv run kloc-cli overrides "App\Processor\OrderProcessor::process()" -s sot.json --direction up
```

## Common Flags

| Flag | Short | Description | Default |
|------|-------|-------------|---------|
| `--sot PATH` | `-s` | Path to sot.json file | (required) |
| `--depth N` | `-d` | BFS depth for expansion | 1 |
| `--limit N` | `-l` | Maximum results returned | 100 |
| `--json` | `-j` | Output as JSON instead of rich console formatting | false |
| `--impl` | `-i` | Include implementations/overrides (context only) | false |
| `--direct` | | Show only direct symbol references (context only) | false |
| `--with-imports` | | Include PHP import/use statements (context only) | false |
| `--direction` | | `up` or `down` (inherit and overrides only) | up |

## Practical Examples

```bash
# What depends on OrderService? Full picture at depth 2
uv run kloc-cli context "App\Service\OrderService" -s sot.json -d 2

# Find all implementations of a repository interface
uv run kloc-cli inherit "App\Repository\OrderRepositoryInterface" -s sot.json --direction down

# Check what a method calls internally
uv run kloc-cli deps "App\Service\OrderService::createOrder()" -s sot.json -d 2

# Who calls this method? Including callers through the interface
uv run kloc-cli context "App\Service\OrderService::createOrder()" -s sot.json --impl

# Get JSON output for scripting/piping
uv run kloc-cli usages "App\Entity\Order" -s sot.json -d 2 --json

# Locate a method in the codebase structure
uv run kloc-cli owners "App\Service\OrderService::createOrder()" -s sot.json

# Find all overrides of an abstract method
uv run kloc-cli overrides "App\Processor\AbstractProcessor::process()" -s sot.json --direction down -d 2

# Resolve a symbol with partial name match
uv run kloc-cli resolve "OrderService" -s sot.json
```

## MCP Server (AI Assistant Integration)

kloc-cli includes an MCP (Model Context Protocol) server for integration with Claude and other AI assistants. The server exposes all query commands as MCP tools over stdio.

### Single Project

```bash
uv run kloc-cli mcp-server --sot /path/to/sot.json
```

### Multi-Project

Create a `kloc.json` configuration file:

```json
{
  "projects": [
    {"name": "my-app", "sot": "/path/to/my-app/sot.json"},
    {"name": "payments", "sot": "/path/to/payments/sot.json"}
  ]
}
```

Then start the server:

```bash
uv run kloc-cli mcp-server --config kloc.json
```

### Available MCP Tools

| Tool | Description |
|------|-------------|
| `kloc_projects` | List available projects |
| `kloc_resolve` | Resolve symbol to definition |
| `kloc_usages` | Find usages with depth expansion |
| `kloc_deps` | Find dependencies with depth expansion |
| `kloc_context` | Bidirectional context (usages + deps + definition) |
| `kloc_owners` | Show containment chain |
| `kloc_inherit` | Show inheritance tree |
| `kloc_overrides` | Show method override tree |

In multi-project mode, every tool accepts an optional `project` parameter to specify which project to query. If only one project is configured, the parameter is optional.

## Symbol Resolution

kloc-cli supports several ways to identify a symbol:

- **Fully-qualified name:** `App\Service\OrderService`
- **Method syntax:** `App\Service\OrderService::createOrder()`
- **Partial match:** `OrderService` (matches if unique)
- **Case-insensitive:** matching is case-insensitive for convenience

If a partial name matches multiple symbols, kloc-cli lists all candidates and asks you to be more specific.
