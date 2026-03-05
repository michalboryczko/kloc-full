"""Cypher queries for inheritance tree (EXTENDS/IMPLEMENTS edges).

Implements BFS traversal for ancestors (up) and descendants (down),
matching kloc-cli's InheritQuery behavior exactly.
"""

from __future__ import annotations

from collections import deque

from ..query_runner import QueryRunner
from ..result_mapper import record_to_node
from ...models.node import NodeData

# ---- Cypher query constants ----

# Get all parent types: extends target + implements targets (outgoing edges)
EXTENDS_PARENTS = """
MATCH (child:Node {node_id: $id})-[:EXTENDS]->(parent:Node)
RETURN parent
ORDER BY parent.node_id
"""

IMPLEMENTS_PARENTS = """
MATCH (child:Node {node_id: $id})-[:IMPLEMENTS]->(iface:Node)
RETURN iface AS parent
ORDER BY parent.node_id
"""

# Get all children: classes that extend this (incoming EXTENDS)
EXTENDS_CHILDREN = """
MATCH (child:Node)-[e:EXTENDS]->(parent:Node {node_id: $id})
RETURN child
ORDER BY e.edge_idx
"""

# Get all implementors: classes that implement this interface (incoming IMPLEMENTS)
IMPLEMENTORS = """
MATCH (child:Node)-[e:IMPLEMENTS]->(parent:Node {node_id: $id})
RETURN child
ORDER BY e.edge_idx
"""

ALLOWED_KINDS = frozenset({"Class", "Interface", "Trait", "Enum"})


# ---- Query functions ----


def _get_node_with_kind(runner: QueryRunner, node_id: str) -> NodeData:
    """Get full node data and validate it exists."""
    record = runner.execute_single(
        "MATCH (n:Node {node_id: $id}) RETURN n", id=node_id
    )
    if record is None:
        raise ValueError(f"Node not found: {node_id}")
    return record_to_node(record)


def _get_all_parents(runner: QueryRunner, node_id: str) -> list[NodeData]:
    """Get all parent types (extends + implements) for a node.

    Returns extends parents first, then implements parents,
    matching kloc-cli's _get_all_parents() ordering.
    """
    parents: list[NodeData] = []

    # Extends parent(s)
    extends_records = runner.execute(EXTENDS_PARENTS, id=node_id)
    for rec in extends_records:
        parents.append(record_to_node(rec, key="parent"))

    # Implements parents
    impl_records = runner.execute(IMPLEMENTS_PARENTS, id=node_id)
    for rec in impl_records:
        parents.append(record_to_node(rec, key="parent"))

    return parents


def _get_all_children(runner: QueryRunner, node_id: str) -> list[NodeData]:
    """Get all children (classes that extend or implement this).

    Returns extends children first, then implementors,
    matching kloc-cli's _get_all_children() ordering.
    """
    children: list[NodeData] = []

    # Classes that extend this
    extends_records = runner.execute(EXTENDS_CHILDREN, id=node_id)
    for rec in extends_records:
        children.append(record_to_node(rec, key="child"))

    # Classes that implement this (for interfaces)
    impl_records = runner.execute(IMPLEMENTORS, id=node_id)
    for rec in impl_records:
        children.append(record_to_node(rec, key="child"))

    return children


def inherit_tree(
    runner: QueryRunner,
    node_id: str,
    direction: str = "up",
    depth: int = 1,
    limit: int = 100,
) -> dict:
    """Build inheritance tree matching kloc-cli BFS behavior.

    Uses BFS with deque and global visited set, matching
    kloc-cli's InheritQuery.execute() exactly.

    Args:
        runner: QueryRunner instance.
        node_id: ID of the target node.
        direction: "up" for ancestors, "down" for descendants.
        depth: Maximum BFS depth (default: 1).
        limit: Maximum total results (default: 100).

    Returns:
        Dict with root, direction, max_depth, tree structure.

    Raises:
        ValueError: If node not found or kind not allowed.
    """
    root = _get_node_with_kind(runner, node_id)

    if root.kind not in ALLOWED_KINDS:
        raise ValueError(
            f"Node must be Class/Interface/Trait/Enum, got: {root.kind}"
        )

    if direction == "up":
        tree = _bfs_ancestors(runner, root, depth, limit)
    else:
        tree = _bfs_descendants(runner, root, depth, limit)

    return {
        "root": root,
        "direction": direction,
        "max_depth": depth,
        "tree": tree,
    }


def _bfs_ancestors(
    runner: QueryRunner, start_node: NodeData, max_depth: int, limit: int
) -> list[dict]:
    """BFS traversal upward to ancestors.

    For "up" direction, includes both extended classes and implemented interfaces.
    """
    tree: list[dict] = []
    visited = {start_node.node_id}
    count = 0

    # Queue: (node_id, current_depth, parent_entry or None)
    queue: deque[tuple[str, int, dict | None]] = deque()

    # Seed with direct parents (extends/implements)
    parent_nodes = _get_all_parents(runner, start_node.node_id)
    for parent in parent_nodes:
        if parent.node_id not in visited:
            queue.append((parent.node_id, 1, None))

    while queue and count < limit:
        current_id, current_depth, parent_entry = queue.popleft()

        if current_id in visited:
            continue
        visited.add(current_id)

        node = _get_node_with_kind(runner, current_id)
        count += 1

        entry = {
            "depth": current_depth,
            "node_id": node.node_id,
            "kind": node.kind,
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
            grandparent_nodes = _get_all_parents(runner, current_id)
            for gp in grandparent_nodes:
                if gp.node_id not in visited:
                    queue.append((gp.node_id, current_depth + 1, entry))

    return tree


def _bfs_descendants(
    runner: QueryRunner, start_node: NodeData, max_depth: int, limit: int
) -> list[dict]:
    """BFS traversal downward to descendants.

    For "down" direction, includes both classes that extend and classes
    that implement this type.
    """
    tree: list[dict] = []
    visited = {start_node.node_id}
    count = 0

    # Queue: (node_id, current_depth, parent_entry or None)
    queue: deque[tuple[str, int, dict | None]] = deque()

    # Seed with direct children (extends children + implementors)
    child_nodes = _get_all_children(runner, start_node.node_id)
    for child in child_nodes:
        if child.node_id not in visited:
            queue.append((child.node_id, 1, None))

    while queue and count < limit:
        current_id, current_depth, parent_entry = queue.popleft()

        if current_id in visited:
            continue
        visited.add(current_id)

        node = _get_node_with_kind(runner, current_id)
        count += 1

        entry = {
            "depth": current_depth,
            "node_id": node.node_id,
            "kind": node.kind,
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
            grandchild_nodes = _get_all_children(runner, current_id)
            for gc in grandchild_nodes:
                if gc.node_id not in visited:
                    queue.append((gc.node_id, current_depth + 1, entry))

    return tree
