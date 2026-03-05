"""Strategy pattern handlers for class USED BY edge classification.

Ported from kloc-cli/src/queries/used_by_handlers.py.
Each handler processes one reference type and appends entries to EntryBucket.

Key difference from kloc-cli: EdgeContext carries pre-fetched data from
Cypher queries instead of holding a SoTIndex reference.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Protocol, Optional


@dataclass(frozen=True)
class EdgeContext:
    """Immutable context for a single edge being classified.

    Replaces SoTIndex with pre-fetched data from Cypher queries.
    """

    start_id: str           # The target node being queried (class/interface)
    source_id: str          # Source of the edge (who uses the target)
    source_kind: str        # Source node kind
    source_fqn: str         # Source node FQN
    source_name: str        # Source node name
    source_file: Optional[str]
    source_start_line: Optional[int]
    source_signature: Optional[str]
    target_kind: str        # Target node kind
    target_fqn: str         # Target node FQN
    target_name: str        # Target node name
    ref_type: str           # Classified reference type
    file: Optional[str]     # Edge location file
    line: Optional[int]     # Edge location line
    call_node_id: Optional[str]  # Associated Call node if found
    classes_with_injection: frozenset[str]  # Classes with property_type injection

    # Pre-fetched containment data
    containing_method_id: Optional[str] = None
    containing_method_fqn: Optional[str] = None
    containing_method_kind: Optional[str] = None
    containing_class_id: Optional[str] = None
    containing_class_fqn: Optional[str] = None
    containing_class_kind: Optional[str] = None
    containing_class_file: Optional[str] = None
    containing_class_start_line: Optional[int] = None

    # Pre-fetched property data (for PropertyTypeHandler)
    resolved_property_id: Optional[str] = None
    resolved_property_fqn: Optional[str] = None
    resolved_property_file: Optional[str] = None
    resolved_property_line: Optional[int] = None

    # Pre-fetched receiver data (for MethodCallHandler, PropertyAccessHandler)
    on_expr: Optional[str] = None
    on_kind: Optional[str] = None

    # Pre-fetched arguments
    arguments: list = field(default_factory=list)


@dataclass
class EntryBucket:
    """Mutable collector for classified USED BY entries with dedup tracking."""

    instantiation: list[dict] = field(default_factory=list)
    extends: list[dict] = field(default_factory=list)
    property_type: list[dict] = field(default_factory=list)
    method_call: list[dict] = field(default_factory=list)
    property_access_groups: dict[str, list[dict]] = field(default_factory=dict)
    param_return: list[dict] = field(default_factory=list)

    # Dedup tracking
    seen_instantiation_methods: set[str] = field(default_factory=set)
    seen_property_type_props: set[str] = field(default_factory=set)


class UsedByHandler(Protocol):
    """Protocol for USED BY edge handlers."""

    def handle(self, ctx: EdgeContext, bucket: EntryBucket) -> None:
        """Process an edge and append entries to the appropriate bucket."""
        ...


class InstantiationHandler:
    """Handle [instantiation] edges -- new ClassName() calls."""

    def handle(self, ctx: EdgeContext, bucket: EntryBucket) -> None:
        method_key = ctx.containing_method_id or ctx.source_id

        if method_key in bucket.seen_instantiation_methods:
            return
        bucket.seen_instantiation_methods.add(method_key)

        if ctx.containing_method_id and ctx.containing_method_fqn:
            entry_fqn = ctx.containing_method_fqn
            entry_kind = ctx.containing_method_kind or ctx.source_kind
        else:
            entry_fqn = ctx.source_fqn
            entry_kind = ctx.source_kind

        if entry_kind == "Method" and not entry_fqn.endswith("()"):
            entry_fqn += "()"

        entry = {
            "depth": 1,
            "node_id": method_key,
            "fqn": entry_fqn,
            "kind": entry_kind,
            "file": ctx.file,
            "line": ctx.line,
            "ref_type": "instantiation",
            "children": [],
            "arguments": ctx.arguments,
        }
        bucket.instantiation.append(entry)


class ExtendsHandler:
    """Handle [extends] edges -- class inheritance."""

    def handle(self, ctx: EdgeContext, bucket: EntryBucket) -> None:
        entry = {
            "depth": 1,
            "node_id": ctx.source_id,
            "fqn": ctx.source_fqn,
            "kind": ctx.source_kind,
            "file": ctx.source_file,
            "line": ctx.source_start_line,
            "ref_type": "extends",
            "children": [],
        }
        bucket.extends.append(entry)


class ImplementsHandler:
    """Handle [implements] edges -- interface implementation."""

    def handle(self, ctx: EdgeContext, bucket: EntryBucket) -> None:
        entry = {
            "depth": 1,
            "node_id": ctx.source_id,
            "fqn": ctx.source_fqn,
            "kind": ctx.source_kind,
            "file": ctx.source_file,
            "line": ctx.source_start_line,
            "ref_type": "implements",
            "children": [],
        }
        # NOTE: implements entries go into bucket.extends (same list as extends)
        bucket.extends.append(entry)


class PropertyTypeHandler:
    """Handle [property_type] edges -- typed property declarations."""

    def handle(self, ctx: EdgeContext, bucket: EntryBucket) -> None:
        prop_fqn = None
        prop_id = None
        prop_file = None
        prop_line = None

        if ctx.source_kind == "Property":
            prop_fqn = ctx.source_fqn
            prop_id = ctx.source_id
            prop_file = ctx.source_file
            prop_line = ctx.source_start_line
        elif ctx.source_kind in ("Method", "Function"):
            # Use pre-fetched resolved property data
            if ctx.resolved_property_fqn:
                prop_fqn = ctx.resolved_property_fqn
                prop_id = ctx.resolved_property_id
                prop_file = ctx.resolved_property_file
                prop_line = ctx.resolved_property_line

        if prop_fqn and prop_id and prop_fqn not in bucket.seen_property_type_props:
            bucket.seen_property_type_props.add(prop_fqn)
            entry = {
                "depth": 1,
                "node_id": prop_id,
                "fqn": prop_fqn,
                "kind": "Property",
                "file": prop_file,
                "line": prop_line,
                "ref_type": "property_type",
                "children": [],
            }
            bucket.property_type.append(entry)


class MethodCallHandler:
    """Handle [method_call] edges -- method invocations on injected properties."""

    def handle(self, ctx: EdgeContext, bucket: EntryBucket) -> None:
        # Suppress method_call if the containing class has a property_type
        # injection for this target class (those calls show at depth 2)
        if ctx.containing_class_id and ctx.containing_class_id in ctx.classes_with_injection:
            return

        callee_name = None
        if ctx.target_kind == "Method":
            callee_name = ctx.target_name + "()"

        if ctx.containing_method_id and ctx.containing_method_fqn:
            method_fqn = ctx.containing_method_fqn
            method_kind = ctx.containing_method_kind or ctx.source_kind
        else:
            method_fqn = ctx.source_fqn
            method_kind = ctx.source_kind

        if method_kind == "Method" and not method_fqn.endswith("()"):
            method_fqn += "()"

        entry = {
            "depth": 1,
            "node_id": ctx.containing_method_id or ctx.source_id,
            "fqn": method_fqn,
            "kind": method_kind,
            "file": ctx.file,
            "line": ctx.line,
            "ref_type": "method_call",
            "callee": callee_name,
            "on": ctx.on_expr,
            "on_kind": ctx.on_kind,
            "children": [],
            "arguments": ctx.arguments,
        }
        bucket.method_call.append(entry)


class PropertyAccessHandler:
    """Handle [property_access] edges -- grouped by property FQN and method."""

    def handle(self, ctx: EdgeContext, bucket: EntryBucket) -> None:
        prop_fqn = ctx.target_fqn

        if ctx.containing_method_id and ctx.containing_method_fqn:
            method_fqn = ctx.containing_method_fqn
            method_kind = ctx.containing_method_kind or ctx.source_kind
        else:
            method_fqn = ctx.source_fqn
            method_kind = ctx.source_kind

        if prop_fqn not in bucket.property_access_groups:
            bucket.property_access_groups[prop_fqn] = []

        found = False
        for group_entry in bucket.property_access_groups[prop_fqn]:
            if (group_entry["method_fqn"] == method_fqn
                    and group_entry["on_expr"] == ctx.on_expr
                    and group_entry["on_kind"] == ctx.on_kind):
                group_entry["lines"].append(ctx.line)
                found = True
                break

        if not found:
            bucket.property_access_groups[prop_fqn].append({
                "method_fqn": method_fqn,
                "method_id": ctx.containing_method_id or ctx.source_id,
                "method_kind": method_kind,
                "lines": [ctx.line],
                "on_expr": ctx.on_expr,
                "on_kind": ctx.on_kind,
                "file": ctx.file,
            })


class ParamReturnHandler:
    """Handle [parameter_type], [return_type], [type_hint] edges."""

    def handle(self, ctx: EdgeContext, bucket: EntryBucket) -> None:
        # For return_type, show method-level FQN instead of class-level
        if ctx.ref_type == "return_type" and ctx.source_kind in ("Method", "Function"):
            method_fqn = ctx.source_fqn
            if ctx.source_kind == "Method" and not method_fqn.endswith("()"):
                method_fqn += "()"
            already_exists = any(e["fqn"] == method_fqn for e in bucket.param_return)
            if not already_exists:
                entry = {
                    "depth": 1,
                    "node_id": ctx.source_id,
                    "fqn": method_fqn,
                    "kind": ctx.source_kind,
                    "file": ctx.source_file,
                    "line": ctx.source_start_line,
                    "signature": ctx.source_signature,
                    "ref_type": ctx.ref_type,
                    "children": [],
                }
                bucket.param_return.append(entry)
            return

        # Group by containing class -- traverse up to find class
        cls_id = ctx.containing_class_id
        cls_fqn = ctx.containing_class_fqn
        cls_kind = ctx.containing_class_kind
        cls_file = ctx.containing_class_file
        cls_line = ctx.containing_class_start_line

        # If we don't have class data, try to use source directly
        if not cls_fqn:
            if ctx.source_kind in ("Class", "Interface", "Trait", "Enum"):
                cls_id = ctx.source_id
                cls_fqn = ctx.source_fqn
                cls_kind = ctx.source_kind
                cls_file = ctx.source_file
                cls_line = ctx.source_start_line
            else:
                # Skip file-level references and non-class nodes
                return

        if cls_fqn is None:
            return

        already_exists = any(e["fqn"] == cls_fqn for e in bucket.param_return)
        if not already_exists:
            entry = {
                "depth": 1,
                "node_id": cls_id,
                "fqn": cls_fqn,
                "kind": cls_kind,
                "file": cls_file,
                "line": cls_line,
                "ref_type": ctx.ref_type,
                "children": [],
            }
            bucket.param_return.append(entry)


# Handler registry: maps ref_type to handler instance
_param_return_handler = ParamReturnHandler()

USED_BY_HANDLERS: dict[str, UsedByHandler] = {
    "instantiation": InstantiationHandler(),
    "extends": ExtendsHandler(),
    "implements": ImplementsHandler(),
    "property_type": PropertyTypeHandler(),
    "method_call": MethodCallHandler(),
    "property_access": PropertyAccessHandler(),
    "parameter_type": _param_return_handler,
    "return_type": _param_return_handler,
    "type_hint": _param_return_handler,
}
