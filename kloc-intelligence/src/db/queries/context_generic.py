"""Cypher queries for generic context: USED BY and USES for non-class nodes.

Handles Enum, Trait, and other node kinds that use the generic build_tree
approach in kloc-cli (not the class-level grouped USED BY).

Key difference from class_context: emits one entry per edge (not per source),
matching kloc-cli's generic build_tree behavior.
"""

from __future__ import annotations

from ..query_runner import QueryRunner


# ─────────────────────────────────────────────────────────────────────
# Q1: All Incoming USES Edges (per-edge, not deduplicated by source)
# ─────────────────────────────────────────────────────────────────────

GENERIC_INCOMING_USAGES = """
MATCH (cls_target:Node {node_id: $id})
OPTIONAL MATCH (cls_target)-[:CONTAINS*1..10]->(member:Node)
WITH cls_target, COLLECT(member) + [cls_target] AS all_targets
UNWIND all_targets AS target
WITH target, all_targets
MATCH (target)<-[e:USES]-(source:Node)
WHERE source.kind <> 'File'
  AND NOT source IN all_targets
WITH source, e, target
OPTIONAL MATCH path_method = (source)<-[:CONTAINS*1..10]-(method:Node)
WHERE method.kind IN ['Method', 'Function']
WITH source, e, target, method, length(path_method) AS m_dist
ORDER BY m_dist ASC
WITH source, e, target, COLLECT(method)[0] AS containing_method
RETURN source.node_id AS source_id,
       source.fqn AS source_fqn,
       source.kind AS source_kind,
       source.file AS source_file,
       source.start_line AS source_start_line,
       target.node_id AS target_id,
       target.fqn AS target_fqn,
       target.kind AS target_kind,
       e.loc_file AS edge_file,
       e.loc_line AS edge_line,
       containing_method.node_id AS method_id,
       containing_method.fqn AS method_fqn,
       containing_method.kind AS method_kind,
       containing_method.file AS method_file,
       containing_method.start_line AS method_start_line
ORDER BY COALESCE(e.loc_file, source.file, ''), COALESCE(e.loc_line, source.start_line, 0)
"""


# ─────────────────────────────────────────────────────────────────────
# Q2: Outgoing Dependencies (uses edges from node and its members)
# ─────────────────────────────────────────────────────────────────────

GENERIC_OUTGOING_DEPS = """
MATCH (n:Node {node_id: $id})
OPTIONAL MATCH (n)-[:CONTAINS*1..10]->(member:Node)
WITH n, COLLECT(member) + [n] AS all_nodes
UNWIND all_nodes AS source
WITH source
MATCH (source)-[e:USES]->(target:Node)
RETURN target.node_id AS target_id,
       target.fqn AS target_fqn,
       target.kind AS target_kind,
       target.file AS target_file,
       target.start_line AS target_start_line,
       e.loc_file AS edge_file,
       e.loc_line AS edge_line
ORDER BY COALESCE(e.loc_file, target.file, ''), COALESCE(e.loc_line, target.start_line, 0)
"""


def fetch_generic_incoming_usages(runner: QueryRunner, node_id: str) -> list[dict]:
    """Fetch all incoming USES edges for generic context.

    Returns one record per edge (not deduplicated by source).
    """
    records = runner.execute(GENERIC_INCOMING_USAGES, id=node_id)
    return [dict(r) for r in records]


def fetch_generic_outgoing_deps(runner: QueryRunner, node_id: str) -> list[dict]:
    """Fetch outgoing dependency edges for generic context."""
    records = runner.execute(GENERIC_OUTGOING_DEPS, id=node_id)
    return [dict(r) for r in records]
