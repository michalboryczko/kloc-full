"""Cypher queries for dependencies (outgoing USES edges).

Mirrors the usages module but follows outgoing edges instead of incoming.
Implements flat deps and iterative BFS tree building matching
kloc-cli's DepsQuery behavior exactly.
"""

from __future__ import annotations

from ..query_runner import QueryRunner
from ..result_mapper import record_to_node
from ...models.node import NodeData

# ---- Cypher query constants ----

# Direct dependencies of a node (outgoing USES edges)
DEPS_DIRECT = """
MATCH (source:Node {node_id: $id})-[e:USES]->(target:Node)
RETURN target AS n, e.loc_file AS loc_file, e.loc_line AS loc_line
ORDER BY e.edge_idx
"""

# Direct children of a container (ordered by contains edge insertion order)
CONTAINS_CHILDREN = """
MATCH (parent:Node {node_id: $id})-[c:CONTAINS]->(child:Node)
RETURN child.node_id AS child_id
ORDER BY c.edge_idx
"""

CONTAINER_KINDS = frozenset({"Class", "Interface", "Trait", "Enum", "File"})


# ---- Query functions ----


def _get_node_kind(runner: QueryRunner, node_id: str) -> str | None:
    """Get the kind of a node by ID."""
    record = runner.execute_single(
        "MATCH (n:Node {node_id: $id}) RETURN n.kind AS kind", id=node_id
    )
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


def _get_deps_edges(runner: QueryRunner, node_id: str) -> list[dict]:
    """Get deps of a node with member expansion matching kloc-cli ordering.

    For container types, uses DFS traversal of contains tree to collect
    deps in the same order as kloc-cli's get_deps():
    1. Direct deps of the container
    2. For each child (in contains edge order): deps of that child
    3. Recurse into child's children

    Deduplicates by target node_id, keeping the first occurrence.
    """
    kind = _get_node_kind(runner, node_id)
    if kind is None:
        raise ValueError(f"Node not found: {node_id}")

    # Get direct deps (always needed)
    records = runner.execute(DEPS_DIRECT, id=node_id)
    results = []
    seen_targets: set[str] = set()

    for record in records:
        target = record["n"]
        target_id = target["node_id"]
        if target_id not in seen_targets:
            seen_targets.add(target_id)
            results.append(record)

    # For container types, collect member deps via DFS
    if kind in CONTAINER_KINDS:
        def collect_member_deps(parent_id: str) -> None:
            # Get children in contains edge order
            child_records = runner.execute(CONTAINS_CHILDREN, id=parent_id)
            for child_rec in child_records:
                child_id = child_rec["child_id"]
                # Get deps of this child
                child_deps = runner.execute(DEPS_DIRECT, id=child_id)
                for record in child_deps:
                    target = record["n"]
                    target_id = target["node_id"]
                    if target_id not in seen_targets:
                        seen_targets.add(target_id)
                        results.append(record)
                # Recurse into child's children
                collect_member_deps(child_id)

        collect_member_deps(node_id)

    return results


def deps_flat(
    runner: QueryRunner, node_id: str, limit: int = 100
) -> list[dict]:
    """Get flat list of dependencies for a node.

    For container types (Class/Interface/Trait/Enum/File), includes member
    dependencies matching kloc-cli's get_deps() behavior.

    Args:
        runner: QueryRunner instance.
        node_id: ID of the target node.
        limit: Maximum results.

    Returns:
        List of dicts with: {node_id, fqn, file, line}
    """
    records = _get_deps_edges(runner, node_id)

    results = []
    for record in records:
        target = record["n"]
        target_id = target["node_id"]
        loc_file = record["loc_file"]
        loc_line = record["loc_line"]

        # kloc-cli deps: location from edge only, fallback to target file but NOT target line
        results.append({
            "node_id": target_id,
            "fqn": target["fqn"],
            "file": loc_file if loc_file is not None else target.get("file"),
            "line": loc_line,  # No fallback to target start_line for deps
        })

        if len(results) >= limit:
            break

    return results


def deps_tree(
    runner: QueryRunner, node_id: str, depth: int = 1, limit: int = 100
) -> dict:
    """Build deps tree matching kloc-cli BFS behavior.

    Uses iterative depth expansion with a global visited set and count,
    matching kloc-cli's DepsQuery.execute() exactly.

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

        # Get deps with member expansion (ordering matches kloc-cli)
        records = _get_deps_edges(runner, current_id)

        entries = []
        for record in records:
            dep = record["n"]
            dep_id = dep["node_id"]

            if dep_id in visited:
                continue
            visited.add(dep_id)

            if count[0] >= limit:
                break
            count[0] += 1

            loc_file = record["loc_file"]
            loc_line = record["loc_line"]

            entry = {
                "depth": current_depth,
                "node_id": dep_id,
                "fqn": dep["fqn"],
                "file": loc_file if loc_file is not None else dep.get("file"),
                "line": loc_line,  # No fallback to target start_line for deps
                "children": [],
            }

            # Recurse for deeper levels
            if current_depth < depth:
                entry["children"] = build_tree(dep_id, current_depth + 1)

            entries.append(entry)

        return entries

    tree = build_tree(node_id, 1)

    return {
        "target": target,
        "max_depth": depth,
        "tree": tree,
    }
