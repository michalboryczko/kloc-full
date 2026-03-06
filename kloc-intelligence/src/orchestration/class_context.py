"""Class context orchestrators: build USED BY and USES trees for Class nodes.

Ported from kloc-cli's class context logic. Each public function fetches
pre-resolved data from Neo4j (via batch query functions) then applies pure
logic (handlers, reference type inference, graph helpers) to produce lists
of ContextEntry objects.

Key design rules:
- All Neo4j access is via batch fetch functions — no per-entry round-trips.
- Handlers produce dicts; dicts are converted to ContextEntry at the end.
- Extends entries from Q1 are built directly, bypassing the handler pipeline.
- Injection suppression: method_call from a class with property_type injection
  is suppressed at depth 1 (MethodCallHandler already does this).
- All line numbers are stored 0-based; conversion to 1-based happens in OutputEntry.
"""

from ..db.query_runner import QueryRunner
from ..db.queries.context_class import (
    Q5_CALLER_CHAIN,
    Q7_INJECTION_POINT_CALLS,
    fetch_class_used_by_data,
)
from ..db.queries.context_class_uses import (
    Q3_BEHAVIORAL_DEPTH2,
    fetch_class_uses_data,
)
from ..logic.handlers import EdgeContext, EntryBucket, USED_BY_HANDLERS
from ..logic.reference_types import (
    infer_reference_type,
    CHAINABLE_REFERENCE_TYPES,
)
from ..logic.graph_helpers import format_method_fqn
from ..models.node import NodeData
from ..models.results import ContextEntry

# Priority order for USES entries
USES_PRIORITY: dict[str, int] = {
    "extends": 0,
    "implements": 1,
    "uses_trait": 2,
    "property_type": 3,
    "method_call": 4,
    "instantiation": 4,
    "property_access": 5,
    "parameter_type": 6,
    "return_type": 6,
    "type_hint": 7,
}


# =============================================================================
# Internal helpers
# =============================================================================


def _dict_to_context_entry(d: dict, depth: int | None = None) -> ContextEntry:
    """Convert a handler-produced dict to a ContextEntry.

    Handler dicts use snake_case keys and may omit optional fields.
    ContextEntry has sensible defaults for all optional fields.

    Args:
        d: Dict produced by a USED_BY handler or directly constructed.
        depth: Override depth (uses d["depth"] if None).

    Returns:
        ContextEntry populated from dict.
    """
    return ContextEntry(
        depth=depth if depth is not None else d.get("depth", 1),
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
        sites=d.get("sites"),
        property_name=d.get("property_name"),
        access_count=d.get("access_count"),
        method_count=d.get("method_count"),
        children=d.get("children", []),
    )


def _build_property_access_entries(
    bucket: EntryBucket,
    target_fqn: str,
    depth: int,
) -> list[ContextEntry]:
    """Convert property_access_groups in bucket to ContextEntry objects.

    Groups multiple accesses to the same property into a single entry with
    a 'sites' list.

    Args:
        bucket: EntryBucket with populated property_access_groups.
        target_fqn: FQN of the target class (for context, not used in output).
        depth: Depth to assign to generated entries.

    Returns:
        List of ContextEntry objects, one per (property_fqn, method, on_expr) group.
    """
    entries: list[ContextEntry] = []
    for prop_fqn, groups in bucket.property_access_groups.items():
        for g in groups:
            sites = [ln for ln in g["lines"] if ln is not None]
            entry = ContextEntry(
                depth=depth,
                node_id=g["method_id"],
                fqn=format_method_fqn(g["method_fqn"], g.get("method_kind", "")),
                kind=g.get("method_kind"),
                file=g.get("file"),
                line=sites[0] if sites else None,
                ref_type="property_access",
                on=g.get("on_expr"),
                on_kind=g.get("on_kind"),
                property_name=prop_fqn,
                access_count=len(sites),
                sites=sites if len(sites) > 1 else None,
            )
            entries.append(entry)
    return entries


def _build_caller_chain_entry(record: dict, depth: int) -> ContextEntry:
    """Build a single ContextEntry from a Q5 caller chain record."""
    fqn = format_method_fqn(record["caller_fqn"], record.get("caller_kind", ""))
    return ContextEntry(
        depth=depth,
        node_id=record["caller_id"],
        fqn=fqn,
        kind=record.get("caller_kind"),
        file=record.get("caller_file"),
        line=record.get("call_line"),
        ref_type="method_call",
    )


# =============================================================================
# build_caller_chain_for_method
# =============================================================================


def build_caller_chain_for_method(
    runner: QueryRunner,
    method_id: str,
    depth: int,
    max_depth: int,
    visited: set[str],
) -> list[ContextEntry]:
    """Recursively fetch callers of a method up to max_depth.

    Only CHAINABLE_REFERENCE_TYPES expand. Visited set prevents cycles.

    Args:
        runner: Active QueryRunner.
        method_id: Node ID of the method to find callers for.
        depth: Current depth in the expansion (starts at 2).
        max_depth: Maximum depth allowed.
        visited: Set of already-visited method node IDs (mutated in place).

    Returns:
        List of ContextEntry objects for callers, with recursive children.
    """
    if depth > max_depth:
        return []

    records = runner.execute(Q5_CALLER_CHAIN, method_id=method_id)
    entries: list[ContextEntry] = []

    for r in records:
        caller_id = r["caller_id"]
        if caller_id in visited:
            continue
        visited.add(caller_id)

        entry = _build_caller_chain_entry(dict(r), depth)

        # Recurse if within depth and caller is chainable
        if depth < max_depth and entry.ref_type in CHAINABLE_REFERENCE_TYPES:
            entry.children = build_caller_chain_for_method(
                runner, caller_id, depth + 1, max_depth, visited
            )

        entries.append(entry)

    return entries


# =============================================================================
# build_class_used_by
# =============================================================================


def build_class_used_by(
    runner: QueryRunner,
    node: NodeData,
    max_depth: int = 1,
    limit: int = 100,
) -> list[ContextEntry]:
    """Build the USED BY section for a Class node context.

    Fetches all required data in a single batch call, then applies the handler
    pipeline to produce a priority-ordered list of ContextEntry objects.

    Args:
        runner: Active QueryRunner connected to Neo4j.
        node: NodeData for the target class.
        max_depth: Maximum depth for caller chain expansion.
        limit: Maximum number of entries to return.

    Returns:
        Ordered list of ContextEntry objects representing who uses the class.
    """
    data = fetch_class_used_by_data(runner, node.node_id)

    extends_children: list[dict] = data["extends_children"]
    incoming_usages: list[dict] = data["incoming_usages"]
    injected_classes: set[str] = data["injected_classes"]
    call_nodes: list[dict] = data["call_nodes"]
    property_types: list[dict] = data["property_types"]
    ref_type_data: dict[str, dict] = data["ref_type_data"]

    # ------------------------------------------------------------------
    # Build extends entries directly from Q1 (bypass handler pipeline)
    # ------------------------------------------------------------------
    extends_entries: list[ContextEntry] = []
    for child in extends_children:
        rel = child.get("rel_type", "EXTENDS").lower()
        ref = "extends" if rel == "extends" else "implements"
        extends_entries.append(ContextEntry(
            depth=1,
            node_id=child["id"],
            fqn=child["fqn"],
            kind=child.get("kind"),
            file=child.get("file"),
            line=child.get("start_line"),
            ref_type=ref,
        ))

    # ------------------------------------------------------------------
    # Build call node index: (source_id, file, line) -> call record
    # ------------------------------------------------------------------
    call_index: dict[tuple, dict] = {}
    for cn in call_nodes:
        key = (cn["source_id"], cn.get("call_file"), cn.get("call_start_line"))
        # Keep first match (there may be multiple calls; we want the best match)
        if key not in call_index:
            call_index[key] = cn

    # ------------------------------------------------------------------
    # Build property_type lookup: source_id -> prop record (from Q6)
    # For constructor promotion: method -> prop via type_hint
    # ------------------------------------------------------------------
    # Q6 gives us properties by TYPE_HINT. We need to map them back to
    # the method source. We index by source_id where source_kind is Method
    # by checking the Q2 source mapping.
    prop_type_by_source: dict[str, dict] = {}
    for pt in property_types:
        # We'll match these up during the Q2 loop below
        prop_type_by_source[pt["prop_id"]] = pt

    # ------------------------------------------------------------------
    # Process incoming usage edges through handlers
    # ------------------------------------------------------------------
    bucket = EntryBucket()
    classes_with_injection: frozenset[str] = frozenset(injected_classes)

    for usage in incoming_usages:
        source_id: str = usage["source_id"]
        source_kind: str = usage.get("source_kind", "")
        source_fqn: str = usage.get("source_fqn", "")
        source_name: str = usage.get("source_name", "") or ""
        source_file = usage.get("source_file")
        source_start_line = usage.get("source_start_line")
        source_signature = usage.get("source_signature")
        edge_type: str = usage.get("edge_type", "USES").upper()
        edge_file = usage.get("edge_file")
        edge_line = usage.get("edge_line")
        containing_class_id = usage.get("containing_class_id")
        containing_method_id = usage.get("containing_method_id")
        containing_method_fqn = usage.get("containing_method_fqn")
        containing_method_kind = usage.get("containing_method_kind")

        # ------------------------------------------------------------------
        # Look up pre-fetched ref type data from Q8
        # ------------------------------------------------------------------
        rtd = ref_type_data.get(source_id, {})
        has_arg_type_hint: bool = bool(rtd.get("has_arg_type_hint", False))
        has_return_type_hint: bool = bool(rtd.get("has_return_type_hint", False))
        has_class_property_type_hint: bool = bool(rtd.get("has_class_property_type_hint", False))
        has_source_class_property_type_hint: bool = bool(
            rtd.get("has_source_class_property_type_hint", False)
        )

        # ------------------------------------------------------------------
        # Infer reference type
        # ------------------------------------------------------------------
        # Normalise edge_type to lowercase for infer_reference_type
        edge_type_lower = edge_type.lower()
        ref_type = infer_reference_type(
            edge_type=edge_type_lower,
            target_kind=node.kind,
            source_kind=source_kind,
            source_id=source_id,
            target_id=node.node_id,
            source_name=source_name,
            has_arg_type_hint=has_arg_type_hint,
            has_return_type_hint=has_return_type_hint,
            has_class_property_type_hint=has_class_property_type_hint,
            has_source_class_property_type_hint=has_source_class_property_type_hint,
        )

        # ------------------------------------------------------------------
        # Find matching Call node from Q4 for this edge location
        # ------------------------------------------------------------------
        call_key = (source_id, edge_file, edge_line)
        call_record = call_index.get(call_key)

        call_node_id = call_record["call_id"] if call_record else None
        call_kind = call_record["call_kind"] if call_record else None

        # Use call_kind for authoritative ref_type override
        if call_kind == "constructor":
            ref_type = "instantiation"

        # ------------------------------------------------------------------
        # Resolve receiver / access_chain info from call record
        # ------------------------------------------------------------------
        access_chain: str | None = None
        on_kind: str | None = None
        if call_record:
            recv_value_kind = call_record.get("recv_value_kind")
            recv_name = call_record.get("recv_name")
            if recv_name:
                access_chain = recv_name
            if recv_value_kind:
                on_kind = recv_value_kind

        # ------------------------------------------------------------------
        # Resolve property data for PropertyTypeHandler (constructor promotion)
        # ------------------------------------------------------------------
        property_node_id: str | None = None
        property_fqn: str | None = None
        property_file: str | None = None
        property_start_line: int | None = None

        if ref_type == "property_type" and source_kind in ("Method", "Function"):
            # Constructor promotion: find the property with type_hint via Q6 data.
            # The property FQN typically shares the class prefix with the method FQN.
            if "::" in source_fqn:
                class_prefix = source_fqn.rsplit("::", 1)[0]
                for pt in property_types:
                    if pt["prop_fqn"].startswith(class_prefix + "::"):
                        property_node_id = pt["prop_id"]
                        property_fqn = pt["prop_fqn"]
                        property_file = pt.get("prop_file")
                        property_start_line = pt.get("prop_start_line")
                        break

        # ------------------------------------------------------------------
        # Build EdgeContext and dispatch to handler
        # ------------------------------------------------------------------
        ctx = EdgeContext(
            start_id=node.node_id,
            source_id=source_id,
            source_kind=source_kind,
            source_fqn=source_fqn,
            source_name=source_name,
            source_file=source_file,
            source_start_line=source_start_line,
            source_signature=source_signature,
            target_kind=node.kind,
            target_fqn=node.fqn,
            target_name=node.name,
            ref_type=ref_type,
            file=edge_file,
            line=edge_line,
            call_node_id=call_node_id,
            classes_with_injection=classes_with_injection,
            containing_method_id=containing_method_id,
            containing_method_fqn=containing_method_fqn,
            containing_method_kind=containing_method_kind,
            containing_class_id=containing_class_id,
            call_kind=call_kind,
            access_chain=access_chain,
            on_kind=on_kind,
            property_node_id=property_node_id,
            property_fqn=property_fqn,
            property_file=property_file,
            property_start_line=property_start_line,
        )

        handler = USED_BY_HANDLERS.get(ref_type)
        if handler:
            handler.handle(ctx, bucket)

    # ------------------------------------------------------------------
    # Convert handler buckets to ContextEntry lists
    # ------------------------------------------------------------------
    result_extends = extends_entries  # already ContextEntry
    result_instantiation = [_dict_to_context_entry(d) for d in bucket.instantiation]
    result_property_type = [_dict_to_context_entry(d) for d in bucket.property_type]
    result_method_call = [_dict_to_context_entry(d) for d in bucket.method_call]
    result_property_access = _build_property_access_entries(bucket, node.fqn, depth=1)
    result_param_return = [_dict_to_context_entry(d) for d in bucket.param_return]

    # ------------------------------------------------------------------
    # Depth 2+: expand caller chains for chainable entries
    # ------------------------------------------------------------------
    if max_depth >= 2:
        visited_methods: set[str] = set()

        # Expand method_call entries
        for entry in result_method_call:
            if entry.node_id and entry.ref_type in CHAINABLE_REFERENCE_TYPES:
                visited_methods.add(entry.node_id)
                entry.children = build_caller_chain_for_method(
                    runner, entry.node_id, 2, max_depth, visited_methods
                )

        # Expand instantiation entries
        for entry in result_instantiation:
            if entry.node_id and entry.ref_type in CHAINABLE_REFERENCE_TYPES:
                visited_methods.add(entry.node_id)
                entry.children = build_caller_chain_for_method(
                    runner, entry.node_id, 2, max_depth, visited_methods
                )

        # For property_type entries at depth 2, add injection point calls
        for entry in result_property_type:
            if entry.node_id and entry.fqn:
                injection_records = runner.execute(
                    Q7_INJECTION_POINT_CALLS,
                    property_id=entry.node_id,
                    property_fqn=entry.fqn,
                )
                seen_methods: set[str] = set()
                for r in injection_records:
                    method_id = r["method_id"]
                    if method_id in seen_methods:
                        continue
                    seen_methods.add(method_id)
                    callee_name = r.get("callee_name")
                    callee_kind = r.get("callee_kind", "")
                    if callee_kind == "Method":
                        callee_display = (callee_name + "()") if callee_name else None
                    else:
                        callee_display = callee_name

                    child = ContextEntry(
                        depth=2,
                        node_id=r["method_id"],
                        fqn=format_method_fqn(r.get("method_fqn", ""), "Method"),
                        kind="Method",
                        line=r.get("call_line"),
                        ref_type="method_call",
                        callee=callee_display,
                    )
                    entry.children.append(child)

    # ------------------------------------------------------------------
    # Combine in priority order and apply limit
    # ------------------------------------------------------------------
    all_entries = (
        result_extends
        + result_instantiation
        + result_property_type
        + result_method_call
        + result_property_access
        + result_param_return
    )

    return all_entries[:limit]


# =============================================================================
# build_class_uses
# =============================================================================


def build_class_uses(
    runner: QueryRunner,
    node: NodeData,
    max_depth: int = 1,
    limit: int = 100,
) -> list[ContextEntry]:
    """Build the USES section for a Class node context.

    Fetches outgoing dependencies (structural and member-level) and assembles
    a priority-ordered list of ContextEntry objects.

    Args:
        runner: Active QueryRunner connected to Neo4j.
        node: NodeData for the target class.
        max_depth: Maximum depth for behavioral expansion (depth 2 adds
            methods called through injected properties).
        limit: Maximum number of entries to return.

    Returns:
        Priority-ordered list of ContextEntry objects for USES section.
    """
    data = fetch_class_uses_data(runner, node.node_id)

    member_deps: list[dict] = data["member_deps"]
    class_rel: list[dict] = data["class_rel"]

    # ------------------------------------------------------------------
    # Class-level structural relationships (extends, implements, uses_trait)
    # ------------------------------------------------------------------
    structural_entries: list[ContextEntry] = []
    extends_implements_fqns: set[str] = set()

    for rel in class_rel:
        rel_type_raw: str = rel.get("rel_type", "EXTENDS").upper()
        if rel_type_raw == "EXTENDS":
            ref = "extends"
        elif rel_type_raw == "IMPLEMENTS":
            ref = "implements"
        else:
            ref = "uses_trait"

        target_fqn = rel.get("target_fqn", "")
        extends_implements_fqns.add(target_fqn)

        structural_entries.append(ContextEntry(
            depth=1,
            node_id=rel.get("target_id", ""),
            fqn=target_fqn,
            kind=rel.get("target_kind"),
            file=rel.get("file"),
            line=rel.get("line"),
            ref_type=ref,
        ))

    # ------------------------------------------------------------------
    # Member-level dependencies: classify and group by target
    # ------------------------------------------------------------------
    # Keep highest-priority ref_type per target (deduplicated by target_id)
    target_best: dict[str, dict] = {}

    for dep in member_deps:
        target_id: str = dep.get("target_id", "")
        target_fqn: str = dep.get("target_fqn", "")
        target_kind: str = dep.get("target_kind", "")
        edge_type: str = dep.get("edge_type", "USES").lower()

        # Skip targets already covered by structural relationships
        if target_fqn in extends_implements_fqns:
            continue

        # Infer reference type for outgoing edge
        ref_type = infer_reference_type(
            edge_type=edge_type,
            target_kind=target_kind,
            source_kind=dep.get("member_kind"),
            source_id=dep.get("member_id", ""),
            target_id=target_id,
            source_name=dep.get("member_name"),
        )

        existing = target_best.get(target_id)
        if existing is None:
            target_best[target_id] = {
                "target_id": target_id,
                "target_fqn": target_fqn,
                "target_kind": target_kind,
                "ref_type": ref_type,
                "file": dep.get("file"),
                "line": dep.get("line"),
            }
        else:
            # Keep the ref_type with lower (higher priority) USES_PRIORITY value
            cur_pri = USES_PRIORITY.get(existing["ref_type"], 99)
            new_pri = USES_PRIORITY.get(ref_type, 99)
            if new_pri < cur_pri:
                existing["ref_type"] = ref_type
                existing["file"] = dep.get("file")
                existing["line"] = dep.get("line")

    member_entries: list[ContextEntry] = []
    for td in target_best.values():
        member_entries.append(ContextEntry(
            depth=1,
            node_id=td["target_id"],
            fqn=td["target_fqn"],
            kind=td.get("target_kind"),
            file=td.get("file"),
            line=td.get("line"),
            ref_type=td["ref_type"],
        ))

    # ------------------------------------------------------------------
    # Depth 2: behavioral expansion for property_type entries
    # ------------------------------------------------------------------
    if max_depth >= 2:
        for entry in member_entries:
            if entry.ref_type == "property_type" and entry.node_id:
                records = runner.execute(
                    Q3_BEHAVIORAL_DEPTH2,
                    class_id=node.node_id,
                    property_fqn=entry.fqn,
                )
                seen: set[str] = set()
                for r in records:
                    callee_id = r["callee_id"]
                    if callee_id in seen:
                        continue
                    seen.add(callee_id)
                    callee_kind = r.get("callee_kind", "")
                    child = ContextEntry(
                        depth=2,
                        node_id=callee_id,
                        fqn=format_method_fqn(r.get("callee_fqn", ""), callee_kind),
                        kind=callee_kind,
                        ref_type="method_call",
                    )
                    entry.children.append(child)

    # ------------------------------------------------------------------
    # Combine and sort by USES priority order
    # ------------------------------------------------------------------
    all_entries = structural_entries + member_entries
    all_entries.sort(
        key=lambda e: (
            USES_PRIORITY.get(e.ref_type or "", 99),
            e.file or "",
            e.line or 0,
        )
    )

    return all_entries[:limit]
