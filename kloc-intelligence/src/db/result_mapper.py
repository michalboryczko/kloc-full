"""Map Neo4j Records to kloc-intelligence data models.

Converts Neo4j Record objects to typed Python dataclasses (NodeData, etc.).
This is the bridge between raw Cypher results and the application layer.
"""

from __future__ import annotations

from neo4j import Record

from ..models.node import NodeData


def record_to_node(record: Record, key: str = "n") -> NodeData:
    """Convert a Neo4j Record containing a node to NodeData.

    Args:
        record: Neo4j Record from a query like "MATCH (n:Node ...) RETURN n"
        key: The variable name used in the RETURN clause.

    Returns:
        NodeData instance with all properties mapped.
    """
    node = record[key]  # Neo4j Node object (dict-like)
    return NodeData(
        node_id=node["node_id"],
        kind=node["kind"],
        name=node["name"],
        fqn=node["fqn"],
        symbol=node["symbol"],
        file=node.get("file"),
        start_line=node.get("start_line"),
        start_col=node.get("start_col"),
        end_line=node.get("end_line"),
        end_col=node.get("end_col"),
        documentation=node.get("documentation", []),
        value_kind=node.get("value_kind"),
        type_symbol=node.get("type_symbol"),
        call_kind=node.get("call_kind"),
    )


def records_to_nodes(records: list[Record], key: str = "n") -> list[NodeData]:
    """Convert multiple records to NodeData list."""
    return [record_to_node(r, key) for r in records]


def record_to_flat_node(record: Record) -> NodeData:
    """Convert a Record with flat properties (not a Neo4j Node object).

    Used when RETURN returns individual properties:
    RETURN n.node_id AS node_id, n.fqn AS fqn, ...
    """
    return NodeData(
        node_id=record["node_id"],
        kind=record["kind"],
        name=record.get("name", ""),
        fqn=record.get("fqn", ""),
        symbol=record.get("symbol", ""),
        file=record.get("file"),
        start_line=record.get("start_line"),
    )
