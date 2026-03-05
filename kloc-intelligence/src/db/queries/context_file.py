"""Cypher queries for file context: USED BY and USES.

File USED BY:
- External methods/functions that reference any symbol defined in this file
- Each entry shows the containing method with member_ref pointing to referenced symbol
- Reference type determined via Call nodes for member references, TYPE_HINT for class refs

File USES:
- Symbols referenced from within this file
- Each entry shows the referenced symbol with member_ref
"""

from __future__ import annotations

from ..query_runner import QueryRunner


# ─────────────────────────────────────────────────────────────────────
# Q1: File USED BY - ALL incoming USES edges to file's class and members
# Returns one row per (source, USES-edge, target) ordered by edge_idx
# to match sot.json iteration order.
#
# For class-target edges: includes TYPE_HINT flags for reference_type.
# For member-target edges: includes Call node info for reference_type.
#
# The query collects incoming USES edges to ALL members of the file:
# classes, methods, properties, etc. This matches kloc-cli's
# get_usages_grouped(file_id) behavior.
# ─────────────────────────────────────────────────────────────────────

FILE_USED_BY_ALL = """
MATCH (file:Node {node_id: $file_id, kind: 'File'})
MATCH (file)-[:CONTAINS*1..10]->(member:Node)
WHERE member.kind IN ['Class', 'Interface', 'Trait', 'Enum', 'Method', 'Function', 'Property', 'StaticProperty', 'Constant']
WITH member
MATCH (member)<-[e:USES]-(source:Node)
WHERE source.kind <> 'File'
  AND source.file <> member.file
WITH source, e, member
// For Method/Function source: source IS the containing method
// For File source: no containing method needed (it's an import)
// For other sources: find nearest containing method
OPTIONAL MATCH path_method = (source)<-[:CONTAINS*1..10]-(ancestor:Node)
WHERE ancestor.kind IN ['Method', 'Function']
  AND NOT source.kind IN ['Method', 'Function', 'File', 'Class', 'Interface', 'Trait', 'Enum']
WITH source, e, member, ancestor, length(path_method) AS m_dist
ORDER BY m_dist ASC
WITH source, e, member, COLLECT(ancestor)[0] AS nearest_ancestor
WITH source, e, member,
     CASE WHEN source.kind IN ['Method', 'Function'] THEN source
          WHEN source.kind = 'File' THEN source
          WHEN source.kind IN ['Class', 'Interface', 'Trait', 'Enum'] THEN source
          ELSE nearest_ancestor
     END AS resolved_source
WHERE resolved_source IS NOT NULL
// For class targets: check TYPE_HINT edges for reference_type classification
OPTIONAL MATCH (resolved_source)-[return_th:TYPE_HINT]->(member)
WHERE member.kind IN ['Class', 'Interface', 'Trait', 'Enum']
  AND resolved_source.kind IN ['Method', 'Function']
OPTIONAL MATCH (resolved_source)-[:CONTAINS]->(arg:Node {kind: 'Argument'})-[arg_th:TYPE_HINT]->(member)
WHERE member.kind IN ['Class', 'Interface', 'Trait', 'Enum']
  AND resolved_source.kind IN ['Method', 'Function']
OPTIONAL MATCH (resolved_source)<-[:CONTAINS]-(parent_class:Node)-[:CONTAINS]->(prop:Node {kind: 'Property'})-[prop_th:TYPE_HINT]->(member)
WHERE member.kind IN ['Class', 'Interface', 'Trait', 'Enum']
  AND resolved_source.kind IN ['Method', 'Function']
  AND resolved_source.name = '__construct'
// For method/property targets: find Call node for reference_type
OPTIONAL MATCH (resolved_source)-[:CONTAINS*1..5]->(call:Node {kind: 'Call'})-[:CALLS]->(member)
WHERE member.kind IN ['Method', 'Function', 'Property', 'StaticProperty']
WITH source, e, member, resolved_source,
     return_th IS NOT NULL AS has_return_th,
     arg_th IS NOT NULL AS has_arg_th,
     prop_th IS NOT NULL AS has_prop_th,
     COLLECT(DISTINCT {call_kind: call.call_kind, call_id: call.node_id, call_file: call.file, call_line: call.start_line})[0] AS call_info
RETURN source.node_id AS source_id,
       source.kind AS source_kind,
       resolved_source.node_id AS method_id,
       resolved_source.fqn AS method_fqn,
       resolved_source.kind AS method_kind,
       resolved_source.file AS method_file,
       resolved_source.start_line AS method_start_line,
       resolved_source.documentation AS method_documentation,
       member.node_id AS target_id,
       member.fqn AS target_fqn,
       member.kind AS target_kind,
       member.name AS target_name,
       e.loc_file AS edge_file,
       e.loc_line AS edge_line,
       e.edge_idx AS edge_idx,
       has_return_th,
       has_arg_th,
       has_prop_th,
       call_info.call_kind AS call_kind,
       call_info.call_id AS call_id,
       call_info.call_file AS call_file,
       call_info.call_line AS call_line
ORDER BY e.edge_idx ASC
"""


# ─────────────────────────────────────────────────────────────────────
# Q2: File USES - symbols referenced from within this file
# ─────────────────────────────────────────────────────────────────────

FILE_USES = """
MATCH (file:Node {node_id: $file_id, kind: 'File'})
MATCH (file)-[:CONTAINS*1..10]->(source:Node)
WITH source
MATCH (source)-[e:USES]->(target:Node)
WHERE target.kind <> 'File'
RETURN source.node_id AS source_id,
       source.fqn AS source_fqn,
       source.kind AS source_kind,
       target.node_id AS target_id,
       target.fqn AS target_fqn,
       target.kind AS target_kind,
       target.name AS target_name,
       e.loc_file AS edge_file,
       e.loc_line AS edge_line
ORDER BY COALESCE(e.loc_file, target.file, ''),
         COALESCE(e.loc_line, target.start_line, 0)
"""


# ─────────────────────────────────────────────────────────────────────
# Q3: Call-based USES - calls from within this file to any callee
# ─────────────────────────────────────────────────────────────────────

FILE_CALL_USES = """
MATCH (file:Node {node_id: $file_id, kind: 'File'})
MATCH (file)-[:CONTAINS*1..10]->(method:Node)-[:CONTAINS]->(call:Node {kind: 'Call'})
WHERE method.kind IN ['Method', 'Function']
MATCH (call)-[:CALLS]->(callee:Node)
OPTIONAL MATCH (call)-[:RECEIVER]->(recv:Node {kind: 'Value'})
OPTIONAL MATCH (recv)<-[:PRODUCES]-(recv_call:Node {kind: 'Call'})-[:CALLS]->(recv_prop:Node)
WHERE recv_prop.kind IN ['Property', 'StaticProperty']
RETURN call.node_id AS call_id,
       call.file AS call_file,
       call.start_line AS call_line,
       call.call_kind AS call_kind,
       callee.node_id AS callee_id,
       callee.fqn AS callee_fqn,
       callee.kind AS callee_kind,
       callee.name AS callee_name,
       recv.value_kind AS recv_value_kind,
       recv.name AS recv_name,
       recv_prop.fqn AS on_property,
       method.node_id AS method_id,
       method.fqn AS method_fqn
ORDER BY call.file, call.start_line
"""


def fetch_file_used_by_all(runner: QueryRunner, file_id: str) -> list[dict]:
    """Fetch ALL incoming USES edges to the file's class and members.

    Returns one row per (source, edge, target) ordered by edge_idx,
    with TYPE_HINT flags for class targets and Call info for method targets.
    """
    records = runner.execute(FILE_USED_BY_ALL, file_id=file_id)
    return [dict(r) for r in records]


def fetch_file_uses(runner: QueryRunner, file_id: str) -> list[dict]:
    """Fetch all USES edges from nodes in this file to targets."""
    records = runner.execute(FILE_USES, file_id=file_id)
    return [dict(r) for r in records]


def fetch_file_call_uses(runner: QueryRunner, file_id: str) -> list[dict]:
    """Fetch calls from within this file to any callee."""
    records = runner.execute(FILE_CALL_USES, file_id=file_id)
    return [dict(r) for r in records]
