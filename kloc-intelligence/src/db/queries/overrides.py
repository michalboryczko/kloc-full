"""Cypher queries for method override chain (OVERRIDES edges).

Implements BFS traversal for overridden methods (up) and overriding
methods (down), matching kloc-cli's OverridesQuery behavior exactly.
"""

from __future__ import annotations

from collections import deque

from ..query_runner import QueryRunner
from ..result_mapper import record_to_node
from ...models.node import NodeData

# ---- Cypher query constants ----

# Get the method this one overrides (outgoing OVERRIDES edge -- single parent)
OVERRIDES_PARENT = """
MATCH (child:Node {node_id: $id})-[:OVERRIDES]->(parent:Node)
RETURN parent
"""

# Get methods that override this one (incoming OVERRIDES edges)
OVERRIDDEN_BY = """
MATCH (child:Node)-[e:OVERRIDES]->(parent:Node {node_id: $id})
RETURN child
ORDER BY e.edge_idx
"""


# ---- Query functions ----


def _get_node(runner: QueryRunner, node_id: str) -> NodeData:
    """Get full node data and validate it exists."""
    record = runner.execute_single(
        "MATCH (n:Node {node_id: $id}) RETURN n", id=node_id
    )
    if record is None:
        raise ValueError(f"Node not found: {node_id}")
    return record_to_node(record)


def overrides_tree(
    runner: QueryRunner,
    node_id: str,
    direction: str = "up",
    depth: int = 1,
    limit: int = 100,
) -> dict:
    """Build overrides tree matching kloc-cli BFS behavior.

    Uses BFS with deque and global visited set, matching
    kloc-cli's OverridesQuery.execute() exactly.

    Args:
        runner: QueryRunner instance.
        node_id: ID of the target method node.
        direction: "up" for overridden methods, "down" for overriding methods.
        depth: Maximum BFS depth (default: 1).
        limit: Maximum total results (default: 100).

    Returns:
        Dict with root, direction, max_depth, tree structure.

    Raises:
        ValueError: If node not found or kind is not Method.
    """
    root = _get_node(runner, node_id)

    if root.kind != "Method":
        raise ValueError(f"Node must be Method, got: {root.kind}")

    if direction == "up":
        tree = _bfs_overrides_up(runner, root, depth, limit)
    else:
        tree = _bfs_overrides_down(runner, root, depth, limit)

    return {
        "root": root,
        "direction": direction,
        "max_depth": depth,
        "tree": tree,
    }


def _bfs_overrides_up(
    runner: QueryRunner, start_node: NodeData, max_depth: int, limit: int
) -> list[dict]:
    """BFS traversal upward to overridden methods.

    For "up" direction, typically a single chain (one parent method).
    """
    tree: list[dict] = []
    visited = {start_node.node_id}
    count = 0

    # Queue: (node_id, current_depth, parent_entry or None)
    queue: deque[tuple[str, int, dict | None]] = deque()

    # Seed: get direct parent (method this overrides)
    parent_records = runner.execute(OVERRIDES_PARENT, id=start_node.node_id)
    for rec in parent_records:
        parent = record_to_node(rec, key="parent")
        if parent.node_id not in visited:
            queue.append((parent.node_id, 1, None))

    while queue and count < limit:
        current_id, current_depth, parent_entry = queue.popleft()

        if current_id in visited:
            continue
        visited.add(current_id)

        node = _get_node(runner, current_id)
        count += 1

        entry = {
            "depth": current_depth,
            "node_id": node.node_id,
            "fqn": node.fqn,
            "file": node.file,
            "line": node.start_line,
            "children": [],
        }

        if parent_entry is None:
            tree.append(entry)
        else:
            parent_entry["children"].append(entry)

        # Continue BFS if within depth limit
        if current_depth < max_depth:
            gp_records = runner.execute(OVERRIDES_PARENT, id=current_id)
            for gp_rec in gp_records:
                gp = record_to_node(gp_rec, key="parent")
                if gp.node_id not in visited:
                    queue.append((gp.node_id, current_depth + 1, entry))

    return tree


def _bfs_overrides_down(
    runner: QueryRunner, start_node: NodeData, max_depth: int, limit: int
) -> list[dict]:
    """BFS traversal downward to overriding methods.

    For "down" direction, a method can be overridden by multiple classes.
    """
    tree: list[dict] = []
    visited = {start_node.node_id}
    count = 0

    # Queue: (node_id, current_depth, parent_entry or None)
    queue: deque[tuple[str, int, dict | None]] = deque()

    # Seed: get methods that override this one
    child_records = runner.execute(OVERRIDDEN_BY, id=start_node.node_id)
    for rec in child_records:
        child = record_to_node(rec, key="child")
        if child.node_id not in visited:
            queue.append((child.node_id, 1, None))

    while queue and count < limit:
        current_id, current_depth, parent_entry = queue.popleft()

        if current_id in visited:
            continue
        visited.add(current_id)

        node = _get_node(runner, current_id)
        count += 1

        entry = {
            "depth": current_depth,
            "node_id": node.node_id,
            "fqn": node.fqn,
            "file": node.file,
            "line": node.start_line,
            "children": [],
        }

        if parent_entry is None:
            tree.append(entry)
        else:
            parent_entry["children"].append(entry)

        # Continue BFS if within depth limit
        if current_depth < max_depth:
            gc_records = runner.execute(OVERRIDDEN_BY, id=current_id)
            for gc_rec in gc_records:
                gc = record_to_node(gc_rec, key="child")
                if gc.node_id not in visited:
                    queue.append((gc.node_id, current_depth + 1, entry))

    return tree
