"""Cypher queries for usages (incoming USES edges).

Implements flat usages and iterative BFS tree building matching
kloc-cli's UsagesQuery behavior exactly.
"""

from __future__ import annotations

from ..query_runner import QueryRunner
from ..result_mapper import record_to_node
from ...models.node import NodeData

# ---- Cypher query constants ----

# Direct usages of a node (incoming USES edges)
USAGES_DIRECT = """
MATCH (target:Node {node_id: $id})<-[e:USES]-(source:Node)
RETURN source AS n, e.loc_file AS loc_file, e.loc_line AS loc_line
ORDER BY e.edge_idx
"""

# Direct children of a container (ordered by contains edge insertion order)
CONTAINS_CHILDREN = """
MATCH (parent:Node {node_id: $id})-[c:CONTAINS]->(child:Node)
RETURN child.node_id AS child_id
ORDER BY c.edge_idx
"""

# Get node kind for container type check
GET_NODE_KIND = """
MATCH (n:Node {node_id: $id})
RETURN n.kind AS kind
"""

CONTAINER_KINDS = frozenset({"Class", "Interface", "Trait", "Enum", "File"})


# ---- Query functions ----


def _get_node_kind(runner: QueryRunner, node_id: str) -> str | None:
    """Get the kind of a node by ID."""
    record = runner.execute_single(GET_NODE_KIND, id=node_id)
    if record is None:
        return None
    return record["kind"]


def _get_target_node(runner: QueryRunner, node_id: str) -> NodeData | None:
    """Get full node data for a target node."""
    record = runner.execute_single(
        "MATCH (n:Node {node_id: $id}) RETURN n", id=node_id
    )
    if record is None:
        return None
    return record_to_node(record)


def _get_usages_edges(runner: QueryRunner, node_id: str) -> list[dict]:
    """Get usages of a node with member expansion matching kloc-cli ordering.

    For container types, uses DFS traversal of contains tree to collect
    usages in the same order as kloc-cli's get_usages():
    1. Direct usages of the container
    2. For each child (in contains edge order): usages of that child
    3. Recurse into child's children

    Deduplicates by source node_id, keeping the first occurrence.
    """
    kind = _get_node_kind(runner, node_id)
    if kind is None:
        raise ValueError(f"Node not found: {node_id}")

    # Get direct usages (always needed)
    records = runner.execute(USAGES_DIRECT, id=node_id)
    results = []
    seen_sources: set[str] = set()

    for record in records:
        source = record["n"]
        source_id = source["node_id"]
        if source_id not in seen_sources:
            seen_sources.add(source_id)
            results.append(record)

    # For container types, collect member usages via DFS
    if kind in CONTAINER_KINDS:
        def collect_member_usages(parent_id: str) -> None:
            # Get children in contains edge order
            child_records = runner.execute(CONTAINS_CHILDREN, id=parent_id)
            for child_rec in child_records:
                child_id = child_rec["child_id"]
                # Get usages of this child
                child_usages = runner.execute(USAGES_DIRECT, id=child_id)
                for record in child_usages:
                    source = record["n"]
                    source_id = source["node_id"]
                    if source_id not in seen_sources:
                        seen_sources.add(source_id)
                        results.append(record)
                # Recurse into child's children
                collect_member_usages(child_id)

        collect_member_usages(node_id)

    return results


def usages_flat(
    runner: QueryRunner, node_id: str, limit: int = 100
) -> list[dict]:
    """Get flat list of usages for a node.

    For container types (Class/Interface/Trait/Enum/File), includes member
    usages matching kloc-cli's get_usages() behavior.

    Args:
        runner: QueryRunner instance.
        node_id: ID of the target node.
        limit: Maximum results.

    Returns:
        List of dicts with: {node_id, fqn, file, line}
    """
    records = _get_usages_edges(runner, node_id)

    results = []
    for record in records:
        source = record["n"]
        source_id = source["node_id"]
        loc_file = record["loc_file"]
        loc_line = record["loc_line"]

        results.append({
            "node_id": source_id,
            "fqn": source["fqn"],
            "file": loc_file if loc_file is not None else source.get("file"),
            "line": loc_line if loc_line is not None else source.get("start_line"),
        })

        if len(results) >= limit:
            break

    return results


def usages_tree(
    runner: QueryRunner, node_id: str, depth: int = 1, limit: int = 100
) -> dict:
    """Build usages tree matching kloc-cli BFS behavior.

    Uses iterative depth expansion with a global visited set and count,
    matching kloc-cli's UsagesQuery.execute() exactly.

    Args:
        runner: QueryRunner instance.
        node_id: ID of the target node.
        depth: Maximum BFS depth.
        limit: Maximum total results across all depths.

    Returns:
        Dict with target, max_depth, tree structure.
    """
    target = _get_target_node(runner, node_id)
    if target is None:
        raise ValueError(f"Node not found: {node_id}")

    visited = {node_id}
    count = [0]

    def build_tree(current_id: str, current_depth: int) -> list[dict]:
        if current_depth > depth or count[0] >= limit:
            return []

        # Get usages with member expansion (ordering matches kloc-cli)
        records = _get_usages_edges(runner, current_id)

        entries = []
        for record in records:
            source = record["n"]
            source_id = source["node_id"]

            if source_id in visited:
                continue
            visited.add(source_id)

            if count[0] >= limit:
                break
            count[0] += 1

            loc_file = record["loc_file"]
            loc_line = record["loc_line"]

            entry = {
                "depth": current_depth,
                "node_id": source_id,
                "fqn": source["fqn"],
                "file": loc_file if loc_file is not None else source.get("file"),
                "line": loc_line if loc_line is not None else source.get("start_line"),
                "children": [],
            }

            # Recurse for deeper levels
            if current_depth < depth:
                entry["children"] = build_tree(source_id, current_depth + 1)

            entries.append(entry)

        return entries

    tree = build_tree(node_id, 1)

    return {
        "target": target,
        "max_depth": depth,
        "tree": tree,
    }
