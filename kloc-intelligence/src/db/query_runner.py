"""Query runner: executes Cypher queries against Neo4j with timing and error handling."""

from __future__ import annotations

import logging
import time
from typing import Any

from neo4j import Record

from .connection import Neo4jConnection

logger = logging.getLogger(__name__)


class QueryRunner:
    """Executes Cypher queries against Neo4j with timing and error handling.

    Usage:
        runner = QueryRunner(connection)
        records = runner.execute("MATCH (n:Node {fqn: $fqn}) RETURN n", fqn="App\\\\Entity\\\\Order")
        node = runner.execute_single("MATCH (n:Node {node_id: $id}) RETURN n", id="abc123")
    """

    def __init__(self, connection: Neo4jConnection):
        self._connection = connection

    def execute(self, query: str, **params) -> list[Record]:
        """Execute a Cypher query and return all result records.

        Args:
            query: Cypher query string with $parameter placeholders.
            **params: Named parameters to bind.

        Returns:
            List of Neo4j Record objects.
        """
        start = time.perf_counter()
        with self._connection.session() as session:
            result = session.run(query, **params)
            records = list(result)
        elapsed = time.perf_counter() - start

        logger.debug(
            "Query executed in %.1fms (%d records): %s",
            elapsed * 1000,
            len(records),
            query[:100].replace("\n", " ").strip(),
        )
        return records

    def execute_single(self, query: str, **params) -> Record | None:
        """Execute a query expecting 0 or 1 results.

        Returns:
            Single Record or None.
        """
        records = self.execute(query, **params)
        if not records:
            return None
        return records[0]

    def execute_value(self, query: str, **params) -> Any:
        """Execute a query expecting a single scalar value.

        Returns:
            The first value from the first record, or None.
        """
        record = self.execute_single(query, **params)
        if record is None:
            return None
        return record[0]

    def execute_count(self, query: str, **params) -> int:
        """Execute a count query."""
        return self.execute_value(query, **params) or 0

    def execute_write(self, query: str, **params) -> dict:
        """Execute a write query and return summary counters."""
        start = time.perf_counter()
        with self._connection.session() as session:
            result = session.run(query, **params)
            summary = result.consume()
        elapsed = time.perf_counter() - start

        counters = summary.counters
        logger.debug(
            "Write executed in %.1fms: nodes_created=%d, rels_created=%d",
            elapsed * 1000,
            counters.nodes_created,
            counters.relationships_created,
        )
        return {
            "nodes_created": counters.nodes_created,
            "nodes_deleted": counters.nodes_deleted,
            "relationships_created": counters.relationships_created,
            "relationships_deleted": counters.relationships_deleted,
        }
