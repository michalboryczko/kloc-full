"""Cypher queries for property context: USED BY (readers) and USES (writers).

Property USED BY:
- Call nodes that access this property (via CALLS edge)
- Each grouped by containing method
- refType="property_access", on="$this->prop (FQN)", onKind="property"

Property USES:
- Constructor calls that pass arguments matching this property's promoted parameter
- Flat args format: {"Class::__construct().$param": "$value"}
"""

from __future__ import annotations

from ..query_runner import QueryRunner


# ─────────────────────────────────────────────────────────────────────
# Q1: Property access sites (who reads this property)
# ─────────────────────────────────────────────────────────────────────

PROPERTY_ACCESS_SITES = """
MATCH (prop:Node {node_id: $prop_id})<-[:CALLS]-(call:Node {kind: 'Call'})
MATCH (call)<-[:CONTAINS*1..10]-(method:Node)
WHERE method.kind IN ['Method', 'Function']
RETURN call.node_id AS call_id,
       call.file AS call_file,
       call.start_line AS call_line,
       call.call_kind AS call_kind,
       method.node_id AS method_id,
       method.fqn AS method_fqn,
       method.kind AS method_kind,
       method.file AS method_file,
       method.start_line AS method_start_line,
       method.documentation AS method_documentation
ORDER BY call.file, call.start_line
"""

# ─────────────────────────────────────────────────────────────────────
# Q2: Constructor calls that write to this property (promoted parameter)
# Find calls to __construct() that pass an argument matching
# the property's promoted parameter FQN.
# ─────────────────────────────────────────────────────────────────────

PROPERTY_WRITERS = """
MATCH (prop:Node {node_id: $prop_id})
OPTIONAL MATCH (prop)-[:ASSIGNED_FROM]->(param:Node {kind: 'Value', value_kind: 'parameter'})
WITH prop, param
WHERE param IS NOT NULL
WITH prop, param
MATCH (call:Node {kind: 'Call'})-[a:ARGUMENT]->(arg_val:Node)
WHERE a.parameter = param.fqn
MATCH (call)<-[:CONTAINS*1..10]-(method:Node)
WHERE method.kind IN ['Method', 'Function']
OPTIONAL MATCH (arg_val)<-[:PRODUCES]-(source_call:Node {kind: 'Call'})-[:CALLS]->(source_callee:Node)
RETURN call.node_id AS call_id,
       call.file AS call_file,
       call.start_line AS call_line,
       call.call_kind AS call_kind,
       method.node_id AS method_id,
       method.fqn AS method_fqn,
       method.kind AS method_kind,
       method.file AS method_file,
       method.start_line AS method_start_line,
       a.parameter AS param_fqn,
       a.expression AS arg_expression,
       arg_val.name AS value_expr,
       arg_val.value_kind AS value_source,
       source_callee.name AS source_callee_name,
       source_call.call_kind AS source_call_kind
ORDER BY call.file, call.start_line
"""


def fetch_property_access_sites(runner: QueryRunner, prop_id: str) -> list[dict]:
    """Fetch all Call nodes that access (read) this property."""
    records = runner.execute(PROPERTY_ACCESS_SITES, prop_id=prop_id)
    return [dict(r) for r in records]


def fetch_property_writers(runner: QueryRunner, prop_id: str) -> list[dict]:
    """Fetch constructor calls that write to this property via promoted parameter."""
    records = runner.execute(PROPERTY_WRITERS, prop_id=prop_id)
    return [dict(r) for r in records]
