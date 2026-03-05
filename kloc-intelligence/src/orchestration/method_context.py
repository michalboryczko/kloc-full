"""Method context orchestrator: builds USED BY and USES trees for Method nodes.

Method USED BY:
- Callers of the method (via Call nodes)
- Each caller has: signature, member_ref, arguments (rich format)
- Depth 2: callers of the caller (same format)

Method USES (execution flow):
- Call children of the method in line-number order
- Kind 1: local_variable with source_call (new SomeClass(), result assigned to local)
- Kind 2: direct call ($obj->method(), static::method())
- Consumed calls (whose result feeds into another call) are excluded from top-level
- Depth 2: recursively expand callee's execution flow
"""

from __future__ import annotations

from typing import Optional

from ..db.query_runner import QueryRunner
from ..db.queries.context_method import (
    fetch_method_callers,
    fetch_method_call_arguments,
    fetch_method_calls,
    fetch_consumed_calls,
)
from ..db.queries.definition import _extract_signature_from_doc
from ..models.results import ArgumentInfo, ContextEntry, MemberRef


# =================================================================
# USED BY: Method callers
# =================================================================


def _build_rich_arguments(
    runner: QueryRunner,
    call_id: str,
    caller_fqn: str,
) -> list[ArgumentInfo]:
    """Build rich argument list for a call site.

    Produces ArgumentInfo objects with:
    - position, param_name, value_expr, value_source
    - value_type (from TYPE_OF edge on the arg value)
    - param_fqn (target parameter FQN)
    - value_ref_symbol (for parameter/local sources: the arg value's FQN)
    - source_chain (for result sources: the producing call chain)
    """
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

        # param_name from param_fqn:
        # "App\Foo::method().$param" -> "$param"
        # "App\Foo::$prop" (promoted) -> "$prop"
        param_name = None
        if param_fqn:
            if ".$" in param_fqn:
                param_name = "$" + param_fqn.split(".$")[-1]
            elif "::$" in param_fqn:
                param_name = "$" + param_fqn.split("::$")[-1]

        # value_expr: prefer expression, fall back to value name
        value_expr = arg_expression or value_expr_name or ""

        # value_ref_symbol: for parameter/local sources, the value's FQN
        value_ref_symbol = None
        if value_source in ("parameter", "local") and value_fqn:
            value_ref_symbol = value_fqn

        # source_chain: for result sources, build from the source call
        source_chain = None
        if value_source == "result" and arg.get("source_callee_fqn"):
            chain_entry: dict = {
                "fqn": arg["source_callee_fqn"],
                "kind": arg.get("source_callee_kind") or "Method",
            }

            # reference_type from call_kind
            source_call_kind = arg.get("source_call_kind") or ""
            if source_call_kind == "constructor":
                chain_entry["reference_type"] = "constructor"
                # Constructor calls: no on/on_file/on_line
            else:
                chain_entry["reference_type"] = "method"
                # Method calls: include on/on_file/on_line for the receiver
                source_file = arg.get("source_call_file")
                source_line = arg.get("source_call_line")
                if source_file and source_line is not None:
                    # source_line from Neo4j is 0-based; kloc-cli uses it as-is
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


def _build_member_ref_for_caller(
    caller_record: dict,
) -> MemberRef:
    """Build MemberRef for a caller entry (USED BY)."""
    target_fqn = caller_record.get("target_fqn", "")
    target_name = caller_record.get("target_name", "")
    target_kind = caller_record.get("target_kind", "Method")

    # Display name: "method()" for methods
    display_name = target_name
    if target_kind in ("Method", "Function"):
        display_name = f"{target_name}()"

    # reference_type from call_kind
    call_kind = caller_record.get("call_kind", "")
    ref_type = _call_kind_to_reference_type(call_kind)

    # call_line from Neo4j is 0-based; store call_line - 1 so
    # OutputMemberRef's +1 conversion produces the raw call_line
    call_line = caller_record.get("call_line")
    ref_line = call_line - 1 if call_line is not None else None

    return MemberRef(
        target_name=display_name,
        target_fqn=target_fqn,
        target_kind=target_kind,
        file=caller_record.get("call_file"),
        line=ref_line,
        reference_type=ref_type,
    )


def _call_kind_to_reference_type(call_kind: str) -> str:
    """Map call_kind to reference_type for member_ref."""
    mapping = {
        "constructor": "instantiation",
        "method": "method_call",
        "method_static": "static_call",
        "function": "function_call",
    }
    return mapping.get(call_kind, "method_call")


def build_method_used_by(
    runner: QueryRunner,
    start_id: str,
    max_depth: int,
    limit: int,
    include_impl: bool = False,
) -> list[ContextEntry]:
    """Build the USED BY tree for a Method node.

    Finds all callers of this method and builds entries with:
    - Caller method FQN, kind, file, line, signature
    - member_ref pointing to the target method with reference_type
    - arguments with rich parameter mapping

    Args:
        runner: QueryRunner instance.
        start_id: Target method node ID.
        max_depth: Maximum depth for caller chain expansion.
        limit: Maximum entries.
        include_impl: Whether to include implementation details.

    Returns:
        List of ContextEntry objects.
    """
    callers = fetch_method_callers(runner, start_id)

    # Group by caller method (deduplicate multiple call sites in same method)
    seen_callers: dict[str, dict] = {}
    for caller in callers:
        caller_id = caller["caller_id"]
        if caller_id not in seen_callers:
            seen_callers[caller_id] = caller

    entries: list[ContextEntry] = []
    for caller_id, caller in seen_callers.items():
        # Build signature from documentation
        caller_doc = caller.get("caller_documentation") or []
        signature = _extract_signature_from_doc(caller_doc)

        # Build member_ref
        member_ref = _build_member_ref_for_caller(caller)

        # Build rich arguments
        call_id = caller.get("call_id")
        arguments = []
        if call_id:
            arguments = _build_rich_arguments(runner, call_id, caller.get("caller_fqn", ""))

        # For method USED BY, the entry's file/line is the call site
        # (where the target is called), not the caller method's definition.
        # call_line from Neo4j is 0-based; we store call_line - 1 so
        # OutputEntry's +1 conversion produces the raw call_line value
        # (matching kloc-cli's behavior of using 0-based call lines as-is).
        call_line = caller.get("call_line")
        call_file = caller.get("call_file") or caller.get("caller_file")
        entry_line = call_line - 1 if call_line is not None else caller.get("caller_start_line")

        entry = ContextEntry(
            depth=1,
            node_id=caller_id,
            fqn=caller.get("caller_fqn", ""),
            kind=caller.get("caller_kind", "Method"),
            file=call_file,
            line=entry_line,
            signature=signature,
            member_ref=member_ref,
            arguments=arguments,
            children=[],
        )

        # Depth-2: callers of the caller
        if max_depth >= 2:
            entry.children = _build_caller_children(
                runner, caller_id, 2, max_depth
            )

        entries.append(entry)

    # Sort by file, line
    entries.sort(key=lambda e: (e.file or "", e.line if e.line is not None else 0))
    return entries[:limit]


def _build_caller_children(
    runner: QueryRunner,
    method_id: str,
    depth: int,
    max_depth: int,
) -> list[ContextEntry]:
    """Build depth-2+ caller chain children for a method.

    Each child is a caller of the given method, with member_ref, arguments, signature.
    """
    if depth > max_depth:
        return []

    callers = fetch_method_callers(runner, method_id)

    seen: dict[str, dict] = {}
    for caller in callers:
        cid = caller["caller_id"]
        if cid not in seen:
            seen[cid] = caller

    children: list[ContextEntry] = []
    for cid, caller in seen.items():
        caller_doc = caller.get("caller_documentation") or []
        signature = _extract_signature_from_doc(caller_doc)

        member_ref = _build_member_ref_for_caller(caller)

        call_id = caller.get("call_id")
        arguments = []
        if call_id:
            arguments = _build_rich_arguments(runner, call_id, caller.get("caller_fqn", ""))

        # Use call site line, same convention as build_method_used_by
        call_line = caller.get("call_line")
        call_file = caller.get("call_file") or caller.get("caller_file")
        entry_line = call_line - 1 if call_line is not None else caller.get("caller_start_line")

        child = ContextEntry(
            depth=depth,
            node_id=cid,
            fqn=caller.get("caller_fqn", ""),
            kind=caller.get("caller_kind", "Method"),
            file=call_file,
            line=entry_line,
            signature=signature,
            member_ref=member_ref,
            arguments=arguments,
            children=[],
        )

        # Recurse deeper
        if depth < max_depth:
            child.children = _build_caller_children(runner, cid, depth + 1, max_depth)

        children.append(child)

    children.sort(key=lambda e: (e.file or "", e.line if e.line is not None else 0))
    return children


# =================================================================
# USES: Method execution flow
# =================================================================


def _build_member_ref_for_call(
    call: dict,
    receiver_access_chain: Optional[str] = None,
    receiver_access_chain_symbol: Optional[str] = None,
    on_kind: Optional[str] = None,
    on_file: Optional[str] = None,
    on_line: Optional[int] = None,
) -> Optional[MemberRef]:
    """Build MemberRef for a call in execution flow (USES)."""
    callee_fqn = call.get("callee_fqn") or call.get("call_name") or ""
    callee_name = call.get("callee_name") or call.get("call_name") or ""
    callee_kind = call.get("callee_kind") or ""
    call_kind = call.get("call_kind") or ""

    # Display name
    if callee_kind in ("Method", "Function") or call_kind in ("method", "method_static", "function"):
        display_name = f"{callee_name}()"
    elif call_kind == "constructor":
        display_name = "__construct()"
    else:
        display_name = callee_name or callee_fqn

    # reference_type
    ref_type = _call_kind_to_reference_type(call_kind)

    return MemberRef(
        target_name=display_name,
        target_fqn=callee_fqn,
        target_kind=callee_kind if callee_kind else (
            "constructor" if call_kind == "constructor" else "method"
        ),
        file=call.get("call_file"),
        line=call.get("call_line"),  # 0-based
        reference_type=ref_type,
        access_chain=receiver_access_chain,
        access_chain_symbol=receiver_access_chain_symbol,
        on_kind=on_kind,
        on_file=on_file,
        on_line=on_line,
    )


def _resolve_receiver_info(call: dict) -> tuple[Optional[str], Optional[str], Optional[str], Optional[str], Optional[int]]:
    """Resolve receiver (access_chain, access_chain_symbol, on_kind, on_file, on_line) for a call.

    Returns:
        Tuple of (access_chain, access_chain_symbol, on_kind, on_file, on_line).
    """
    recv_value_kind = call.get("recv_value_kind")
    recv_name = call.get("recv_name")
    on_property = call.get("on_property")

    if not recv_value_kind and not recv_name:
        return None, None, None, None, None

    access_chain = None
    access_chain_symbol = None
    on_kind = None
    on_file = None
    on_line = None

    # recv_name from Neo4j already has $ prefix (e.g., "$estate")
    # so we use it directly without adding another $
    recv_file = call.get("recv_file")
    recv_start_line = call.get("recv_start_line")

    if on_property:
        # Receiver resolved to a property: $this->prop
        prop_short = recv_name.lstrip("$") if recv_name else ""
        access_chain = f"$this->{prop_short}" if prop_short else None
        access_chain_symbol = on_property
        on_kind = "property"
    elif recv_value_kind == "parameter":
        access_chain = recv_name  # already "$param"
        on_kind = "param"
        on_file = recv_file
        on_line = recv_start_line
    elif recv_value_kind == "local":
        access_chain = recv_name  # already "$local"
        on_kind = "local"
        on_file = recv_file
        on_line = recv_start_line
    elif recv_value_kind == "self":
        access_chain = "$this"
        on_kind = "self"
    elif recv_value_kind == "result":
        access_chain = recv_name  # already "$varName"
        on_kind = "result"

    return access_chain, access_chain_symbol, on_kind, on_file, on_line


def build_method_uses(
    runner: QueryRunner,
    start_id: str,
    max_depth: int,
    limit: int,
    include_impl: bool = False,
) -> list[ContextEntry]:
    """Build the USES tree for a Method node (execution flow).

    Processes call children of the method in line-number order:
    - Kind 1 (local_variable): Call produces a result assigned to a local variable
    - Kind 2 (direct call): Call with no local variable assignment

    Consumed calls (whose result is used as receiver/argument by another call)
    are excluded from top-level entries but appear as source_call within
    local_variable entries.

    Args:
        runner: QueryRunner instance.
        start_id: Target method node ID.
        max_depth: Maximum depth for execution flow expansion.
        limit: Maximum entries.
        include_impl: Whether to include implementation details.

    Returns:
        List of ContextEntry objects representing execution flow.
    """
    calls = fetch_method_calls(runner, start_id)
    consumed = fetch_consumed_calls(runner, start_id)

    entries: list[ContextEntry] = []
    seen_locals: set[str] = set()

    for call in calls:
        call_id = call.get("call_id")

        # Check if this call has a local variable assignment (Kind 1)
        local_id = call.get("local_id")
        local_fqn = call.get("local_fqn")
        local_name = call.get("local_name")
        local_type = call.get("local_type_name")

        if local_id and local_fqn and local_fqn not in seen_locals:
            seen_locals.add(local_fqn)

            # Build the source_call entry (the constructor/method that produces the value)
            source_call_entry = _build_source_call_entry(runner, call, 1)

            entry = ContextEntry(
                depth=1,
                node_id=local_id,
                fqn=local_fqn,
                kind="Value",
                file=call.get("call_file"),
                line=call.get("call_line"),
                entry_type="local_variable",
                variable_name=local_name if local_name else None,  # already has $ prefix
                variable_symbol=local_fqn,
                variable_type=local_type,
                source_call=source_call_entry,
                children=[],
            )

            # Depth-2: execution flow of the constructor (calls made on $local)
            if max_depth >= 2:
                callee_id = call.get("callee_id")
                if callee_id:
                    entry.children = _build_execution_flow_children(
                        runner, callee_id, local_fqn, local_name, 2, max_depth
                    )

            entries.append(entry)

        elif call_id and call_id not in consumed:
            # Kind 2: direct call (not consumed by another call)
            callee_fqn = call.get("callee_fqn") or call.get("call_name") or ""
            callee_kind = call.get("callee_kind") or ""
            callee_id = call.get("callee_id")
            callee_doc = call.get("callee_documentation") or []

            # Build receiver info
            access_chain, access_chain_symbol, on_kind, on_file, on_line = _resolve_receiver_info(call)

            # Build member_ref
            member_ref = _build_member_ref_for_call(
                call,
                receiver_access_chain=access_chain,
                receiver_access_chain_symbol=access_chain_symbol,
                on_kind=on_kind,
                on_file=on_file,
                on_line=on_line,
            )

            # Build arguments
            arguments = []
            if call_id:
                arguments = _build_rich_arguments(runner, call_id, callee_fqn)

            # Signature
            signature = _extract_signature_from_doc(callee_doc)

            # For USES call entries, use callee's file but call site line
            # The line is where the call happens in the method, not where callee is defined
            entry = ContextEntry(
                depth=1,
                node_id=callee_id or call_id,
                fqn=callee_fqn,
                kind=callee_kind if callee_kind else "Method",
                file=call.get("callee_file") or call.get("call_file"),
                line=call.get("call_line"),
                signature=signature,
                member_ref=member_ref,
                arguments=arguments,
                entry_type="call",
                children=[],
            )

            # Depth-2: execution flow of the callee
            if max_depth >= 2 and callee_id:
                entry.children = _build_execution_flow_children(
                    runner, callee_id, None, None, 2, max_depth
                )

            entries.append(entry)

    return entries[:limit]


def _build_source_call_entry(
    runner: QueryRunner,
    call: dict,
    depth: int,
) -> Optional[ContextEntry]:
    """Build a source_call ContextEntry for a local_variable entry.

    This represents the constructor or method call that produces the value
    assigned to the local variable.
    """
    callee_fqn = call.get("callee_fqn") or ""
    callee_kind = call.get("callee_kind") or ""
    callee_id = call.get("callee_id")
    callee_doc = call.get("callee_documentation") or []
    call_kind = call.get("call_kind") or ""

    # Build member_ref for the source call
    member_ref = _build_member_ref_for_call(call)

    # Build arguments
    arguments = []
    call_id = call.get("call_id")
    if call_id:
        arguments = _build_rich_arguments(runner, call_id, callee_fqn)

    # Signature
    signature = _extract_signature_from_doc(callee_doc)

    return ContextEntry(
        depth=depth,
        node_id=callee_id or call_id or "",
        fqn=callee_fqn,
        kind=callee_kind if callee_kind else "Method",
        file=call.get("call_file"),
        line=call.get("call_line"),
        signature=signature,
        member_ref=member_ref,
        arguments=arguments,
        entry_type="call",
        children=[],
    )


def _build_execution_flow_children(
    runner: QueryRunner,
    method_id: str,
    parent_local_fqn: Optional[str],
    parent_local_name: Optional[str],
    depth: int,
    max_depth: int,
) -> list[ContextEntry]:
    """Build depth-2+ execution flow children for a callee method.

    For depth-2 under a local_variable entry, these are the calls made within
    the constructor (e.g., $this->prop = new Collection(), $this->prop->add()).

    For depth-2 under a call entry, these are the calls made within that method.
    """
    if depth > max_depth:
        return []

    calls = fetch_method_calls(runner, method_id)
    consumed = fetch_consumed_calls(runner, method_id)

    children: list[ContextEntry] = []

    for call in calls:
        call_id = call.get("call_id")

        # For depth-2, show all calls (including consumed ones if they produce something visible)
        callee_fqn = call.get("callee_fqn") or call.get("call_name") or ""
        callee_kind = call.get("callee_kind") or ""
        callee_id = call.get("callee_id")
        callee_doc = call.get("callee_documentation") or []

        # Build receiver info
        access_chain, access_chain_symbol, on_kind, on_file, on_line = _resolve_receiver_info(call)

        # Build member_ref
        member_ref = _build_member_ref_for_call(
            call,
            receiver_access_chain=access_chain,
            receiver_access_chain_symbol=access_chain_symbol,
            on_kind=on_kind,
            on_file=on_file,
            on_line=on_line,
        )

        # Build arguments
        arguments = []
        if call_id:
            arguments = _build_rich_arguments(runner, call_id, callee_fqn)

        # Signature
        signature = _extract_signature_from_doc(callee_doc)

        child = ContextEntry(
            depth=depth,
            node_id=callee_id or call_id or "",
            fqn=callee_fqn,
            kind=callee_kind if callee_kind else (
                "constructor" if call.get("call_kind") == "constructor" else "method"
            ),
            file=call.get("call_file"),
            line=call.get("call_line"),
            signature=signature,
            member_ref=member_ref,
            arguments=arguments,
            entry_type="call",
            children=[],
        )

        children.append(child)

    return children
