"""Shared pytest fixtures for kloc-intelligence tests.

Provides Neo4j connection fixtures that handle setup/teardown.
Tests requiring Neo4j should use the `neo4j_connection` fixture.
"""

from __future__ import annotations

from pathlib import Path

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

# Dataset path for snapshot/resolve tests
DATASET_PATH = Path(__file__).parent.parent.parent / "data" / "uestate" / "sot.json"

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


# Cache parsed SoT data so we don't re-parse the JSON file repeatedly
_parsed_data_cache: tuple | None = None

# A known symbol that must exist in the uestate dataset
_CANARY_FQN = "App\\Domain\\Model\\Estate\\Estate"


def _data_is_loaded(conn: Neo4jConnection) -> bool:
    """Check if uestate data is loaded by looking for a known symbol."""
    with conn.session() as session:
        result = session.run(
            "MATCH (n:Node {fqn: $fqn}) RETURN count(n) AS cnt",
            fqn=_CANARY_FQN,
        )
        record = result.single()
        return record is not None and record["cnt"] > 0


def _ensure_data_loaded(conn: Neo4jConnection) -> None:
    """Ensure uestate data is loaded into Neo4j; reload if cleared by other tests."""
    global _parsed_data_cache

    if _data_is_loaded(conn):
        return

    from src.db.importer import parse_sot, import_nodes, import_edges
    from src.db.schema import drop_all, ensure_schema

    # Need to load/reload data
    if _parsed_data_cache is None:
        _parsed_data_cache = parse_sot(str(DATASET_PATH))

    nodes, edges = _parsed_data_cache
    drop_all(conn)
    ensure_schema(conn)
    import_nodes(conn, nodes, batch_size=5000)
    import_edges(conn, edges, batch_size=5000)


@pytest.fixture
def loaded_database():
    """Provide a Neo4j connection with uestate data loaded.

    Checks if data is present; reloads if another test cleared the database.
    Parsed data is cached so reloads only cost the import, not re-parsing.
    """
    if not NEO4J_AVAILABLE:
        pytest.skip("Neo4j is not available")

    if not DATASET_PATH.exists():
        pytest.skip(f"Dataset not found: {DATASET_PATH}")

    config = Neo4jConfig.from_env()
    conn = Neo4jConnection(config)
    _ensure_data_loaded(conn)
    yield conn
    conn.close()
