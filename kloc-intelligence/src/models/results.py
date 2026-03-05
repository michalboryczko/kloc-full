"""Query result types ported from kloc-cli.

Contains ContextEntry (28 fields), MemberRef, ArgumentInfo,
ContextResult, and DefinitionInfo dataclasses.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Optional


@dataclass
class MemberRef:
    """A specific member usage reference within a USES relationship.

    When a source uses members of a class (properties, methods), each reference
    is captured here so the output shows the execution flow.
    """

    target_name: str  # Short display name: "$prop", "method()"
    target_fqn: str  # Full FQN: "App\\Foo::method()"
    target_kind: Optional[str] = None  # "Method", "Property", etc.
    file: Optional[str] = None  # Where the reference occurs
    line: Optional[int] = None  # Line of the reference (0-indexed)
    reference_type: Optional[str] = None  # "method_call", "type_hint", etc.
    access_chain: Optional[str] = None  # "$this->orderRepository" or None
    access_chain_symbol: Optional[str] = None  # "App\\Foo::$orderRepository" or None
    on_kind: Optional[str] = None  # "local" or "param" for Value receivers
    on_file: Optional[str] = None  # File where Value is defined
    on_line: Optional[int] = None  # Line where Value is defined (0-indexed)


@dataclass
class ArgumentInfo:
    """Argument-to-parameter mapping at a call site."""

    position: int  # 0-based argument position
    param_name: Optional[str] = None  # Formal parameter name
    value_expr: Optional[str] = None  # Source expression text
    value_source: Optional[str] = None  # Value kind: "parameter", "local", etc.
    value_type: Optional[str] = None  # Resolved type name(s)
    param_fqn: Optional[str] = None  # Full FQN of callee's Argument node
    value_ref_symbol: Optional[str] = None  # Graph symbol the value resolves to
    source_chain: Optional[list] = None  # Access chain steps


@dataclass
class ContextEntry:
    """Single entry in context tree (used_by or uses).

    28 fields matching kloc-cli's ContextEntry exactly.
    """

    depth: int
    node_id: str
    fqn: str
    kind: Optional[str] = None
    file: Optional[str] = None
    line: Optional[int] = None  # 0-based internally
    signature: Optional[str] = None
    children: list[ContextEntry] = field(default_factory=list)
    implementations: list[ContextEntry] = field(default_factory=list)
    via_interface: bool = False
    member_ref: Optional[MemberRef] = None
    arguments: list[ArgumentInfo] = field(default_factory=list)
    result_var: Optional[str] = None
    entry_type: Optional[str] = None  # "call" or "local_variable"
    variable_name: Optional[str] = None
    variable_symbol: Optional[str] = None
    variable_type: Optional[str] = None
    source_call: Optional[ContextEntry] = None
    crossed_from: Optional[str] = None
    ref_type: Optional[str] = None  # "instantiation", "extends", etc.
    callee: Optional[str] = None
    on: Optional[str] = None  # receiver expression
    on_kind: Optional[str] = None  # "property", "param", "local", "self"
    sites: Optional[list] = None
    via: Optional[str] = None  # FQN of interface
    property_name: Optional[str] = None
    access_count: Optional[int] = None
    method_count: Optional[int] = None


@dataclass
class ContextResult:
    """Result of context query with tree structure."""

    target: "NodeData"  # noqa: F821
    max_depth: int
    used_by: list[ContextEntry] = field(default_factory=list)
    uses: list[ContextEntry] = field(default_factory=list)
    definition: Optional[DefinitionInfo] = None


@dataclass
class DefinitionInfo:
    """Symbol definition metadata for the DEFINITION section.

    Provides structural information about a symbol: its signature, typed
    arguments, return type, containing class, properties, methods, and
    inheritance relationships.
    """

    fqn: str
    kind: str
    file: Optional[str] = None
    line: Optional[int] = None  # 0-based internally
    signature: Optional[str] = None
    arguments: list[dict] = field(default_factory=list)
    return_type: Optional[dict] = None
    declared_in: Optional[dict] = None
    properties: list[dict] = field(default_factory=list)
    methods: list[dict] = field(default_factory=list)
    extends: Optional[str] = None
    implements: list[str] = field(default_factory=list)
    uses_traits: list[str] = field(default_factory=list)
    value_kind: Optional[str] = None
    type_info: Optional[dict] = None
    source: Optional[dict] = None
    constructor_deps: list[dict] = field(default_factory=list)
