"""Cypher queries for symbol resolution.

Implements the same cascade search as kloc-cli's SoTIndex.resolve_symbol():
1. Exact FQN match
2. Case-insensitive FQN match
3. Suffix match (FQN ends with query)
4. Contains match (trie search in kloc-cli)
5. Linear suffix match (fallback)
6. Short name match
7. Short name without parens
"""

from __future__ import annotations

from neo4j import Record

from ..query_runner import QueryRunner
from ..result_mapper import records_to_nodes
from ...models.node import NodeData

# ---- Cypher queries as constants ----

RESOLVE_EXACT_FQN = """
MATCH (n:Node {fqn: $symbol})
WHERE n.kind IN $searchable_kinds
RETURN n
"""

RESOLVE_CASE_INSENSITIVE = """
MATCH (n:Node)
WHERE toLower(n.fqn) = toLower($symbol) AND n.kind IN $searchable_kinds
RETURN n
"""

RESOLVE_SUFFIX = """
MATCH (n:Node)
WHERE (n.fqn ENDS WITH $symbol OR n.fqn ENDS WITH ('::' + $symbol))
  AND n.kind IN $searchable_kinds
RETURN n
LIMIT 50
"""

RESOLVE_CONTAINS = """
MATCH (n:Node)
WHERE n.fqn CONTAINS $symbol AND n.kind IN $searchable_kinds
RETURN n
LIMIT 50
"""

RESOLVE_NAME = """
MATCH (n:Node {name: $symbol})
WHERE n.kind IN $searchable_kinds
RETURN n
LIMIT 50
"""

# ---- Searchable kinds (excludes internal: Call, Value, Argument) ----

SEARCHABLE_KINDS = [
    "Class",
    "Interface",
    "Trait",
    "Enum",
    "Method",
    "Function",
    "Property",
    "Const",
    "EnumCase",
    "File",
]


# ---- Query execution functions ----


def resolve_symbol(runner: QueryRunner, query: str) -> list[NodeData]:
    """Cascade symbol resolution matching kloc-cli's SoTIndex.resolve_symbol().

    Search order:
    1. Exact FQN match (with Value/Argument dedup)
    2. Case-insensitive FQN match
    3. Suffix match (FQN ends with query or ::query)
    4. Contains match (FQN contains query)
    5. Linear suffix fallback (same as step 3 but after contains)
    6. Short name match
    7. Short name without parens

    Args:
        runner: QueryRunner for executing Cypher queries.
        query: Symbol query string (FQN, partial name, or short name).

    Returns:
        List of matching NodeData objects, empty if no matches.
    """
    # Normalize
    normalized = query.strip()
    if normalized.startswith("\\"):
        normalized = normalized[1:]

    kinds = SEARCHABLE_KINDS

    # Step 1: Exact FQN
    records = runner.execute(RESOLVE_EXACT_FQN, symbol=normalized, searchable_kinds=kinds)
    if records:
        records = _dedup_value_argument(records)
        return records_to_nodes(records)

    # Step 2: Case-insensitive FQN
    records = runner.execute(
        RESOLVE_CASE_INSENSITIVE, symbol=normalized, searchable_kinds=kinds
    )
    if records:
        return records_to_nodes(records)

    # Step 3: Suffix match (equivalent to trie suffix search in kloc-cli)
    records = runner.execute(RESOLVE_SUFFIX, symbol=normalized, searchable_kinds=kinds)
    if records:
        return records_to_nodes(records)

    # Step 4: Contains match (equivalent to trie contains search in kloc-cli)
    records = runner.execute(RESOLVE_CONTAINS, symbol=normalized, searchable_kinds=kinds)
    if records:
        return records_to_nodes(records)

    # Step 5: Short name match
    short_name = normalized.split("::")[-1] if "::" in normalized else normalized
    records = runner.execute(RESOLVE_NAME, symbol=short_name, searchable_kinds=kinds)
    if records:
        return records_to_nodes(records)

    # Step 6: Short name without parens
    short_no_parens = short_name.rstrip("()")
    if short_no_parens != short_name:
        records = runner.execute(RESOLVE_NAME, symbol=short_no_parens, searchable_kinds=kinds)
        if records:
            return records_to_nodes(records)

    return []


def _dedup_value_argument(records: list[Record]) -> list[Record]:
    """When Value and Argument nodes share the same FQN, keep only Value.

    This matches kloc-cli behavior where exact FQN matches may return
    both a Value and an Argument node for the same parameter.
    """
    if len(records) <= 1:
        return records
    has_value = any(r["n"]["kind"] == "Value" for r in records)
    if has_value:
        return [r for r in records if r["n"]["kind"] != "Argument"]
    return records
