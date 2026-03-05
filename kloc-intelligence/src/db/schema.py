"""Neo4j schema management: constraints, indexes, and data model.

Defines the graph schema for kloc-intelligence and provides functions
to create, verify, and drop schema elements.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from .connection import Neo4jConnection


# 13 NodeKinds from sot.json (matching kloc-mapper/src/models.py)
NODE_KINDS = [
    "Class",
    "Interface",
    "Trait",
    "Enum",
    "Method",
    "Function",
    "Property",
    "Const",
    "EnumCase",
    "Argument",
    "Value",
    "Call",
    "File",
]

# 14 EdgeTypes from sot.json (matching kloc-mapper/src/models.py + uses_trait)
EDGE_TYPES = [
    "contains",
    "uses",
    "extends",
    "implements",
    "overrides",
    "type_hint",
    "calls",
    "receiver",
    "argument",
    "produces",
    "assigned_from",
    "type_of",
    "return_type",
    "uses_trait",
]

# Unique constraint: node_id must be unique across all Node-labeled nodes
CONSTRAINTS = [
    "CREATE CONSTRAINT node_id_unique IF NOT EXISTS FOR (n:Node) REQUIRE n.node_id IS UNIQUE",
]

# Indexes for fast lookups (matching kloc-cli's lookup patterns)
INDEXES = [
    # Primary lookup indexes
    "CREATE INDEX node_fqn IF NOT EXISTS FOR (n:Node) ON (n.fqn)",
    "CREATE INDEX node_name IF NOT EXISTS FOR (n:Node) ON (n.name)",
    "CREATE INDEX node_kind IF NOT EXISTS FOR (n:Node) ON (n.kind)",
    "CREATE INDEX node_symbol IF NOT EXISTS FOR (n:Node) ON (n.symbol)",
    "CREATE INDEX node_file IF NOT EXISTS FOR (n:Node) ON (n.file)",
    # Kind-specific label indexes (for filtered queries)
    "CREATE INDEX class_fqn IF NOT EXISTS FOR (n:Class) ON (n.fqn)",
    "CREATE INDEX method_fqn IF NOT EXISTS FOR (n:Method) ON (n.fqn)",
    "CREATE INDEX interface_fqn IF NOT EXISTS FOR (n:Interface) ON (n.fqn)",
    "CREATE INDEX value_kind IF NOT EXISTS FOR (n:Value) ON (n.value_kind)",
    "CREATE INDEX call_kind IF NOT EXISTS FOR (n:Call) ON (n.call_kind)",
]


def ensure_schema(connection: Neo4jConnection) -> dict:
    """Create all constraints and indexes. Idempotent (IF NOT EXISTS).

    Returns:
        Dict with counts: {"constraints": N, "indexes": N}
    """
    with connection.session() as session:
        for constraint in CONSTRAINTS:
            session.run(constraint)
        for index in INDEXES:
            session.run(index)

    return verify_schema(connection)


def verify_schema(connection: Neo4jConnection) -> dict:
    """Verify all expected constraints and indexes exist.

    Returns:
        Dict with constraint/index counts and any missing items.
    """
    with connection.session() as session:
        constraints = session.run("SHOW CONSTRAINTS").data()
        indexes = session.run("SHOW INDEXES").data()

    return {
        "constraints": len(constraints),
        "indexes": len(indexes),
    }


def drop_all(connection: Neo4jConnection) -> None:
    """Drop all nodes, relationships, constraints, and indexes.

    WARNING: Destroys all data. Used by reset scripts and test teardown.
    Uses batched deletion to handle large datasets (700K+ nodes).
    """
    with connection.session() as session:
        # Drop all constraints first
        constraints = session.run("SHOW CONSTRAINTS").data()
        for c in constraints:
            session.run(f"DROP CONSTRAINT {c['name']} IF EXISTS")

        # Drop all indexes
        indexes = session.run("SHOW INDEXES").data()
        for idx in indexes:
            # Skip lookup indexes (they cannot be dropped)
            if idx.get("type") == "LOOKUP":
                continue
            session.run(f"DROP INDEX {idx['name']} IF EXISTS")

    # Delete all data in batches to handle large datasets
    while True:
        with connection.session() as session:
            result = session.run(
                "MATCH (n) WITH n LIMIT 10000 DETACH DELETE n RETURN count(*) AS deleted"
            )
            record = result.single()
            deleted = record["deleted"] if record else 0
            if deleted == 0:
                break


def get_node_count(connection: Neo4jConnection) -> int:
    """Return the total number of nodes in the database."""
    with connection.session() as session:
        result = session.run("MATCH (n:Node) RETURN count(n) AS count")
        record = result.single()
        return record["count"] if record else 0


def get_edge_count(connection: Neo4jConnection) -> int:
    """Return the total number of relationships in the database."""
    with connection.session() as session:
        result = session.run("MATCH ()-[r]->() RETURN count(r) AS count")
        record = result.single()
        return record["count"] if record else 0
