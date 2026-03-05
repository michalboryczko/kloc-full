"""Cypher queries for class context: USES side.

Batch-fetches data for the class USES orchestrator -- outgoing
dependencies grouped by member with dedup and behavioral depth-2.
"""

from __future__ import annotations

from ..query_runner import QueryRunner


# ─────────────────────────────────────────────────────────────────────
# U1: Class-level relationships (extends, implements, uses_trait)
# ─────────────────────────────────────────────────────────────────────

CLASS_RELATIONSHIPS = """
MATCH (cls:Node {node_id: $id})-[r]->(target:Node)
WHERE type(r) IN ['EXTENDS', 'IMPLEMENTS', 'USES_TRAIT']
RETURN target.node_id AS target_id,
       target.fqn AS target_fqn,
       target.kind AS target_kind,
       type(r) AS rel_type,
       cls.file AS file,
       cls.start_line AS line
"""

# ─────────────────────────────────────────────────────────────────────
# U2: Member-level dependencies (USES edges from class members)
# ─────────────────────────────────────────────────────────────────────

MEMBER_DEPS = """
MATCH (cls:Node {node_id: $id})-[:CONTAINS]->(member:Node)
WHERE member.kind IN ['Method', 'Property', 'Const', 'Constant']
MATCH (member)-[e:USES]->(target:Node)
WHERE target.node_id <> $id
  AND target.kind IN ['Class', 'Interface', 'Trait', 'Enum', 'Method', 'Property', 'Function', 'Const', 'Constant']
RETURN member.node_id AS member_id,
       member.fqn AS member_fqn,
       member.kind AS member_kind,
       member.name AS member_name,
       target.node_id AS target_id,
       target.fqn AS target_fqn,
       target.kind AS target_kind,
       target.name AS target_name,
       target.file AS target_file,
       target.start_line AS target_start_line,
       e.loc_file AS file,
       e.loc_line AS line
ORDER BY member.fqn, e.loc_line
"""

# ─────────────────────────────────────────────────────────────────────
# U3: Type hints from class members (property_type, return_type, parameter_type)
# ─────────────────────────────────────────────────────────────────────

MEMBER_TYPE_HINTS = """
MATCH (cls:Node {node_id: $id})-[:CONTAINS]->(member:Node)
WHERE member.kind IN ['Method', 'Property']
OPTIONAL MATCH (member)-[:TYPE_HINT]->(direct_type:Node)
WHERE direct_type.kind IN ['Class', 'Interface', 'Trait', 'Enum']
WITH cls, member, COLLECT({target_id: direct_type.node_id, target_fqn: direct_type.fqn, member_kind: member.kind, member_name: member.name, member_file: member.file, member_line: member.start_line}) AS direct_hints
OPTIONAL MATCH (member)-[:CONTAINS]->(arg:Node {kind: 'Argument'})-[:TYPE_HINT]->(arg_type:Node)
WHERE arg_type.kind IN ['Class', 'Interface', 'Trait', 'Enum']
WITH cls, member, direct_hints, COLLECT({target_id: arg_type.node_id, target_fqn: arg_type.fqn, member_kind: 'Argument', member_name: arg_type.name, member_file: member.file, member_line: member.start_line}) AS arg_hints
UNWIND (direct_hints + arg_hints) AS hint
WITH hint, member
WHERE hint.target_id IS NOT NULL
RETURN DISTINCT hint.target_id AS target_id,
       hint.target_fqn AS target_fqn,
       hint.member_kind AS source_kind,
       hint.member_name AS source_name,
       hint.member_file AS file,
       hint.member_line AS line,
       member.node_id AS member_id,
       member.kind AS member_kind
"""

# ─────────────────────────────────────────────────────────────────────
# U4: Instantiation targets (constructor calls from class methods)
# ─────────────────────────────────────────────────────────────────────

INSTANTIATION_TARGETS = """
MATCH (cls:Node {node_id: $id})-[:CONTAINS]->(method:Node {kind: 'Method'})
MATCH (method)-[:CONTAINS]->(call:Node {kind: 'Call'})-[:CALLS]->(constructor:Node {kind: 'Method'})
WHERE constructor.name = '__construct'
MATCH (constructor)<-[:CONTAINS]-(target_cls:Node)
WHERE target_cls.kind IN ['Class', 'Interface', 'Trait', 'Enum']
  AND target_cls.node_id <> $id
RETURN DISTINCT target_cls.node_id AS target_id,
       target_cls.fqn AS target_fqn,
       target_cls.kind AS target_kind,
       call.file AS file,
       call.start_line AS line
"""

# ─────────────────────────────────────────────────────────────────────
# U5: Behavioral depth-2 (method calls through injected property)
# ─────────────────────────────────────────────────────────────────────

BEHAVIORAL_DEPTH2 = """
MATCH (cls:Node {node_id: $class_id})-[:CONTAINS]->(method:Node {kind: 'Method'})
MATCH (method)-[:CONTAINS]->(call:Node {kind: 'Call'})-[:CALLS]->(callee:Node)
MATCH (call)-[:RECEIVER]->(recv:Node)
WHERE recv.kind = 'Value'
MATCH (recv)<-[:PRODUCES]-(recv_call:Node)-[:CALLS]->(prop:Node)
WHERE prop.fqn = $property_fqn
RETURN DISTINCT callee.node_id AS callee_id,
       callee.fqn AS callee_fqn,
       callee.kind AS callee_kind,
       callee.name AS callee_name,
       method.fqn AS from_method,
       call.file AS call_file,
       call.start_line AS call_line
ORDER BY callee.fqn
"""

# ─────────────────────────────────────────────────────────────────────
# U6: Node deps (for recursive expansion)
# ─────────────────────────────────────────────────────────────────────

NODE_DEPS = """
MATCH (n:Node {node_id: $id})
OPTIONAL MATCH (n)-[ext:EXTENDS]->(ext_target:Node)
WITH n, COLLECT({target_id: ext_target.node_id, target_fqn: ext_target.fqn, target_kind: ext_target.kind, target_file: n.file, target_line: n.start_line, edge_type: 'extends'}) AS ext_deps

OPTIONAL MATCH (n)-[:CONTAINS]->(member:Node)
WHERE member.kind IN ['Method', 'Property', 'Constant']
OPTIONAL MATCH (member)-[:USES]->(dep:Node)
WHERE dep.node_id <> $id
  AND dep.kind IN ['Class', 'Interface', 'Trait', 'Enum', 'Method', 'Property', 'Function', 'Constant']
WITH n, ext_deps, COLLECT({target_id: dep.node_id, target_fqn: dep.fqn, target_kind: dep.kind, target_file: dep.file, target_line: dep.start_line, edge_type: 'uses', source_kind: member.kind}) AS member_deps

OPTIONAL MATCH (n)-[:CONTAINS]->(prop:Node {kind: 'Property'})-[:TYPE_HINT]->(pt:Node)
WHERE pt.kind IN ['Class', 'Interface', 'Trait', 'Enum']
  AND pt.node_id <> $id
WITH n, ext_deps, member_deps, COLLECT({target_id: pt.node_id, target_fqn: pt.fqn, target_kind: pt.kind, target_file: prop.file, target_line: prop.start_line, edge_type: 'type_hint', prop_name: prop.name}) AS prop_type_deps

UNWIND (ext_deps + member_deps + prop_type_deps) AS dep
WITH dep
WHERE dep.target_id IS NOT NULL
RETURN DISTINCT dep.target_id AS target_id,
       dep.target_fqn AS target_fqn,
       dep.target_kind AS target_kind,
       dep.target_file AS target_file,
       dep.target_line AS target_line,
       dep.edge_type AS edge_type,
       dep.source_kind AS source_kind,
       dep.prop_name AS prop_name
"""

# ─────────────────────────────────────────────────────────────────────
# U7: Extends/implements children (for USES depth-2 implements expansion)
# ─────────────────────────────────────────────────────────────────────

EXTENDS_CHILDREN_OF = """
MATCH (parent:Node {node_id: $id})<-[:EXTENDS]-(child:Node)
RETURN child.node_id AS id, child.fqn AS fqn, child.kind AS kind,
       child.file AS file, child.start_line AS start_line
ORDER BY child.file, child.start_line
"""

# ─────────────────────────────────────────────────────────────────────
# U8: Override detection for extends depth-2
# ─────────────────────────────────────────────────────────────────────

OVERRIDES_AND_INHERITED = """
MATCH (parent:Node {node_id: $parent_id})-[:CONTAINS]->(parent_method:Node {kind: 'Method'})
WHERE parent_method.name <> '__construct'
WITH parent_method
OPTIONAL MATCH (child_method:Node)-[:OVERRIDES]->(parent_method)
MATCH (child_method)<-[:CONTAINS]-(child_cls:Node {node_id: $class_id})
RETURN parent_method.node_id AS parent_method_id,
       parent_method.fqn AS parent_method_fqn,
       parent_method.name AS parent_method_name,
       parent_method.file AS parent_method_file,
       parent_method.start_line AS parent_method_start_line,
       parent_method.signature AS parent_method_signature,
       child_method.node_id AS child_method_id,
       child_method.fqn AS child_method_fqn,
       child_method.file AS child_method_file,
       child_method.start_line AS child_method_start_line,
       child_method.signature AS child_method_signature
ORDER BY parent_method.file, parent_method.start_line
"""


# =================================================================
# Data fetching functions
# =================================================================

def fetch_class_uses_data(runner: QueryRunner, node_id: str) -> dict:
    """Fetch all data needed for the class USES orchestrator.

    Args:
        runner: QueryRunner instance.
        node_id: Target class node ID.

    Returns:
        Dict with keys: class_rels, member_deps, type_hints,
        instantiation_targets.
    """
    class_rels = runner.execute(CLASS_RELATIONSHIPS, id=node_id)
    member_deps = runner.execute(MEMBER_DEPS, id=node_id)
    type_hints = runner.execute(MEMBER_TYPE_HINTS, id=node_id)
    instantiation_targets = runner.execute(INSTANTIATION_TARGETS, id=node_id)

    return {
        "class_rels": [dict(r) for r in class_rels],
        "member_deps": [dict(r) for r in member_deps],
        "type_hints": [dict(r) for r in type_hints],
        "instantiation_targets": [dict(r) for r in instantiation_targets],
    }


def fetch_behavioral_depth2(
    runner: QueryRunner, class_id: str, property_fqn: str
) -> list[dict]:
    """Fetch behavioral depth-2 data for property_type deps (U5).

    Args:
        runner: QueryRunner instance.
        class_id: Source class node ID.
        property_fqn: Property FQN for access chain matching.

    Returns:
        List of callee records.
    """
    records = runner.execute(
        BEHAVIORAL_DEPTH2, class_id=class_id, property_fqn=property_fqn
    )
    return [dict(r) for r in records]


def fetch_node_deps(runner: QueryRunner, node_id: str) -> list[dict]:
    """Fetch deps for recursive class expansion (U6).

    Args:
        runner: QueryRunner instance.
        node_id: Node ID.

    Returns:
        List of dependency records.
    """
    records = runner.execute(NODE_DEPS, id=node_id)
    return [dict(r) for r in records]


def fetch_extends_children(runner: QueryRunner, node_id: str) -> list[dict]:
    """Fetch extends children of a class (U7).

    Args:
        runner: QueryRunner instance.
        node_id: Parent class node ID.

    Returns:
        List of child records.
    """
    records = runner.execute(EXTENDS_CHILDREN_OF, id=node_id)
    return [dict(r) for r in records]


def fetch_overrides_and_inherited(
    runner: QueryRunner, class_id: str, parent_id: str
) -> list[dict]:
    """Fetch override and inherited methods for extends depth-2 (U8).

    Args:
        runner: QueryRunner instance.
        class_id: The class doing the extending.
        parent_id: The parent class being extended.

    Returns:
        List of method records with override status.
    """
    records = runner.execute(
        OVERRIDES_AND_INHERITED, class_id=class_id, parent_id=parent_id
    )
    return [dict(r) for r in records]
