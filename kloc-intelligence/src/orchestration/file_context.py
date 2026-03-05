"""File context orchestrator: builds USED BY and USES trees for File nodes.

Replicates kloc-cli's generic build_tree behavior for File nodes:
- USED BY: collect all incoming USES edges to file + all contained members,
  group by source method, emit one entry per edge. File sources (imports)
  and internal self-references are excluded at the query level.
- USES: deduplicates outgoing dependencies by target FQN.
"""

from __future__ import annotations

from ..db.query_runner import QueryRunner
from ..db.queries.context_file import (
    fetch_file_used_by_all,
    fetch_file_uses,
    fetch_file_call_uses,
)
from ..db.queries.context_method import fetch_method_call_arguments
from ..db.queries.definition import _extract_signature_from_doc
from ..models.results import ArgumentInfo, ContextEntry, MemberRef


def _classify_reference_type_for_class(edge: dict) -> str:
    """Classify reference_type for USES edges targeting a Class/Interface/Trait/Enum.

    Uses TYPE_HINT flags from the query:
    - has_prop_th -> property_type (promoted constructor params)
    - has_arg_th -> parameter_type
    - has_return_th -> return_type
    - none -> type_hint (fallback)
    """
    if edge.get("has_prop_th"):
        return "property_type"
    if edge.get("has_arg_th"):
        return "parameter_type"
    if edge.get("has_return_th"):
        return "return_type"
    return "type_hint"


def _call_kind_to_reference_type(call_kind: str | None) -> str:
    """Map call_kind to reference_type for member_ref."""
    if not call_kind:
        return "method_call"
    mapping = {
        "constructor": "instantiation",
        "method": "method_call",
        "method_static": "static_call",
        "function": "function_call",
        "access": "property_access",
    }
    return mapping.get(call_kind, "method_call")


def _infer_reference_type_from_target(target_kind: str) -> str:
    """Infer reference_type from target node kind when no Call info available."""
    if target_kind in ("Method", "Function"):
        return "method_call"
    if target_kind in ("Property", "StaticProperty"):
        return "property_access"
    if target_kind == "Constant":
        return "constant_access"
    return "type_hint"


def _build_rich_arguments_for_call(
    runner: QueryRunner,
    call_id: str,
) -> list[ArgumentInfo]:
    """Build rich argument list for a call site (file context format)."""
    raw_args = fetch_method_call_arguments(runner, call_id)
    arguments: list[ArgumentInfo] = []

    for arg in raw_args:
        position = arg.get("position", 0)
        param_fqn = arg.get("param_fqn") or ""
        arg_expression = arg.get("arg_expression") or ""
        value_expr_name = arg.get("value_expr") or ""
        value_source = arg.get("value_source") or ""
        value_fqn = arg.get("value_fqn") or ""
        value_type_name = arg.get("value_type_name")

        # param_name from param_fqn
        param_name = None
        if param_fqn:
            if ".$" in param_fqn:
                param_name = "$" + param_fqn.split(".$")[-1]
            elif "::$" in param_fqn:
                param_name = "$" + param_fqn.split("::$")[-1]

        value_expr = arg_expression or value_expr_name or ""

        # value_ref_symbol
        value_ref_symbol = None
        if value_source in ("parameter", "local") and value_fqn:
            value_ref_symbol = value_fqn

        # source_chain for result sources
        source_chain = None
        if value_source == "result" and arg.get("source_callee_fqn"):
            chain_entry: dict = {
                "fqn": arg["source_callee_fqn"],
                "kind": arg.get("source_callee_kind") or "Method",
            }

            source_call_kind = arg.get("source_call_kind") or ""
            if source_call_kind == "constructor":
                chain_entry["reference_type"] = "constructor"
            else:
                chain_entry["reference_type"] = "method"
                source_file = arg.get("source_call_file")
                source_line = arg.get("source_call_line")
                if source_file and source_line is not None:
                    chain_entry["on"] = f"{source_file}:{source_line}:(result)"
                    chain_entry["on_file"] = source_file
                    chain_entry["on_line"] = source_line

            source_chain = [chain_entry]

        info = ArgumentInfo(
            position=position,
            param_name=param_name,
            value_expr=value_expr,
            value_source=value_source if value_source else None,
            value_type=value_type_name,
            param_fqn=param_fqn if param_fqn else None,
            value_ref_symbol=value_ref_symbol,
            source_chain=source_chain,
        )
        arguments.append(info)

    return arguments


def build_file_used_by(
    runner: QueryRunner,
    start_id: str,
    max_depth: int,
    limit: int,
    include_impl: bool = False,
) -> list[ContextEntry]:
    """Build the USED BY tree for a File node.

    Collects ALL incoming USES edges to the file's contained members
    (classes, methods, properties, etc.), excluding:
    - File source nodes (import/use statements)
    - Internal self-references (source.file == member.file)

    Each USES edge becomes a separate entry with member_ref showing
    the specific symbol referenced.

    Reference type classification:
    - Class targets: TYPE_HINT flags determine parameter_type/return_type/
      property_type/type_hint
    - Method/Property targets: Call node info determines method_call/
      static_call/instantiation, or inferred from target kind
    """
    all_edges = fetch_file_used_by_all(runner, start_id)

    entries: list[ContextEntry] = []
    seen_keys: set[str] = set()

    for edge in all_edges:
        method_fqn = edge.get("method_fqn", "")
        method_kind = edge.get("method_kind", "")

        if not method_fqn:
            continue

        # Ensure FQN ends with () for methods
        if not method_fqn.endswith("()") and method_kind in ("Method", "Function"):
            method_fqn += "()"

        target_fqn = edge.get("target_fqn", "")
        target_kind = edge.get("target_kind", "")
        target_name = edge.get("target_name", "")

        # Determine reference_type
        if target_kind in ("Class", "Interface", "Trait", "Enum"):
            # Class target: check for constructor Call first
            call_kind = edge.get("call_kind")
            if call_kind == "constructor":
                ref_type = "instantiation"
            else:
                ref_type = _classify_reference_type_for_class(edge)
        else:
            # Member target: use Call node info if available
            call_kind = edge.get("call_kind")
            if call_kind:
                ref_type = _call_kind_to_reference_type(call_kind)
            else:
                ref_type = _infer_reference_type_from_target(target_kind)

        # Edge location
        edge_file = edge.get("edge_file") or edge.get("method_file")
        edge_line = edge.get("edge_line")
        if edge_line is None:
            edge_line = edge.get("method_start_line")

        # Dedup key
        dedup_key = f"{method_fqn}|{target_fqn}|{ref_type}|{edge_line}"
        if dedup_key in seen_keys:
            continue
        seen_keys.add(dedup_key)

        # Signature
        method_doc = edge.get("method_documentation") or []
        signature = _extract_signature_from_doc(method_doc)

        # Display name for member_ref target
        display_name = target_name
        if target_kind in ("Class", "Interface", "Trait", "Enum"):
            if "\\" in target_fqn:
                display_name = target_fqn.rsplit("\\", 1)[-1]
        elif target_kind in ("Method", "Function"):
            display_name = f"{target_name}()"

        member_ref = MemberRef(
            target_name=display_name,
            target_fqn=target_fqn,
            target_kind=target_kind,
            file=edge_file,
            line=edge_line,
            reference_type=ref_type,
        )

        # Build arguments for method_call/static_call/instantiation entries
        arguments = []
        call_id = edge.get("call_id")
        if call_id and ref_type in ("method_call", "static_call", "instantiation"):
            arguments = _build_rich_arguments_for_call(runner, call_id)

        entry = ContextEntry(
            depth=1,
            node_id=edge.get("method_id", ""),
            fqn=method_fqn,
            kind=method_kind,
            file=edge_file,
            line=edge_line,
            signature=signature,
            member_ref=member_ref,
            arguments=arguments,
            children=[],
        )
        entries.append(entry)

    # Sort by (file, line)
    entries.sort(key=lambda e: (e.file or "", e.line if e.line is not None else 0))
    return entries[:limit]


def build_file_uses(
    runner: QueryRunner,
    start_id: str,
    max_depth: int,
    limit: int,
    include_impl: bool = False,
) -> list[ContextEntry]:
    """Build the USES tree for a File node.

    Combines two data sources:
    1. USES edges from nodes in this file to targets (type references)
    2. Call edges from methods in this file to callees (invocations)

    USES edges are processed first (they provide the base dependency set).
    Call edges add targets not already covered by USES edges.

    Deduplicates by target FQN: each dependency appears once with the
    first (lowest line) occurrence and the correct reference_type.
    """
    dep_map: dict[str, dict] = {}

    # Get the file path for filtering
    file_info = runner.execute_single(
        "MATCH (n:Node {node_id: $id}) RETURN n.file AS file",
        id=start_id,
    )
    file_path = file_info["file"] if file_info else ""

    # 1) USES-edge based references FIRST
    uses_edges = fetch_file_uses(runner, start_id)
    for edge in uses_edges:
        target_fqn = edge.get("target_fqn", "")
        target_kind = edge.get("target_kind", "")
        target_name = edge.get("target_name", "")
        source_kind = edge.get("source_kind", "")

        edge_file = edge.get("edge_file") or file_path
        edge_line = edge.get("edge_line")

        # Infer reference_type from target kind
        ref_type = _infer_reference_type_from_target(target_kind)

        # Dedup by target_fqn: keep first occurrence (lowest line)
        if target_fqn in dep_map:
            existing_line = dep_map[target_fqn].get("line")
            if existing_line is not None and edge_line is not None and edge_line < existing_line:
                dep_map[target_fqn]["line"] = edge_line
                dep_map[target_fqn]["ref_type"] = ref_type
                dep_map[target_fqn]["file"] = edge_file
            continue

        dep_map[target_fqn] = {
            "target_fqn": target_fqn,
            "target_kind": target_kind,
            "target_name": target_name,
            "target_id": edge.get("target_id", ""),
            "ref_type": ref_type,
            "line": edge_line,
            "file": edge_file,
        }

    # 2) Call-edge based references
    call_edges = fetch_file_call_uses(runner, start_id)
    for call in call_edges:
        callee_fqn = call.get("callee_fqn", "")
        callee_kind = call.get("callee_kind", "")
        callee_name = call.get("callee_name", "")
        call_kind = call.get("call_kind", "")

        call_file = call.get("call_file") or file_path
        call_line = call.get("call_line")

        if callee_kind in ("Property", "StaticProperty"):
            ref_type = "property_access"
        else:
            ref_type = _call_kind_to_reference_type(call_kind)

        if callee_fqn in dep_map:
            continue

        dep_map[callee_fqn] = {
            "target_fqn": callee_fqn,
            "target_kind": callee_kind,
            "target_name": callee_name,
            "target_id": call.get("callee_id", ""),
            "ref_type": ref_type,
            "line": call_line,
            "file": call_file,
        }

    # Build entries
    entries: list[ContextEntry] = []
    for dep in dep_map.values():
        target_fqn = dep["target_fqn"]
        target_kind = dep["target_kind"]
        target_name = dep["target_name"]
        ref_type = dep["ref_type"]
        dep_file = dep["file"]
        dep_line = dep["line"]

        display_name = target_name
        if target_kind in ("Class", "Interface", "Trait", "Enum"):
            if "\\" in target_fqn:
                display_name = target_fqn.rsplit("\\", 1)[-1]
        elif target_kind in ("Method", "Function"):
            display_name = f"{target_name}()"
        elif target_kind in ("Property", "StaticProperty"):
            if not target_name.startswith("$"):
                display_name = f"${target_name}"

        member_ref = MemberRef(
            target_name=display_name,
            target_fqn=target_fqn,
            target_kind=target_kind,
            file=dep_file,
            line=dep_line,
            reference_type=ref_type,
        )

        entry = ContextEntry(
            depth=1,
            node_id=dep["target_id"],
            fqn=target_fqn,
            kind=target_kind,
            file=dep_file,
            line=dep_line,
            member_ref=member_ref,
            children=[],
        )
        entries.append(entry)

    entries.sort(key=lambda e: (e.file or "", e.line if e.line is not None else 0))
    return entries[:limit]
