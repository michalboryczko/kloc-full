"""Cypher queries for fetching definition data.

Batched queries that replace many individual SoTIndex lookups with
a small number of comprehensive Cypher queries.
"""

from __future__ import annotations

from typing import Optional

from ..query_runner import QueryRunner
from ...models.results import DefinitionInfo
from ...logic.definition import build_definition


# ─────────────────────────────────────────────────────────────────────
# Query 1: Node + Parent
# ─────────────────────────────────────────────────────────────────────

NODE_AND_PARENT = """
MATCH (n:Node {node_id: $node_id})
OPTIONAL MATCH (n)<-[:CONTAINS]-(parent:Node)
RETURN n.node_id AS id, n.fqn AS fqn, n.kind AS kind,
       n.file AS file, n.start_line AS start_line,
       n.name AS name, n.documentation AS documentation,
       n.value_kind AS value_kind,
       parent.node_id AS parent_id, parent.fqn AS parent_fqn,
       parent.kind AS parent_kind, parent.file AS parent_file,
       parent.start_line AS parent_line,
       parent.documentation AS parent_documentation
"""

# ─────────────────────────────────────────────────────────────────────
# Query 2: Children (ordered by edge_idx)
# ─────────────────────────────────────────────────────────────────────

CHILDREN = """
MATCH (n:Node {node_id: $node_id})-[c:CONTAINS]->(child:Node)
OPTIONAL MATCH (child)-[:OVERRIDES]->(override_parent:Node)
RETURN child.node_id AS id, child.fqn AS fqn, child.kind AS kind,
       child.name AS name, child.file AS file,
       child.start_line AS start_line,
       child.documentation AS documentation,
       child.value_kind AS value_kind,
       CASE WHEN override_parent IS NOT NULL THEN true ELSE false END AS has_override
ORDER BY c.edge_idx
"""

# ─────────────────────────────────────────────────────────────────────
# Query 3: Type hint edges for node + children
# ─────────────────────────────────────────────────────────────────────

TYPE_HINTS = """
MATCH (n:Node {node_id: $node_id})
OPTIONAL MATCH (n)-[:CONTAINS]->(child:Node)
WITH collect(child.node_id) + [$node_id] AS node_ids
UNWIND node_ids AS nid
MATCH (source:Node {node_id: nid})-[:TYPE_HINT]->(target:Node)
RETURN source.node_id AS source_id,
       target.node_id AS target_id,
       target.fqn AS target_fqn,
       target.name AS target_name
"""

# ─────────────────────────────────────────────────────────────────────
# Query 4: Inheritance (extends, implements, uses_trait)
# ─────────────────────────────────────────────────────────────────────

INHERITANCE = """
MATCH (n:Node {node_id: $node_id})
OPTIONAL MATCH (n)-[:EXTENDS]->(parent_class:Node)
OPTIONAL MATCH (n)-[:IMPLEMENTS]->(iface:Node)
OPTIONAL MATCH (n)-[:USES_TRAIT]->(trait:Node)
RETURN parent_class.fqn AS extends_fqn,
       COLLECT(DISTINCT iface.fqn) AS implements_fqns,
       COLLECT(DISTINCT trait.fqn) AS uses_trait_fqns
"""

# ─────────────────────────────────────────────────────────────────────
# Query 5: Constructor deps (promoted properties)
# ─────────────────────────────────────────────────────────────────────

CONSTRUCTOR_DEPS = """
MATCH (n:Node {node_id: $node_id})-[:CONTAINS]->(prop:Node)
WHERE prop.kind = 'Property'
MATCH (prop)-[:ASSIGNED_FROM]->(val:Node)
WHERE val.kind = 'Value' AND val.value_kind = 'parameter' AND val.fqn CONTAINS '__construct()'
OPTIONAL MATCH (prop)-[:TYPE_HINT]->(prop_type:Node)
RETURN prop.name AS prop_name, prop.fqn AS prop_fqn,
       prop_type.name AS type_name
ORDER BY val.start_line ASC
"""

# ─────────────────────────────────────────────────────────────────────
# Query 6: Value node source resolution
# ─────────────────────────────────────────────────────────────────────

VALUE_SOURCE = """
MATCH (v:Node {node_id: $node_id})
OPTIONAL MATCH (v)-[:ASSIGNED_FROM]->(source:Node)
OPTIONAL MATCH (source)<-[:PRODUCES]-(source_call:Node)-[:CALLS]->(callee:Node)
OPTIONAL MATCH (v)-[:TYPE_OF]->(vtype:Node)
RETURN source.kind AS source_kind, source.fqn AS source_fqn,
       source.node_id AS source_id, source.file AS source_file,
       source.start_line AS source_line,
       source_call.fqn AS source_call_fqn, source_call.file AS source_call_file,
       source_call.start_line AS source_call_line,
       callee.fqn AS callee_fqn, callee.name AS callee_name, callee.kind AS callee_kind,
       COLLECT(DISTINCT {fqn: vtype.fqn, name: vtype.name}) AS type_of_all
"""

# ─────────────────────────────────────────────────────────────────────
# Query 7: Value node scope (containing method/function)
# ─────────────────────────────────────────────────────────────────────

VALUE_SCOPE = """
MATCH path = (v:Node {node_id: $node_id})<-[:CONTAINS*1..10]-(ancestor:Node)
WHERE ancestor.kind IN ['Method', 'Function']
WITH ancestor, length(path) AS dist
ORDER BY dist ASC
LIMIT 1
RETURN ancestor.node_id AS scope_id, ancestor.fqn AS scope_fqn,
       ancestor.kind AS scope_kind, ancestor.file AS scope_file,
       ancestor.start_line AS scope_line
"""

# ─────────────────────────────────────────────────────────────────────
# Query 8: Property promoted detection
# ─────────────────────────────────────────────────────────────────────

PROPERTY_PROMOTED = """
MATCH (prop:Node {node_id: $node_id})-[:ASSIGNED_FROM]->(val:Node)
WHERE val.kind = 'Value' AND val.value_kind = 'parameter' AND val.fqn CONTAINS '__construct()'
RETURN count(*) > 0 AS is_promoted
"""

# ─────────────────────────────────────────────────────────────────────
# Query 9: Method signature extraction (for children)
# ─────────────────────────────────────────────────────────────────────

CHILD_SIGNATURES = """
MATCH (n:Node {node_id: $node_id})-[:CONTAINS]->(child:Node)
WHERE child.kind = 'Method' AND child.documentation IS NOT NULL
RETURN child.node_id AS id, child.documentation AS documentation
"""


def _extract_signature_from_doc(documentation: list[str]) -> Optional[str]:
    """Extract method signature from documentation, matching NodeData.signature property."""
    import re as _re
    _RE_VISIBILITY = _re.compile(
        r"^(?:public\s+|protected\s+|private\s+|static\s+|final\s+|abstract\s+)*function\s+"
    )
    _RE_ATTRIBUTES = _re.compile(r"#\[[^\]]*\]\s*")
    _RE_WHITESPACE = _re.compile(r"\s+")

    if not documentation:
        return None

    for doc in documentation:
        clean = doc.replace("```php", "").replace("```", "").strip()
        if "function " in clean:
            sig_lines = []
            capturing = False
            for line in clean.split("\n"):
                line = line.strip()
                if "function " in line:
                    capturing = True
                if capturing:
                    sig_lines.append(line)
                    if ")" in line:
                        break

            if not sig_lines:
                continue

            full_sig = " ".join(sig_lines)
            full_sig = _RE_VISIBILITY.sub("", full_sig)
            full_sig = _RE_ATTRIBUTES.sub("", full_sig)
            full_sig = _RE_WHITESPACE.sub(" ", full_sig).strip()

            if "(" in full_sig and ")" in full_sig:
                return full_sig
            if "(" in full_sig:
                method_name = full_sig.split("(")[0]
                return f"{method_name}(...)"
            return full_sig
    return None


def fetch_definition_data(runner: QueryRunner, node_id: str) -> dict:
    """Fetch all data needed for build_definition() using Cypher queries.

    Runs batched queries and merges results into a single data dict
    suitable for passing to build_definition().

    Args:
        runner: QueryRunner instance.
        node_id: Node ID to build definition for.

    Returns:
        Merged data dict with all definition fields.
    """
    # Query 1: Node + Parent
    node_record = runner.execute_single(NODE_AND_PARENT, node_id=node_id)
    if not node_record:
        return {"fqn": "unknown", "kind": "unknown"}

    data: dict = {
        "id": node_record["id"],
        "fqn": node_record["fqn"],
        "kind": node_record["kind"],
        "file": node_record["file"],
        "start_line": node_record["start_line"],
        "name": node_record["name"],
        "documentation": node_record["documentation"] or [],
        "value_kind": node_record["value_kind"],
        "parent_id": node_record["parent_id"],
        "parent_fqn": node_record["parent_fqn"],
        "parent_kind": node_record["parent_kind"],
        "parent_file": node_record["parent_file"],
        "parent_line": node_record["parent_line"],
        "parent_documentation": node_record["parent_documentation"] or [],
    }

    # Extract signature from documentation for the node itself
    data["signature"] = _extract_signature_from_doc(data["documentation"])

    kind = data["kind"]

    # Query 2: Children (for class/interface/method/function)
    if kind in ("Class", "Interface", "Trait", "Enum", "Method", "Function"):
        children_records = runner.execute(CHILDREN, node_id=node_id)
        children = []
        for rec in children_records:
            child = {
                "id": rec["id"],
                "fqn": rec["fqn"],
                "kind": rec["kind"],
                "name": rec["name"],
                "file": rec["file"],
                "start_line": rec["start_line"],
                "documentation": rec["documentation"] or [],
                "value_kind": rec["value_kind"],
                "has_override": rec["has_override"],
            }
            # Extract signature for method children
            if child["kind"] == "Method":
                child["signature"] = _extract_signature_from_doc(child["documentation"])
            children.append(child)
        data["children"] = children

    # Query 3: Type hints
    if kind in ("Class", "Interface", "Trait", "Enum", "Method", "Function", "Property", "Argument"):
        type_hint_records = runner.execute(TYPE_HINTS, node_id=node_id)
        type_hints: dict[str, list[dict]] = {}
        for rec in type_hint_records:
            source_id = rec["source_id"]
            if source_id not in type_hints:
                type_hints[source_id] = []
            type_hints[source_id].append({
                "target_id": rec["target_id"],
                "target_fqn": rec["target_fqn"],
                "target_name": rec["target_name"],
            })
        data["type_hints"] = type_hints

    # Query 4: Inheritance
    if kind in ("Class", "Interface", "Trait", "Enum"):
        inherit_record = runner.execute_single(INHERITANCE, node_id=node_id)
        if inherit_record:
            data["inheritance"] = {
                "extends_fqn": inherit_record["extends_fqn"],
                "implements_fqns": [f for f in (inherit_record["implements_fqns"] or []) if f],
                "uses_trait_fqns": [f for f in (inherit_record["uses_trait_fqns"] or []) if f],
            }
        else:
            data["inheritance"] = {}

    # Query 5: Constructor deps
    if kind in ("Class", "Trait", "Enum"):
        deps_records = runner.execute(CONSTRUCTOR_DEPS, node_id=node_id)
        data["constructor_deps"] = [
            {
                "prop_name": rec["prop_name"],
                "prop_fqn": rec["prop_fqn"],
                "type_name": rec["type_name"],
            }
            for rec in deps_records
        ]

    # Property-specific: promoted detection
    if kind == "Property":
        promoted_record = runner.execute_single(PROPERTY_PROMOTED, node_id=node_id)
        data["is_promoted"] = bool(promoted_record and promoted_record["is_promoted"])

    # Value-specific: source + type resolution
    if kind == "Value":
        value_record = runner.execute_single(VALUE_SOURCE, node_id=node_id)
        value_data: dict = {}
        if value_record:
            # Type resolution
            type_of_all = value_record["type_of_all"] or []
            # Filter out null entries
            type_of_all = [t for t in type_of_all if t.get("fqn") or t.get("name")]
            value_data["type_of_all"] = type_of_all

            # Source resolution
            source_kind = value_record["source_kind"]
            if source_kind == "Property":
                # Property promotion
                value_data["source"] = {
                    "call_fqn": None,
                    "method_fqn": value_record["source_fqn"],
                    "method_name": f"promotes to {value_record['source_fqn']}",
                    "file": value_record["source_file"],
                    "line": value_record["source_line"],
                }
            elif value_record["source_call_fqn"] and value_record["callee_fqn"]:
                method_display = value_record["callee_name"]
                if value_record["callee_kind"] in ("Method", "Function"):
                    method_display = f"{value_record['callee_name']}()"
                value_data["source"] = {
                    "call_fqn": value_record["source_call_fqn"],
                    "method_fqn": value_record["callee_fqn"],
                    "method_name": method_display,
                    "file": value_record["source_call_file"],
                    "line": value_record["source_call_line"],
                }
            elif source_kind is None and data.get("value_kind") == "result":
                # For result values: source_call points directly via PRODUCES
                # This case is already handled by the query's PRODUCES join
                pass

        # Scope resolution
        scope_record = runner.execute_single(VALUE_SCOPE, node_id=node_id)
        if scope_record and scope_record.get("scope_id"):
            value_data["scope"] = {
                "fqn": scope_record["scope_fqn"],
                "kind": scope_record["scope_kind"],
                "file": scope_record["scope_file"],
                "line": scope_record["scope_line"],
            }

        data["value_data"] = value_data

    return data


def definition_for_node(runner: QueryRunner, node_id: str) -> DefinitionInfo:
    """Fetch and build definition for a node.

    This is the main entry point: fetches data from Neo4j and builds
    the DefinitionInfo using the pure-logic builder.

    Args:
        runner: QueryRunner instance.
        node_id: Node ID to build definition for.

    Returns:
        DefinitionInfo with symbol metadata.
    """
    data = fetch_definition_data(runner, node_id)
    return build_definition(data)
