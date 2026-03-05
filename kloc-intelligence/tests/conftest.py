"""Shared pytest fixtures for kloc-intelligence tests.

Provides Neo4j connection fixtures that handle setup/teardown.
Tests requiring Neo4j should use the `neo4j_connection` fixture.
"""

from __future__ import annotations

import pytest

from src.config import Neo4jConfig
from src.db.connection import Neo4jConnection, Neo4jConnectionError


def _neo4j_available() -> bool:
    """Check if Neo4j is available for testing."""
    config = Neo4jConfig.from_env()
    try:
        conn = Neo4jConnection(config)
        conn.verify_connectivity()
        conn.close()
        return True
    except (Neo4jConnectionError, Exception):
        return False


# Check once at import time
NEO4J_AVAILABLE = _neo4j_available()

requires_neo4j = pytest.mark.skipif(
    not NEO4J_AVAILABLE,
    reason="Neo4j is not available (start with: docker compose up -d)",
)


@pytest.fixture
def neo4j_config() -> Neo4jConfig:
    """Provide Neo4j configuration from environment."""
    return Neo4jConfig.from_env()


@pytest.fixture
def neo4j_connection(neo4j_config: Neo4jConfig):
    """Provide a Neo4j connection, cleaned up after the test."""
    conn = Neo4jConnection(neo4j_config)
    yield conn
    conn.close()
