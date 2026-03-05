"""Data import pipeline: parse sot.json and load into Neo4j.

Handles parsing, batch node import, batch edge import, and validation.
"""

from __future__ import annotations

import logging
from pathlib import Path
from typing import Optional

import msgspec

from .connection import Neo4jConnection

logger = logging.getLogger(__name__)

# --------------------------------------------------------------------------
# sot.json msgspec structs (reimplemented from kloc-cli for independence)
# --------------------------------------------------------------------------


class RangeSpec(msgspec.Struct, omit_defaults=True):
    """Range specification in SoT JSON."""

    start_line: int
    start_col: int
    end_line: int
    end_col: int


class LocationSpec(msgspec.Struct, omit_defaults=True):
    """Location specification in SoT JSON."""

    file: str
    line: int
    col: Optional[int] = None


class NodeSpec(msgspec.Struct, omit_defaults=True):
    """Node specification in SoT JSON."""

    id: str
    kind: str
    name: str
    fqn: str
    symbol: str
    file: Optional[str] = None
    range: Optional[dict] = None
    documentation: list[str] = []
    value_kind: Optional[str] = None
    type_symbol: Optional[str] = None
    call_kind: Optional[str] = None
    enclosing_range: Optional[dict] = None


class EdgeSpec(msgspec.Struct, omit_defaults=True):
    """Edge specification in SoT JSON."""

    type: str
    source: str
    target: str
    location: Optional[dict] = None
    position: Optional[int] = None
    expression: Optional[str] = None
    parameter: Optional[str] = None


class SoTSpec(msgspec.Struct, omit_defaults=True):
    """Full SoT JSON specification."""

    version: str = "1.0"
    metadata: dict = {}
    nodes: list[NodeSpec] = []
    edges: list[EdgeSpec] = []


# Reusable decoder for performance
_decoder = msgspec.json.Decoder(SoTSpec)

# --------------------------------------------------------------------------
# Constants
# --------------------------------------------------------------------------

BATCH_SIZE = 5000

# Map sot.json kind to Neo4j label
KIND_TO_LABEL = {
    "Class": "Class",
    "Interface": "Interface",
    "Trait": "Trait",
    "Enum": "Enum",
    "Method": "Method",
    "Function": "Function",
    "Property": "Property",
    "Const": "Const",
    "EnumCase": "EnumCase",
    "Argument": "Argument",
    "Value": "Value",
    "Call": "Call",
    "File": "File",
}

# Map sot.json edge type to Neo4j relationship type
EDGE_TYPE_TO_REL = {
    "contains": "CONTAINS",
    "uses": "USES",
    "extends": "EXTENDS",
    "implements": "IMPLEMENTS",
    "overrides": "OVERRIDES",
    "type_hint": "TYPE_HINT",
    "calls": "CALLS",
    "receiver": "RECEIVER",
    "argument": "ARGUMENT",
    "produces": "PRODUCES",
    "assigned_from": "ASSIGNED_FROM",
    "type_of": "TYPE_OF",
    "return_type": "RETURN_TYPE",
}


# --------------------------------------------------------------------------
# Exceptions
# --------------------------------------------------------------------------


class ImportValidationError(Exception):
    """Raised when import validation fails."""


# --------------------------------------------------------------------------
# Parser functions
# --------------------------------------------------------------------------


def parse_sot(sot_path: str | Path) -> tuple[list[dict], list[dict]]:
    """Parse sot.json and return (node_props_list, edge_props_list).

    Each dict contains properties ready for Neo4j UNWIND.

    Args:
        sot_path: Path to sot.json file.

    Returns:
        Tuple of (node_props_list, edge_props_list).

    Raises:
        FileNotFoundError: If the file doesn't exist.
        msgspec.DecodeError: If the file is not valid JSON.
    """
    path = Path(sot_path)
    if not path.exists():
        raise FileNotFoundError(f"sot.json not found: {path}")

    with open(path, "rb") as f:
        data = _decoder.decode(f.read())

    nodes = [node_to_props(n) for n in data.nodes]
    edges = [edge_to_props(e) for e in data.edges]

    return nodes, edges


def node_to_props(node: NodeSpec) -> dict:
    """Convert a NodeSpec to Neo4j-ready property dict.

    Key transformations:
    - id -> node_id (avoid Neo4j internal id conflict)
    - range dict -> flat start_line, start_col, end_line, end_col
    - documentation list -> stored as-is (Neo4j supports list properties)
    """
    props: dict = {
        "node_id": node.id,
        "kind": node.kind,
        "name": node.name,
        "fqn": node.fqn,
        "symbol": node.symbol,
    }

    # Optional fields
    if node.file is not None:
        props["file"] = node.file
    if node.range:
        props["start_line"] = node.range.get("start_line")
        props["start_col"] = node.range.get("start_col")
        props["end_line"] = node.range.get("end_line")
        props["end_col"] = node.range.get("end_col")
    if node.documentation:
        props["documentation"] = node.documentation
    if node.value_kind is not None:
        props["value_kind"] = node.value_kind
    if node.type_symbol is not None:
        props["type_symbol"] = node.type_symbol
    if node.call_kind is not None:
        props["call_kind"] = node.call_kind

    return props


def edge_to_props(edge: EdgeSpec) -> dict:
    """Convert an EdgeSpec to Neo4j-ready property dict.

    Key transformations:
    - location dict -> flat loc_file, loc_line
    - type preserved as separate key (used to create typed relationships)
    """
    props: dict = {
        "type": edge.type,
        "source_id": edge.source,
        "target_id": edge.target,
    }

    if edge.location:
        props["loc_file"] = edge.location.get("file")
        props["loc_line"] = edge.location.get("line")
    else:
        props["loc_file"] = None
        props["loc_line"] = None

    if edge.position is not None:
        props["position"] = edge.position
    else:
        props["position"] = None

    if edge.expression is not None:
        props["expression"] = edge.expression
    else:
        props["expression"] = None

    if edge.parameter is not None:
        props["parameter"] = edge.parameter
    else:
        props["parameter"] = None

    return props


# --------------------------------------------------------------------------
# Batch import functions
# --------------------------------------------------------------------------


def import_nodes(
    connection: Neo4jConnection,
    nodes: list[dict],
    batch_size: int = BATCH_SIZE,
    progress_callback=None,
) -> int:
    """Import nodes into Neo4j in batches using UNWIND.

    Groups nodes by kind for efficient label assignment.

    Args:
        connection: Neo4j connection.
        nodes: List of node property dicts from node_to_props().
        batch_size: Number of nodes per UNWIND batch.
        progress_callback: Optional callable(count) for progress reporting.

    Returns:
        Total number of nodes imported.
    """
    # Group by kind for label-specific creation
    by_kind: dict[str, list[dict]] = {}
    for node in nodes:
        kind = node["kind"]
        by_kind.setdefault(kind, []).append(node)

    total_imported = 0

    for kind, kind_nodes in by_kind.items():
        label = KIND_TO_LABEL.get(kind, kind)

        # Cypher query with UNWIND -- creates :Node + :KindLabel
        query = f"""
        UNWIND $batch AS props
        CREATE (n:Node:{label})
        SET n = props
        """

        # Process in batches
        for i in range(0, len(kind_nodes), batch_size):
            batch = kind_nodes[i : i + batch_size]
            with connection.session() as session:
                session.run(query, batch=batch)
            total_imported += len(batch)
            if progress_callback:
                progress_callback(len(batch))

    return total_imported


def import_edges(
    connection: Neo4jConnection,
    edges: list[dict],
    batch_size: int = BATCH_SIZE,
    progress_callback=None,
) -> int:
    """Import edges into Neo4j as typed relationships.

    Groups edges by type for efficient batch creation with typed relationship patterns.

    Args:
        connection: Neo4j connection.
        edges: List of edge property dicts from edge_to_props().
        batch_size: Number of edges per UNWIND batch.
        progress_callback: Optional callable(count) for progress reporting.

    Returns:
        Total number of edges imported.
    """
    # Group by edge type
    by_type: dict[str, list[dict]] = {}
    for edge in edges:
        edge_type = edge["type"]
        by_type.setdefault(edge_type, []).append(edge)

    total_imported = 0

    for edge_type, type_edges in by_type.items():
        rel_type = EDGE_TYPE_TO_REL.get(edge_type, edge_type.upper())

        # Cypher query with UNWIND -- relationship type must be literal
        query = f"""
        UNWIND $batch AS props
        MATCH (source:Node {{node_id: props.source_id}})
        MATCH (target:Node {{node_id: props.target_id}})
        CREATE (source)-[r:{rel_type}]->(target)
        SET r.loc_file = props.loc_file,
            r.loc_line = props.loc_line,
            r.position = props.position,
            r.expression = props.expression,
            r.parameter = props.parameter
        """

        for i in range(0, len(type_edges), batch_size):
            batch = type_edges[i : i + batch_size]
            with connection.session() as session:
                session.run(query, batch=batch)
            total_imported += len(batch)
            if progress_callback:
                progress_callback(len(batch))

    return total_imported


# --------------------------------------------------------------------------
# Validation
# --------------------------------------------------------------------------


def validate_import(
    connection: Neo4jConnection,
    expected_nodes: int,
    expected_edges: int,
) -> dict:
    """Validate import completeness. Returns validation report.

    Args:
        connection: Neo4j connection.
        expected_nodes: Expected number of nodes.
        expected_edges: Expected number of edges.

    Returns:
        Validation report dict.

    Raises:
        ImportValidationError: If counts do not match.
    """
    with connection.session() as session:
        # Total node count
        node_count = session.run("MATCH (n:Node) RETURN count(n) AS cnt").single()["cnt"]

        # Nodes by kind
        kind_counts = {
            row["kind"]: row["cnt"]
            for row in session.run(
                "MATCH (n:Node) RETURN n.kind AS kind, count(n) AS cnt ORDER BY cnt DESC"
            )
        }

        # Total edge count
        edge_count = session.run("MATCH ()-[r]->() RETURN count(r) AS cnt").single()["cnt"]

        # Edges by type
        type_counts = {
            row["type"]: row["cnt"]
            for row in session.run(
                "MATCH ()-[r]->() RETURN type(r) AS type, count(r) AS cnt ORDER BY cnt DESC"
            )
        }

    report = {
        "node_count": node_count,
        "expected_nodes": expected_nodes,
        "node_match": node_count == expected_nodes,
        "edge_count": edge_count,
        "expected_edges": expected_edges,
        "edge_match": edge_count == expected_edges,
        "kind_counts": kind_counts,
        "type_counts": type_counts,
        "valid": node_count == expected_nodes and edge_count == expected_edges,
    }

    if not report["valid"]:
        raise ImportValidationError(
            f"Import validation failed: "
            f"nodes {node_count}/{expected_nodes}, "
            f"edges {edge_count}/{expected_edges}"
        )

    return report


def spot_check_properties(
    connection: Neo4jConnection, sample_nodes: list[dict]
) -> list[str]:
    """Check that specific node properties match expected values.

    Args:
        sample_nodes: List of dicts with {node_id, fqn, kind} to verify.

    Returns:
        List of error messages (empty if all pass).
    """
    errors = []
    with connection.session() as session:
        for expected in sample_nodes:
            result = session.run(
                "MATCH (n:Node {node_id: $nid}) RETURN n",
                nid=expected["node_id"],
            ).single()

            if result is None:
                errors.append(f"Node not found: {expected['node_id']}")
                continue

            node = dict(result["n"])
            if node.get("fqn") != expected.get("fqn"):
                errors.append(
                    f"FQN mismatch for {expected['node_id']}: "
                    f"expected {expected.get('fqn')}, got {node.get('fqn')}"
                )
            if node.get("kind") != expected.get("kind"):
                errors.append(
                    f"Kind mismatch for {expected['node_id']}: "
                    f"expected {expected.get('kind')}, got {node.get('kind')}"
                )

    return errors
