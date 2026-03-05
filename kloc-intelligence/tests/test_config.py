"""Tests for configuration module."""

from src.config import Neo4jConfig


def test_default_config():
    """Default config has sensible values."""
    config = Neo4jConfig()
    assert config.uri == "bolt://localhost:7687"
    assert config.username == "neo4j"
    assert config.password == "kloc-intelligence"
    assert config.database == "neo4j"
    assert config.max_connection_pool_size == 50
    assert config.connection_acquisition_timeout == 60.0


def test_from_env(monkeypatch):
    """Config reads from environment variables."""
    monkeypatch.setenv("NEO4J_URI", "bolt://custom:7688")
    monkeypatch.setenv("NEO4J_USERNAME", "admin")
    monkeypatch.setenv("NEO4J_PASSWORD", "secret")
    monkeypatch.setenv("NEO4J_DATABASE", "mydb")
    monkeypatch.setenv("NEO4J_MAX_POOL_SIZE", "100")
    monkeypatch.setenv("NEO4J_CONNECTION_TIMEOUT", "30.0")

    config = Neo4jConfig.from_env()
    assert config.uri == "bolt://custom:7688"
    assert config.username == "admin"
    assert config.password == "secret"
    assert config.database == "mydb"
    assert config.max_connection_pool_size == 100
    assert config.connection_acquisition_timeout == 30.0


def test_from_env_defaults(monkeypatch):
    """Config uses defaults when env vars are not set."""
    # Clear any existing env vars
    for key in ["NEO4J_URI", "NEO4J_USERNAME", "NEO4J_PASSWORD", "NEO4J_DATABASE"]:
        monkeypatch.delenv(key, raising=False)

    config = Neo4jConfig.from_env()
    assert config.uri == "bolt://localhost:7687"
    assert config.username == "neo4j"
    assert config.password == "kloc-intelligence"
    assert config.database == "neo4j"
