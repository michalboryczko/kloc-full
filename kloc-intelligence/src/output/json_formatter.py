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


def owners_chain_to_dict(chain: list[NodeData]) -> dict:
    """Convert owners chain to JSON matching kloc-cli format.

    Output format:
    {
        "chain": [
            {"kind": "Method", "fqn": ..., "file": ..., "line": N},
            {"kind": "Class", "fqn": ..., "file": ..., "line": N},
            {"kind": "File", "fqn": ..., "file": ..., "line": null}
        ]
    }

    Line numbers converted from 0-based to 1-based.
    """
    return {
        "chain": [
            {
                "kind": node.kind,
                "fqn": node.fqn,
                "file": node.file,
                "line": node.start_line + 1 if node.start_line is not None else None,
            }
            for node in chain
        ],
    }


def _inherit_entry_to_dict(entry: dict) -> dict:
    """Convert an inherit tree entry to JSON-serializable dict.

    Includes 'kind' field (unlike usages/deps entries).
    """
    return {
        "depth": entry["depth"],
        "kind": entry["kind"],
        "fqn": entry["fqn"],
        "file": entry.get("file"),
        "line": entry["line"] + 1 if entry.get("line") is not None else None,
        "children": [_inherit_entry_to_dict(c) for c in entry.get("children", [])],
    }


def inherit_tree_to_dict(
    root: NodeData, direction: str, max_depth: int, tree: list[dict]
) -> dict:
    """Convert inherit tree result to JSON matching kloc-cli format.

    Output format:
    {
        "root": {"fqn": ..., "file": ...},
        "direction": "up"|"down",
        "max_depth": N,
        "total": N,
        "tree": [{"depth": N, "kind": ..., "fqn": ..., "file": ..., "line": N, "children": [...]}]
    }
    """
    serialized_tree = [_inherit_entry_to_dict(e) for e in tree]
    return {
        "root": {
            "fqn": root.fqn,
            "file": root.file,
        },
        "direction": direction,
        "max_depth": max_depth,
        "total": _count_tree_nodes(serialized_tree),
        "tree": serialized_tree,
    }


def _overrides_entry_to_dict(entry: dict) -> dict:
    """Convert an overrides tree entry to JSON-serializable dict.

    Note: overrides entries do NOT include 'kind' (unlike inherit entries).
    """
    return {
        "depth": entry["depth"],
        "fqn": entry["fqn"],
        "file": entry.get("file"),
        "line": entry["line"] + 1 if entry.get("line") is not None else None,
        "children": [_overrides_entry_to_dict(c) for c in entry.get("children", [])],
    }


def overrides_tree_to_dict(
    root: NodeData, direction: str, max_depth: int, tree: list[dict]
) -> dict:
    """Convert overrides tree result to JSON matching kloc-cli format.

    Output format:
    {
        "root": {"fqn": ..., "file": ...},
        "direction": "up"|"down",
        "max_depth": N,
        "total": N,
        "tree": [{"depth": N, "fqn": ..., "file": ..., "line": N, "children": [...]}]
    }

    Note: overrides tree entries do NOT include 'kind' (unlike inherit).
    """
    serialized_tree = [_overrides_entry_to_dict(e) for e in tree]
    return {
        "root": {
            "fqn": root.fqn,
            "file": root.file,
        },
        "direction": direction,
        "max_depth": max_depth,
        "total": _count_tree_nodes(serialized_tree),
        "tree": serialized_tree,
    }
