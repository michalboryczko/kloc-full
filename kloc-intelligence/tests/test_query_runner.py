"""Tests for QueryRunner."""

from __future__ import annotations

from tests.conftest import requires_neo4j
from src.db.query_runner import QueryRunner


@requires_neo4j
class TestQueryRunner:
    """Test QueryRunner against a live Neo4j instance."""

    def test_execute_returns_list(self, neo4j_connection):
        """execute() should return a list of Records."""
        runner = QueryRunner(neo4j_connection)
        records = runner.execute("RETURN 1 AS n")
        assert isinstance(records, list)
        assert len(records) == 1
        assert records[0]["n"] == 1

    def test_execute_empty_result(self, neo4j_connection):
        """execute() returns empty list for no matches."""
        runner = QueryRunner(neo4j_connection)
        records = runner.execute(
            "MATCH (n:Node {fqn: 'nonexistent_12345'}) RETURN n"
        )
        assert records == []

    def test_execute_single_returns_record(self, neo4j_connection):
        """execute_single() returns a single Record."""
        runner = QueryRunner(neo4j_connection)
        record = runner.execute_single("RETURN 42 AS value")
        assert record is not None
        assert record["value"] == 42

    def test_execute_single_returns_none(self, neo4j_connection):
        """execute_single() returns None for no matches."""
        runner = QueryRunner(neo4j_connection)
        record = runner.execute_single(
            "MATCH (n:Node {fqn: 'nonexistent_12345'}) RETURN n"
        )
        assert record is None

    def test_execute_value(self, neo4j_connection):
        """execute_value() returns a scalar value."""
        runner = QueryRunner(neo4j_connection)
        value = runner.execute_value("RETURN 'hello' AS greeting")
        assert value == "hello"

    def test_execute_value_none(self, neo4j_connection):
        """execute_value() returns None for no matches."""
        runner = QueryRunner(neo4j_connection)
        value = runner.execute_value(
            "MATCH (n:Node {fqn: 'nonexistent_12345'}) RETURN n.fqn"
        )
        assert value is None

    def test_execute_count(self, neo4j_connection):
        """execute_count() returns an integer."""
        runner = QueryRunner(neo4j_connection)
        count = runner.execute_count("RETURN 5 AS count")
        assert count == 5

    def test_execute_count_zero(self, neo4j_connection):
        """execute_count() returns 0 for no matches."""
        runner = QueryRunner(neo4j_connection)
        count = runner.execute_count(
            "MATCH (n:Node {fqn: 'nonexistent_12345'}) RETURN count(n)"
        )
        assert count == 0

    def test_parameterized_query(self, neo4j_connection):
        """Parameterized queries work correctly."""
        runner = QueryRunner(neo4j_connection)
        records = runner.execute("RETURN $name AS n", name="test")
        assert len(records) == 1
        assert records[0]["n"] == "test"

    def test_execute_write(self, neo4j_connection):
        """execute_write() returns summary counters."""
        runner = QueryRunner(neo4j_connection)
        # Create a temporary node
        result = runner.execute_write(
            "CREATE (n:TestNode {name: 'test_write'}) RETURN n"
        )
        assert result["nodes_created"] == 1
        # Clean up
        runner.execute_write("MATCH (n:TestNode {name: 'test_write'}) DETACH DELETE n")
