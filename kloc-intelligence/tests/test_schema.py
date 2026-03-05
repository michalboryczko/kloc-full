"""Tests for Neo4j schema management."""

from __future__ import annotations

import pytest

from src.db.connection import Neo4jConnection
from src.db.schema import (
    CONSTRAINTS,
    INDEXES,
    NODE_KINDS,
    EDGE_TYPES,
    ensure_schema,
    verify_schema,
    drop_all,
    get_node_count,
    get_edge_count,
)
from tests.conftest import requires_neo4j


class TestSchemaConstants:
    """Tests for schema constant definitions."""

    def test_node_kinds_count(self):
        """There are exactly 13 NodeKinds."""
        assert len(NODE_KINDS) == 13

    def test_edge_types_count(self):
        """There are exactly 14 EdgeTypes."""
        assert len(EDGE_TYPES) == 14

    def test_node_kinds_contains_expected(self):
        """NodeKinds includes all expected types."""
        expected = {
            "Class", "Interface", "Trait", "Enum",
            "Method", "Function",
            "Property", "Const", "EnumCase",
            "Argument", "Value", "Call",
            "File",
        }
        assert set(NODE_KINDS) == expected

    def test_edge_types_contains_expected(self):
        """EdgeTypes includes all expected types."""
        expected = {
            "contains", "uses", "extends", "implements",
            "overrides", "type_hint",
            "calls", "receiver", "argument",
            "produces", "assigned_from", "type_of",
            "return_type", "uses_trait",
        }
        assert set(EDGE_TYPES) == expected

    def test_constraints_not_empty(self):
        """At least one constraint is defined."""
        assert len(CONSTRAINTS) >= 1

    def test_indexes_not_empty(self):
        """Multiple indexes are defined."""
        assert len(INDEXES) >= 5


@requires_neo4j
class TestSchemaIntegration:
    """Integration tests for schema operations."""

    def test_ensure_schema(self, neo4j_connection: Neo4jConnection):
        """ensure_schema() creates constraints and indexes."""
        result = ensure_schema(neo4j_connection)
        assert result["constraints"] >= 1
        assert result["indexes"] >= 5

    def test_ensure_schema_idempotent(self, neo4j_connection: Neo4jConnection):
        """Running ensure_schema() twice is safe."""
        result1 = ensure_schema(neo4j_connection)
        result2 = ensure_schema(neo4j_connection)
        assert result1 == result2

    def test_verify_schema(self, neo4j_connection: Neo4jConnection):
        """verify_schema() confirms schema exists."""
        ensure_schema(neo4j_connection)
        result = verify_schema(neo4j_connection)
        assert result["constraints"] >= 1
        assert result["indexes"] >= 5

    def test_drop_all(self, neo4j_connection: Neo4jConnection):
        """drop_all() removes all data."""
        ensure_schema(neo4j_connection)
        # Insert a test node
        with neo4j_connection.session() as session:
            session.run(
                "CREATE (n:Node:Class {node_id: 'test-drop', name: 'Test', fqn: 'Test', kind: 'Class'})"
            )
        assert get_node_count(neo4j_connection) >= 1

        drop_all(neo4j_connection)
        assert get_node_count(neo4j_connection) == 0

    def test_node_id_uniqueness_constraint(self, neo4j_connection: Neo4jConnection):
        """Uniqueness constraint prevents duplicate node_id values."""
        drop_all(neo4j_connection)
        ensure_schema(neo4j_connection)

        with neo4j_connection.session() as session:
            session.run(
                "CREATE (n:Node {node_id: 'unique-test', name: 'A', fqn: 'A', kind: 'Class'})"
            )
            with pytest.raises(Exception):
                session.run(
                    "CREATE (n:Node {node_id: 'unique-test', name: 'B', fqn: 'B', kind: 'Class'})"
                )

        # Clean up
        drop_all(neo4j_connection)

    def test_get_counts_on_empty_db(self, neo4j_connection: Neo4jConnection):
        """Node and edge counts are 0 on empty database."""
        drop_all(neo4j_connection)
        assert get_node_count(neo4j_connection) == 0
        assert get_edge_count(neo4j_connection) == 0
