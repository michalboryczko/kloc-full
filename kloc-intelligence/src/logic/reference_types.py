"""Reference type inference engine.

Ported from kloc-cli/src/queries/reference_types.py.
Classifies how a source node references a target node based on
edge type, node kinds, and type_hint edges.

Key difference from kloc-cli: receives pre-fetched data instead of
accessing SoTIndex directly. All graph lookups are done in Cypher
before this function is called.
"""

from __future__ import annotations

from typing import Optional


# =============================================================================
# Depth Chaining Rules (R8)
# =============================================================================
# Only these reference types represent actual call/data flow relationships
# and should be followed when expanding USED BY depth N -> N+1.
# Structural/declarative references (type_hint, extends, implements, use_trait)
# are leaf nodes -- they do not imply that callers of the source are callers
# of the target.
CHAINABLE_REFERENCE_TYPES = {"method_call", "property_access", "instantiation", "static_call"}


# =============================================================================
# Reference Type Priority (for USED BY entry ordering)
# =============================================================================
REF_TYPE_PRIORITY = {
    "instantiation": 0,
    "extends": 1,
    "implements": 1,
    "property_type": 2,
    "method_call": 3,
    "static_call": 3,
    "property_access": 4,
    "parameter_type": 5,
    "return_type": 5,
    "type_hint": 6,
}


def infer_reference_type(
    edge_type: str,
    target_kind: Optional[str],
    source_kind: Optional[str],
    source_name: Optional[str] = None,
    *,
    has_arg_type_hint: bool = False,
    has_return_type_hint: bool = False,
    has_class_property_type_hint: bool = False,
) -> str:
    """Infer reference type from edge and node metadata.

    Ported from kloc-cli's _infer_reference_type() with pre-fetched data
    replacing SoTIndex lookups.

    Args:
        edge_type: Relationship type (uses, extends, implements, uses_trait).
        target_kind: Target node kind (Class, Method, Property, etc.).
        source_kind: Source node kind.
        source_name: Source node name (needed for constructor promotion check).
        has_arg_type_hint: True if any Argument child of source has type_hint to target.
        has_return_type_hint: True if source method has type_hint to target.
        has_class_property_type_hint: True if parent class's Property child has
            type_hint to target (constructor promotion or class-level property).

    Returns:
        Reference type string (e.g., "method_call", "parameter_type").
    """
    # Direct edge type mappings
    if edge_type == "extends":
        return "extends"
    if edge_type == "implements":
        return "implements"
    if edge_type == "uses_trait":
        return "uses_trait"

    # For 'uses' edges, infer from target node kind
    if edge_type == "uses" and target_kind:
        if target_kind == "Method":
            return "method_call"
        if target_kind == "Property":
            return "property_access"
        if target_kind in ("Class", "Interface", "Trait", "Enum"):
            if source_kind:
                if source_kind == "Argument":
                    return "parameter_type"
                if source_kind == "Property":
                    return "property_type"
                if source_kind in ("Method", "Function"):
                    if has_arg_type_hint:
                        return "parameter_type"
                    if has_return_type_hint:
                        return "return_type"
                    # Constructor promotion: __construct with no arg match,
                    # check parent class Property children
                    if source_name == "__construct" and has_class_property_type_hint:
                        return "property_type"
                if source_kind in ("Class", "Interface", "Trait", "Enum"):
                    if has_class_property_type_hint:
                        return "property_type"
            return "type_hint"
        if target_kind == "Constant":
            return "constant_access"
        if target_kind == "Function":
            return "function_call"
        if target_kind == "Argument":
            return "argument_ref"
        if target_kind == "Variable":
            return "variable_ref"

    # TYPE_HINT edges: classify by source node kind
    if edge_type == "type_hint" and source_kind:
        if source_kind == "Argument":
            return "parameter_type"
        if source_kind == "Property":
            return "property_type"
        if source_kind in ("Method", "Function"):
            return "return_type"
        return "type_hint"

    # Fallback for unknown edge types or missing target node
    return "uses"


def get_reference_type_from_call_kind(call_kind: Optional[str]) -> str:
    """Map a Call node's call_kind to a reference type string.

    Args:
        call_kind: Call node kind (method, constructor, access, etc.).

    Returns:
        Reference type string.
    """
    kind_map = {
        "method": "method_call",
        "method_static": "static_call",
        "constructor": "instantiation",
        "access": "property_access",
        "access_static": "static_property",
        "function": "function_call",
    }
    return kind_map.get(call_kind or "", "unknown")


def sort_entries_by_priority(entries: list, ref_type_attr: str = "ref_type") -> list:
    """Sort entries by reference type priority, then file/line.

    Args:
        entries: List of entries (dicts or objects with ref_type, file, line).
        ref_type_attr: Attribute name for reference type.

    Returns:
        Sorted list.
    """
    def sort_key(e):
        if isinstance(e, dict):
            rt = e.get(ref_type_attr, "")
            f = e.get("file", "") or ""
            ln = e.get("line") if e.get("line") is not None else 0
        else:
            rt = getattr(e, ref_type_attr, "") or ""
            f = getattr(e, "file", "") or ""
            ln = getattr(e, "line", None)
            ln = ln if ln is not None else 0
        pri = REF_TYPE_PRIORITY.get(rt, 10)
        return (pri, f, ln)
    return sorted(entries, key=sort_key)


def sort_entries_by_location(entries: list) -> list:
    """Sort entries by file path + line number.

    Args:
        entries: List of entries (dicts or objects with file, line).

    Returns:
        Sorted list.
    """
    def sort_key(e):
        if isinstance(e, dict):
            f = e.get("file", "") or ""
            ln = e.get("line") if e.get("line") is not None else 0
        else:
            f = getattr(e, "file", "") or ""
            ln = getattr(e, "line", None)
            ln = ln if ln is not None else 0
        return (f, ln)
    return sorted(entries, key=sort_key)
