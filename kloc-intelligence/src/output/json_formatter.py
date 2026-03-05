"""JSON output formatting matching kloc-cli's serialization.

All line numbers are converted from 0-based (internal) to 1-based (output)
at this boundary.
"""

from __future__ import annotations

import json
from typing import Any

from ..models.node import NodeData


def print_json(data: Any) -> None:
    """Print data as formatted JSON to stdout, matching kloc-cli format."""
    print(json.dumps(data, indent=2, ensure_ascii=False))


def _count_tree_nodes(entries: list[dict]) -> int:
    """Count total nodes in a tree structure recursively."""
    total = 0
    for entry in entries:
        total += 1
        if entry.get("children"):
            total += _count_tree_nodes(entry["children"])
    return total


def _entry_to_dict(entry: dict) -> dict:
    """Convert a tree entry to JSON-serializable dict (usages/deps format)."""
    d = {
        "depth": entry["depth"],
        "fqn": entry["fqn"],
        "file": entry.get("file"),
        "line": entry["line"] + 1 if entry.get("line") is not None else None,
        "children": [_entry_to_dict(c) for c in entry.get("children", [])],
    }
    return d


def usages_tree_to_dict(target: NodeData, max_depth: int, tree: list[dict]) -> dict:
    """Convert usages tree result to JSON matching kloc-cli format.

    Output format:
    {
        "target": {"fqn": ..., "file": ...},
        "max_depth": N,
        "total": N,
        "tree": [{"depth": N, "fqn": ..., "file": ..., "line": N, "children": [...]}]
    }
    """
    serialized_tree = [_entry_to_dict(e) for e in tree]
    return {
        "target": {
            "fqn": target.fqn,
            "file": target.file,
        },
        "max_depth": max_depth,
        "total": _count_tree_nodes(serialized_tree),
        "tree": serialized_tree,
    }


def deps_tree_to_dict(target: NodeData, max_depth: int, tree: list[dict]) -> dict:
    """Convert deps tree result to JSON matching kloc-cli format.

    Same structure as usages_tree_to_dict.
    """
    serialized_tree = [_entry_to_dict(e) for e in tree]
    return {
        "target": {
            "fqn": target.fqn,
            "file": target.file,
        },
        "max_depth": max_depth,
        "total": _count_tree_nodes(serialized_tree),
        "tree": serialized_tree,
    }
