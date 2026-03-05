"""Configuration for kloc-intelligence.

Loads settings from environment variables with sensible defaults.
"""

import os
from dataclasses import dataclass


@dataclass
class Neo4jConfig:
    """Neo4j connection configuration."""

    uri: str = "bolt://localhost:7687"
    username: str = "neo4j"
    password: str = "kloc-intelligence"
    database: str = "neo4j"
    max_connection_pool_size: int = 50
    connection_acquisition_timeout: float = 60.0

    @classmethod
    def from_env(cls) -> "Neo4jConfig":
        """Create configuration from environment variables.

        Environment variables:
            NEO4J_URI: Bolt URI (default: bolt://localhost:7687)
            NEO4J_USERNAME: Username (default: neo4j)
            NEO4J_PASSWORD: Password (default: kloc-intelligence)
            NEO4J_DATABASE: Database name (default: neo4j)
            NEO4J_MAX_POOL_SIZE: Max connection pool size (default: 50)
            NEO4J_CONNECTION_TIMEOUT: Connection acquisition timeout (default: 60.0)
        """
        return cls(
            uri=os.getenv("NEO4J_URI", "bolt://localhost:7687"),
            username=os.getenv("NEO4J_USERNAME", "neo4j"),
            password=os.getenv("NEO4J_PASSWORD", "kloc-intelligence"),
            database=os.getenv("NEO4J_DATABASE", "neo4j"),
            max_connection_pool_size=int(os.getenv("NEO4J_MAX_POOL_SIZE", "50")),
            connection_acquisition_timeout=float(
                os.getenv("NEO4J_CONNECTION_TIMEOUT", "60.0")
            ),
        )
