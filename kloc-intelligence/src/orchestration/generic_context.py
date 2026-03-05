"""Generic context orchestrator for non-class nodes (Enum, Trait, etc.).

Matches kloc-cli's generic build_tree behavior:
- Emits one entry per edge (not deduplicated by source)
- Uses containing method FQN for entry display
- Sorts by (file, line)
- No refType grouping
"""

from __future__ import annotations

from ..db.query_runner import QueryRunner
from ..db.queries.context_generic import (
    fetch_generic_incoming_usages,
    fetch_generic_outgoing_deps,
)
from ..models.results import ContextEntry


def build_generic_used_by(
    runner: QueryRunner,
    start_id: str,
    max_depth: int,
    limit: int,
    include_impl: bool = False,
) -> list[ContextEntry]:
    """Build the USED BY tree for a generic node (Enum, Trait, etc.).

    Matches kloc-cli's generic build_tree: one entry per incoming edge,
    using the containing method's FQN. No refType grouping.

    Args:
        runner: QueryRunner instance.
        start_id: Node ID to query.
        max_depth: Maximum depth (currently only depth 1 is supported).
        limit: Maximum entries.
        include_impl: Not used for generic context.

    Returns:
        List of ContextEntry objects.
    """
    edges = fetch_generic_incoming_usages(runner, start_id)

    entries: list[ContextEntry] = []
    visited: set[str] = {start_id}
    # Track source_ids for the visited set (one source can have multiple edges)
    source_visited: set[str] = set()

    for edge in edges:
        if len(entries) >= limit:
            break

        source_id = edge["source_id"]
        method_id = edge.get("method_id")
        method_fqn = edge.get("method_fqn")
        method_kind = edge.get("method_kind")

        # Use containing method if available, otherwise source
        if method_id and method_fqn:
            entry_id = method_id
            entry_fqn = method_fqn
            entry_kind = method_kind or edge["source_kind"]
        else:
            entry_id = source_id
            entry_fqn = edge["source_fqn"]
            entry_kind = edge["source_kind"]

        # Add () suffix for Method kind
        if entry_kind == "Method" and not entry_fqn.endswith("()"):
            entry_fqn += "()"

        # Edge location
        edge_file = edge.get("edge_file")
        edge_line = edge.get("edge_line")

        # Fall back to method/source location
        if edge_file is None:
            edge_file = edge.get("method_file") or edge.get("source_file")
        if edge_line is None:
            edge_line = edge.get("method_start_line") or edge.get("source_start_line")

        entry = ContextEntry(
            depth=1,
            node_id=entry_id,
            fqn=entry_fqn,
            kind=entry_kind,
            file=edge_file,
            line=edge_line,
            children=[],
        )
        entries.append(entry)

        # Track source_id as visited (for depth > 1 expansion)
        source_visited.add(source_id)

    # Entries are already sorted by (file, line) from the Cypher ORDER BY
    return entries


def build_generic_uses(
    runner: QueryRunner,
    start_id: str,
    max_depth: int,
    limit: int,
    include_impl: bool = False,
) -> list[ContextEntry]:
    """Build the USES tree for a generic node (Enum, Trait, etc.).

    Fetches outgoing dependencies and returns them as flat entries.

    Args:
        runner: QueryRunner instance.
        start_id: Node ID to query.
        max_depth: Maximum depth (currently only depth 1).
        limit: Maximum entries.
        include_impl: Not used for generic context.

    Returns:
        List of ContextEntry objects.
    """
    deps = fetch_generic_outgoing_deps(runner, start_id)

    entries: list[ContextEntry] = []
    seen: set[str] = {start_id}

    for dep in deps:
        if len(entries) >= limit:
            break

        target_id = dep["target_id"]
        if target_id in seen:
            continue
        seen.add(target_id)

        entry_fqn = dep["target_fqn"]
        entry_kind = dep["target_kind"]

        # Location: use edge location if available, else target location
        file = dep.get("edge_file") or dep.get("target_file")
        line = dep.get("edge_line")
        if line is None:
            line = dep.get("target_start_line")

        entry = ContextEntry(
            depth=1,
            node_id=target_id,
            fqn=entry_fqn,
            kind=entry_kind,
            file=file,
            line=line,
            children=[],
        )
        entries.append(entry)

    return entries
