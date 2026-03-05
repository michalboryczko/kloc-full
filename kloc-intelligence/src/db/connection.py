"""Neo4j connection manager.

Handles driver initialization, connection pooling, session creation,
and graceful shutdown.
"""

from __future__ import annotations

from neo4j import GraphDatabase, Driver, Session
from neo4j.exceptions import ServiceUnavailable, AuthError

from ..config import Neo4jConfig


class Neo4jConnectionError(Exception):
    """Raised when Neo4j connection fails."""


class Neo4jConnection:
    """Manages Neo4j driver lifecycle and session creation.

    Usage:
        conn = Neo4jConnection(config)
        with conn.session() as session:
            result = session.run("MATCH (n) RETURN count(n)")
        conn.close()

    Or as context manager:
        with Neo4jConnection(config) as conn:
            with conn.session() as session:
                ...
    """

    def __init__(self, config: Neo4jConfig | None = None):
        self._config = config or Neo4jConfig.from_env()
        try:
            self._driver: Driver = GraphDatabase.driver(
                self._config.uri,
                auth=(self._config.username, self._config.password),
                max_connection_pool_size=self._config.max_connection_pool_size,
                connection_acquisition_timeout=self._config.connection_acquisition_timeout,
            )
        except Exception as e:
            raise Neo4jConnectionError(
                f"Failed to create Neo4j driver for {self._config.uri}: {e}"
            ) from e

    def verify_connectivity(self) -> None:
        """Verify the connection to Neo4j is working.

        Raises:
            Neo4jConnectionError: If Neo4j is not reachable or auth fails.
        """
        try:
            self._driver.verify_connectivity()
        except ServiceUnavailable as e:
            raise Neo4jConnectionError(
                f"Neo4j is not available at {self._config.uri}. "
                f"Is Neo4j running? (docker compose up -d)"
            ) from e
        except AuthError as e:
            raise Neo4jConnectionError(
                f"Neo4j authentication failed for user '{self._config.username}'. "
                f"Check NEO4J_USERNAME and NEO4J_PASSWORD environment variables."
            ) from e

    def session(self, **kwargs) -> Session:
        """Create a new session with the configured database."""
        return self._driver.session(
            database=self._config.database,
            **kwargs,
        )

    def close(self) -> None:
        """Close the driver and release all connections."""
        self._driver.close()

    @property
    def config(self) -> Neo4jConfig:
        """Return the connection configuration."""
        return self._config

    def __enter__(self) -> "Neo4jConnection":
        return self

    def __exit__(self, *args) -> None:
        self.close()
