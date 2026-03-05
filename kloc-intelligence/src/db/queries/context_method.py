"""Cypher queries for method context: USED BY (callers) and USES (execution flow).

Method USED BY:
- Callers of this method (via Call nodes)
- Full argument mapping with expressions and source chains

Method USES:
- Execution flow: Call children in line-number order
- Kind 1: local_variable with source_call
- Kind 2: direct call
- Type references from structural USES edges
"""

from __future__ import annotations

from ..query_runner import QueryRunner


# ─────────────────────────────────────────────────────────────────────
# Q1: Callers of a Method
# ─────────────────────────────────────────────────────────────────────

METHOD_CALLERS = """
MATCH (target:Node {node_id: $id})
MATCH (call:Node {kind: 'Call'})-[:CALLS]->(target)
MATCH (call)<-[:CONTAINS*1..10]-(method:Node)
WHERE method.kind IN ['Method', 'Function']
WITH call, method, target
OPTIONAL MATCH (call)-[:RECEIVER]->(recv:Node {kind: 'Value'})
OPTIONAL MATCH (recv)<-[:PRODUCES]-(recv_call:Node {kind: 'Call'})-[:CALLS]->(recv_prop:Node)
WHERE recv_prop.kind IN ['Property', 'StaticProperty']
RETURN call.node_id AS call_id,
       call.call_kind AS call_kind,
       call.file AS call_file,
       call.start_line AS call_line,
       method.node_id AS caller_id,
       method.fqn AS caller_fqn,
       method.kind AS caller_kind,
       method.file AS caller_file,
       method.start_line AS caller_start_line,
       method.documentation AS caller_documentation,
       target.fqn AS target_fqn,
       target.name AS target_name,
       target.kind AS target_kind,
       recv.value_kind AS recv_value_kind,
       recv.name AS recv_name,
       recv_prop.fqn AS on_property
ORDER BY call.file, call.start_line
"""

# ─────────────────────────────────────────────────────────────────────
# Q1b: Rich argument info for a call (method-level format)
# ─────────────────────────────────────────────────────────────────────

METHOD_CALL_ARGUMENTS = """
MATCH (call:Node {node_id: $call_id})-[a:ARGUMENT]->(arg_val:Node)
OPTIONAL MATCH (param_node:Node {fqn: a.parameter, kind: 'Value', value_kind: 'parameter'})
OPTIONAL MATCH (promoted_prop:Node {kind: 'Property'})-[:ASSIGNED_FROM]->(param_node)
OPTIONAL MATCH (arg_val)-[:TYPE_OF]->(arg_type:Node)
OPTIONAL MATCH (arg_val)<-[:PRODUCES]-(source_call:Node {kind: 'Call'})-[:CALLS]->(source_callee:Node)
RETURN a.position AS position,
       COALESCE(promoted_prop.fqn, a.parameter) AS param_fqn,
       a.expression AS arg_expression,
       arg_val.name AS value_expr,
       arg_val.value_kind AS value_source,
       arg_val.kind AS value_kind,
       arg_val.fqn AS value_fqn,
       arg_type.name AS value_type_name,
       source_call.node_id AS source_call_id,
       source_callee.fqn AS source_callee_fqn,
       source_callee.kind AS source_callee_kind,
       source_callee.name AS source_callee_name,
       source_call.file AS source_call_file,
       source_call.start_line AS source_call_line,
       source_call.call_kind AS source_call_kind
ORDER BY a.position
"""

# ─────────────────────────────────────────────────────────────────────
# Q1c: Source chain for a result-type argument value
# ─────────────────────────────────────────────────────────────────────

ARGUMENT_SOURCE_CHAIN = """
MATCH (arg_val:Node {node_id: $arg_val_id})<-[:PRODUCES]-(call:Node {kind: 'Call'})-[:CALLS]->(callee:Node)
OPTIONAL MATCH (call)-[:RECEIVER]->(recv:Node {kind: 'Value'})
OPTIONAL MATCH (recv)<-[:PRODUCES]-(recv_call:Node {kind: 'Call'})-[:CALLS]->(recv_prop:Node)
WHERE recv_prop.kind IN ['Property', 'StaticProperty']
RETURN callee.fqn AS callee_fqn,
       callee.kind AS callee_kind,
       callee.name AS callee_name,
       call.call_kind AS call_kind,
       call.file AS call_file,
       call.start_line AS call_line,
       recv.value_kind AS recv_value_kind,
       recv.name AS recv_name,
       recv_prop.fqn AS recv_prop_fqn
"""

# ─────────────────────────────────────────────────────────────────────
# Q2: All Call children within a method (for execution flow)
# ─────────────────────────────────────────────────────────────────────

METHOD_CALLS = """
MATCH (method:Node {node_id: $method_id})-[:CONTAINS]->(call:Node {kind: 'Call'})
OPTIONAL MATCH (call)-[:CALLS]->(callee:Node)
OPTIONAL MATCH (call)-[:RECEIVER]->(recv:Node {kind: 'Value'})
OPTIONAL MATCH (recv)<-[:PRODUCES]-(recv_call:Node {kind: 'Call'})-[:CALLS]->(recv_prop:Node)
WHERE recv_prop.kind IN ['Property', 'StaticProperty']
OPTIONAL MATCH (call)-[:PRODUCES]->(result:Node {kind: 'Value'})
OPTIONAL MATCH (local:Node {kind: 'Value', value_kind: 'local'})-[:ASSIGNED_FROM]->(result)
OPTIONAL MATCH (local)-[:TYPE_OF]->(local_type:Node)
RETURN call.node_id AS call_id,
       call.call_kind AS call_kind,
       call.name AS call_name,
       call.file AS call_file,
       call.start_line AS call_line,
       callee.node_id AS callee_id,
       callee.fqn AS callee_fqn,
       callee.kind AS callee_kind,
       callee.name AS callee_name,
       callee.file AS callee_file,
       callee.start_line AS callee_start_line,
       callee.documentation AS callee_documentation,
       recv.value_kind AS recv_value_kind,
       recv.name AS recv_name,
       recv.fqn AS recv_fqn,
       recv.file AS recv_file,
       recv.start_line AS recv_start_line,
       recv_prop.fqn AS on_property,
       result.node_id AS result_id,
       local.node_id AS local_id,
       local.fqn AS local_fqn,
       local.name AS local_name,
       local_type.name AS local_type_name
ORDER BY call.start_line
"""

# ─────────────────────────────────────────────────────────────────────
# Q3: Consumed Call Detection (calls whose result is used by another call)
# ─────────────────────────────────────────────────────────────────────

CONSUMED_CALLS = """
MATCH (method:Node {node_id: $method_id})-[:CONTAINS]->(consumer:Node {kind: 'Call'})
MATCH (consumer)-[:RECEIVER]->(recv:Node {kind: 'Value', value_kind: 'result'})<-[:PRODUCES]-(src:Node {kind: 'Call'})
WHERE (method)-[:CONTAINS]->(src)
RETURN DISTINCT src.node_id AS consumed_call_id

UNION

MATCH (method:Node {node_id: $method_id})-[:CONTAINS]->(consumer:Node {kind: 'Call'})
MATCH (consumer)-[a:ARGUMENT]->(arg:Node {kind: 'Value', value_kind: 'result'})<-[:PRODUCES]-(src:Node {kind: 'Call'})
WHERE (method)-[:CONTAINS]->(src)
RETURN DISTINCT src.node_id AS consumed_call_id
"""


def fetch_method_callers(runner: QueryRunner, method_id: str) -> list[dict]:
    """Fetch callers of a method."""
    records = runner.execute(METHOD_CALLERS, id=method_id)
    return [dict(r) for r in records]


def fetch_method_call_arguments(runner: QueryRunner, call_id: str) -> list[dict]:
    """Fetch rich argument info for a call (method-level format)."""
    records = runner.execute(METHOD_CALL_ARGUMENTS, call_id=call_id)
    return [dict(r) for r in records]


def fetch_method_calls(runner: QueryRunner, method_id: str) -> list[dict]:
    """Fetch all Call children within a method for execution flow."""
    records = runner.execute(METHOD_CALLS, method_id=method_id)
    return [dict(r) for r in records]


def fetch_consumed_calls(runner: QueryRunner, method_id: str) -> set[str]:
    """Fetch IDs of calls whose results are consumed by other calls."""
    records = runner.execute(CONSUMED_CALLS, method_id=method_id)
    return {r["consumed_call_id"] for r in records}
