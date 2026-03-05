"""Tests for Neo4j connection manager."""

from __future__ import annotations

import pytest

from src.config import Neo4jConfig
from src.db.connection import Neo4jConnection, Neo4jConnectionError
from tests.conftest import requires_neo4j


class TestNeo4jConnectionUnit:
    """Unit tests that don't require Neo4j."""

    def test_connection_with_bad_uri_raises_clear_error(self):
        """Connection to unavailable Neo4j raises Neo4jConnectionError."""
        config = Neo4jConfig(
            uri="bolt://localhost:19999",
            username="neo4j",
            password="wrong",
        )
        conn = Neo4jConnection(config)
        with pytest.raises(Neo4jConnectionError, match="not available"):
            conn.verify_connectivity()
        conn.close()

    def test_config_property(self):
        """Connection exposes its config."""
        config = Neo4jConfig(uri="bolt://localhost:7687")
        conn = Neo4jConnection(config)
        assert conn.config == config
        conn.close()


@requires_neo4j
class TestNeo4jConnectionIntegration:
    """Integration tests requiring a running Neo4j instance."""

    def test_verify_connectivity(self, neo4j_connection: Neo4jConnection):
        """verify_connectivity() succeeds when Neo4j is running."""
        neo4j_connection.verify_connectivity()

    def test_session_executes_query(self, neo4j_connection: Neo4jConnection):
        """Session can execute a simple Cypher query."""
        with neo4j_connection.session() as session:
            result = session.run("RETURN 1 AS n")
            record = result.single()
            assert record["n"] == 1

    def test_context_manager(self, neo4j_config: Neo4jConfig):
        """Context manager protocol works correctly."""
        with Neo4jConnection(neo4j_config) as conn:
            conn.verify_connectivity()
            with conn.session() as session:
                result = session.run("RETURN 'hello' AS msg")
                record = result.single()
                assert record["msg"] == "hello"
        # After exiting context manager, driver should be closed

    def test_close_is_safe_to_call_multiple_times(
        self, neo4j_config: Neo4jConfig
    ):
        """close() can be called multiple times without error."""
        conn = Neo4jConnection(neo4j_config)
        conn.close()
        conn.close()  # Should not raise
