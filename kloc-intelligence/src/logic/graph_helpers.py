"""Graph helper functions for context command.

Ported from kloc-cli/src/queries/graph_utils.py.
Functions that needed SoTIndex access now use QueryRunner for Cypher queries.
Pure logic functions are ported as-is.
"""

from __future__ import annotations

from typing import Optional

from ..db.query_runner import QueryRunner


# =============================================================================
# Pure logic helpers (no graph access)
# =============================================================================


def member_display_name(kind: str, name: str) -> str:
    """Format a short member display name: '$prop', 'method()', 'CONST'.

    Args:
        kind: Node kind.
        name: Node name.

    Returns:
        Formatted display name.
    """
    if kind in ("Method", "Function"):
        return f"{name}()"
    if kind == "Property":
        return name if name.startswith("$") else f"${name}"
    return name


# =============================================================================
# Cypher-based helpers
# =============================================================================


# Get the containing Method/Function for a node
CONTAINING_METHOD_QUERY = """
MATCH (n:Node {node_id: $id})
WHERE n.kind IN ['Method', 'Function']
RETURN n.node_id AS method_id, n.fqn AS method_fqn, n.kind AS method_kind
"""

CONTAINING_METHOD_TRAVERSAL = """
MATCH path = (n:Node {node_id: $id})<-[:CONTAINS*1..10]-(ancestor:Node)
WHERE ancestor.kind IN ['Method', 'Function']
WITH ancestor, length(path) AS dist
ORDER BY dist ASC
LIMIT 1
RETURN ancestor.node_id AS method_id, ancestor.fqn AS method_fqn, ancestor.kind AS method_kind
"""


def resolve_containing_method(
    runner: QueryRunner, node_id: str
) -> tuple[Optional[str], Optional[str], Optional[str]]:
    """Resolve the containing Method/Function for a given node.

    If the node IS a Method/Function, returns it directly.
    If the node is a File, returns (None, None, None).
    Otherwise traverses containment upward.

    Args:
        runner: QueryRunner instance.
        node_id: Node ID to resolve.

    Returns:
        Tuple of (method_id, method_fqn, method_kind) or (None, None, None).
    """
    # First check if node itself is a Method/Function
    record = runner.execute_single(CONTAINING_METHOD_QUERY, id=node_id)
    if record and record["method_id"]:
        return record["method_id"], record["method_fqn"], record["method_kind"]

    # Check if node is a File (don't chain from files)
    kind_record = runner.execute_single(
        "MATCH (n:Node {node_id: $id}) RETURN n.kind AS kind", id=node_id
    )
    if kind_record and kind_record["kind"] == "File":
        return None, None, None

    # Traverse containment upward
    record = runner.execute_single(CONTAINING_METHOD_TRAVERSAL, id=node_id)
    if record and record["method_id"]:
        return record["method_id"], record["method_fqn"], record["method_kind"]

    return None, None, None


def is_internal_reference(
    runner: QueryRunner, source_id: str, target_class_id: str
) -> bool:
    """Check if a source node is internal to the target class (R3).

    Args:
        runner: QueryRunner instance.
        source_id: The source node of the reference.
        target_class_id: The class being queried.

    Returns:
        True if the source is contained within the target class.
    """
    record = runner.execute_single(
        """
        MATCH path = (source:Node {node_id: $source_id})<-[:CONTAINS*]-(cls:Node {node_id: $cls_id})
        RETURN count(path) > 0 AS is_internal
        """,
        source_id=source_id,
        cls_id=target_class_id,
    )
    if record:
        return record["is_internal"]
    return False


def get_contains_parent(runner: QueryRunner, node_id: str) -> Optional[str]:
    """Get the parent that contains this node.

    Args:
        runner: QueryRunner instance.
        node_id: Node ID.

    Returns:
        Parent node ID or None.
    """
    record = runner.execute_single(
        "MATCH (child:Node {node_id: $id})<-[:CONTAINS]-(parent:Node) RETURN parent.node_id AS pid",
        id=node_id,
    )
    if record:
        return record["pid"]
    return None


def get_contains_children(runner: QueryRunner, node_id: str) -> list[str]:
    """Get child node IDs contained by this node.

    Args:
        runner: QueryRunner instance.
        node_id: Parent node ID.

    Returns:
        List of child node IDs.
    """
    records = runner.execute(
        "MATCH (parent:Node {node_id: $id})-[c:CONTAINS]->(child:Node) RETURN child.node_id AS cid ORDER BY c.edge_idx",
        id=node_id,
    )
    return [r["cid"] for r in records]


def resolve_containing_class(
    runner: QueryRunner, node_id: str
) -> tuple[Optional[str], Optional[str]]:
    """Resolve the containing class for a node.

    Traverses containment upward to find the nearest Class/Interface/Trait/Enum.

    Args:
        runner: QueryRunner instance.
        node_id: Node ID.

    Returns:
        Tuple of (class_id, class_fqn) or (None, None).
    """
    record = runner.execute_single(
        """
        MATCH path = (n:Node {node_id: $id})<-[:CONTAINS*1..10]-(ancestor:Node)
        WHERE ancestor.kind IN ['Class', 'Interface', 'Trait', 'Enum']
        WITH ancestor, length(path) AS dist
        ORDER BY dist ASC
        LIMIT 1
        RETURN ancestor.node_id AS cls_id, ancestor.fqn AS cls_fqn
        """,
        id=node_id,
    )
    if record and record["cls_id"]:
        return record["cls_id"], record["cls_fqn"]
    return None, None
