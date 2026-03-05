"""Class context orchestrator: builds USED BY and USES trees for Class nodes.

Replaces kloc-cli's class_context.py (~1,567 lines) by wiring Cypher query
results to the business logic from T07 (handlers, reference types).

Main entry points:
    - build_class_used_by() -- USED BY tree for a class
    - build_class_uses() -- USES tree for a class
"""

from __future__ import annotations

from typing import Optional

from ..db.query_runner import QueryRunner
from ..db.queries.context_class import (
    fetch_class_used_by_data,
    fetch_caller_chain,
    fetch_injection_point_calls,
    fetch_override_methods,
    fetch_override_internals,
    is_internal_reference,
)
from ..db.queries.context_class_uses import (
    fetch_class_uses_data,
    fetch_behavioral_depth2,
    fetch_node_deps,
    fetch_extends_children,
    fetch_overrides_and_inherited,
)
from ..db.queries.definition import _extract_signature_from_doc
from ..logic.reference_types import (
    CHAINABLE_REFERENCE_TYPES,
    REF_TYPE_PRIORITY,
    infer_reference_type,
    get_reference_type_from_call_kind,
    sort_entries_by_location,
    sort_entries_by_priority,
)
from ..logic.handlers import (
    EdgeContext,
    EntryBucket,
    USED_BY_HANDLERS,
)
from ..models.results import ContextEntry


# =================================================================
# USED BY: Class-level incoming references
# =================================================================


def build_class_used_by(
    runner: QueryRunner,
    start_id: str,
    max_depth: int,
    limit: int,
    include_impl: bool = False,
) -> list[ContextEntry]:
    """Build the USED BY tree for a Class node.

    Two-pass approach matching kloc-cli:
    1. Fetch all data (Q1-Q6)
    2. Classify edges, dispatch to handlers, build entries
    3. Expand depth-2+ for chainable entries

    Args:
        runner: QueryRunner instance.
        start_id: Target class node ID.
        max_depth: Maximum depth for tree expansion.
        limit: Maximum number of entries.
        include_impl: If True, include interface implementation usages.

    Returns:
        List of ContextEntry objects (USED BY tree).
    """
    # Step 1: Batch-fetch all data
    data = fetch_class_used_by_data(runner, start_id)

    # Get target node info
    target_record = runner.execute_single(
        "MATCH (n:Node {node_id: $id}) RETURN n.kind AS kind, n.fqn AS fqn, n.name AS name",
        id=start_id,
    )
    if not target_record:
        return []

    target_kind = target_record["kind"]
    target_fqn = target_record["fqn"]
    target_name = target_record["name"]

    # Step 2: Build extends/implements entries from Q1
    extends_entries = _build_extends_entries(data["extends_children"])

    # Step 3: Build prop type index for PropertyTypeHandler
    prop_type_index = {}
    for pt in data["prop_types"]:
        prop_type_index[pt["prop_id"]] = pt

    # Step 4: Build call node index keyed by (source_id) for matching
    call_index = _build_call_index(data["call_nodes"])

    # Step 5: Build EdgeContexts and dispatch to handlers
    bucket = EntryBucket()
    injection_classes = frozenset(data["injection_classes"])

    # Build visited_sources from extends entries -- matching kloc-cli's behavior
    # where extends/implements sources are added to visited_sources first,
    # preventing their USES edges from generating duplicate entries.
    visited_sources: set[str] = {start_id}
    for ext_entry in extends_entries:
        visited_sources.add(ext_entry["node_id"])

    for edge in data["usage_edges"]:
        source_id = edge["source_id"]

        # Skip sources already tracked via extends/implements (prevents
        # duplicate type_hint entries for classes that extend the target)
        if source_id in visited_sources:
            continue

        # R3: Skip internal self-references
        if edge["containing_class_id"] == start_id:
            continue
        # Also check containment for nodes without pre-resolved class
        if not edge["containing_class_id"]:
            if is_internal_reference(runner, start_id, source_id):
                continue

        # Skip extends/implements edges (handled by Q1)
        if edge["edge_type"] in ("EXTENDS", "IMPLEMENTS"):
            continue

        # Use per-edge target info (the actual edge target, which may be a member)
        edge_target_kind = edge.get("target_kind") or target_kind
        edge_target_fqn = edge.get("target_fqn") or target_fqn
        edge_target_name = edge.get("target_name") or target_name

        # Match with call node for authoritative ref_type (only for USES edges)
        edge_target_id = edge.get("target_id")
        call_data = None
        if edge["edge_type"] == "USES":
            call_data = _find_matching_call(
                call_index, source_id, edge["edge_file"], edge["edge_line"],
                edge_target_id=edge_target_id, class_id=start_id,
            )

        # Classify reference type
        ref_type = _classify_edge(
            runner, edge, start_id, edge_target_kind, edge_target_fqn, call_data
        )

        # Resolve property data for PropertyTypeHandler
        resolved_prop = _resolve_property_for_edge(
            edge, ref_type, prop_type_index
        )

        # Resolve on_expr / on_kind from call data
        on_expr = None
        on_kind = None
        if call_data:
            on_expr, on_kind = _resolve_receiver_from_call(call_data)

        # Build arguments if we have a call node
        arguments = []
        if call_data and call_data.get("call_id"):
            from ..db.queries.context_class import fetch_call_arguments
            raw_args = fetch_call_arguments(runner, call_data["call_id"])
            for arg in raw_args:
                param_fqn = arg.get("param_fqn") or ""
                value_expr = arg.get("arg_expression") or arg.get("value_expr") or ""
                # Extract param_name from param_fqn (e.g., App\..::__construct().$code -> $code)
                param_name = ""
                if param_fqn and ".$" in param_fqn:
                    param_name = "$" + param_fqn.split(".$")[-1]
                if param_fqn and value_expr:
                    arguments.append({
                        "param_fqn": param_fqn,
                        "param_name": param_name,
                        "value_expr": value_expr,
                    })

        ctx = EdgeContext(
            start_id=start_id,
            source_id=source_id,
            source_kind=edge["source_kind"],
            source_fqn=edge["source_fqn"],
            source_name=edge["source_name"],
            source_file=edge["source_file"],
            source_start_line=edge["source_start_line"],
            source_signature=edge.get("source_signature"),
            target_kind=edge_target_kind,
            target_fqn=edge_target_fqn,
            target_name=edge_target_name,
            ref_type=ref_type,
            file=edge["edge_file"],
            line=edge["edge_line"],
            call_node_id=call_data["call_id"] if call_data else None,
            classes_with_injection=injection_classes,
            containing_method_id=edge["containing_method_id"],
            containing_method_fqn=edge["containing_method_fqn"],
            containing_method_kind=edge.get("containing_method_kind"),
            containing_class_id=edge["containing_class_id"],
            containing_class_fqn=edge.get("containing_class_fqn"),
            containing_class_kind=edge.get("containing_class_kind"),
            containing_class_file=edge.get("containing_class_file"),
            containing_class_start_line=edge.get("containing_class_start_line"),
            resolved_property_id=resolved_prop.get("prop_id") if resolved_prop else None,
            resolved_property_fqn=resolved_prop.get("prop_fqn") if resolved_prop else None,
            resolved_property_file=resolved_prop.get("prop_file") if resolved_prop else None,
            resolved_property_line=resolved_prop.get("prop_start_line") if resolved_prop else None,
            on_expr=on_expr,
            on_kind=on_kind,
            arguments=arguments,
        )

        handler = USED_BY_HANDLERS.get(ref_type)
        if handler:
            handler.handle(ctx, bucket)

    # Step 6: Build property access entries from groups
    property_access_entries = _build_property_access_entries(bucket.property_access_groups, max_depth)

    # Step 7: Sort each bucket by (file, line) before combining
    def _sort_key(e):
        f = e.get("file", "") or ""
        ln = e.get("line")
        return (f, ln if ln is not None else 0)

    extends_entries.sort(key=_sort_key)
    bucket.extends.sort(key=_sort_key)
    bucket.instantiation.sort(key=_sort_key)
    bucket.property_type.sort(key=_sort_key)
    bucket.method_call.sort(key=_sort_key)
    property_access_entries.sort(key=_sort_key)
    bucket.param_return.sort(key=lambda e: (
        REF_TYPE_PRIORITY.get(e.get("ref_type", ""), 10),
        e.get("file", "") or "",
        e.get("line") if e.get("line") is not None else 0
    ))

    # Combine buckets in priority order
    all_entries_dicts = (
        bucket.extends
        + bucket.instantiation
        + bucket.property_type
        + bucket.method_call
        + property_access_entries
        + bucket.param_return
    )

    # Prepend extends/implements entries from Q1
    all_dicts = extends_entries + all_entries_dicts

    # Step 8: Convert dicts to ContextEntry objects
    entries = [_dict_to_context_entry(d) for d in all_dicts]

    # Step 9: Depth expansion
    if max_depth >= 2:
        for entry in entries:
            _expand_depth(runner, entry, 2, max_depth, start_id, data)

    return entries[:limit]


def _build_extends_entries(extends_children: list[dict]) -> list[dict]:
    """Build extends/implements entries from Q1 results."""
    entries = []
    for child in extends_children:
        ref_type = "extends" if child["rel_type"] == "EXTENDS" else "implements"
        entries.append({
            "depth": 1,
            "node_id": child["id"],
            "fqn": child["fqn"],
            "kind": child["kind"],
            "file": child["file"],
            "line": child["start_line"],
            "ref_type": ref_type,
            "children": [],
        })
    return entries


def _build_call_index(call_nodes: list[dict]) -> dict[str, list[dict]]:
    """Index call nodes by source_id for matching."""
    index: dict[str, list[dict]] = {}
    for call in call_nodes:
        source_id = call["source_id"]
        if source_id not in index:
            index[source_id] = []
        index[source_id].append(call)
    return index


def _find_matching_call(
    call_index: dict[str, list[dict]],
    source_id: str,
    edge_file: Optional[str],
    edge_line: Optional[int],
    edge_target_id: Optional[str] = None,
    class_id: Optional[str] = None,
) -> Optional[dict]:
    """Find the Call node matching an edge by source + location + target.

    Args:
        call_index: Index of call nodes keyed by source_id.
        source_id: Source node ID.
        edge_file: File path from the edge.
        edge_line: Line number from the edge (0-based).
        edge_target_id: The target node ID of the USES edge.
        class_id: The class node ID (start_id) for constructor matching.
    """
    calls = call_index.get(source_id, [])
    if not calls:
        return None

    # Try exact location match first, with target validation
    if edge_file and edge_line is not None:
        for call in calls:
            call_line = call.get("call_start_line")
            if call_line is not None and call_line == edge_line:
                if _call_matches_edge_target(call, edge_target_id, class_id):
                    return call

        # Try +/-1 line tolerance for constructor calls
        for call in calls:
            call_line = call.get("call_start_line")
            if (call_line is not None
                    and abs(call_line - edge_line) <= 1
                    and call.get("call_kind") == "constructor"):
                if _call_matches_edge_target(call, edge_target_id, class_id):
                    return call

    # Fall back: find any call from this source that targets the edge target
    if edge_target_id:
        for call in calls:
            if _call_matches_edge_target(call, edge_target_id, class_id):
                return call

    # Last resort: first call
    return calls[0] if calls else None


def _call_matches_edge_target(
    call: dict,
    edge_target_id: Optional[str],
    class_id: Optional[str],
) -> bool:
    """Check if a call node's callee matches the expected edge target.

    Handles constructor special case: Call targets __construct() but
    edge targets the Class node.
    """
    if not edge_target_id:
        return True  # No target to validate against

    callee_id = call.get("callee_id")
    if callee_id == edge_target_id:
        return True

    # Constructor special case: edge targets the Class, but call targets __construct()
    # which is contained by that class
    if call.get("call_kind") == "constructor" and class_id:
        if edge_target_id == class_id:
            return True

    return False


def _classify_edge(
    runner: QueryRunner,
    edge: dict,
    start_id: str,
    target_kind: str,
    target_fqn: str,
    call_data: Optional[dict],
) -> str:
    """Classify the reference type for an edge."""
    edge_type = edge["edge_type"].lower()

    # Use Call node classification for authoritative ref_type, but only for
    # USES edges (not TYPE_HINT). For class-target USES edges, only use Call
    # node if it's a constructor call -- other call kinds (static_call, method)
    # on the same line are coincidental, not the reference we're classifying.
    if call_data and call_data.get("call_kind") and edge_type == "uses":
        call_kind = call_data["call_kind"]
        is_class_target = target_kind in ("Class", "Interface", "Trait", "Enum")
        if not is_class_target or call_kind == "constructor":
            ref_type = get_reference_type_from_call_kind(call_kind)
            if ref_type != "unknown":
                return ref_type
    source_kind = edge["source_kind"]
    source_name = edge["source_name"]

    # Check type hint booleans (expensive -- only for 'uses' edges to class targets)
    has_arg = False
    has_return = False
    has_class_prop = False

    if (edge_type == "uses"
            and target_kind in ("Class", "Interface", "Trait", "Enum")
            and source_kind in ("Method", "Function", "Property", "Argument",
                                "Class", "Interface", "Trait", "Enum")):
        from ..db.queries.context_class import check_type_hints
        hints = check_type_hints(runner, edge["source_id"], start_id)
        has_arg = hints["has_arg_type_hint"]
        has_return = hints["has_return_type_hint"]
        has_class_prop = hints["has_class_property_type_hint"]

    return infer_reference_type(
        edge_type=edge_type,
        target_kind=target_kind,
        source_kind=source_kind,
        source_name=source_name,
        has_arg_type_hint=has_arg,
        has_return_type_hint=has_return,
        has_class_property_type_hint=has_class_prop,
    )


def _resolve_property_for_edge(
    edge: dict,
    ref_type: str,
    prop_type_index: dict[str, dict],
) -> Optional[dict]:
    """Resolve property data for PropertyTypeHandler context."""
    if ref_type != "property_type":
        return None

    # If source is a Property, it IS the property
    if edge["source_kind"] == "Property":
        return {
            "prop_id": edge["source_id"],
            "prop_fqn": edge["source_fqn"],
            "prop_file": edge["source_file"],
            "prop_start_line": edge["source_start_line"],
        }

    # For Method/Function sources, find the property in the same containing class
    # that has a TYPE_HINT to the target. Match via containing_class_fqn prefix.
    containing_class_fqn = edge.get("containing_class_fqn", "")
    if containing_class_fqn:
        for prop_id, prop_data in prop_type_index.items():
            prop_fqn = prop_data.get("prop_fqn", "")
            # Property FQN starts with its class FQN (e.g., App\Foo::$bar starts with App\Foo)
            if prop_fqn.startswith(containing_class_fqn + "::"):
                return prop_data

    # Fallback: return first match (legacy behavior)
    for prop_id, prop_data in prop_type_index.items():
        return prop_data

    return None


def _resolve_receiver_from_call(call_data: dict) -> tuple[Optional[str], Optional[str]]:
    """Extract on_expr and on_kind from call data."""
    on_expr = None
    on_kind = None

    access_chain_symbol = call_data.get("access_chain_symbol")
    if access_chain_symbol:
        # Extract property name from FQN: App\Foo::$bar -> $this->bar
        if "::$" in access_chain_symbol:
            prop_name = access_chain_symbol.split("::$")[-1]
            on_expr = f"$this->{prop_name}"
            on_kind = "property"
        elif "::" in access_chain_symbol:
            on_expr = access_chain_symbol
            on_kind = "static"
    elif call_data.get("recv_value_kind"):
        recv_kind = call_data["recv_value_kind"]
        recv_name = call_data.get("recv_name")
        if recv_kind == "parameter":
            on_expr = recv_name
            on_kind = "param"
        elif recv_kind == "local":
            on_expr = recv_name
            on_kind = "local"
        elif recv_kind in ("this", "self"):
            on_kind = "self"
    else:
        # Default to self for method calls without explicit receiver
        call_kind = call_data.get("call_kind", "")
        if call_kind in ("method", "access"):
            on_kind = "self"

    return on_expr, on_kind


def _resolve_arg_value_expr(arg: dict) -> str:
    """Resolve the best human-readable expression for an argument value.

    Handles:
    - Literal values: "(literal)" -> use arg_expression if available
    - Parameter/local values: "$paramName" -> use as-is
    - Result values: resolve through access chain if available
    """
    value_expr = arg.get("value_expr") or ""
    value_source = arg.get("value_source") or ""
    arg_expression = arg.get("arg_expression")

    # Use explicit expression from edge if available
    if arg_expression:
        return arg_expression

    # For result values, try to build an access chain expression
    if value_source == "result":
        chain_fqn = arg.get("access_chain_fqn")
        if chain_fqn:
            # Convert FQN like App\Dto\AddressInput::$countryCode
            # to something like $input->address->countryCode
            if "::$" in chain_fqn:
                prop = "$" + chain_fqn.split("::$")[-1]
                return prop
        # Fallback for result: show raw value
        return value_expr

    # For literal values, strip parentheses from name
    if value_source == "literal" and value_expr.startswith("(") and value_expr.endswith(")"):
        inner = value_expr[1:-1]
        return inner if inner else value_expr

    return value_expr


def _build_property_access_entries(groups: dict[str, list[dict]], max_depth: int = 1) -> list[dict]:
    """Convert property_access_groups to PropertyGroup entry dicts.

    Matches kloc-cli's format:
    - PropertyGroup kind with short FQN (ClassName::$prop)
    - file=None, line=None at depth 1
    - accessCount, methodCount
    - depth-2 children show per-method breakdown
    """
    entries = []
    for prop_fqn, group_entries in groups.items():
        total_accesses = sum(len(g["lines"]) for g in group_entries)
        total_methods = len(group_entries)

        # Short FQN for display: ClassName::$prop
        if "::" in prop_fqn:
            class_short = prop_fqn.split("::")[0].split("\\")[-1]
            prop_short = prop_fqn.split("::")[-1]
            display_fqn = f"{class_short}::{prop_short}"
        else:
            display_fqn = prop_fqn

        # Build depth-2 children (per-method breakdown)
        method_children = []
        if max_depth >= 2:
            for group in group_entries:
                method_fqn = group["method_fqn"]
                method_short = method_fqn.split("::")[-1] if "::" in method_fqn else method_fqn
                method_kind = group["method_kind"]
                if method_kind == "Method" and not method_short.endswith("()"):
                    method_short += "()"
                class_part = method_fqn.split("::")[0].split("\\")[-1] if "::" in method_fqn else ""
                child_display = f"{class_part}::{method_short}" if class_part else method_short

                count = len(group["lines"])
                lines_sorted = sorted(l for l in group["lines"] if l is not None)
                first_line = lines_sorted[0] if lines_sorted else None

                sites = None
                if count > 1 and lines_sorted:
                    sites = [{"line": l} for l in lines_sorted]

                child_entry = {
                    "depth": 2,
                    "node_id": group["method_id"],
                    "fqn": child_display,
                    "kind": method_kind,
                    "file": group["file"],
                    "line": first_line,
                    "ref_type": "property_access",
                    "on": group["on_expr"],
                    "on_kind": group["on_kind"],
                    "sites": sites,
                    "children": [],
                }
                method_children.append(child_entry)

            method_children.sort(key=lambda e: (e.get("file") or "", e.get("line") if e.get("line") is not None else 0))

        entry = {
            "depth": 1,
            "node_id": prop_fqn,
            "fqn": display_fqn,
            "kind": "PropertyGroup",
            "file": None,
            "line": None,
            "ref_type": "property_access",
            "access_count": total_accesses,
            "method_count": total_methods,
            "children": method_children,
        }
        entries.append(entry)
    return entries


def _dict_to_context_entry(d: dict) -> ContextEntry:
    """Convert a handler dict to a ContextEntry dataclass."""
    from ..models.results import ArgumentInfo

    # Convert argument dicts to ArgumentInfo objects
    raw_args = d.get("arguments", [])
    arg_infos = []
    for i, arg in enumerate(raw_args):
        if isinstance(arg, dict):
            arg_infos.append(ArgumentInfo(
                position=i,
                param_name=arg.get("param_name"),
                param_fqn=arg.get("param_fqn"),
                value_expr=arg.get("value_expr"),
            ))
        elif isinstance(arg, ArgumentInfo):
            arg_infos.append(arg)

    return ContextEntry(
        depth=d.get("depth", 1),
        node_id=d.get("node_id", ""),
        fqn=d.get("fqn", ""),
        kind=d.get("kind"),
        file=d.get("file"),
        line=d.get("line"),
        signature=d.get("signature"),
        ref_type=d.get("ref_type"),
        callee=d.get("callee"),
        on=d.get("on"),
        on_kind=d.get("on_kind"),
        children=[_dict_to_context_entry(c) for c in d.get("children", [])],
        sites=d.get("sites"),
        property_name=d.get("property_name"),
        access_count=d.get("access_count"),
        method_count=d.get("method_count"),
        crossed_from=d.get("crossed_from"),
        via=d.get("via"),
        arguments=arg_infos,
    )


def _expand_depth(
    runner: QueryRunner,
    entry: ContextEntry,
    depth: int,
    max_depth: int,
    start_id: str,
    data: dict,
) -> None:
    """Expand a depth-1 entry with children at depth 2+.

    Handles different expansion strategies based on ref_type.
    """
    if depth > max_depth:
        return

    ref_type = entry.ref_type

    if ref_type in ("extends", "implements"):
        # Show override methods in the subclass
        entry.children = _build_override_children(
            runner, entry.node_id, start_id, depth, max_depth
        )

    elif ref_type == "property_type":
        # Show method calls through the injected property
        entry.children = _build_injection_point_children(
            runner, entry.node_id, entry.fqn, depth, max_depth
        )

    elif ref_type == "instantiation":
        # Find callers of the containing method (caller chain)
        method_id = entry.node_id
        if method_id:
            entry.children = build_caller_chain_for_method(
                runner, method_id, depth, max_depth
            )


def _build_override_children(
    runner: QueryRunner,
    subclass_id: str,
    parent_class_id: str,
    depth: int,
    max_depth: int,
) -> list[ContextEntry]:
    """Build override method entries for a subclass at depth 2."""
    if depth > max_depth:
        return []

    overrides = fetch_override_methods(runner, subclass_id)
    entries = []

    for method in overrides:
        sig = method.get("method_signature")
        if not sig and method.get("method_documentation"):
            sig = _extract_signature_from_doc(method["method_documentation"])

        entry = ContextEntry(
            depth=depth,
            node_id=method["method_id"],
            fqn=method["method_fqn"],
            kind="Method",
            file=method["method_file"],
            line=method["method_start_line"],
            signature=sig,
            ref_type="override",
            children=[],
        )

        # Depth 3: show internal actions of the override
        if depth < max_depth:
            entry.children = _build_override_internals(
                runner, method["method_id"], depth + 1, max_depth
            )

        entries.append(entry)

    entries.sort(key=lambda e: (e.file or "", e.line if e.line is not None else 0))
    return entries


def _build_override_internals(
    runner: QueryRunner,
    method_id: str,
    depth: int,
    max_depth: int,
) -> list[ContextEntry]:
    """Show internal actions of an override method (method calls, property access)."""
    if depth > max_depth:
        return []

    internals = fetch_override_internals(runner, method_id)
    entries = []

    for action in internals:
        ref_type = get_reference_type_from_call_kind(action.get("call_kind"))
        if ref_type == "unknown":
            ref_type = "method_call"

        call_line = action.get("call_start_line")

        on_expr = None
        on_kind = None
        if action.get("on_property"):
            prop_name = action["on_property"].split("::$")[-1] if "::$" in action["on_property"] else action["on_property"]
            on_expr = f"$this->{prop_name}"
            on_kind = "property"

        entry = ContextEntry(
            depth=depth,
            node_id=action["target_id"],
            fqn=action["target_fqn"],
            kind=action["target_kind"],
            file=action["call_file"],
            line=call_line,
            ref_type=ref_type,
            on=on_expr,
            on_kind=on_kind,
            children=[],
        )
        entries.append(entry)

    entries.sort(key=lambda e: (e.file or "", e.line if e.line is not None else 0))
    return entries


def _build_injection_point_children(
    runner: QueryRunner,
    property_id: str,
    property_fqn: str,
    depth: int,
    max_depth: int,
) -> list[ContextEntry]:
    """Build method call entries under a property_type injection point."""
    if depth > max_depth:
        return []

    calls = fetch_injection_point_calls(runner, property_id, property_fqn)
    if not calls:
        return []

    # Group by callee FQN for dedup
    entries_by_fqn: dict[str, ContextEntry] = {}
    entries: list[ContextEntry] = []

    # Resolve the containing class of the property for crossed_from
    cls_record = runner.execute_single(
        """
        MATCH (prop:Node {node_id: $pid})<-[:CONTAINS*1..10]-(cls:Node)
        WHERE cls.kind IN ['Class', 'Interface', 'Trait', 'Enum']
        RETURN cls.fqn AS cls_fqn LIMIT 1
        """,
        pid=property_id,
    )
    crossed_from_fqn = cls_record["cls_fqn"] if cls_record else None

    for call in calls:
        callee_fqn = call["callee_fqn"]
        callee_name = call["callee_name"]
        if call["callee_kind"] == "Method":
            callee_name = f"{callee_name}()"

        call_line = call.get("call_start_line")

        if callee_fqn in entries_by_fqn:
            existing = entries_by_fqn[callee_fqn]
            if existing.sites is None:
                existing.sites = [{"method": call.get("_first_method_name", ""), "line": existing.line}]
                existing.line = None
            existing.sites.append({"method": call["method_name"], "line": call_line})
            continue

        # Resolve on expression from property FQN
        on_expr = None
        if property_fqn and "::$" in property_fqn:
            prop_name = property_fqn.split("::$")[-1]
            on_expr = f"$this->{prop_name}"

        entry = ContextEntry(
            depth=depth,
            node_id=call["callee_id"],
            fqn=callee_fqn,
            kind=call["callee_kind"],
            file=call.get("call_file"),
            line=call_line,
            ref_type="method_call",
            callee=callee_name,
            on=on_expr,
            on_kind="property",
            children=[],
            crossed_from=crossed_from_fqn,
        )

        # Store method name for potential sites merging
        entry._first_method_name = call["method_name"]  # type: ignore[attr-defined]

        # Depth 3: show callers of the method containing this call
        if depth < max_depth and call.get("method_id"):
            callers = build_caller_chain_for_method(
                runner, call["method_id"], depth + 1, max_depth
            )
            if callers:
                entry.children = callers
            else:
                # Terminal: show the containing method as a caller
                method_fqn = call["method_fqn"]
                if not method_fqn.endswith("()"):
                    method_fqn += "()"
                entry.children = [ContextEntry(
                    depth=depth + 1,
                    node_id=call["method_id"],
                    fqn=method_fqn,
                    kind="Method",
                    file=None,
                    line=None,
                    ref_type="caller",
                    children=[],
                )]

        entries.append(entry)
        entries_by_fqn[callee_fqn] = entry

    entries.sort(key=lambda e: (e.file or "", e.line if e.line is not None else 0))
    return entries


# =================================================================
# Caller Chain
# =================================================================


def build_caller_chain_for_method(
    runner: QueryRunner,
    method_id: str,
    depth: int,
    max_depth: int,
    visited: Optional[set] = None,
) -> list[ContextEntry]:
    """Build caller entries for a method at the given depth.

    Args:
        runner: QueryRunner instance.
        method_id: Method node ID to find callers of.
        depth: Current depth level.
        max_depth: Maximum depth.
        visited: Set of already-visited method IDs for cycle prevention.

    Returns:
        List of ContextEntry objects representing callers.
    """
    if depth > max_depth:
        return []

    if visited is None:
        visited = set()

    callers = fetch_caller_chain(runner, method_id)
    entries = []

    for caller in callers:
        caller_id = caller["caller_id"]
        if caller_id in visited:
            continue
        visited.add(caller_id)

        caller_fqn = caller["caller_fqn"]
        if caller["caller_kind"] == "Method" and not caller_fqn.endswith("()"):
            caller_fqn += "()"

        call_line = caller.get("call_line")

        # Determine on_expr and on_kind
        on_expr = None
        on_kind = None
        if caller.get("on_property"):
            prop_fqn = caller["on_property"]
            if "::$" in prop_fqn:
                prop_name = prop_fqn.split("::$")[-1]
                on_expr = f"$this->{prop_name}"
            else:
                on_expr = prop_fqn
            on_kind = "property"
        else:
            # Check call_kind for self calls
            call_kind = caller.get("call_kind")
            if call_kind == "method":
                on_expr = "$this"
                on_kind = "self"

        # ref_type: use "caller" for depth 3+, "method_call" for depth 2
        entry_ref_type = "caller" if depth >= 3 else "method_call"

        # Resolve crossed_from: the method being called
        method_record = runner.execute_single(
            "MATCH (n:Node {node_id: $id}) RETURN n.fqn AS fqn, n.name AS name",
            id=method_id,
        )
        crossed_from = method_record["fqn"] if method_record else None
        # Callee: the short name of the method being expanded
        callee_short = None
        if method_record:
            mn = method_record["name"]
            callee_short = f"{mn}()" if mn else None

        # Fetch arguments for this caller chain call
        caller_arguments = []
        if caller.get("call_id"):
            from ..db.queries.context_class import fetch_call_arguments
            raw_args = fetch_call_arguments(runner, caller["call_id"])
            for arg in raw_args:
                param_fqn = arg.get("param_fqn") or ""
                value_expr = arg.get("arg_expression") or arg.get("value_expr") or ""
                param_name = ""
                if param_fqn and ".$" in param_fqn:
                    param_name = "$" + param_fqn.split(".$")[-1]
                if param_fqn and value_expr:
                    caller_arguments.append({
                        "param_fqn": param_fqn,
                        "param_name": param_name,
                        "value_expr": value_expr,
                    })

        # Convert caller_arguments to ArgumentInfo
        from ..models.results import ArgumentInfo
        arg_infos = [
            ArgumentInfo(
                position=i,
                param_name=a.get("param_name"),
                param_fqn=a.get("param_fqn"),
                value_expr=a.get("value_expr"),
            )
            for i, a in enumerate(caller_arguments)
        ]

        entry = ContextEntry(
            depth=depth,
            node_id=caller_id,
            fqn=caller_fqn,
            kind=caller["caller_kind"],
            file=caller["caller_file"],
            line=call_line,
            ref_type=entry_ref_type,
            callee=callee_short,
            on=on_expr,
            on_kind=on_kind,
            children=[],
            crossed_from=crossed_from,
            arguments=arg_infos,
        )

        # Recursive depth expansion
        if depth < max_depth:
            entry.children = build_caller_chain_for_method(
                runner, caller_id, depth + 1, max_depth, visited
            )

        entries.append(entry)

    entries.sort(key=lambda e: (e.file or "", e.line if e.line is not None else 0))
    return entries


# =================================================================
# USES: Class-level outgoing dependencies
# =================================================================


def build_class_uses(
    runner: QueryRunner,
    start_id: str,
    max_depth: int,
    limit: int,
    include_impl: bool = False,
) -> list[ContextEntry]:
    """Build the USES tree for a Class node.

    Shows one entry per unique external dependency class/interface.
    Classifies each as [extends], [implements], [property_type],
    [parameter_type], [return_type], [instantiation].

    Args:
        runner: QueryRunner instance.
        start_id: Target class node ID.
        max_depth: Maximum depth for expansion.
        limit: Maximum entries.
        include_impl: Include implementation usages.

    Returns:
        List of ContextEntry objects (USES tree).
    """
    # Fetch all data
    uses_data = fetch_class_uses_data(runner, start_id)

    # Get the class node info
    cls_record = runner.execute_single(
        "MATCH (n:Node {node_id: $id}) RETURN n.file AS file, n.start_line AS start_line",
        id=start_id,
    )
    cls_file = cls_record["file"] if cls_record else None
    cls_line = cls_record["start_line"] if cls_record else None

    # Track targets already seen
    visited: set[str] = {start_id}

    # target_info: target_id -> best entry info
    target_info: dict[str, dict] = {}

    # Step 1: Process class-level relationships (extends, implements, uses_trait)
    for rel in uses_data["class_rels"]:
        target_id = rel["target_id"]
        if target_id == start_id or target_id in target_info:
            continue

        rel_type_map = {
            "EXTENDS": "extends",
            "IMPLEMENTS": "implements",
            "USES_TRAIT": "uses_trait",
        }
        ref_type = rel_type_map.get(rel["rel_type"], "type_hint")

        target_info[target_id] = {
            "ref_type": ref_type,
            "file": cls_file,
            "line": cls_line,
            "property_name": None,
            "target_fqn": rel["target_fqn"],
            "target_kind": rel["target_kind"],
        }

    # Step 2: Build type_hint classification
    type_hint_info: dict[str, dict] = {}
    for hint in uses_data["type_hints"]:
        tid = hint["target_id"]
        if tid == start_id:
            continue

        source_kind = hint.get("source_kind", "")
        member_kind = hint.get("member_kind", "")

        if member_kind == "Property" or source_kind == "Property":
            th_ref = "property_type"
            prop_name = hint.get("source_name", "")
            if prop_name and not prop_name.startswith("$"):
                prop_name = f"${prop_name}"
        elif source_kind == "Argument" or member_kind == "Argument":
            th_ref = "parameter_type"
            prop_name = None
        else:
            th_ref = "return_type"
            prop_name = None

        existing = type_hint_info.get(tid)
        # Priority: property_type > parameter_type > return_type
        th_priority = {"property_type": 0, "parameter_type": 1, "return_type": 2}
        if not existing or th_priority.get(th_ref, 10) < th_priority.get(existing.get("ref_type", ""), 10):
            type_hint_info[tid] = {
                "ref_type": th_ref,
                "property_name": prop_name,
                "file": hint.get("file"),
                "line": hint.get("line"),
            }

    # Step 3: Build instantiation targets
    instantiation_targets: dict[str, dict] = {}
    for inst in uses_data["instantiation_targets"]:
        tid = inst["target_id"]
        if tid not in instantiation_targets:
            instantiation_targets[tid] = {
                "file": inst.get("file"),
                "line": inst.get("line"),
            }

    # Step 4: Process member deps -- resolve to containing class, classify, dedup
    for dep in uses_data["member_deps"]:
        target_id = dep["target_id"]
        target_kind = dep["target_kind"]

        # Resolve member references to their containing class
        resolved_id = target_id
        resolved_fqn = dep["target_fqn"]
        resolved_kind = target_kind

        if target_kind in ("Method", "Property", "Argument", "Value", "Call", "Constant"):
            parent_record = runner.execute_single(
                """
                MATCH (n:Node {node_id: $id})<-[:CONTAINS]-(parent:Node)
                WHERE parent.kind IN ['Class', 'Interface', 'Trait', 'Enum']
                RETURN parent.node_id AS pid, parent.fqn AS pfqn, parent.kind AS pkind
                """,
                id=target_id,
            )
            if parent_record:
                resolved_id = parent_record["pid"]
                resolved_fqn = parent_record["pfqn"]
                resolved_kind = parent_record["pkind"]
            else:
                continue

        if resolved_id == start_id or resolved_id in visited:
            continue

        # Skip already-tracked extends/implements
        if resolved_id in target_info and target_info[resolved_id]["ref_type"] in ("extends", "implements"):
            continue

        file = dep.get("file") or dep.get("target_file")
        line = dep.get("line") or dep.get("target_start_line")

        # Classify reference
        ref_type = None
        property_name = None

        # Check type_hint-based classification
        if resolved_id in type_hint_info:
            th_info = type_hint_info[resolved_id]
            th_ref = th_info["ref_type"]
            if th_ref in ("property_type", "return_type"):
                ref_type = th_ref
                property_name = th_info.get("property_name")
                file = th_info.get("file") or file
                line = th_info.get("line") if th_info.get("line") is not None else line

        # Check instantiation
        if ref_type is None and resolved_id in instantiation_targets:
            inst_info = instantiation_targets[resolved_id]
            ref_type = "instantiation"
            file = inst_info.get("file") or file
            line = inst_info.get("line") if inst_info.get("line") is not None else line

        # Fall back to remaining type_hint (parameter_type)
        if ref_type is None and resolved_id in type_hint_info:
            th_info = type_hint_info[resolved_id]
            ref_type = th_info["ref_type"]
            property_name = th_info.get("property_name")
            file = th_info.get("file") or file
            line = th_info.get("line") if th_info.get("line") is not None else line

        # Fall back to edge-level inference
        if ref_type is None:
            ref_type = infer_reference_type(
                edge_type="uses",
                target_kind=resolved_kind,
                source_kind=dep.get("member_kind"),
            )

        # Priority dedup
        priority_map = {
            "instantiation": 0,
            "property_type": 1,
            "method_call": 2,
            "property_access": 2,
            "parameter_type": 3,
            "return_type": 4,
            "type_hint": 5,
        }

        if resolved_id in target_info:
            existing = target_info[resolved_id]
            existing_priority = priority_map.get(existing["ref_type"], 10)
            new_priority = priority_map.get(ref_type, 10)
            if new_priority < existing_priority:
                target_info[resolved_id] = {
                    "ref_type": ref_type,
                    "file": file or existing.get("file"),
                    "line": line if line is not None else existing.get("line"),
                    "property_name": property_name or existing.get("property_name"),
                    "target_fqn": resolved_fqn,
                    "target_kind": resolved_kind,
                }
            elif property_name and not existing.get("property_name"):
                existing["property_name"] = property_name
        else:
            target_info[resolved_id] = {
                "ref_type": ref_type,
                "file": file,
                "line": line,
                "property_name": property_name,
                "target_fqn": resolved_fqn,
                "target_kind": resolved_kind,
            }

    # Step 5: Ensure type_hint targets not reached via uses edges are included
    for tid, th_info in type_hint_info.items():
        if tid in target_info or tid == start_id:
            continue
        # Fetch target node info
        target_record = runner.execute_single(
            "MATCH (n:Node {node_id: $id}) WHERE n.kind IN ['Class', 'Interface', 'Trait', 'Enum'] RETURN n.fqn AS fqn, n.kind AS kind, n.file AS file, n.start_line AS start_line",
            id=tid,
        )
        if not target_record:
            continue
        target_info[tid] = {
            "ref_type": th_info["ref_type"],
            "file": th_info.get("file") or target_record["file"],
            "line": th_info.get("line") if th_info.get("line") is not None else target_record["start_line"],
            "property_name": th_info.get("property_name"),
            "target_fqn": target_record["fqn"],
            "target_kind": target_record["kind"],
        }

    # Step 6: Build entries
    entries: list[ContextEntry] = []
    for target_id, info in target_info.items():
        visited.add(target_id)

        entry = ContextEntry(
            depth=1,
            node_id=target_id,
            fqn=info["target_fqn"],
            kind=info["target_kind"],
            file=info.get("file"),
            line=info.get("line"),
            ref_type=info["ref_type"],
            property_name=info.get("property_name"),
            children=[],
        )

        # Depth 2 expansion
        if max_depth >= 2:
            ref_type = info["ref_type"]
            if ref_type == "extends":
                entry.children = _build_extends_depth2(
                    runner, start_id, target_id, 2, max_depth
                )
            elif ref_type == "implements":
                entry.children = _build_implements_depth2(
                    runner, start_id, target_id, 2, max_depth
                )
            elif ref_type == "property_type":
                prop_name = info.get("property_name")
                # Find the actual property FQN
                prop_fqn = _resolve_property_fqn_for_uses(
                    runner, start_id, target_id, prop_name
                )
                if prop_fqn:
                    entry.children = _build_behavioral_depth2_uses(
                        runner, start_id, target_id, prop_fqn, 2, max_depth
                    )
            else:
                # Pass only {start_id} as visited, matching kloc-cli behavior:
                # depth-2 children can repeat depth-1 entries
                entry.children = build_class_uses_recursive(
                    runner, target_id, 2, max_depth, limit, {start_id}
                )

        entries.append(entry)

    # Step 7: Sort by USES priority
    uses_priority = {
        "extends": 0,
        "implements": 0,
        "uses_trait": 0,
        "property_type": 1,
        "parameter_type": 2,
        "return_type": 2,
        "instantiation": 3,
        "type_hint": 4,
        "method_call": 5,
        "property_access": 5,
    }

    def sort_key(e: ContextEntry):
        pri = uses_priority.get(e.ref_type or "", 10)
        return (pri, e.file or "", e.line if e.line is not None else 0)

    entries.sort(key=sort_key)
    return entries[:limit]


def _resolve_property_fqn_for_uses(
    runner: QueryRunner,
    class_id: str,
    dep_class_id: str,
    prop_name: Optional[str],
) -> Optional[str]:
    """Find the property FQN in class_id that has type_hint to dep_class_id."""
    records = runner.execute(
        """
        MATCH (cls:Node {node_id: $cls_id})-[:CONTAINS]->(prop:Node {kind: 'Property'})
        MATCH (prop)-[:TYPE_HINT]->(target:Node {node_id: $target_id})
        RETURN prop.fqn AS prop_fqn
        LIMIT 1
        """,
        cls_id=class_id,
        target_id=dep_class_id,
    )
    if records:
        return records[0]["prop_fqn"]
    return None


def _build_extends_depth2(
    runner: QueryRunner,
    class_id: str,
    parent_id: str,
    depth: int,
    max_depth: int,
) -> list[ContextEntry]:
    """Build depth-2 for [extends]: show override and inherited methods."""
    if depth > max_depth:
        return []

    override_entries: list[ContextEntry] = []
    inherited_entries: list[ContextEntry] = []

    # Fetch parent's methods and check which ones are overridden
    records = runner.execute(
        """
        MATCH (parent:Node {node_id: $parent_id})-[:CONTAINS]->(pm:Node {kind: 'Method'})
        WHERE pm.name <> '__construct'
        OPTIONAL MATCH (child_cls:Node {node_id: $class_id})-[:CONTAINS]->(cm:Node {kind: 'Method'})-[:OVERRIDES]->(pm)
        RETURN pm.node_id AS parent_method_id, pm.fqn AS parent_method_fqn,
               pm.name AS parent_method_name, pm.file AS parent_method_file,
               pm.start_line AS parent_method_start_line,
               pm.signature AS parent_method_signature,
               pm.documentation AS parent_method_documentation,
               cm.node_id AS child_method_id, cm.fqn AS child_method_fqn,
               cm.file AS child_method_file, cm.start_line AS child_method_start_line,
               cm.signature AS child_method_signature,
               cm.documentation AS child_method_documentation
        ORDER BY pm.file, pm.start_line
        """,
        parent_id=parent_id,
        class_id=class_id,
    )

    for rec in records:
        parent_sig = rec.get("parent_method_signature")
        if not parent_sig and rec.get("parent_method_documentation"):
            parent_sig = _extract_signature_from_doc(rec["parent_method_documentation"])

        if rec["child_method_id"]:
            # Override
            child_sig = rec.get("child_method_signature")
            if not child_sig and rec.get("child_method_documentation"):
                child_sig = _extract_signature_from_doc(rec["child_method_documentation"])

            entry = ContextEntry(
                depth=depth,
                node_id=rec["child_method_id"],
                fqn=rec["child_method_fqn"],
                kind="Method",
                file=rec["child_method_file"],
                line=rec["child_method_start_line"],
                signature=child_sig,
                ref_type="override",
                children=[],
            )
            if depth < max_depth:
                entry.children = _build_override_internals(
                    runner, rec["child_method_id"], depth + 1, max_depth
                )
            override_entries.append(entry)
        else:
            # Inherited
            entry = ContextEntry(
                depth=depth,
                node_id=rec["parent_method_id"],
                fqn=rec["parent_method_fqn"],
                kind="Method",
                file=rec["parent_method_file"],
                line=rec["parent_method_start_line"],
                signature=parent_sig,
                ref_type="inherited",
                children=[],
            )
            if depth < max_depth:
                entry.children = _build_override_internals(
                    runner, rec["parent_method_id"], depth + 1, max_depth
                )
            inherited_entries.append(entry)

    return override_entries + inherited_entries


def _build_implements_depth2(
    runner: QueryRunner,
    class_id: str,
    interface_id: str,
    depth: int,
    max_depth: int,
) -> list[ContextEntry]:
    """Build depth-2 for [implements]: override methods and extends subclasses."""
    if depth > max_depth:
        return []

    override_entries: list[ContextEntry] = []

    # Find override methods in the implementing class
    overrides = fetch_override_methods(runner, class_id)
    for method in overrides:
        sig = method.get("method_signature")
        if not sig and method.get("method_documentation"):
            sig = _extract_signature_from_doc(method["method_documentation"])

        entry = ContextEntry(
            depth=depth,
            node_id=method["method_id"],
            fqn=method["method_fqn"],
            kind="Method",
            file=method["method_file"],
            line=method["method_start_line"],
            signature=sig,
            ref_type="override",
            children=[],
        )
        if depth < max_depth:
            entry.children = _build_override_internals(
                runner, method["method_id"], depth + 1, max_depth
            )
        override_entries.append(entry)

    # Find extends subclasses
    extends_entries: list[ContextEntry] = []
    _collect_extends_entries(runner, class_id, depth, max_depth, extends_entries, set())

    override_entries.sort(key=lambda e: (e.file or "", e.line if e.line is not None else 0))
    extends_entries.sort(key=lambda e: (e.file or "", e.line if e.line is not None else 0))
    return override_entries + extends_entries


def _collect_extends_entries(
    runner: QueryRunner,
    class_id: str,
    depth: int,
    max_depth: int,
    result: list[ContextEntry],
    visited: set[str],
) -> None:
    """Recursively collect extends entries for subclasses."""
    if class_id in visited:
        return
    visited.add(class_id)

    children = fetch_extends_children(runner, class_id)
    for child in children:
        child_id = child["id"]
        if child_id in visited:
            continue

        entry = ContextEntry(
            depth=depth,
            node_id=child_id,
            fqn=child["fqn"],
            kind=child["kind"],
            file=child["file"],
            line=child["start_line"],
            ref_type="extends",
            children=[],
        )

        result.append(entry)
        _collect_extends_entries(runner, child_id, depth, max_depth, result, visited)


def _build_behavioral_depth2_uses(
    runner: QueryRunner,
    class_id: str,
    dep_class_id: str,
    property_fqn: str,
    depth: int,
    max_depth: int,
) -> list[ContextEntry]:
    """Build behavioral depth-2 for property_type USES deps."""
    if depth > max_depth:
        return []

    calls = fetch_behavioral_depth2(runner, class_id, property_fqn)
    entries: list[ContextEntry] = []
    seen_callees: set[str] = set()

    for call in calls:
        callee_fqn = call["callee_fqn"]
        if callee_fqn in seen_callees:
            continue
        seen_callees.add(callee_fqn)

        callee_name = call["callee_name"]
        if call["callee_kind"] == "Method":
            callee_name = f"{callee_name}()"

        call_line = call.get("call_line")

        entry = ContextEntry(
            depth=depth,
            node_id=call["callee_id"],
            fqn=callee_fqn,
            kind=call["callee_kind"],
            file=call.get("call_file"),
            line=call_line,
            ref_type="method_call",
            callee=callee_name,
            on=None,
            on_kind="property",
            children=[],
        )
        entries.append(entry)

    entries.sort(key=lambda e: (e.file or "", e.line if e.line is not None else 0))
    return entries


def build_class_uses_recursive(
    runner: QueryRunner,
    target_id: str,
    depth: int,
    max_depth: int,
    limit: int,
    visited: set[str],
) -> list[ContextEntry]:
    """Recursive class-level expansion for non-property USES deps.

    Args:
        runner: QueryRunner instance.
        target_id: Target class node ID.
        depth: Current depth.
        max_depth: Maximum depth.
        limit: Maximum entries.
        visited: Already-visited node IDs.

    Returns:
        List of ContextEntry objects.
    """
    if depth > max_depth or target_id in visited:
        return []
    visited.add(target_id)

    # Check that target is a class-level node
    target_record = runner.execute_single(
        "MATCH (n:Node {node_id: $id}) WHERE n.kind IN ['Class', 'Interface', 'Trait', 'Enum'] RETURN n.kind AS kind",
        id=target_id,
    )
    if not target_record:
        return []

    deps = fetch_node_deps(runner, target_id)
    entries: list[ContextEntry] = []
    local_visited: set[str] = set()

    for dep in deps:
        dep_id = dep["target_id"]
        dep_kind = dep["target_kind"]

        # Resolve to containing class if needed
        resolved_id = dep_id
        resolved_fqn = dep["target_fqn"]
        resolved_kind = dep_kind

        if dep_kind in ("Method", "Property", "Argument", "Value", "Call"):
            parent_record = runner.execute_single(
                """
                MATCH (n:Node {node_id: $id})<-[:CONTAINS]-(parent:Node)
                WHERE parent.kind IN ['Class', 'Interface', 'Trait', 'Enum']
                RETURN parent.node_id AS pid, parent.fqn AS pfqn, parent.kind AS pkind
                """,
                id=dep_id,
            )
            if parent_record:
                resolved_id = parent_record["pid"]
                resolved_fqn = parent_record["pfqn"]
                resolved_kind = parent_record["pkind"]
            else:
                continue

        if resolved_id == target_id or resolved_id in local_visited or resolved_id in visited:
            continue
        local_visited.add(resolved_id)

        # Classify reference type
        edge_type = dep.get("edge_type", "uses")
        ref_type = infer_reference_type(
            edge_type=edge_type,
            target_kind=dep_kind,
            source_kind=dep.get("source_kind"),
        )

        # Resolve property name
        property_name = None
        prop_name_raw = dep.get("prop_name")
        if ref_type == "property_type" and prop_name_raw:
            property_name = prop_name_raw if prop_name_raw.startswith("$") else f"${prop_name_raw}"

        file = dep.get("target_file")
        line = dep.get("target_line")

        entry = ContextEntry(
            depth=depth,
            node_id=resolved_id,
            fqn=resolved_fqn,
            kind=resolved_kind,
            file=file,
            line=line,
            ref_type=ref_type,
            property_name=property_name,
            children=[],
        )

        # Recursive expansion
        if depth < max_depth and resolved_kind in ("Class", "Interface", "Trait", "Enum"):
            entry.children = build_class_uses_recursive(
                runner, resolved_id, depth + 1, max_depth, limit, visited | local_visited
            )

        entries.append(entry)

    # Sort by priority
    def sort_key(e: ContextEntry):
        pri = REF_TYPE_PRIORITY.get(e.ref_type or "", 10)
        return (pri, e.file or "", e.line if e.line is not None else 0)

    entries.sort(key=sort_key)
    return entries
