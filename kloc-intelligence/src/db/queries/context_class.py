"""Cypher queries for class context: USED BY side.

Batch-fetches all data needed by the class USED BY orchestrator
in a small number of targeted queries. Replaces hundreds of
individual SoTIndex lookups with 6 queries (Q1-Q6).
"""

from __future__ import annotations

from ..query_runner import QueryRunner


# ─────────────────────────────────────────────────────────────────────
# Q1: Extends/Implements Children
# ─────────────────────────────────────────────────────────────────────

EXTENDS_CHILDREN = """
MATCH (target:Node {node_id: $id})<-[r:EXTENDS|IMPLEMENTS]-(child:Node)
RETURN child.node_id AS id, child.fqn AS fqn, child.kind AS kind,
       child.file AS file, child.start_line AS start_line,
       type(r) AS rel_type
ORDER BY child.file, child.start_line
"""

# ─────────────────────────────────────────────────────────────────────
# Q2: All Incoming Usages with Context
# ─────────────────────────────────────────────────────────────────────

INCOMING_USAGES = """
MATCH (cls_target:Node {node_id: $id})
OPTIONAL MATCH (cls_target)-[:CONTAINS*1..10]->(member:Node)
WITH cls_target, COLLECT(member) + [cls_target] AS all_targets
UNWIND all_targets AS target
WITH target
MATCH (target)<-[e]-(source:Node)
WHERE type(e) IN ['USES', 'EXTENDS', 'IMPLEMENTS', 'USES_TRAIT']
  AND source.kind <> 'File'
WITH source, e, target
OPTIONAL MATCH path_cls = (source)<-[:CONTAINS*1..10]-(cls:Node)
WHERE cls.kind IN ['Class', 'Interface', 'Trait', 'Enum']
WITH source, e, target, cls, length(path_cls) AS cls_dist
ORDER BY cls_dist ASC
WITH source, e, target, COLLECT(cls)[0] AS containing_class
OPTIONAL MATCH path_method = (source)<-[:CONTAINS*1..10]-(method:Node)
WHERE method.kind IN ['Method', 'Function']
WITH source, e, target, containing_class, method, length(path_method) AS m_dist
ORDER BY m_dist ASC
WITH source, e, target, containing_class, COLLECT(method)[0] AS containing_method
RETURN source.node_id AS source_id,
       source.fqn AS source_fqn,
       source.kind AS source_kind,
       source.name AS source_name,
       source.file AS source_file,
       source.start_line AS source_start_line,
       source.signature AS source_signature,
       target.node_id AS target_id,
       target.fqn AS target_fqn,
       target.kind AS target_kind,
       target.name AS target_name,
       type(e) AS edge_type,
       e.loc_file AS edge_file,
       e.loc_line AS edge_line,
       containing_class.node_id AS containing_class_id,
       containing_class.fqn AS containing_class_fqn,
       containing_class.kind AS containing_class_kind,
       containing_class.file AS containing_class_file,
       containing_class.start_line AS containing_class_start_line,
       containing_method.node_id AS containing_method_id,
       containing_method.fqn AS containing_method_fqn,
       containing_method.kind AS containing_method_kind
"""

# ─────────────────────────────────────────────────────────────────────
# Q3: Injection Detection
# ─────────────────────────────────────────────────────────────────────

INJECTION_DETECTION = """
MATCH (target:Node {node_id: $id})<-[:TYPE_HINT]-(prop:Node)
WHERE prop.kind = 'Property'
MATCH (prop)<-[:CONTAINS*1..10]-(cls:Node)
WHERE cls.kind IN ['Class', 'Interface', 'Trait', 'Enum']
RETURN DISTINCT cls.node_id AS class_with_injection
"""

# ─────────────────────────────────────────────────────────────────────
# Q4: Call Node Resolution
# ─────────────────────────────────────────────────────────────────────

CALL_NODES_CONSTRUCTOR = """
MATCH (cls_target:Node {node_id: $id})
OPTIONAL MATCH (cls_target)-[:CONTAINS*1..10]->(member:Node)
WITH cls_target, COLLECT(member) + [cls_target] AS all_targets
UNWIND all_targets AS target
WITH target, cls_target
MATCH (target)<-[:USES]-(source:Node)
WHERE source.kind <> 'File'
MATCH (source)-[:CONTAINS]->(call:Node {kind: 'Call', call_kind: 'constructor'})
MATCH (call)-[:CALLS]->(constructor:Node {kind: 'Method', name: '__construct'})
MATCH (constructor)<-[:CONTAINS]-(cls:Node {node_id: $id})
OPTIONAL MATCH (call)-[:RECEIVER]->(recv:Node {kind: 'Value'})
OPTIONAL MATCH (recv)<-[:PRODUCES]-(recv_call:Node)-[:CALLS]->(recv_target:Node {kind: 'Property'})
RETURN source.node_id AS source_id,
       call.node_id AS call_id,
       call.call_kind AS call_kind,
       call.file AS call_file,
       call.start_line AS call_start_line,
       constructor.node_id AS callee_id,
       constructor.fqn AS callee_fqn,
       recv.value_kind AS recv_value_kind,
       recv.name AS recv_name,
       recv_target.fqn AS access_chain_symbol
"""

CALL_NODES_MEMBER = """
MATCH (cls_target:Node {node_id: $id})-[:CONTAINS*1..10]->(member:Node)
WITH cls_target, COLLECT(member) + [cls_target] AS all_targets
UNWIND all_targets AS target
WITH target, cls_target
MATCH (target)<-[:USES]-(source:Node)
WHERE source.kind <> 'File'
MATCH (source)-[:CONTAINS]->(call:Node {kind: 'Call'})
MATCH (call)-[:CALLS]->(callee:Node)
MATCH (callee)<-[:CONTAINS*0..10]-(cls:Node {node_id: $id})
WHERE callee.name <> '__construct' OR call.call_kind = 'method_static'
OPTIONAL MATCH (call)-[:RECEIVER]->(recv:Node {kind: 'Value'})
OPTIONAL MATCH (recv)<-[:PRODUCES]-(recv_call:Node)-[:CALLS]->(recv_target:Node {kind: 'Property'})
RETURN source.node_id AS source_id,
       call.node_id AS call_id,
       call.call_kind AS call_kind,
       call.file AS call_file,
       call.start_line AS call_start_line,
       callee.node_id AS callee_id,
       callee.fqn AS callee_fqn,
       recv.value_kind AS recv_value_kind,
       recv.name AS recv_name,
       recv_target.fqn AS access_chain_symbol
"""

# ─────────────────────────────────────────────────────────────────────
# Q5: Caller Chain (for depth-2+ expansion)
# ─────────────────────────────────────────────────────────────────────

CALLER_CHAIN = """
MATCH (method:Node {node_id: $method_id})<-[:CALLS]-(call:Node)<-[:CONTAINS]-(caller:Node)
WHERE caller.kind IN ['Method', 'Function']
OPTIONAL MATCH (call)-[:RECEIVER]->(recv:Node)
WHERE recv.kind = 'Value'
OPTIONAL MATCH (recv)<-[:PRODUCES]-(recv_call:Node)-[:CALLS]->(recv_prop:Node)
WHERE recv_prop.kind = 'Property'
OPTIONAL MATCH (caller)-[e:USES]->(method:Node {node_id: $method_id})
RETURN caller.node_id AS caller_id,
       caller.fqn AS caller_fqn,
       caller.kind AS caller_kind,
       caller.file AS caller_file,
       caller.start_line AS caller_start_line,
       call.node_id AS call_id,
       COALESCE(e.loc_line, call.start_line) AS call_line,
       call.call_kind AS call_kind,
       recv_prop.fqn AS on_property
ORDER BY caller.file, call.start_line
"""

# ─────────────────────────────────────────────────────────────────────
# Q6: Property Type Resolution (for PropertyTypeHandler)
# ─────────────────────────────────────────────────────────────────────

PROPERTY_TYPES = """
MATCH (target:Node {node_id: $id})<-[:TYPE_HINT]-(prop:Node)
WHERE prop.kind = 'Property'
RETURN prop.node_id AS prop_id, prop.fqn AS prop_fqn,
       prop.name AS prop_name,
       prop.file AS prop_file, prop.start_line AS prop_start_line
"""

# ─────────────────────────────────────────────────────────────────────
# Q7: Internal Reference Check (for R3 filtering)
# ─────────────────────────────────────────────────────────────────────

INTERNAL_CHECK = """
MATCH (target:Node {node_id: $target_id})-[:CONTAINS*1..10]->(source:Node {node_id: $source_id})
RETURN count(*) > 0 AS is_internal
"""

# ─────────────────────────────────────────────────────────────────────
# Q8: Injection Point Calls (depth-2 under property_type)
# ─────────────────────────────────────────────────────────────────────

INJECTION_POINT_CALLS = """
MATCH (prop:Node {node_id: $property_id})<-[:CONTAINS*1..10]-(cls:Node)
WHERE cls.kind IN ['Class', 'Interface', 'Trait', 'Enum']
WITH cls, prop LIMIT 1
MATCH (cls)-[:CONTAINS]->(method:Node)-[:CONTAINS]->(call:Node)
WHERE method.kind = 'Method' AND call.kind = 'Call'
MATCH (call)-[:CALLS]->(callee:Node)
OPTIONAL MATCH (call)-[:RECEIVER]->(recv:Node)
WHERE recv.kind = 'Value'
OPTIONAL MATCH (recv)<-[:PRODUCES]-(recv_call:Node)-[:CALLS]->(recv_target:Node)
WHERE recv_target.fqn = $property_fqn
WITH method, call, callee, recv_target
WHERE recv_target IS NOT NULL
RETURN method.node_id AS method_id, method.fqn AS method_fqn,
       method.name AS method_name,
       call.node_id AS call_id, call.call_kind AS call_kind,
       call.start_line AS call_start_line, call.file AS call_file,
       callee.node_id AS callee_id, callee.fqn AS callee_fqn,
       callee.name AS callee_name, callee.kind AS callee_kind
ORDER BY method.fqn, call.start_line
"""

# ─────────────────────────────────────────────────────────────────────
# Q9: Override methods for subclass (USED BY depth-2 under extends)
# ─────────────────────────────────────────────────────────────────────

OVERRIDE_METHODS = """
MATCH (subclass:Node {node_id: $subclass_id})-[:CONTAINS]->(method:Node)
WHERE method.kind = 'Method' AND method.name <> '__construct'
MATCH (method)-[:OVERRIDES]->(parent_method:Node)
RETURN method.node_id AS method_id, method.fqn AS method_fqn,
       method.kind AS method_kind, method.name AS method_name,
       method.file AS method_file, method.start_line AS method_start_line,
       method.signature AS method_signature, method.documentation AS method_documentation
ORDER BY method.file, method.start_line
"""

# ─────────────────────────────────────────────────────────────────────
# Q10: Override method internals (what an override method does)
# ─────────────────────────────────────────────────────────────────────

OVERRIDE_INTERNALS = """
MATCH (method:Node {node_id: $method_id})-[:CONTAINS]->(call:Node)
WHERE call.kind = 'Call'
MATCH (call)-[:CALLS]->(target:Node)
WHERE target.kind NOT IN ['Property', 'StaticProperty']
OPTIONAL MATCH (call)-[:RECEIVER]->(recv:Node)
WHERE recv.kind = 'Value'
OPTIONAL MATCH (recv)<-[:PRODUCES]-(recv_call:Node)-[:CALLS]->(recv_prop:Node)
WHERE recv_prop.kind = 'Property'
RETURN call.node_id AS call_id,
       call.file AS call_file,
       call.start_line AS call_start_line,
       call.call_kind AS call_kind,
       target.node_id AS target_id, target.fqn AS target_fqn,
       target.kind AS target_kind,
       recv_prop.fqn AS on_property
ORDER BY call.file, call.start_line
"""

# ─────────────────────────────────────────────────────────────────────
# Q11: Argument info for a call
# ─────────────────────────────────────────────────────────────────────

CALL_ARGUMENTS = """
MATCH (call:Node {node_id: $call_id})-[a:ARGUMENT]->(arg_val:Node)
OPTIONAL MATCH (param_node:Node {fqn: a.parameter, kind: 'Value', value_kind: 'parameter'})
OPTIONAL MATCH (promoted_prop:Node {kind: 'Property'})-[:ASSIGNED_FROM]->(param_node)
RETURN a.position AS position,
       COALESCE(promoted_prop.fqn, a.parameter) AS param_fqn,
       a.expression AS arg_expression,
       arg_val.name AS value_expr,
       arg_val.value_kind AS value_source,
       arg_val.kind AS value_kind
ORDER BY a.position
"""

# ─────────────────────────────────────────────────────────────────────
# Q12: Type hint boolean checks for infer_reference_type
# ─────────────────────────────────────────────────────────────────────

TYPE_HINT_CHECKS = """
MATCH (source:Node {node_id: $source_id})
OPTIONAL MATCH (source)-[:CONTAINS]->(arg:Node {kind: 'Argument'})-[:TYPE_HINT]->(target:Node {node_id: $target_id})
WITH source, CASE WHEN arg IS NOT NULL THEN true ELSE false END AS has_arg_type_hint

OPTIONAL MATCH (source)-[:TYPE_HINT]->(target2:Node {node_id: $target_id})
WHERE source.kind IN ['Method', 'Function']
WITH source, has_arg_type_hint,
     CASE WHEN target2 IS NOT NULL THEN true ELSE false END AS has_return_type_hint

OPTIONAL MATCH (source)<-[:CONTAINS*0..1]-(parent_cls:Node)
WHERE parent_cls.kind IN ['Class', 'Interface', 'Trait', 'Enum']
OPTIONAL MATCH (parent_cls)-[:CONTAINS]->(prop:Node {kind: 'Property'})-[:TYPE_HINT]->(target3:Node {node_id: $target_id})
WITH has_arg_type_hint, has_return_type_hint,
     CASE WHEN prop IS NOT NULL THEN true ELSE false END AS has_class_property_type_hint

RETURN has_arg_type_hint, has_return_type_hint, has_class_property_type_hint
"""


# =================================================================
# Data fetching functions
# =================================================================

def fetch_class_used_by_data(runner: QueryRunner, node_id: str) -> dict:
    """Fetch all data needed for the class USED BY orchestrator.

    Runs Q1, Q2, Q3, Q4, Q6 sequentially (Phase 1 is synchronous).

    Args:
        runner: QueryRunner instance.
        node_id: Target class node ID.

    Returns:
        Dict with keys: extends_children, usage_edges, injection_classes,
        call_nodes, prop_types.
    """
    extends_children = runner.execute(EXTENDS_CHILDREN, id=node_id)
    usage_edges = runner.execute(INCOMING_USAGES, id=node_id)
    injection_records = runner.execute(INJECTION_DETECTION, id=node_id)

    # Call nodes: merge results from constructor and member queries
    call_constructor = runner.execute(CALL_NODES_CONSTRUCTOR, id=node_id)
    call_member = runner.execute(CALL_NODES_MEMBER, id=node_id)

    prop_types = runner.execute(PROPERTY_TYPES, id=node_id)

    # Merge and dedup call nodes by call_id
    seen_call_ids = set()
    all_call_nodes = []
    for record_set in (call_constructor, call_member):
        for r in record_set:
            d = dict(r)
            cid = d.get("call_id")
            if cid and cid not in seen_call_ids:
                seen_call_ids.add(cid)
                all_call_nodes.append(d)

    return {
        "extends_children": [dict(r) for r in extends_children],
        "usage_edges": [dict(r) for r in usage_edges],
        "injection_classes": {r["class_with_injection"] for r in injection_records},
        "call_nodes": all_call_nodes,
        "prop_types": [dict(r) for r in prop_types],
    }


def fetch_caller_chain(runner: QueryRunner, method_id: str) -> list[dict]:
    """Fetch callers of a method (Q5).

    Args:
        runner: QueryRunner instance.
        method_id: Method node ID.

    Returns:
        List of caller records.
    """
    records = runner.execute(CALLER_CHAIN, method_id=method_id)
    return [dict(r) for r in records]


def fetch_injection_point_calls(
    runner: QueryRunner, property_id: str, property_fqn: str
) -> list[dict]:
    """Fetch method calls through an injected property (Q8).

    Args:
        runner: QueryRunner instance.
        property_id: Property node ID.
        property_fqn: Property FQN for access chain matching.

    Returns:
        List of call records.
    """
    records = runner.execute(
        INJECTION_POINT_CALLS,
        property_id=property_id,
        property_fqn=property_fqn,
    )
    return [dict(r) for r in records]


def fetch_override_methods(runner: QueryRunner, subclass_id: str) -> list[dict]:
    """Fetch override methods in a subclass (Q9).

    Args:
        runner: QueryRunner instance.
        subclass_id: Subclass node ID.

    Returns:
        List of override method records.
    """
    records = runner.execute(OVERRIDE_METHODS, subclass_id=subclass_id)
    return [dict(r) for r in records]


def fetch_override_internals(runner: QueryRunner, method_id: str) -> list[dict]:
    """Fetch internal actions of an override method (Q10).

    Args:
        runner: QueryRunner instance.
        method_id: Override method node ID.

    Returns:
        List of call action records.
    """
    records = runner.execute(OVERRIDE_INTERNALS, method_id=method_id)
    return [dict(r) for r in records]


def fetch_call_arguments(runner: QueryRunner, call_id: str) -> list[dict]:
    """Fetch arguments for a call (Q11).

    Args:
        runner: QueryRunner instance.
        call_id: Call node ID.

    Returns:
        List of argument records.
    """
    records = runner.execute(CALL_ARGUMENTS, call_id=call_id)
    return [dict(r) for r in records]


def check_type_hints(
    runner: QueryRunner, source_id: str, target_id: str
) -> dict:
    """Check type hint relationships for reference type inference (Q12).

    Args:
        runner: QueryRunner instance.
        source_id: Source node ID.
        target_id: Target node ID.

    Returns:
        Dict with has_arg_type_hint, has_return_type_hint, has_class_property_type_hint.
    """
    record = runner.execute_single(
        TYPE_HINT_CHECKS, source_id=source_id, target_id=target_id
    )
    if record:
        return {
            "has_arg_type_hint": bool(record["has_arg_type_hint"]),
            "has_return_type_hint": bool(record["has_return_type_hint"]),
            "has_class_property_type_hint": bool(record["has_class_property_type_hint"]),
        }
    return {
        "has_arg_type_hint": False,
        "has_return_type_hint": False,
        "has_class_property_type_hint": False,
    }


def is_internal_reference(runner: QueryRunner, target_id: str, source_id: str) -> bool:
    """Check if source is contained within target (R3 filtering).

    Args:
        runner: QueryRunner instance.
        target_id: Target class node ID.
        source_id: Source node ID.

    Returns:
        True if source is internal to target.
    """
    record = runner.execute_single(
        INTERNAL_CHECK, target_id=target_id, source_id=source_id
    )
    if record:
        return bool(record["is_internal"])
    return False
