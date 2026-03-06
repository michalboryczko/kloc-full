"""Value context orchestrators: build consumer and source chains for Value nodes.

Ported from kloc-cli's value_context.py. Handles Value consumer chains (USED BY)
and source chains (USES), including cross-method boundary tracing via parameter
FQNs and return value type matching.

Key design rules:
- All Neo4j access is via query functions in src/db/queries/context_value.py.
- Visited set prevents infinite cycles across Value nodes.
- seen_calls prevents duplicate entries for the same Call.
- Cross-method boundary via parameter FQN matching (callee direction).
- Cross-method return via type matching (caller direction).
- max_crossings limits return crossings per chain (defaults to min(max_depth, 10)).
- All line numbers are 0-based.
"""

from __future__ import annotations

from ..db.query_runner import QueryRunner
from ..db.queries.context_value import (
    Q4_ARGUMENT_PARAMS,
    Q5_RESOLVE_PARAM,
    Q6_LOCAL_FOR_RESULT,
    Q7_TYPE_OF,
    Q8_METHOD_CALLERS,
    Q10_PARAMETER_USES,
    Q11_CALL_ARGUMENTS,
    Q12_CONTAINING_METHOD,
    fetch_value_consumer_data,
    fetch_value_source_data,
)
from ..models.results import ContextEntry, MemberRef, ArgumentInfo


# =============================================================================
# Internal helpers
# =============================================================================


def _build_callee_display(name: str | None, kind: str | None) -> str | None:
    """Format callee name for display (e.g., 'save()' for Method)."""
    if not name:
        return None
    if kind == "Method":
        return name + "()"
    if kind == "Property":
        return name if name.startswith("$") else "$" + name
    return name


def _build_arguments_from_records(
    runner: QueryRunner,
    call_id: str,
) -> list[ArgumentInfo]:
    """Build ArgumentInfo list for a call by querying Q11."""
    records = runner.execute(Q11_CALL_ARGUMENTS, call_id=call_id)
    infos: list[ArgumentInfo] = []
    for rec in records:
        position = rec.get("position")
        if position is None:
            continue
        infos.append(ArgumentInfo(
            position=int(position),
            value_expr=rec.get("expression"),
            value_source=rec.get("value_kind"),
            value_type=rec.get("value_type"),
            param_fqn=rec.get("parameter"),
            value_ref_symbol=rec.get("value_fqn"),
        ))
    infos.sort(key=lambda a: a.position)
    return infos


def _infer_ref_type_from_call_kind(call_kind: str | None, target_kind: str | None) -> str | None:
    """Infer reference type from call_kind and target_kind."""
    if call_kind == "constructor":
        return "instantiation"
    if target_kind == "Method":
        return "method_call"
    if target_kind == "Property":
        return "property_access"
    if target_kind == "Function":
        return "function_call"
    return None


def _resolve_on_kind(recv_value_kind: str | None) -> str | None:
    """Map receiver value_kind to on_kind display string."""
    if recv_value_kind == "parameter":
        return "param"
    if recv_value_kind == "local":
        return "local"
    if recv_value_kind == "self":
        return "self"
    if recv_value_kind == "result":
        return "property"
    return recv_value_kind


# =============================================================================
# build_value_consumer_chain
# =============================================================================


def build_value_consumer_chain(
    runner: QueryRunner,
    value_id: str,
    depth: int,
    max_depth: int,
    limit: int,
    visited: set[str] | None = None,
    crossing_count: int = 0,
    max_crossings: int | None = None,
) -> list[ContextEntry]:
    """Build consumer chain for a Value node (USED BY section).

    Finds all Calls that consume this Value -- either as a receiver (property
    access) or directly as an argument. Groups property accesses by the
    downstream Call that consumes the accessed property result.

    Three-part processing:
      Part 1: Receiver edges -> grouped by consuming Call
      Part 2: Standalone property accesses (not consumed as arguments)
      Part 3: Direct argument edges

    Args:
        runner: Active QueryRunner connected to Neo4j.
        value_id: The Value node ID to find consumers for.
        depth: Current depth level.
        max_depth: Maximum depth to expand.
        limit: Maximum number of entries.
        visited: Set of visited Value IDs for cycle detection.
        crossing_count: Number of return crossings so far in this chain.
        max_crossings: Maximum return crossings allowed per chain.

    Returns:
        List of ContextEntry representing consuming Calls, sorted by (file, line).
    """
    if max_crossings is None:
        max_crossings = min(max_depth, 10)

    if depth > max_depth:
        return []

    if visited is None:
        visited = set()
    if value_id in visited:
        return []
    visited.add(value_id)

    data = fetch_value_consumer_data(runner, value_id)
    receiver_records: list[dict] = data["receiver_chain"]
    direct_arg_records: list[dict] = data["direct_arguments"]

    entries: list[ContextEntry] = []
    count = 0
    seen_calls: set[str] = set()

    # === Part 1: Receiver edges grouped by consuming Call ===
    # Group: consumer_call_id -> list of access info dicts
    consumer_groups: dict[str, list[dict]] = {}
    standalone_accesses: list[dict] = []

    for rec in receiver_records:
        access_call_id = rec.get("access_call_id")
        if not access_call_id:
            continue

        consumer_call_id = rec.get("consumer_call_id")
        assigned_local_id = rec.get("assigned_local_id")

        found_consumer = False

        if consumer_call_id:
            # Result is consumed as argument by another Call
            if consumer_call_id not in consumer_groups:
                consumer_groups[consumer_call_id] = []
            consumer_groups[consumer_call_id].append({
                "prop_name": rec.get("target_name", "?"),
                "prop_fqn": rec.get("target_fqn"),
                "position": rec.get("arg_position", 0),
                "expression": rec.get("arg_expression"),
                "access_call_id": access_call_id,
                "access_call_line": rec.get("access_call_line"),
                "consumer_target_id": rec.get("consumer_target_id"),
                "consumer_target_fqn": rec.get("consumer_target_fqn"),
                "consumer_target_name": rec.get("consumer_target_name"),
                "consumer_target_kind": rec.get("consumer_target_kind"),
                "consumer_target_signature": rec.get("consumer_target_signature"),
                "consumer_call_file": rec.get("consumer_call_file"),
                "consumer_call_line": rec.get("consumer_call_line"),
                "consumer_call_kind": rec.get("consumer_call_kind"),
            })
            found_consumer = True

        if not found_consumer and assigned_local_id:
            # Result assigned to a local variable
            found_consumer = True
            standalone_accesses.append(rec)

        if not found_consumer:
            standalone_accesses.append(rec)

    # Build entries for each consuming Call (grouped property accesses)
    for consumer_call_id, access_infos in consumer_groups.items():
        if count >= limit:
            break
        if consumer_call_id in seen_calls:
            continue
        seen_calls.add(consumer_call_id)

        # Use first access info for representative consumer target data
        first = access_infos[0]
        consumer_target_id = first.get("consumer_target_id")
        consumer_target_fqn = first.get("consumer_target_fqn") or ""
        consumer_target_name = first.get("consumer_target_name")
        consumer_target_kind = first.get("consumer_target_kind")
        consumer_target_sig = first.get("consumer_target_signature")
        call_file = first.get("consumer_call_file")
        call_line = first.get("consumer_call_line")
        call_kind = first.get("consumer_call_kind")

        # Build arguments for this consuming Call
        arguments = _build_arguments_from_records(runner, consumer_call_id)

        # Build flat fields
        ref_type = _infer_ref_type_from_call_kind(call_kind, consumer_target_kind)
        callee = _build_callee_display(consumer_target_name, consumer_target_kind)

        entry = ContextEntry(
            depth=depth,
            node_id=consumer_target_id or consumer_call_id,
            fqn=consumer_target_fqn,
            kind=consumer_target_kind,
            file=call_file,
            line=call_line,
            signature=consumer_target_sig,
            children=[],
            arguments=arguments,
            ref_type=ref_type,
            callee=callee,
        )

        # Depth expansion: cross into callee
        if depth < max_depth and consumer_target_id and consumer_target_kind in ("Method", "Function"):
            cross_into_callee(
                runner, consumer_call_id, consumer_target_id,
                entry, depth, max_depth, limit, visited,
                crossing_count=crossing_count, max_crossings=max_crossings,
            )

        count += 1
        entries.append(entry)

    # === Part 2: Standalone property accesses ===
    for rec in standalone_accesses:
        if count >= limit:
            break
        access_call_id = rec.get("access_call_id")
        if not access_call_id:
            continue
        if access_call_id in seen_calls:
            continue
        seen_calls.add(access_call_id)

        target_id = rec.get("target_id")
        target_fqn = rec.get("target_fqn") or ""
        target_name = rec.get("target_name")
        target_kind = rec.get("target_kind")
        call_file = rec.get("access_call_file")
        call_line = rec.get("access_call_line")
        call_kind = rec.get("access_call_kind")

        ref_type = _infer_ref_type_from_call_kind(call_kind, target_kind)
        callee = _build_callee_display(target_name, target_kind)

        entry = ContextEntry(
            depth=depth,
            node_id=target_id or access_call_id,
            fqn=target_fqn,
            kind=target_kind,
            file=call_file,
            line=call_line,
            children=[],
            ref_type=ref_type,
            callee=callee,
        )
        count += 1
        entries.append(entry)

    # === Part 3: Direct argument edges ===
    for rec in direct_arg_records:
        if count >= limit:
            break
        consumer_call_id = rec.get("consumer_call_id")
        if not consumer_call_id:
            continue
        if consumer_call_id in seen_calls:
            continue
        seen_calls.add(consumer_call_id)

        consumer_target_id = rec.get("consumer_target_id")
        consumer_target_fqn = rec.get("consumer_target_fqn") or ""
        consumer_target_name = rec.get("consumer_target_name")
        consumer_target_kind = rec.get("consumer_target_kind")
        consumer_target_sig = rec.get("consumer_target_signature")
        call_file = rec.get("consumer_call_file")
        call_line = rec.get("consumer_call_line")
        call_kind = rec.get("consumer_call_kind")

        # Build arguments
        arguments = _build_arguments_from_records(runner, consumer_call_id)

        ref_type = _infer_ref_type_from_call_kind(call_kind, consumer_target_kind)
        callee = _build_callee_display(consumer_target_name, consumer_target_kind)

        entry = ContextEntry(
            depth=depth,
            node_id=consumer_target_id or consumer_call_id,
            fqn=consumer_target_fqn,
            kind=consumer_target_kind,
            file=call_file,
            line=call_line,
            signature=consumer_target_sig,
            children=[],
            arguments=arguments,
            ref_type=ref_type,
            callee=callee,
        )

        # Cross into callee for direct arguments too
        if depth < max_depth and consumer_target_id and consumer_target_kind in ("Method", "Function"):
            cross_into_callee(
                runner, consumer_call_id, consumer_target_id,
                entry, depth, max_depth, limit, visited,
                crossing_count=crossing_count, max_crossings=max_crossings,
            )

        count += 1
        entries.append(entry)

    # Sort all entries by (file, line)
    entries.sort(key=lambda e: (e.file or "", e.line if e.line is not None else 0))

    return entries


# =============================================================================
# cross_into_callee
# =============================================================================


def cross_into_callee(
    runner: QueryRunner,
    call_node_id: str,
    callee_id: str,
    entry: ContextEntry,
    depth: int,
    max_depth: int,
    limit: int,
    visited: set[str],
    crossing_count: int = 0,
    max_crossings: int | None = None,
) -> None:
    """Cross method boundary from caller into callee for USED BY tracing.

    For each argument edge with a parameter FQN, finds the matching
    Value(parameter) node in the callee and recursively traces its consumers.

    Also follows the return value path: if the call produces a result
    assigned to a local variable, traces that local's consumers.

    Args:
        runner: Active QueryRunner.
        call_node_id: The Call node in the caller.
        callee_id: The callee Method/Function node ID.
        entry: The ContextEntry to attach children to.
        depth: Current depth level.
        max_depth: Maximum depth.
        limit: Maximum entries.
        visited: Visited Value IDs for cycle detection.
        crossing_count: Number of return crossings so far.
        max_crossings: Maximum return crossings allowed.
    """
    if max_crossings is None:
        max_crossings = min(max_depth, 10)

    # Cross into callee via argument parameter FQNs
    arg_records = runner.execute(Q4_ARGUMENT_PARAMS, call_id=call_node_id)
    for rec in arg_records:
        parameter_fqn = rec.get("parameter_fqn")
        if not parameter_fqn:
            continue

        # Resolve parameter FQN to Value(parameter) node
        param_rec = runner.execute_single(Q5_RESOLVE_PARAM, param_fqn=parameter_fqn)
        if not param_rec:
            continue
        param_value_id = param_rec.get("value_id") if isinstance(param_rec, dict) else param_rec["value_id"]
        if not param_value_id or param_value_id in visited:
            continue

        child_entries = build_value_consumer_chain(
            runner, param_value_id, depth + 1, max_depth, limit, visited,
            crossing_count=crossing_count, max_crossings=max_crossings,
        )
        for ce in child_entries:
            if not ce.crossed_from:
                ce.crossed_from = parameter_fqn
        entry.children.extend(child_entries)

    # Return value path: if callee return flows back to a local in caller
    local_rec = runner.execute_single(Q6_LOCAL_FOR_RESULT, call_id=call_node_id)
    local_id = None
    if local_rec:
        local_id = local_rec.get("local_id") if isinstance(local_rec, dict) else local_rec["local_id"]

    if local_id and local_id not in visited:
        return_entries = build_value_consumer_chain(
            runner, local_id, depth + 1, max_depth, limit, visited,
            crossing_count=crossing_count, max_crossings=max_crossings,
        )
        entry.children.extend(return_entries)
    elif not local_id:
        # No local assignment -- check for return expression crossing
        cross_into_callers_via_return(
            runner, call_node_id, entry, depth, max_depth, limit, visited,
            crossing_count=crossing_count, max_crossings=max_crossings,
        )


# =============================================================================
# cross_into_callers_via_return
# =============================================================================


def cross_into_callers_via_return(
    runner: QueryRunner,
    call_node_id: str,
    entry: ContextEntry,
    depth: int,
    max_depth: int,
    limit: int,
    visited: set[str],
    crossing_count: int = 0,
    max_crossings: int | None = None,
) -> None:
    """Cross from callee return back into caller scope via return value.

    When a Value is consumed by a Call that is the return expression of a method,
    and there is no local variable assigned from the result, this traces the
    return value into caller scopes where the result IS assigned to a local.

    Safety guards:
    - Depth budget: depth >= max_depth -> STOP
    - Crossing limit: crossing_count >= max_crossings -> STOP
    - Method-level cycle prevention via visited set
    - Fan-out cap: callers capped at limit
    - Type guard: consumer result type must match caller local type

    Args:
        runner: Active QueryRunner.
        call_node_id: The consumer Call node ID.
        entry: The ContextEntry to attach children to.
        depth: Current depth level.
        max_depth: Maximum depth.
        limit: Maximum entries.
        visited: Visited Value/method-crossing IDs.
        crossing_count: Number of return crossings so far.
        max_crossings: Maximum return crossings allowed.
    """
    if max_crossings is None:
        max_crossings = min(max_depth, 10)

    # Guard 1: Depth budget
    if depth >= max_depth:
        return

    # Guard 2: Crossing limit
    if crossing_count >= max_crossings:
        return

    # Step 1: Get produced Value(result) from consumer Call
    local_rec = runner.execute_single(Q6_LOCAL_FOR_RESULT, call_id=call_node_id)
    if not local_rec:
        return

    result_id = local_rec.get("result_id") if isinstance(local_rec, dict) else local_rec["result_id"]
    if not result_id:
        return

    # Step 2: Verify no local assignment (inline return check)
    check_local = local_rec.get("local_id") if isinstance(local_rec, dict) else local_rec["local_id"]
    if check_local:
        return  # Has a local assignment, not an inline return

    # Step 3: TYPE GUARD -- get consumer result type_of
    type_rec = runner.execute_single(Q7_TYPE_OF, value_id=result_id)
    if not type_rec:
        return  # No type info, don't cross (conservative)
    consumer_type_id = type_rec.get("type_id") if isinstance(type_rec, dict) else type_rec["type_id"]
    if not consumer_type_id:
        return

    # Step 4: Find containing method of the consumer Call
    method_rec = runner.execute_single(Q12_CONTAINING_METHOD, node_id=call_node_id)
    if not method_rec:
        return
    containing_method_id = method_rec.get("method_id") if isinstance(method_rec, dict) else method_rec["method_id"]
    if not containing_method_id:
        return

    # Guard 3: Method-level cycle prevention
    method_key = f"return_crossing:{containing_method_id}"
    if method_key in visited:
        return

    # Step 5: Find callers of the containing method
    caller_records = runner.execute(Q8_METHOD_CALLERS, method_id=containing_method_id)

    # Guard 4: Fan-out cap
    for rec in caller_records[:limit]:
        caller_local_id = rec.get("caller_local_id")
        if not caller_local_id or caller_local_id in visited:
            continue

        # Guard 5: Type matching
        caller_type_id = rec.get("caller_type_id")
        if caller_type_id != consumer_type_id:
            continue

        # Mark method as visited NOW (lazy marking after type match)
        visited.add(method_key)

        # Continue tracing from caller's local (increment crossing_count)
        return_entries = build_value_consumer_chain(
            runner, caller_local_id, depth + 1, max_depth, limit, visited,
            crossing_count=crossing_count + 1, max_crossings=max_crossings,
        )

        # Set crossed_from to show caller method FQN
        caller_method_fqn = rec.get("caller_method_fqn")
        if caller_method_fqn:
            for child_entry in return_entries:
                if not child_entry.crossed_from:
                    child_entry.crossed_from = caller_method_fqn

        entry.children.extend(return_entries)


# =============================================================================
# build_value_source_chain
# =============================================================================


def build_value_source_chain(
    runner: QueryRunner,
    value_id: str,
    depth: int,
    max_depth: int,
    limit: int,
    visited: set[str] | None = None,
) -> list[ContextEntry]:
    """Build source chain for a Value node (USES section).

    Traces backwards: $savedOrder <- save($processedOrder) <- process($order) <- new Order(...)
    Each depth level follows assigned_from -> produces -> Call, then recursively
    traces the Call's argument Values' source chains.

    For parameter Values: crosses method boundary to find callers via argument
    edges with matching parameter FQN.

    Args:
        runner: Active QueryRunner.
        value_id: ID of the Value node to trace from.
        depth: Current depth level.
        max_depth: Maximum depth for recursion.
        limit: Maximum number of entries.
        visited: Set of visited Value IDs for cycle detection.

    Returns:
        List of ContextEntry instances representing the source chain.
    """
    if depth > max_depth:
        return []

    if visited is None:
        visited = set()
    if value_id in visited:
        return []
    visited.add(value_id)

    # Fetch source chain data
    source_data = fetch_value_source_data(runner, value_id)
    if not source_data:
        return []

    value_kind = source_data.get("value_kind")
    value_fqn = source_data.get("value_fqn")

    # Parameter Values: cross method boundary to find callers
    if value_kind == "parameter":
        return build_parameter_uses(
            runner, value_id, value_fqn or "", depth, max_depth, limit, visited
        )

    # Follow assigned_from to source
    source_id = source_data.get("source_id")

    # If no assigned_from and value is result: result IS the source
    if not source_id and value_kind == "result":
        source_id = value_id

    if not source_id:
        return []

    # Find the Call that produced the source Value
    call_id = source_data.get("call_id")
    if not call_id:
        return []

    # Get call target (callee method/constructor)
    callee_id = source_data.get("callee_id")
    callee_fqn = source_data.get("callee_fqn") or ""
    callee_name = source_data.get("callee_name")
    callee_kind = source_data.get("callee_kind")
    callee_signature = source_data.get("callee_signature")
    call_file = source_data.get("call_file")
    call_line = source_data.get("call_line")
    call_kind_val = source_data.get("call_kind")

    if not callee_id:
        return []

    # Build reference type
    ref_type = _infer_ref_type_from_call_kind(call_kind_val, callee_kind)

    # Build receiver info for access chain
    recv_value_kind = source_data.get("recv_value_kind")
    recv_name = source_data.get("recv_name")
    recv_prop_fqn = source_data.get("recv_prop_fqn")

    on = recv_name
    on_kind = _resolve_on_kind(recv_value_kind)
    if recv_prop_fqn:
        on = recv_prop_fqn
        on_kind = "property"

    # Build member ref
    member_ref = MemberRef(
        target_name=_build_callee_display(callee_name, callee_kind) or "",
        target_fqn=callee_fqn,
        target_kind=callee_kind,
        file=call_file,
        line=call_line,
        reference_type=ref_type,
        access_chain=on,
        on_kind=on_kind,
    )

    # Build arguments
    arguments = _build_arguments_from_records(runner, call_id)

    entry = ContextEntry(
        depth=depth,
        node_id=callee_id,
        fqn=callee_fqn,
        kind=callee_kind,
        file=call_file,
        line=call_line,
        signature=callee_signature,
        children=[],
        member_ref=member_ref,
        arguments=arguments,
        entry_type="call",
    )

    # Recursively trace each argument's source chain at depth+1
    if depth < max_depth:
        for arg in arguments:
            if arg.value_ref_symbol:
                # Find Value node by FQN and trace its source
                param_rec = runner.execute_single(Q5_RESOLVE_PARAM, param_fqn=arg.value_ref_symbol)
                if param_rec:
                    arg_val_id = param_rec.get("value_id") if isinstance(param_rec, dict) else param_rec["value_id"]
                    if arg_val_id and arg_val_id not in visited:
                        children = build_value_source_chain(
                            runner, arg_val_id, depth + 1, max_depth, limit, visited
                        )
                        entry.children.extend(children)

    return [entry]


# =============================================================================
# build_parameter_uses
# =============================================================================


def build_parameter_uses(
    runner: QueryRunner,
    param_value_id: str,
    param_fqn: str,
    depth: int,
    max_depth: int,
    limit: int,
    visited: set[str],
) -> list[ContextEntry]:
    """Find callers of a parameter Value via argument edges matching parameter FQN.

    Searches all argument edges where the `parameter` field matches this Value's
    FQN, then traces the source of each caller's argument Value.

    Args:
        runner: Active QueryRunner.
        param_value_id: ID of the parameter Value node.
        param_fqn: FQN of the parameter Value.
        depth: Current depth level.
        max_depth: Maximum depth.
        limit: Maximum entries.
        visited: Visited Value IDs for cycle detection.

    Returns:
        List of ContextEntry representing caller-provided sources.
    """
    records = runner.execute(Q10_PARAMETER_USES, param_fqn=param_fqn)
    entries: list[ContextEntry] = []

    for rec in records:
        if len(entries) >= limit:
            break

        call_id = rec.get("call_id")
        if not call_id:
            continue

        scope_id = rec.get("scope_id")
        scope_fqn = rec.get("scope_fqn") or ""
        scope_kind = rec.get("scope_kind")
        scope_signature = rec.get("scope_signature")
        call_file = rec.get("call_file")
        call_line = rec.get("call_line")
        caller_value_id = rec.get("value_id")

        entry = ContextEntry(
            depth=depth,
            node_id=scope_id or call_id,
            fqn=scope_fqn,
            kind=scope_kind,
            file=call_file,
            line=call_line,
            signature=scope_signature,
            children=[],
            crossed_from=param_fqn,
        )

        # Trace the caller's argument Value's source chain at depth+1
        if depth < max_depth and caller_value_id and caller_value_id not in visited:
            child_entries = build_value_source_chain(
                runner, caller_value_id, depth + 1, max_depth, limit, visited
            )
            entry.children.extend(child_entries)

        entries.append(entry)

    # Sort by (file, line)
    entries.sort(key=lambda e: (e.file or "", e.line if e.line is not None else 0))
    return entries
