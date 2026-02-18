# KLOC MVP - Understanding Summary

## Project Goal

**KLOC** (Knowledge of Code) aims to provide the best possible context for AI coding agents by mapping code structure and relationships into a queryable format.

## Scope of This MVP

This MVP implements two components:

1. **kloc-mapper**: Parse SCIP index → Produce Source-of-Truth JSON (SoT JSON)
2. **kloc-cli**: Load SoT JSON → Answer queries about usages, dependencies, context

**NOT in scope**: Graph database persistence, embeddings, semantic search.

---

## Source-of-Truth Model

### Node Types (13 kinds)

| Type | Description |
|------|-------------|
| File | Source file container |
| Class | Class definition |
| Interface | Interface definition |
| Trait | Trait definition |
| Enum | Enum definition |
| Method | Method definition (class/trait/interface member) |
| Function | Standalone function (not a class member) |
| Property | Class/trait property |
| Const | Constant (class or global) |
| Argument | Function/method parameter |
| EnumCase | Enum case value |
| Value | Runtime value: parameter, local variable, result, literal, or constant tracked for data flow |
| Call | Call site: method call, property access, constructor invocation, or operator expression |

### Edge Types (13 types)

#### Structural edges (from SCIP index)

| Edge | Description |
|------|-------------|
| `contains` | Structural containment (File→Class, Class→Method, etc.) |
| `extends` | Direct class/interface inheritance |
| `implements` | Direct interface implementation |
| `uses_trait` | Direct trait usage |
| `overrides` | Direct method override (same signature, direct parent) |
| `uses` | Direct reference (calls, type hints, instantiation) |
| `type_hint` | Type annotation reference (Argument/Property → Class/Interface) |

#### Call graph edges (from calls.json)

| Edge | Description |
|------|-------------|
| `calls` | Call → Method/Function/Property/Constructor being invoked |
| `receiver` | Call → Value that is the receiver object (for method calls / property access) |
| `argument` | Call → Value passed as argument (has `position` field for 0-based index) |
| `produces` | Call → Value that is the result of the call |
| `assigned_from` | Value → Value indicating assignment source |
| `type_of` | Value → Class/Interface indicating the runtime type |

**Key Principle**: Only direct relationships are stored. No transitive edges.

---

## Containment Hierarchy

### Structural entities (from SCIP index)

```
File
├── Class
│   ├── Property
│   ├── Const
│   └── Method
│       └── Argument
├── Interface
│   └── Method (signatures)
│       └── Argument
├── Trait
│   ├── Property
│   ├── Const
│   └── Method
│       └── Argument
├── Enum
│   ├── EnumCase
│   ├── Const
│   └── Method
│       └── Argument
├── Function
│   └── Argument
└── Const
```

### Runtime entities (from calls.json)

```
Method / Function
├── Call (call sites within the method/function body)
└── Value (parameters, locals, results, literals, constants)
```

Call and Value nodes are contained by the Method or Function in which they appear. They connect to structural nodes via `calls`, `receiver`, `argument`, `produces`, `assigned_from`, and `type_of` edges.

---

## CLI Commands (MVP)

| Command | Purpose |
|---------|---------|
| `resolve <symbol>` | Resolve symbol to node, show definition location |
| `usages <symbol>` | Find all direct usages (incoming `uses` edges) |
| `deps <symbol>` | Find all dependencies (outgoing `uses` edges) |
| `context <symbol> --depth N` | Combined usages + deps with BFS expansion |
| `owners <symbol>` | Show containment chain (Method→Class→File) |
| `inherit <class> --direction up\|down` | Show inheritance chain |
| `overrides <method> --direction up\|down` | Show override chain |

All commands support `--json` for machine-readable output.

---

## SCIP Index Structure

SCIP (Sourcegraph Code Intelligence Protocol) provides:

- **Documents**: Files with symbols and occurrences
- **Symbols**: Definitions with documentation and relationships
- **Occurrences**: References with file positions and roles
- **Relationships**: extends/implements metadata

Key fields from SCIP:
- `symbol`: Unique identifier (scheme + manager + package + version + descriptor)
- `range`: Position as `[line, startChar, endChar]` or `[startLine, startChar, endLine, endChar]`
- `symbol_roles`: Bitmask (Definition=0x1, WriteAccess=0x4, ReadAccess=0x8, etc.)
- `relationships`: Contains `is_implementation`, `is_reference`, etc.

---

## Mapping Pipeline

```
SCIP Index (.scip)
       │
       ▼
┌──────────────────┐
│  Parse Protobuf  │
└──────────────────┘
       │
       ▼
┌──────────────────┐
│ Extract Symbols  │  → File, Class, Method, etc.
└──────────────────┘
       │
       ▼
┌──────────────────┐
│ Build Edges      │  → contains, extends, uses, etc.
└──────────────────┘
       │
       ▼
┌──────────────────┐
│ Normalize IDs    │  → Stable, deterministic identifiers
└──────────────────┘
       │
       ▼
    SoT JSON
```

---

## In-Memory Indexes (CLI)

At startup, the CLI builds:

- `symbol_to_node_id`: Fast symbol lookup
- `node_id_to_node`: Node data access
- `incoming_edges[node_id][edge_type]`: Reverse edge lookup
- `outgoing_edges[node_id][edge_type]`: Forward edge lookup

This enables O(1) lookups without modifying the SoT.

---

## Key Files from PoC

| File | Purpose |
|------|---------|
| `python/parse_scip.py` | Existing SCIP parser (reference) |
| `python/scip_pb2.py` | Protobuf bindings for SCIP |
| `index.scip` | Sample SCIP index from PHP codebase |

---

## Important Correctness Notes

1. **parent:: resolution**: Must resolve to actual ancestor method, not just parent class node
2. **Deterministic output**: JSON must be reproducible for same input
3. **Direct facts only**: No transitive closures in SoT
4. **Override detection**: Only for same signature, direct parent chain
