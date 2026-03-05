"""Interface context orchestrator for USED BY and USES sections.

Interface USED BY:
- Implementors with [implements] refType
- Injection points (properties typed to interface) with [property_type] refType
- Contract relevance filtering for injection points
- Depth 2: override methods, injection point calls

Interface USES:
- Parent interface with [extends] refType
- Method signature types with [parameter_type] / [return_type] refType
"""

from __future__ import annotations

from typing import Optional

from ..db.query_runner import QueryRunner
from ..db.queries.context_interface import (
    fetch_implementors,
    fetch_injection_points,
    fetch_implementor_injection_points,
    fetch_contract_method_names,
    check_contract_relevance,
    fetch_interface_signature_types,
    fetch_parent_interface,
)
from ..models.results import ContextEntry


def build_interface_used_by(
    runner: QueryRunner,
    start_id: str,
    max_depth: int,
    limit: int,
    include_impl: bool = False,
) -> list[ContextEntry]:
    """Build the USED BY tree for an Interface node.

    Collects implementors and injection points (properties typed to the interface).
    Injection points are filtered by contract relevance.

    Args:
        runner: QueryRunner instance.
        start_id: Interface node ID.
        max_depth: Maximum depth for tree expansion.
        limit: Maximum entries.
        include_impl: Whether to include implementation details.

    Returns:
        List of ContextEntry objects.
    """
    # Step 1: Fetch implementors
    implementors = fetch_implementors(runner, start_id)

    # Step 2: Fetch injection points (direct + transitive through hierarchy)
    injection_points = fetch_injection_points(runner, start_id)

    # Step 3: Fetch injection points from concrete implementors
    impl_injection_points = fetch_implementor_injection_points(runner, start_id)

    # Step 4: Get contract method names for relevance filtering
    contract_method_names = fetch_contract_method_names(runner, start_id)

    # Step 5: Build implementor entries
    entries: list[ContextEntry] = []

    for impl in implementors:
        entry = ContextEntry(
            depth=1,
            node_id=impl["id"],
            fqn=impl["fqn"],
            kind=impl["kind"],
            file=impl.get("file"),
            line=impl.get("start_line"),
            ref_type="implements",
            children=[],
        )

        # Depth 2: override methods
        if max_depth >= 2:
            from .class_context import _build_override_children
            entry.children = _build_override_children(
                runner, impl["id"], start_id, 2, max_depth
            )

        entries.append(entry)

    # Step 6: Build injection point entries (with contract relevance)
    seen_props: set[str] = set()

    # Merge direct and implementor injection points
    all_injection_points = injection_points + impl_injection_points

    for ip in all_injection_points:
        prop_id = ip["prop_id"]
        if prop_id in seen_props:
            continue
        seen_props.add(prop_id)

        # Contract relevance check
        if contract_method_names:
            prop_fqn_parts = ip["prop_fqn"].rsplit("::", 1)
            if len(prop_fqn_parts) == 2:
                # Use the property FQN for access chain matching
                is_relevant = check_contract_relevance(
                    runner, prop_id, ip["prop_fqn"], contract_method_names
                )
                if not is_relevant:
                    continue

        entry = ContextEntry(
            depth=1,
            node_id=prop_id,
            fqn=ip["prop_fqn"],
            kind=ip.get("prop_kind", "Property"),
            file=ip.get("prop_file"),
            line=ip.get("prop_start_line"),
            ref_type="property_type",
            children=[],
        )

        # Depth 2: method calls through injection point
        if max_depth >= 2:
            from .class_context import _build_injection_point_children
            entry.children = _build_injection_point_children(
                runner, prop_id, ip["prop_fqn"], 2, max_depth
            )

        entries.append(entry)

    # Step 7: Sort: implements first, then property_type, then by (file, line)
    def _sort_key(e: ContextEntry):
        priority = {"implements": 0, "property_type": 1}.get(e.ref_type or "", 2)
        return (priority, e.file or "", e.line if e.line is not None else 0)

    entries.sort(key=_sort_key)
    return entries[:limit]


def build_interface_uses(
    runner: QueryRunner,
    start_id: str,
    max_depth: int,
    limit: int,
    include_impl: bool = False,
) -> list[ContextEntry]:
    """Build the USES tree for an Interface node.

    Collects parent interface (extends) and method signature types.

    Args:
        runner: QueryRunner instance.
        start_id: Interface node ID.
        max_depth: Maximum depth.
        limit: Maximum entries.
        include_impl: Whether to include implementation details.

    Returns:
        List of ContextEntry objects.
    """
    entries: list[ContextEntry] = []
    seen: set[str] = {start_id}

    # Step 1: Parent interface (extends)
    parents = fetch_parent_interface(runner, start_id)
    for parent in parents:
        pid = parent["id"]
        if pid in seen:
            continue
        seen.add(pid)
        entry = ContextEntry(
            depth=1,
            node_id=pid,
            fqn=parent["fqn"],
            kind=parent["kind"],
            file=parent.get("file"),
            line=parent.get("start_line"),
            ref_type="extends",
            children=[],
        )
        entries.append(entry)

    # Step 2: Method signature types
    # Collect target_id -> {ref_type, file, line} with dedup rules:
    # parameter_type wins over return_type, but doesn't overwrite extends
    target_info: dict[str, dict] = {}

    sig_types = fetch_interface_signature_types(runner, start_id)
    for sig in sig_types:
        method_file = sig.get("method_file")
        method_line = sig.get("method_line")

        # Return type
        ret_id = sig.get("ret_type_id")
        if ret_id and ret_id not in seen and ret_id != start_id:
            existing = target_info.get(ret_id)
            if not existing or existing["ref_type"] == "return_type":
                target_info[ret_id] = {
                    "ref_type": "return_type",
                    "fqn": sig["ret_type_fqn"],
                    "kind": sig["ret_type_kind"],
                    "file": method_file,
                    "line": method_line,
                }

        # Parameter types
        for pt in sig.get("param_types", []):
            pt_id = pt.get("id")
            if pt_id and pt_id not in seen and pt_id != start_id:
                existing = target_info.get(pt_id)
                if not existing or existing["ref_type"] == "return_type":
                    target_info[pt_id] = {
                        "ref_type": "parameter_type",
                        "fqn": pt["fqn"],
                        "kind": pt["kind"],
                        "file": method_file,
                        "line": method_line,
                    }

    for tid, info in target_info.items():
        seen.add(tid)
        entry = ContextEntry(
            depth=1,
            node_id=tid,
            fqn=info["fqn"],
            kind=info["kind"],
            file=info.get("file"),
            line=info.get("line"),
            ref_type=info["ref_type"],
            children=[],
        )

        # Depth 2: recursive class-level USES expansion
        if max_depth >= 2:
            from .class_context import build_class_uses_recursive
            entry.children = build_class_uses_recursive(
                runner, tid, 2, max_depth, limit, {start_id}
            )

        entries.append(entry)

    # Sort: extends first, then parameter_type/return_type
    uses_priority = {
        "extends": 0,
        "implements": 1,
        "property_type": 2,
        "parameter_type": 3,
        "return_type": 3,
        "type_hint": 4,
    }

    def _sort_key(e: ContextEntry):
        pri = uses_priority.get(e.ref_type or "", 10)
        return (pri, e.file or "", e.line if e.line is not None else 0)

    entries.sort(key=_sort_key)
    return entries[:limit]
