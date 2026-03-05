"""Tests for batch node and edge import, and validation."""

from __future__ import annotations

import pytest

from src.db.connection import Neo4jConnection
from src.db.schema import ensure_schema, drop_all
from src.db.importer import (
    import_nodes,
    import_edges,
    validate_import,
    spot_check_properties,
    ImportValidationError,
)
from tests.conftest import requires_neo4j


@requires_neo4j
class TestNodeImport:
    """Integration tests for node import."""

    def _setup_db(self, conn: Neo4jConnection):
        drop_all(conn)
        ensure_schema(conn)

    def test_import_single_node(self, neo4j_connection: Neo4jConnection):
        """Import a single node."""
        self._setup_db(neo4j_connection)
        nodes = [
            {
                "node_id": "n1",
                "kind": "Class",
                "name": "User",
                "fqn": "App\\Entity\\User",
                "symbol": "scip-php ...",
                "file": "src/Entity/User.php",
            }
        ]
        count = import_nodes(neo4j_connection, nodes)
        assert count == 1

        with neo4j_connection.session() as session:
            result = session.run(
                "MATCH (n:Node:Class {node_id: 'n1'}) RETURN n"
            ).single()
            assert result is not None
            node = dict(result["n"])
            assert node["fqn"] == "App\\Entity\\User"
            assert node["kind"] == "Class"

    def test_import_multiple_kinds(self, neo4j_connection: Neo4jConnection):
        """Import nodes with different kinds."""
        self._setup_db(neo4j_connection)
        nodes = [
            {"node_id": "n1", "kind": "Class", "name": "A", "fqn": "A", "symbol": "s1"},
            {"node_id": "n2", "kind": "Method", "name": "foo", "fqn": "A::foo()", "symbol": "s2"},
            {"node_id": "n3", "kind": "Interface", "name": "I", "fqn": "I", "symbol": "s3"},
            {"node_id": "n4", "kind": "File", "name": "A.php", "fqn": "A.php", "symbol": "s4"},
        ]
        count = import_nodes(neo4j_connection, nodes)
        assert count == 4

        with neo4j_connection.session() as session:
            # Check each has correct labels
            assert session.run("MATCH (n:Class) RETURN count(n) AS c").single()["c"] == 1
            assert session.run("MATCH (n:Method) RETURN count(n) AS c").single()["c"] == 1
            assert session.run("MATCH (n:Interface) RETURN count(n) AS c").single()["c"] == 1
            assert session.run("MATCH (n:File) RETURN count(n) AS c").single()["c"] == 1

    def test_import_node_with_optional_props(self, neo4j_connection: Neo4jConnection):
        """Optional properties are stored on the node."""
        self._setup_db(neo4j_connection)
        nodes = [
            {
                "node_id": "n1",
                "kind": "Value",
                "name": "$user",
                "fqn": "m::$user",
                "symbol": "s1",
                "value_kind": "parameter",
                "type_symbol": "App\\Entity\\User",
                "start_line": 10,
                "start_col": 4,
                "end_line": 10,
                "end_col": 20,
                "documentation": ["some docs"],
            }
        ]
        import_nodes(neo4j_connection, nodes)

        with neo4j_connection.session() as session:
            result = session.run("MATCH (n:Node {node_id: 'n1'}) RETURN n").single()
            node = dict(result["n"])
            assert node["value_kind"] == "parameter"
            assert node["type_symbol"] == "App\\Entity\\User"
            assert node["start_line"] == 10
            assert node["documentation"] == ["some docs"]


@requires_neo4j
class TestEdgeImport:
    """Integration tests for edge import."""

    def _setup_with_nodes(self, conn: Neo4jConnection):
        drop_all(conn)
        ensure_schema(conn)
        nodes = [
            {"node_id": "n1", "kind": "Class", "name": "A", "fqn": "A", "symbol": "s1"},
            {"node_id": "n2", "kind": "Method", "name": "foo", "fqn": "A::foo()", "symbol": "s2"},
            {"node_id": "n3", "kind": "Class", "name": "B", "fqn": "B", "symbol": "s3"},
        ]
        import_nodes(conn, nodes)

    def test_import_single_edge(self, neo4j_connection: Neo4jConnection):
        """Import a single edge."""
        self._setup_with_nodes(neo4j_connection)
        edges = [
            {
                "type": "contains",
                "source_id": "n1",
                "target_id": "n2",
                "loc_file": None,
                "loc_line": None,
                "position": None,
                "expression": None,
                "parameter": None,
            }
        ]
        count = import_edges(neo4j_connection, edges)
        assert count == 1

        with neo4j_connection.session() as session:
            result = session.run(
                "MATCH (a:Node {node_id: 'n1'})-[r:CONTAINS]->(b:Node {node_id: 'n2'}) RETURN r"
            ).single()
            assert result is not None

    def test_import_edge_with_properties(self, neo4j_connection: Neo4jConnection):
        """Edge properties (location, etc.) are stored on the relationship."""
        self._setup_with_nodes(neo4j_connection)
        edges = [
            {
                "type": "uses",
                "source_id": "n2",
                "target_id": "n3",
                "loc_file": "src/A.php",
                "loc_line": 25,
                "position": None,
                "expression": None,
                "parameter": None,
            }
        ]
        import_edges(neo4j_connection, edges)

        with neo4j_connection.session() as session:
            result = session.run(
                "MATCH ()-[r:USES]->() RETURN r"
            ).single()
            rel = dict(result["r"])
            assert rel["loc_file"] == "src/A.php"
            assert rel["loc_line"] == 25

    def test_import_multiple_edge_types(self, neo4j_connection: Neo4jConnection):
        """Multiple edge types are imported correctly."""
        self._setup_with_nodes(neo4j_connection)
        edges = [
            {
                "type": "contains", "source_id": "n1", "target_id": "n2",
                "loc_file": None, "loc_line": None, "position": None,
                "expression": None, "parameter": None,
            },
            {
                "type": "uses", "source_id": "n2", "target_id": "n3",
                "loc_file": "src/A.php", "loc_line": 25, "position": None,
                "expression": None, "parameter": None,
            },
            {
                "type": "extends", "source_id": "n1", "target_id": "n3",
                "loc_file": None, "loc_line": None, "position": None,
                "expression": None, "parameter": None,
            },
        ]
        count = import_edges(neo4j_connection, edges)
        assert count == 3

        with neo4j_connection.session() as session:
            assert session.run("MATCH ()-[r:CONTAINS]->() RETURN count(r) AS c").single()["c"] == 1
            assert session.run("MATCH ()-[r:USES]->() RETURN count(r) AS c").single()["c"] == 1
            assert session.run("MATCH ()-[r:EXTENDS]->() RETURN count(r) AS c").single()["c"] == 1


@requires_neo4j
class TestValidation:
    """Integration tests for import validation."""

    def _import_test_data(self, conn: Neo4jConnection):
        drop_all(conn)
        ensure_schema(conn)
        nodes = [
            {"node_id": "n1", "kind": "Class", "name": "A", "fqn": "A", "symbol": "s1"},
            {"node_id": "n2", "kind": "Method", "name": "foo", "fqn": "A::foo()", "symbol": "s2"},
        ]
        edges = [
            {
                "type": "contains", "source_id": "n1", "target_id": "n2",
                "loc_file": None, "loc_line": None, "position": None,
                "expression": None, "parameter": None,
            },
        ]
        import_nodes(conn, nodes)
        import_edges(conn, edges)
        return len(nodes), len(edges)

    def test_validate_success(self, neo4j_connection: Neo4jConnection):
        """Validation passes when counts match."""
        n_nodes, n_edges = self._import_test_data(neo4j_connection)
        report = validate_import(neo4j_connection, n_nodes, n_edges)
        assert report["valid"] is True
        assert report["node_match"] is True
        assert report["edge_match"] is True

    def test_validate_node_mismatch(self, neo4j_connection: Neo4jConnection):
        """Validation fails when expected node count is wrong."""
        self._import_test_data(neo4j_connection)
        with pytest.raises(ImportValidationError, match="validation failed"):
            validate_import(neo4j_connection, expected_nodes=999, expected_edges=1)

    def test_validate_edge_mismatch(self, neo4j_connection: Neo4jConnection):
        """Validation fails when expected edge count is wrong."""
        self._import_test_data(neo4j_connection)
        with pytest.raises(ImportValidationError, match="validation failed"):
            validate_import(neo4j_connection, expected_nodes=2, expected_edges=999)

    def test_spot_check_success(self, neo4j_connection: Neo4jConnection):
        """Spot check passes for correct properties."""
        self._import_test_data(neo4j_connection)
        errors = spot_check_properties(
            neo4j_connection,
            [{"node_id": "n1", "fqn": "A", "kind": "Class"}],
        )
        assert errors == []

    def test_spot_check_missing_node(self, neo4j_connection: Neo4jConnection):
        """Spot check detects missing nodes."""
        self._import_test_data(neo4j_connection)
        errors = spot_check_properties(
            neo4j_connection,
            [{"node_id": "nonexistent", "fqn": "X", "kind": "Class"}],
        )
        assert len(errors) == 1
        assert "not found" in errors[0]
