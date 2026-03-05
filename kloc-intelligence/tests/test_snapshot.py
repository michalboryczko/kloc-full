"""Snapshot tests: compare kloc-intelligence output against kloc-cli golden files.

Usage:
    pytest -m snapshot -v            # Run all snapshot tests
    pytest -m snapshot -k resolve    # Run resolve tests only
    pytest -m "not snapshot"         # Skip snapshot tests
"""

from __future__ import annotations

import json
from pathlib import Path

import pytest
import yaml

from tests.snapshot_compare import compare_snapshot, format_diff_report

CORPUS_PATH = Path(__file__).parent / "snapshots" / "corpus.yaml"
GOLDEN_DIR = Path(__file__).parent / "snapshots" / "golden"

# Commands that have been implemented.
# Updated as T04-T12 are completed.
IMPLEMENTED_COMMANDS: set[str] = {"resolve", "usages", "deps", "owners", "inherit", "overrides", "context"}


def load_corpus():
    """Load corpus queries from YAML."""
    if not CORPUS_PATH.exists():
        return []
    with open(CORPUS_PATH) as f:
        corpus = yaml.safe_load(f)
    return corpus.get("queries", [])


def corpus_ids():
    """Generate test IDs from corpus."""
    return [q["id"] for q in load_corpus()]


def _get_golden(query_id: str) -> dict | None:
    """Load golden file for a query."""
    golden_path = GOLDEN_DIR / f"{query_id}.json"
    if not golden_path.exists():
        return None
    with open(golden_path) as f:
        return json.load(f)


def execute_query(connection, query):
    """Execute a corpus query against kloc-intelligence.

    This function is the bridge between the test framework and
    kloc-intelligence query execution. It maps corpus query definitions
    to actual function calls.

    Returns the JSON-serializable output dict, or a dict with "error" key
    if the symbol is not found (matching kloc-cli behavior).
    """
    from src.db.query_runner import QueryRunner
    from src.db.queries.resolve import resolve_symbol

    command = query["command"]
    args = query.get("args", [])
    options = query.get("options", {})

    runner = QueryRunner(connection)

    if command == "resolve":
        symbol = args[0]
        candidates = resolve_symbol(runner, symbol)

        if not candidates:
            return {"error": "Symbol not found", "query": symbol}

        if len(candidates) == 1:
            node = candidates[0]
            return {
                "id": node.id,
                "kind": node.kind,
                "name": node.name,
                "fqn": node.fqn,
                "file": node.file,
                "line": node.start_line + 1 if node.start_line is not None else None,
            }
        else:
            return [
                {
                    "id": n.id,
                    "kind": n.kind,
                    "fqn": n.fqn,
                    "file": n.file,
                    "line": n.start_line + 1 if n.start_line is not None else None,
                }
                for n in candidates
            ]

    elif command == "usages":
        from src.db.queries.usages import usages_tree
        from src.output.json_formatter import usages_tree_to_dict

        symbol = args[0]
        depth = options.get("depth", 1)
        limit = options.get("limit", 100)

        candidates = resolve_symbol(runner, symbol)
        if not candidates:
            return {"error": "Symbol not found", "query": symbol}

        node = candidates[0]
        result = usages_tree(runner, node.node_id, depth=depth, limit=limit)
        return usages_tree_to_dict(result["target"], result["max_depth"], result["tree"])

    elif command == "deps":
        from src.db.queries.deps import deps_tree
        from src.output.json_formatter import deps_tree_to_dict

        symbol = args[0]
        depth = options.get("depth", 1)
        limit = options.get("limit", 100)

        candidates = resolve_symbol(runner, symbol)
        if not candidates:
            return {"error": "Symbol not found", "query": symbol}

        node = candidates[0]
        result = deps_tree(runner, node.node_id, depth=depth, limit=limit)
        return deps_tree_to_dict(result["target"], result["max_depth"], result["tree"])

    elif command == "owners":
        from src.db.queries.owners import owners_chain
        from src.output.json_formatter import owners_chain_to_dict

        symbol = args[0]
        candidates = resolve_symbol(runner, symbol)
        if not candidates:
            return {"error": "Symbol not found", "query": symbol}

        node = candidates[0]
        result = owners_chain(runner, node.node_id)
        return owners_chain_to_dict(result["chain"])

    elif command == "inherit":
        from src.db.queries.inherit import inherit_tree
        from src.output.json_formatter import inherit_tree_to_dict

        symbol = args[0]
        direction = options.get("direction", "up")
        depth = options.get("depth", 1)
        limit = options.get("limit", 100)

        candidates = resolve_symbol(runner, symbol)
        if not candidates:
            return {"error": "Symbol not found", "query": symbol}

        node = candidates[0]
        result = inherit_tree(runner, node.node_id, direction=direction, depth=depth, limit=limit)
        return inherit_tree_to_dict(
            result["root"], result["direction"], result["max_depth"], result["tree"]
        )

    elif command == "overrides":
        from src.db.queries.overrides import overrides_tree
        from src.output.json_formatter import overrides_tree_to_dict

        symbol = args[0]
        direction = options.get("direction", "up")
        depth = options.get("depth", 1)
        limit = options.get("limit", 100)

        candidates = resolve_symbol(runner, symbol)
        if not candidates:
            return {"error": "Symbol not found", "query": symbol}

        node = candidates[0]
        result = overrides_tree(runner, node.node_id, direction=direction, depth=depth, limit=limit)
        return overrides_tree_to_dict(
            result["root"], result["direction"], result["max_depth"], result["tree"]
        )

    elif command == "context":
        from src.db.queries.definition import definition_for_node
        from src.orchestration.class_context import build_class_used_by, build_class_uses
        from src.orchestration.generic_context import build_generic_used_by, build_generic_uses
        from src.models.results import ContextResult
        from src.models.output import ContextOutput

        symbol = args[0]
        depth = options.get("depth", 1)
        limit = options.get("limit", 100)
        include_impl = options.get("impl", False)

        candidates = resolve_symbol(runner, symbol)
        if not candidates:
            return {"error": "Symbol not found", "query": symbol}

        node = candidates[0]

        # Build definition
        definition = definition_for_node(runner, node.node_id)

        # Build used_by and uses based on kind
        used_by = []
        uses = []

        if node.kind == "Class":
            # Class: grouped, sorted, deduped USED BY (build_class_used_by)
            used_by = build_class_used_by(runner, node.node_id, depth, limit, include_impl)
            uses = build_class_uses(runner, node.node_id, depth, limit, include_impl)
        elif node.kind == "Interface":
            from src.orchestration.interface_context import build_interface_used_by, build_interface_uses
            used_by = build_interface_used_by(runner, node.node_id, depth, limit, include_impl)
            uses = build_interface_uses(runner, node.node_id, depth, limit, include_impl)
        elif node.kind == "Method":
            from src.orchestration.method_context import build_method_used_by, build_method_uses
            used_by = build_method_used_by(runner, node.node_id, depth, limit, include_impl)
            uses = build_method_uses(runner, node.node_id, depth, limit, include_impl)
        elif node.kind in ("Enum", "Trait"):
            # Enum/Trait: generic build_tree (one entry per edge, no refType grouping)
            used_by = build_generic_used_by(runner, node.node_id, depth, limit, include_impl)
            uses = build_generic_uses(runner, node.node_id, depth, limit, include_impl)
        elif node.kind in ("Property", "Value", "Constant"):
            # Property/Value/Constant: generic build_tree
            used_by = build_generic_used_by(runner, node.node_id, depth, limit, include_impl)
            uses = build_generic_uses(runner, node.node_id, depth, limit, include_impl)
        # TODO: File context in T11

        result = ContextResult(
            target=node,
            max_depth=depth,
            used_by=used_by,
            uses=uses,
            definition=definition,
        )

        output = ContextOutput.from_result(result)
        return output.to_dict()

    # Other commands: not yet implemented
    raise NotImplementedError(f"Query not implemented: {command}")


@pytest.mark.snapshot
@pytest.mark.parametrize("query", load_corpus(), ids=corpus_ids())
def test_snapshot(query, loaded_database):
    """Run a corpus query and compare against golden output.

    Initially most tests xfail because no queries are implemented yet.
    As T04-T12 are completed, commands are added to IMPLEMENTED_COMMANDS.
    """
    query_id = query["id"]
    command = query["command"]

    # Check if golden file exists
    golden = _get_golden(query_id)
    if golden is None:
        pytest.skip(f"Golden file not found for {query_id}")

    # Mark as expected failure if command not yet implemented
    if command not in IMPLEMENTED_COMMANDS:
        pytest.xfail(f"Command '{command}' not yet implemented")

    # Execute query and compare
    actual_output = execute_query(loaded_database, query)
    result = compare_snapshot(query_id, golden, actual_output)
    if not result.passed:
        report = format_diff_report(result)
        pytest.fail(f"Snapshot mismatch:\n{report}")
