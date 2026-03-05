"""Cypher queries for interface context: USED BY and USES.

Interface USED BY:
- Implementors (IMPLEMENTS edge)
- Injection points (properties typed to the interface via TYPE_HINT)
- Contract relevance filtering

Interface USES:
- Method signature types (parameter_type, return_type)
- Parent interface (extends)
"""

from __future__ import annotations

from ..query_runner import QueryRunner


# ─────────────────────────────────────────────────────────────────────
# Q1: Direct Implementors
# ─────────────────────────────────────────────────────────────────────

IMPLEMENTORS = """
MATCH (iface:Node {node_id: $id})<-[:IMPLEMENTS]-(impl:Node)
RETURN impl.node_id AS id, impl.fqn AS fqn, impl.kind AS kind,
       impl.file AS file, impl.start_line AS start_line
ORDER BY impl.file, impl.start_line
"""

# ─────────────────────────────────────────────────────────────────────
# Q2: Injection Points (properties typed to the interface or its hierarchy)
# ─────────────────────────────────────────────────────────────────────

INJECTION_POINTS = """
MATCH (iface:Node {node_id: $id})<-[:EXTENDS*0..]-(child_iface:Node)
WHERE child_iface.kind = 'Interface'
WITH COLLECT(DISTINCT child_iface) AS all_ifaces
UNWIND all_ifaces AS target_iface
MATCH (target_iface)<-[:TYPE_HINT]-(prop:Node {kind: 'Property'})
MATCH (prop)<-[:CONTAINS*1..3]-(cls:Node)
WHERE cls.kind IN ['Class', 'Interface', 'Trait', 'Enum']
RETURN DISTINCT prop.node_id AS prop_id, prop.fqn AS prop_fqn,
       prop.kind AS prop_kind,
       prop.file AS prop_file, prop.start_line AS prop_start_line,
       cls.node_id AS class_id, cls.fqn AS class_fqn
ORDER BY prop.file, prop.start_line
"""

# ─────────────────────────────────────────────────────────────────────
# Q3: Injection Points from Concrete Implementors
# ─────────────────────────────────────────────────────────────────────

IMPLEMENTOR_INJECTION_POINTS = """
MATCH (iface:Node {node_id: $id})<-[:IMPLEMENTS]-(impl:Node)
MATCH (impl)<-[:TYPE_HINT]-(prop:Node {kind: 'Property'})
MATCH (prop)<-[:CONTAINS*1..3]-(cls:Node)
WHERE cls.kind IN ['Class', 'Interface', 'Trait', 'Enum']
  AND cls.node_id <> impl.node_id
RETURN DISTINCT prop.node_id AS prop_id, prop.fqn AS prop_fqn,
       prop.kind AS prop_kind,
       prop.file AS prop_file, prop.start_line AS prop_start_line,
       cls.node_id AS class_id, cls.fqn AS class_fqn
ORDER BY prop.file, prop.start_line
"""

# ─────────────────────────────────────────────────────────────────────
# Q4: Contract Method Names
# ─────────────────────────────────────────────────────────────────────

CONTRACT_METHOD_NAMES = """
MATCH (iface:Node {node_id: $id})-[:CONTAINS]->(method:Node {kind: 'Method'})
RETURN method.name AS method_name
"""

# ─────────────────────────────────────────────────────────────────────
# Q5: Contract Relevance Check
# ─────────────────────────────────────────────────────────────────────

CONTRACT_RELEVANCE_CHECK = """
MATCH (prop:Node {node_id: $prop_id})<-[:CONTAINS*1..3]-(cls:Node)
WHERE cls.kind IN ['Class', 'Interface', 'Trait', 'Enum']
MATCH (cls)-[:CONTAINS]->(method:Node {kind: 'Method'})-[:CONTAINS]->(call:Node {kind: 'Call'})
MATCH (call)-[:RECEIVER]->(recv:Node {kind: 'Value'})
MATCH (recv)<-[:PRODUCES]-(recv_call:Node {kind: 'Call'})-[:CALLS]->(recv_target:Node)
WHERE recv_target.fqn = $prop_fqn
MATCH (call)-[:CALLS]->(callee:Node)
WHERE callee.name IN $contract_method_names
RETURN count(*) > 0 AS calls_contract
"""

# ─────────────────────────────────────────────────────────────────────
# Q6: Interface Method Signature Types (for USES section)
# ─────────────────────────────────────────────────────────────────────

INTERFACE_SIGNATURE_TYPES = """
MATCH (iface:Node {node_id: $id})-[:CONTAINS]->(method:Node {kind: 'Method'})
WITH method
OPTIONAL MATCH (method)-[:TYPE_HINT]->(ret_type:Node)
WHERE ret_type.node_id <> $id
  AND ret_type.kind IN ['Class', 'Interface', 'Trait', 'Enum']
WITH method, ret_type
OPTIONAL MATCH (method)-[:CONTAINS]->(arg:Node {kind: 'Argument'})-[:TYPE_HINT]->(param_type:Node)
WHERE param_type.node_id <> $id
  AND param_type.kind IN ['Class', 'Interface', 'Trait', 'Enum']
RETURN method.node_id AS method_id,
       method.file AS method_file,
       method.start_line AS method_line,
       ret_type.node_id AS ret_type_id,
       ret_type.fqn AS ret_type_fqn,
       ret_type.kind AS ret_type_kind,
       COLLECT(DISTINCT {
         id: param_type.node_id,
         fqn: param_type.fqn,
         kind: param_type.kind
       }) AS param_types
"""

# ─────────────────────────────────────────────────────────────────────
# Q7: Parent Interface (extends)
# ─────────────────────────────────────────────────────────────────────

PARENT_INTERFACE = """
MATCH (iface:Node {node_id: $id})-[:EXTENDS]->(parent:Node)
WHERE parent.kind = 'Interface'
RETURN parent.node_id AS id, parent.fqn AS fqn, parent.kind AS kind,
       parent.file AS file, parent.start_line AS start_line
"""


def fetch_implementors(runner: QueryRunner, iface_id: str) -> list[dict]:
    """Fetch direct implementors of an interface."""
    records = runner.execute(IMPLEMENTORS, id=iface_id)
    return [dict(r) for r in records]


def fetch_injection_points(runner: QueryRunner, iface_id: str) -> list[dict]:
    """Fetch injection points (properties typed to interface hierarchy)."""
    records = runner.execute(INJECTION_POINTS, id=iface_id)
    return [dict(r) for r in records]


def fetch_implementor_injection_points(runner: QueryRunner, iface_id: str) -> list[dict]:
    """Fetch injection points from concrete implementor types."""
    records = runner.execute(IMPLEMENTOR_INJECTION_POINTS, id=iface_id)
    return [dict(r) for r in records]


def fetch_contract_method_names(runner: QueryRunner, iface_id: str) -> list[str]:
    """Fetch contract method names for the interface."""
    records = runner.execute(CONTRACT_METHOD_NAMES, id=iface_id)
    return [r["method_name"] for r in records]


def check_contract_relevance(
    runner: QueryRunner, prop_id: str, prop_fqn: str, contract_method_names: list[str]
) -> bool:
    """Check if a property's class calls any contract method through it."""
    if not contract_method_names:
        return True  # No contract methods = always relevant
    record = runner.execute_single(
        CONTRACT_RELEVANCE_CHECK,
        prop_id=prop_id,
        prop_fqn=prop_fqn,
        contract_method_names=contract_method_names,
    )
    return bool(record and record["calls_contract"])


def fetch_interface_signature_types(runner: QueryRunner, iface_id: str) -> list[dict]:
    """Fetch method signature types for USES section."""
    records = runner.execute(INTERFACE_SIGNATURE_TYPES, id=iface_id)
    return [dict(r) for r in records]


def fetch_parent_interface(runner: QueryRunner, iface_id: str) -> list[dict]:
    """Fetch parent interface (extends)."""
    records = runner.execute(PARENT_INTERFACE, id=iface_id)
    return [dict(r) for r in records]
