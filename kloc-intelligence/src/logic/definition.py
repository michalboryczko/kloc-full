"""Definition builders for the DEFINITION section of context output.

Ported from kloc-cli/src/queries/definition.py.
All functions accept pre-fetched data dicts instead of SoTIndex.
"""

from __future__ import annotations

import re
from typing import Optional

from ..models.results import DefinitionInfo


def build_definition(data: dict) -> DefinitionInfo:
    """Build definition metadata from pre-fetched Cypher data.

    Args:
        data: Pre-fetched node data dict with keys:
            fqn, kind, file, start_line, signature, name, documentation,
            parent_fqn, parent_kind, parent_file, parent_line,
            children, type_hints, inheritance, constructor_deps, value_data

    Returns:
        DefinitionInfo with symbol metadata.
    """
    info = DefinitionInfo(
        fqn=data["fqn"],
        kind=data["kind"],
        file=data.get("file"),
        line=data.get("start_line"),
        signature=data.get("signature"),
    )

    # Containing parent
    if data.get("parent_fqn"):
        info.declared_in = {
            "fqn": data["parent_fqn"],
            "kind": data["parent_kind"],
            "file": data.get("parent_file"),
            "line": data.get("parent_line"),
        }

    # Dispatch to kind-specific builder
    kind = data["kind"]
    if kind in ("Method", "Function"):
        _build_method_definition(data, info)
    elif kind in ("Class", "Trait", "Enum"):
        _build_class_definition(data, info)
    elif kind == "Interface":
        _build_interface_definition(data, info)
    elif kind == "Property":
        _build_property_definition(data, info)
    elif kind == "Argument":
        _build_argument_definition(data, info)
    elif kind == "Value":
        _build_value_definition(data, info)

    return info


def _build_method_definition(data: dict, info: DefinitionInfo) -> None:
    """Populate definition for Method/Function nodes."""
    children = data.get("children", [])
    type_hints = data.get("type_hints", {})

    # Collect typed arguments
    for child in children:
        if child.get("kind") == "Argument":
            arg_dict: dict = {"name": child["name"], "position": None}
            child_id = child.get("id")
            if child_id and child_id in type_hints:
                hints = type_hints[child_id]
                if hints:
                    arg_dict["type"] = hints[0].get("target_name")
            info.arguments.append(arg_dict)

    # Resolve return type from type_hint edges on the method itself
    node_id = data.get("id")
    if node_id and node_id in type_hints:
        hints = type_hints[node_id]
        if hints:
            info.return_type = {
                "fqn": hints[0].get("target_fqn"),
                "name": hints[0].get("target_name"),
            }


def _build_class_definition(data: dict, info: DefinitionInfo) -> None:
    """Populate definition for Class/Trait/Enum nodes."""
    children = data.get("children", [])
    type_hints = data.get("type_hints", {})
    inheritance = data.get("inheritance", {})
    constructor_deps_data = data.get("constructor_deps", [])

    for child in children:
        child_kind = child.get("kind")
        child_id = child.get("id")

        if child_kind == "Property":
            prop_dict: dict = {"name": child["name"]}
            # Type from type_hint edges
            if child_id and child_id in type_hints:
                hints = type_hints[child_id]
                if hints:
                    prop_dict["type"] = hints[0].get("target_name")

            # Parse property metadata from documentation
            doc = child.get("documentation", [])
            vis, readonly, static, doc_type = parse_property_doc(
                child["name"], doc
            )
            if vis:
                prop_dict["visibility"] = vis
            if readonly:
                prop_dict["readonly"] = True
            if static:
                prop_dict["static"] = True

            # If no class type from edges, use type from docs
            if "type" not in prop_dict and doc_type:
                prop_dict["type"] = doc_type

            # Detect promoted from constructor_deps_data
            for dep in constructor_deps_data:
                if dep.get("prop_fqn") == child.get("fqn"):
                    prop_dict["promoted"] = True
                    break

            info.properties.append(prop_dict)

        elif child_kind == "Method":
            # Skip __construct -- implied by promoted properties
            if child["name"] == "__construct":
                continue

            method_dict: dict = {"name": child["name"]}
            if child.get("signature"):
                method_dict["signature"] = child["signature"]

            # Method tags: [override], [abstract]
            tags: list[str] = []
            if child.get("has_override"):
                tags.append("override")
            doc = child.get("documentation", [])
            if _is_abstract_from_doc(doc):
                tags.append("abstract")

            if tags:
                method_dict["tags"] = tags
            info.methods.append(method_dict)

    # Sort methods: override first, then inherited, then regular
    def _method_sort_key(m: dict) -> int:
        tags = m.get("tags", [])
        if "override" in tags:
            return 0
        if "inherited" in tags:
            return 1
        return 2
    info.methods.sort(key=_method_sort_key)

    # Constructor deps
    for dep in constructor_deps_data:
        dep_entry: dict = {"name": dep["prop_name"]}
        if dep.get("type_name"):
            dep_entry["type"] = dep["type_name"]
        else:
            # Try scalar type from property docs
            prop_name = dep["prop_name"]
            for child in children:
                if child.get("kind") == "Property" and child.get("name") == prop_name:
                    _, _, _, doc_type = parse_property_doc(
                        prop_name, child.get("documentation", [])
                    )
                    if doc_type:
                        dep_entry["type"] = doc_type
                    break
        info.constructor_deps.append(dep_entry)

    # Inheritance
    if inheritance.get("extends_fqn"):
        info.extends = inheritance["extends_fqn"]
    for impl_fqn in inheritance.get("implements_fqns", []):
        if impl_fqn:
            info.implements.append(impl_fqn)
    for trait_fqn in inheritance.get("uses_trait_fqns", []):
        if trait_fqn:
            info.uses_traits.append(trait_fqn)


def _build_interface_definition(data: dict, info: DefinitionInfo) -> None:
    """Populate definition for Interface nodes.

    Shows method signatures only (no properties, no implements).
    Shows extends if interface extends another interface.
    """
    children = data.get("children", [])
    inheritance = data.get("inheritance", {})

    for child in children:
        if child.get("kind") == "Method":
            method_dict: dict = {"name": child["name"]}
            if child.get("signature"):
                method_dict["signature"] = child["signature"]
            info.methods.append(method_dict)

    # Interface extends (interface extending interface)
    if inheritance.get("extends_fqn"):
        info.extends = inheritance["extends_fqn"]


def _build_property_definition(data: dict, info: DefinitionInfo) -> None:
    """Populate definition for Property nodes."""
    type_hints = data.get("type_hints", {})
    node_id = data.get("id")

    # Type from type_hint edges (class types)
    if node_id and node_id in type_hints:
        hints = type_hints[node_id]
        if hints:
            info.return_type = {
                "fqn": hints[0].get("target_fqn"),
                "name": hints[0].get("target_name"),
            }

    # Parse visibility, readonly, static, and scalar type from documentation
    doc = data.get("documentation", [])
    vis, readonly, static, doc_type = parse_property_doc(data.get("name", ""), doc)
    if vis:
        if not info.return_type:
            info.return_type = {}
        info.return_type["visibility"] = vis
    if readonly:
        if not info.return_type:
            info.return_type = {}
        info.return_type["readonly"] = True
    if static:
        if not info.return_type:
            info.return_type = {}
        info.return_type["static"] = True

    # If property itself isn't readonly, check if containing class is readonly
    if not readonly:
        parent_doc = data.get("parent_documentation", [])
        if parent_doc:
            for pdoc in parent_doc:
                if "readonly class" in pdoc or "readonly " in pdoc:
                    readonly = True
                    break
        if readonly:
            if not info.return_type:
                info.return_type = {}
            info.return_type["readonly"] = True

    # If no class type from edges, use type from documentation
    if not info.return_type or "name" not in info.return_type:
        if doc_type:
            if info.return_type is None:
                info.return_type = {}
            info.return_type["name"] = doc_type
            info.return_type["fqn"] = doc_type

    # Detect promoted from pre-fetched data
    if data.get("is_promoted"):
        if not info.return_type:
            info.return_type = {}
        info.return_type["promoted"] = True


def _build_argument_definition(data: dict, info: DefinitionInfo) -> None:
    """Populate definition for Argument nodes."""
    type_hints = data.get("type_hints", {})
    node_id = data.get("id")

    if node_id and node_id in type_hints:
        hints = type_hints[node_id]
        if hints:
            info.return_type = {
                "fqn": hints[0].get("target_fqn"),
                "name": hints[0].get("target_name"),
            }


def _build_value_definition(data: dict, info: DefinitionInfo) -> None:
    """Populate definition for Value nodes with data flow metadata."""
    info.value_kind = data.get("value_kind")

    value_data = data.get("value_data", {})

    # Type resolution
    type_of_all = value_data.get("type_of_all", [])
    if type_of_all:
        type_names = [t["name"] for t in type_of_all if t.get("name")]
        if type_names:
            first = type_of_all[0]
            if len(type_of_all) == 1:
                info.type_info = {"fqn": first.get("fqn"), "name": first.get("name")}
            else:
                info.type_info = {
                    "fqn": "|".join(t.get("fqn", "") for t in type_of_all if t.get("fqn")),
                    "name": "|".join(type_names),
                }

    # Source resolution
    source_data = value_data.get("source")
    if source_data:
        info.source = source_data

    # Scope: resolve containing method/function
    scope_data = value_data.get("scope")
    if scope_data and not info.declared_in:
        info.declared_in = scope_data


def parse_property_doc(
    name: str, documentation: list[str]
) -> tuple[Optional[str], bool, bool, Optional[str]]:
    """Parse property documentation for visibility, readonly, static, type.

    SCIP documentation for properties looks like:
        ```php\\npublic string $customerEmail\\n```
        ```php\\nprivate static array $sentEmails = []\\n```
        ```php\\nprivate readonly \\App\\Service\\CustomerService $customerService\\n```

    Args:
        name: Property name (without $).
        documentation: List of documentation strings.

    Returns:
        (visibility, readonly, static, scalar_type)
    """
    visibility = None
    readonly = False
    static = False
    doc_type = None

    if not documentation:
        return visibility, readonly, static, doc_type

    for doc in documentation:
        clean = doc.replace("```php", "").replace("```", "").strip()
        if not clean:
            continue
        for line in clean.split("\n"):
            line = line.strip()
            if name not in line:
                continue
            # Extract visibility
            if line.startswith("public "):
                visibility = "public"
            elif line.startswith("protected "):
                visibility = "protected"
            elif line.startswith("private "):
                visibility = "private"
            # Check modifiers
            if " readonly " in line or line.startswith("readonly "):
                readonly = True
            if " static " in line or line.startswith("static "):
                static = True
            # Extract type
            match = re.search(
                r'(?:public|protected|private)?\s*(?:static\s+)?(?:readonly\s+)?(\S+)\s+\$',
                line
            )
            if match:
                raw_type = match.group(1)
                if raw_type not in ("public", "protected", "private", "static", "readonly"):
                    if raw_type.startswith("\\"):
                        raw_type = raw_type.lstrip("\\")
                    doc_type = raw_type.rsplit("\\", 1)[-1] if "\\" in raw_type else raw_type
            break
        if visibility or doc_type:
            break

    return visibility, readonly, static, doc_type


def _is_abstract_from_doc(documentation: list[str]) -> bool:
    """Check if method is abstract from documentation."""
    if not documentation:
        return False
    for doc in documentation:
        clean = doc.replace("```php", "").replace("```", "").strip()
        for line in clean.split("\n"):
            line = line.strip()
            if "function " in line and "abstract " in line:
                return True
    return False
