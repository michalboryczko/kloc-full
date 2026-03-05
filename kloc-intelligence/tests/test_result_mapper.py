"""Tests for result mapper functions."""

from __future__ import annotations

from tests.conftest import requires_neo4j
from src.db.query_runner import QueryRunner
from src.db.result_mapper import record_to_node, records_to_nodes
from src.models.node import NodeData


@requires_neo4j
class TestResultMapper:
    """Test result mapper against live Neo4j data."""

    def test_record_to_node_basic(self, loaded_database):
        """record_to_node maps all basic properties."""
        runner = QueryRunner(loaded_database)
        records = runner.execute(
            "MATCH (n:Node {fqn: $fqn}) RETURN n",
            fqn="App\\Domain\\Model\\Estate\\Estate",
        )
        assert len(records) == 1

        node = record_to_node(records[0])
        assert isinstance(node, NodeData)
        assert node.node_id.startswith("node:")
        assert node.kind == "Class"
        assert node.name == "Estate"
        assert node.fqn == "App\\Domain\\Model\\Estate\\Estate"
        assert node.file is not None
        assert node.start_line is not None

    def test_record_to_node_id_property(self, loaded_database):
        """NodeData.id returns node_id."""
        runner = QueryRunner(loaded_database)
        records = runner.execute(
            "MATCH (n:Node {fqn: $fqn}) RETURN n",
            fqn="App\\Domain\\Model\\Estate\\Estate",
        )
        node = record_to_node(records[0])
        assert node.id == node.node_id

    def test_records_to_nodes(self, loaded_database):
        """records_to_nodes converts multiple records."""
        runner = QueryRunner(loaded_database)
        records = runner.execute(
            "MATCH (n:Node) WHERE n.kind = 'Class' RETURN n LIMIT 5"
        )
        nodes = records_to_nodes(records)
        assert len(nodes) == 5
        for node in nodes:
            assert isinstance(node, NodeData)
            assert node.kind == "Class"

    def test_record_to_node_optional_fields(self, loaded_database):
        """Optional fields default to None when not present."""
        runner = QueryRunner(loaded_database)
        records = runner.execute(
            "MATCH (n:Node {fqn: $fqn}) RETURN n",
            fqn="App\\Domain\\Model\\Estate\\Estate",
        )
        node = record_to_node(records[0])
        # These fields may or may not be set, but should not error
        _ = node.value_kind
        _ = node.type_symbol
        _ = node.call_kind

    def test_record_to_node_with_custom_key(self, loaded_database):
        """record_to_node supports custom RETURN key."""
        runner = QueryRunner(loaded_database)
        records = runner.execute(
            "MATCH (node:Node {fqn: $fqn}) RETURN node",
            fqn="App\\Domain\\Model\\Estate\\Estate",
        )
        node = record_to_node(records[0], key="node")
        assert node.kind == "Class"
