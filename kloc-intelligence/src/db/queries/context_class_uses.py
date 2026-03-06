"""Cypher queries for Class node USES context.

Pre-fetches all data needed to build the USES section of a class context
response (outgoing dependencies from the class).
"""

from ..query_runner import QueryRunner

# =============================================================================
# Q1: Member Dependencies
# =============================================================================
# All outgoing USES edges from the class's members (Methods + Properties)
# to external nodes, grouped with the containing member for display.

Q1_MEMBER_DEPENDENCIES = """
MATCH (cls:Node {node_id: $id})-[:CONTAINS]->(member)
WHERE member.kind IN ['Method', 'Property']
MATCH (member)-[e:USES]->(target:Node)
WHERE target.node_id <> $id
  AND target.kind IN ['Class', 'Interface', 'Trait', 'Enum', 'Method', 'Property', 'Function']
RETURN member.node_id AS member_id,
       member.fqn AS member_fqn,
       member.kind AS member_kind,
       member.name AS member_name,
       target.node_id AS target_id,
       target.fqn AS target_fqn,
       target.kind AS target_kind,
       target.name AS target_name,
       type(e) AS edge_type,
       e.loc_file AS file,
       e.loc_line AS line
ORDER BY member.fqn, e.loc_line
"""

# =============================================================================
# Q2: Class-level Relationships
# =============================================================================
# Direct structural edges: extends, implements, uses_trait.

Q2_CLASS_LEVEL_RELATIONSHIPS = """
MATCH (cls:Node {node_id: $id})-[r]->(target)
WHERE type(r) IN ['EXTENDS', 'IMPLEMENTS', 'USES_TRAIT']
RETURN target.node_id AS target_id,
       target.fqn AS target_fqn,
       target.kind AS target_kind,
       type(r) AS rel_type,
       target.file AS file,
       target.start_line AS line
"""

# =============================================================================
# Q3: Behavioral Depth-2 (Method Execution Flow through injected property)
# =============================================================================
# Find methods called through a specific injected property.

Q3_BEHAVIORAL_DEPTH2 = """
MATCH (cls:Node {node_id: $class_id})-[:CONTAINS]->(method:Method)
MATCH (method)-[:CONTAINS]->(call:Call)-[:CALLS]->(callee)
MATCH (call)-[:RECEIVER]->(recv:Value)<-[:PRODUCES]-(recv_call:Call)-[:CALLS]->(prop)
WHERE prop.fqn = $property_fqn
RETURN DISTINCT callee.node_id AS callee_id,
       callee.fqn AS callee_fqn,
       callee.kind AS callee_kind,
       callee.name AS callee_name,
       method.fqn AS from_method
ORDER BY callee.fqn
"""


def fetch_class_uses_data(runner: QueryRunner, node_id: str) -> dict:
    """Batch-fetch all data needed to build the USES section for a Class.

    Runs Q1 and Q2 and returns a dict keyed by query name. Q3 is intentionally
    omitted here — it is invoked per-property by the orchestration layer when
    building depth-2 behavioral entries.

    Args:
        runner: Active QueryRunner connected to Neo4j.
        node_id: The node_id of the target Class/Interface/Trait/Enum.

    Returns:
        Dict with keys:
            "member_deps":   list of Q1 records (dicts)
            "class_rel":     list of Q2 records (dicts)
    """
    q1_records = runner.execute(Q1_MEMBER_DEPENDENCIES, id=node_id)
    q2_records = runner.execute(Q2_CLASS_LEVEL_RELATIONSHIPS, id=node_id)

    return {
        "member_deps": [dict(r) for r in q1_records],
        "class_rel": [dict(r) for r in q2_records],
    }
