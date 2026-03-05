"""Cypher queries for ownership chain (structural containment).

Implements the owners command: traverse CONTAINS edges upward from
a node to its File root, matching kloc-cli's OwnersQuery behavior.
"""

from __future__ import annotations

from ..query_runner import QueryRunner
from ..result_mapper import record_to_node
from ...models.node import NodeData

# ---- Cypher query constants ----

# Get the parent that contains this node
CONTAINS_PARENT = """
MATCH (child:Node {node_id: $id})<-[:CONTAINS]-(parent:Node)
RETURN parent
"""


# ---- Query functions ----


def owners_chain(runner: QueryRunner, node_id: str) -> dict:
    """Get the ownership chain for a node.

    Traverses CONTAINS edges upward from the target node to the
    File root, matching kloc-cli's OwnersQuery.execute() behavior.

    The chain starts with the target node itself and ends with the
    File node (outermost container).

    Args:
        runner: QueryRunner instance.
        node_id: ID of the target node.

    Returns:
        Dict with 'chain' key containing list of NodeData objects.

    Raises:
        ValueError: If node not found.
    """
    # Get the target node
    record = runner.execute_single(
        "MATCH (n:Node {node_id: $id}) RETURN n", id=node_id
    )
    if record is None:
        raise ValueError(f"Node not found: {node_id}")

    target = record_to_node(record)
    chain: list[NodeData] = [target]

    current_id = node_id
    while True:
        parent_record = runner.execute_single(CONTAINS_PARENT, id=current_id)
        if parent_record is None:
            break
        parent_node = record_to_node(parent_record, key="parent")
        chain.append(parent_node)
        current_id = parent_node.node_id

    return {"chain": chain}
