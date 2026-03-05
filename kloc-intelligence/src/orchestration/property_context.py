"""Property context orchestrator: builds USED BY and USES trees for Property nodes.

Property USED BY:
- Methods that read/access this property (via Call nodes targeting the property)
- Each entry has refType="property_access", on="$this->prop (FQN)", onKind="property"
- Line is the call site line (where property is accessed)

Property USES:
- Methods that write to this property (via constructor calls with promoted parameter)
- Each entry has flat args {"Class::__construct().$param": "$value"}
"""

from __future__ import annotations

from ..db.query_runner import QueryRunner
from ..db.queries.context_property import (
    fetch_property_access_sites,
    fetch_property_writers,
)
from ..db.queries.definition import _extract_signature_from_doc
from ..models.results import ArgumentInfo, ContextEntry


def build_property_used_by(
    runner: QueryRunner,
    start_id: str,
    max_depth: int,
    limit: int,
    include_impl: bool = False,
) -> list[ContextEntry]:
    """Build the USED BY tree for a Property node.

    Finds all Call nodes that access this property and builds entries
    with refType="property_access", on/onKind fields.

    Args:
        runner: QueryRunner instance.
        start_id: Target property node ID.
        max_depth: Maximum depth (depth-2 not yet supported for property).
        limit: Maximum entries.
        include_impl: Whether to include implementation details.

    Returns:
        List of ContextEntry objects.
    """
    # Get property FQN for the "on" field
    prop_info = runner.execute_single(
        "MATCH (n:Node {node_id: $id}) RETURN n.fqn AS fqn, n.name AS name",
        id=start_id,
    )
    prop_fqn = prop_info["fqn"] if prop_info else ""
    prop_name = prop_info["name"] if prop_info else ""

    # Build "on" display: "$this->id (App\...\Estate::$id)"
    # prop_name from Neo4j may or may not have $ prefix
    display_name = prop_name
    if display_name and display_name.startswith("$"):
        display_name = display_name[1:]
    on_display = f"$this->{display_name} ({prop_fqn})"

    sites = fetch_property_access_sites(runner, start_id)

    # Group by method (one entry per containing method, using first call site line)
    method_entries: dict[str, dict] = {}
    for site in sites:
        method_id = site["method_id"]
        if method_id not in method_entries:
            method_entries[method_id] = site

    entries: list[ContextEntry] = []
    for method_id, site in method_entries.items():
        method_fqn = site.get("method_fqn", "")
        if not method_fqn.endswith("()"):
            method_fqn += "()"

        # Line: call site line. Property access call_line from Neo4j is 0-based.
        # For property USED BY, normal 0-based -> 1-based conversion via OutputEntry.
        call_line = site.get("call_line")
        entry_line = call_line if call_line is not None else site.get("method_start_line")

        entry = ContextEntry(
            depth=1,
            node_id=method_id,
            fqn=method_fqn,
            kind=site.get("method_kind", "Method"),
            file=site.get("call_file") or site.get("method_file"),
            line=entry_line,
            ref_type="property_access",
            on=on_display,
            on_kind="property",
            children=[],
        )
        entries.append(entry)

    entries.sort(key=lambda e: (e.file or "", e.line if e.line is not None else 0))
    return entries[:limit]


def build_property_uses(
    runner: QueryRunner,
    start_id: str,
    max_depth: int,
    limit: int,
    include_impl: bool = False,
) -> list[ContextEntry]:
    """Build the USES tree for a Property node.

    Finds constructor calls that write to this property (via promoted parameter)
    and builds entries with flat args format.

    Args:
        runner: QueryRunner instance.
        start_id: Target property node ID.
        max_depth: Maximum depth.
        limit: Maximum entries.
        include_impl: Whether to include implementation details.

    Returns:
        List of ContextEntry objects.
    """
    writers = fetch_property_writers(runner, start_id)

    # Group by containing method
    method_entries: dict[str, list[dict]] = {}
    for w in writers:
        method_id = w["method_id"]
        method_entries.setdefault(method_id, []).append(w)

    entries: list[ContextEntry] = []
    for method_id, calls in method_entries.items():
        first = calls[0]
        method_fqn = first.get("method_fqn", "")
        if not method_fqn.endswith("()"):
            method_fqn += "()"

        # Line: call site line, normal 0-based -> 1-based conversion via OutputEntry
        call_line = first.get("call_line")
        entry_line = call_line if call_line is not None else first.get("method_start_line")

        # Build flat arguments from all calls in this method for this property
        arguments: list[ArgumentInfo] = []
        for c in calls:
            param_fqn = c.get("param_fqn") or ""
            value_source = c.get("value_source") or ""

            # For result values (nested constructor calls), use shortened callee name
            # e.g. "new \App\Domain\VO\UuidVO(...)" -> "__construct()"
            if value_source == "result":
                source_callee_name = c.get("source_callee_name") or ""
                source_call_kind = c.get("source_call_kind") or ""
                if source_callee_name:
                    # Use callee name with () for methods
                    if source_call_kind == "constructor":
                        value_expr = f"{source_callee_name}()"
                    else:
                        value_expr = f"{source_callee_name}()"
                else:
                    value_expr = c.get("arg_expression") or c.get("value_expr") or ""
            else:
                value_expr = c.get("arg_expression") or c.get("value_expr") or ""

            # param_name from param_fqn
            param_name = None
            if param_fqn:
                if ".$" in param_fqn:
                    param_name = "$" + param_fqn.split(".$")[-1]
                elif "::$" in param_fqn:
                    param_name = "$" + param_fqn.split("::$")[-1]

            if param_fqn and value_expr:
                arguments.append(ArgumentInfo(
                    position=0,
                    param_fqn=param_fqn,
                    param_name=param_name,
                    value_expr=value_expr,
                ))

        entry = ContextEntry(
            depth=1,
            node_id=method_id,
            fqn=method_fqn,
            kind=first.get("method_kind", "Method"),
            file=first.get("method_file") or first.get("call_file"),
            line=entry_line,
            arguments=arguments,
            children=[],
        )
        entries.append(entry)

    entries.sort(key=lambda e: (e.file or "", e.line if e.line is not None else 0))
    return entries[:limit]
