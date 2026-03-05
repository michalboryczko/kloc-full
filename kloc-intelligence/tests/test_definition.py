"""Tests for definition builders."""

from __future__ import annotations

from src.logic.definition import (
    build_definition,
    parse_property_doc,
)


def _make_data(**overrides) -> dict:
    """Create a definition data dict with sensible defaults."""
    defaults = {
        "id": "node-1",
        "fqn": "App\\Foo",
        "kind": "Class",
        "file": "src/Foo.php",
        "start_line": 10,
        "name": "Foo",
        "documentation": [],
        "signature": None,
        "value_kind": None,
        "parent_fqn": None,
        "parent_kind": None,
        "parent_file": None,
        "parent_line": None,
        "parent_documentation": [],
        "children": [],
        "type_hints": {},
        "inheritance": {},
        "constructor_deps": [],
    }
    defaults.update(overrides)
    return defaults


class TestBuildDefinition:
    """Test build_definition() dispatcher."""

    def test_basic_class(self):
        data = _make_data(kind="Class", fqn="App\\Foo")
        info = build_definition(data)
        assert info.fqn == "App\\Foo"
        assert info.kind == "Class"
        assert info.file == "src/Foo.php"
        assert info.line == 10

    def test_declared_in_set_when_parent_exists(self):
        data = _make_data(
            kind="Method",
            fqn="App\\Foo::bar()",
            parent_fqn="App\\Foo",
            parent_kind="Class",
            parent_file="src/Foo.php",
            parent_line=5,
        )
        info = build_definition(data)
        assert info.declared_in is not None
        assert info.declared_in["fqn"] == "App\\Foo"
        assert info.declared_in["line"] == 5

    def test_no_declared_in_without_parent(self):
        data = _make_data(kind="Class")
        info = build_definition(data)
        assert info.declared_in is None


class TestBuildMethodDefinition:
    """Test method definition building."""

    def test_collects_typed_arguments(self):
        data = _make_data(
            kind="Method",
            fqn="App\\Foo::bar()",
            children=[
                {"id": "arg-1", "kind": "Argument", "name": "$id", "fqn": "App\\Foo::bar().$id"},
                {"id": "arg-2", "kind": "Argument", "name": "$name", "fqn": "App\\Foo::bar().$name"},
            ],
            type_hints={
                "arg-1": [{"target_id": "t-1", "target_fqn": "int", "target_name": "int"}],
            },
        )
        info = build_definition(data)
        assert len(info.arguments) == 2
        assert info.arguments[0]["name"] == "$id"
        assert info.arguments[0]["type"] == "int"
        assert "type" not in info.arguments[1]

    def test_resolves_return_type(self):
        data = _make_data(
            id="method-1",
            kind="Method",
            fqn="App\\Foo::bar()",
            type_hints={
                "method-1": [{"target_id": "t-1", "target_fqn": "App\\Bar", "target_name": "Bar"}],
            },
        )
        info = build_definition(data)
        assert info.return_type is not None
        assert info.return_type["name"] == "Bar"
        assert info.return_type["fqn"] == "App\\Bar"

    def test_no_return_type_without_hints(self):
        data = _make_data(kind="Method", fqn="App\\Foo::bar()")
        info = build_definition(data)
        assert info.return_type is None


class TestBuildClassDefinition:
    """Test class definition building."""

    def test_collects_properties_with_types(self):
        data = _make_data(
            kind="Class",
            children=[
                {
                    "id": "prop-1", "kind": "Property", "name": "$bar",
                    "fqn": "App\\Foo::$bar",
                    "documentation": ["```php\nprivate string $bar\n```"],
                    "has_override": False,
                },
            ],
            type_hints={},
        )
        info = build_definition(data)
        assert len(info.properties) == 1
        assert info.properties[0]["name"] == "$bar"
        assert info.properties[0]["visibility"] == "private"
        assert info.properties[0]["type"] == "string"

    def test_collects_methods_with_tags(self):
        data = _make_data(
            kind="Class",
            children=[
                {
                    "id": "m-1", "kind": "Method", "name": "bar",
                    "fqn": "App\\Foo::bar()", "signature": "bar()",
                    "documentation": [], "has_override": True,
                },
                {
                    "id": "m-2", "kind": "Method", "name": "baz",
                    "fqn": "App\\Foo::baz()", "signature": "baz()",
                    "documentation": ["```php\nabstract function baz()\n```"],
                    "has_override": False,
                },
            ],
        )
        info = build_definition(data)
        assert len(info.methods) == 2
        # Override-tagged methods sorted first
        assert info.methods[0]["name"] == "bar"
        assert "override" in info.methods[0]["tags"]
        assert info.methods[1]["name"] == "baz"
        assert "abstract" in info.methods[1]["tags"]

    def test_skips_constructor(self):
        data = _make_data(
            kind="Class",
            children=[
                {
                    "id": "m-1", "kind": "Method", "name": "__construct",
                    "fqn": "App\\Foo::__construct()", "signature": "__construct()",
                    "documentation": [], "has_override": False,
                },
                {
                    "id": "m-2", "kind": "Method", "name": "bar",
                    "fqn": "App\\Foo::bar()", "signature": "bar()",
                    "documentation": [], "has_override": False,
                },
            ],
        )
        info = build_definition(data)
        assert len(info.methods) == 1
        assert info.methods[0]["name"] == "bar"

    def test_constructor_deps_promoted(self):
        data = _make_data(
            kind="Class",
            children=[
                {
                    "id": "prop-1", "kind": "Property", "name": "$service",
                    "fqn": "App\\Foo::$service",
                    "documentation": [], "has_override": False,
                },
            ],
            type_hints={
                "prop-1": [{"target_id": "t-1", "target_fqn": "App\\Service", "target_name": "Service"}],
            },
            constructor_deps=[
                {"prop_name": "$service", "prop_fqn": "App\\Foo::$service", "type_name": "Service"},
            ],
        )
        info = build_definition(data)
        assert len(info.constructor_deps) == 1
        assert info.constructor_deps[0]["name"] == "$service"
        assert info.constructor_deps[0]["type"] == "Service"
        # Property should also have promoted flag
        assert info.properties[0].get("promoted") is True

    def test_extends_and_implements(self):
        data = _make_data(
            kind="Class",
            inheritance={
                "extends_fqn": "App\\BaseClass",
                "implements_fqns": ["App\\FooInterface", "App\\BarInterface"],
                "uses_trait_fqns": ["App\\LoggableTrait"],
            },
        )
        info = build_definition(data)
        assert info.extends == "App\\BaseClass"
        assert info.implements == ["App\\FooInterface", "App\\BarInterface"]
        assert info.uses_traits == ["App\\LoggableTrait"]

    def test_method_sort_order(self):
        data = _make_data(
            kind="Class",
            children=[
                {"id": "m-1", "kind": "Method", "name": "regular", "fqn": "r", "signature": None,
                 "documentation": [], "has_override": False},
                {"id": "m-2", "kind": "Method", "name": "overrider", "fqn": "o", "signature": None,
                 "documentation": [], "has_override": True},
            ],
        )
        info = build_definition(data)
        assert info.methods[0]["name"] == "overrider"
        assert info.methods[1]["name"] == "regular"

    def test_property_type_from_edge(self):
        data = _make_data(
            kind="Class",
            children=[
                {
                    "id": "prop-1", "kind": "Property", "name": "$bar",
                    "fqn": "App\\Foo::$bar",
                    "documentation": ["```php\nprivate Baz $bar\n```"],
                    "has_override": False,
                },
            ],
            type_hints={
                "prop-1": [{"target_id": "t-1", "target_fqn": "App\\Baz", "target_name": "Baz"}],
            },
        )
        info = build_definition(data)
        # Type from edge takes precedence over doc
        assert info.properties[0]["type"] == "Baz"


class TestBuildInterfaceDefinition:
    """Test interface definition building."""

    def test_shows_methods_only(self):
        data = _make_data(
            kind="Interface",
            children=[
                {"id": "m-1", "kind": "Method", "name": "foo", "fqn": "I::foo()", "signature": "foo(): void",
                 "documentation": [], "has_override": False},
            ],
            inheritance={"extends_fqn": None, "implements_fqns": [], "uses_trait_fqns": []},
        )
        info = build_definition(data)
        assert len(info.methods) == 1
        assert info.methods[0]["name"] == "foo"
        assert info.methods[0]["signature"] == "foo(): void"
        assert len(info.properties) == 0

    def test_interface_extends(self):
        data = _make_data(
            kind="Interface",
            inheritance={
                "extends_fqn": "App\\ParentInterface",
                "implements_fqns": [],
                "uses_trait_fqns": [],
            },
        )
        info = build_definition(data)
        assert info.extends == "App\\ParentInterface"


class TestBuildPropertyDefinition:
    """Test property definition building."""

    def test_type_from_type_hint_edge(self):
        data = _make_data(
            id="prop-1",
            kind="Property",
            name="$bar",
            documentation=["```php\nprivate Foo $bar\n```"],
            type_hints={
                "prop-1": [{"target_id": "t-1", "target_fqn": "App\\Foo", "target_name": "Foo"}],
            },
        )
        info = build_definition(data)
        assert info.return_type is not None
        assert info.return_type["name"] == "Foo"
        assert info.return_type["fqn"] == "App\\Foo"

    def test_visibility_from_documentation(self):
        data = _make_data(
            kind="Property",
            name="$bar",
            documentation=["```php\nprotected string $bar\n```"],
        )
        info = build_definition(data)
        assert info.return_type is not None
        assert info.return_type["visibility"] == "protected"

    def test_readonly_from_documentation(self):
        data = _make_data(
            kind="Property",
            name="$bar",
            documentation=["```php\nprivate readonly string $bar\n```"],
        )
        info = build_definition(data)
        assert info.return_type["readonly"] is True

    def test_promoted_detected(self):
        data = _make_data(
            kind="Property",
            name="$bar",
            documentation=["```php\nprivate string $bar\n```"],
            is_promoted=True,
        )
        info = build_definition(data)
        assert info.return_type["promoted"] is True

    def test_readonly_class_inheritance(self):
        data = _make_data(
            kind="Property",
            name="$bar",
            documentation=["```php\npublic string $bar\n```"],
            parent_documentation=["```php\nreadonly class Foo\n```"],
        )
        info = build_definition(data)
        assert info.return_type["readonly"] is True

    def test_scalar_type_from_docs_fallback(self):
        data = _make_data(
            id="prop-1",
            kind="Property",
            name="$count",
            documentation=["```php\npublic int $count\n```"],
            type_hints={},  # No class type from edges
        )
        info = build_definition(data)
        assert info.return_type is not None
        assert info.return_type["name"] == "int"


class TestBuildValueDefinition:
    """Test value definition building."""

    def test_value_kind_set(self):
        data = _make_data(
            kind="Value",
            value_kind="local",
            value_data={},
        )
        info = build_definition(data)
        assert info.value_kind == "local"

    def test_type_resolution_single(self):
        data = _make_data(
            kind="Value",
            value_kind="local",
            value_data={
                "type_of_all": [{"fqn": "App\\Order", "name": "Order"}],
            },
        )
        info = build_definition(data)
        assert info.type_info is not None
        assert info.type_info["name"] == "Order"
        assert info.type_info["fqn"] == "App\\Order"

    def test_type_resolution_union(self):
        data = _make_data(
            kind="Value",
            value_kind="local",
            value_data={
                "type_of_all": [
                    {"fqn": "int", "name": "int"},
                    {"fqn": "string", "name": "string"},
                ],
            },
        )
        info = build_definition(data)
        assert info.type_info is not None
        assert info.type_info["name"] == "int|string"
        assert info.type_info["fqn"] == "int|string"

    def test_source_chain_resolution(self):
        data = _make_data(
            kind="Value",
            value_kind="result",
            value_data={
                "source": {
                    "call_fqn": "call#1",
                    "method_fqn": "App\\Repo::find()",
                    "method_name": "find()",
                    "file": "src/Repo.php",
                    "line": 20,
                },
            },
        )
        info = build_definition(data)
        assert info.source is not None
        assert info.source["method_name"] == "find()"

    def test_scope_resolution(self):
        data = _make_data(
            kind="Value",
            value_kind="local",
            value_data={
                "scope": {
                    "fqn": "App\\Foo::bar()",
                    "kind": "Method",
                    "file": "src/Foo.php",
                    "line": 10,
                },
            },
        )
        info = build_definition(data)
        assert info.declared_in is not None
        assert info.declared_in["fqn"] == "App\\Foo::bar()"


class TestParsePropertyDoc:
    """Test parse_property_doc()."""

    def test_private_readonly(self):
        vis, readonly, static, doc_type = parse_property_doc(
            "$bar", ["```php\nprivate readonly \\App\\Service\\Foo $bar\n```"]
        )
        assert vis == "private"
        assert readonly is True
        assert static is False
        assert doc_type == "Foo"

    def test_public_static_array(self):
        vis, readonly, static, doc_type = parse_property_doc(
            "$items", ["```php\npublic static array $items = []\n```"]
        )
        assert vis == "public"
        assert readonly is False
        assert static is True
        assert doc_type == "array"

    def test_no_documentation(self):
        vis, readonly, static, doc_type = parse_property_doc("$bar", [])
        assert vis is None
        assert readonly is False
        assert static is False
        assert doc_type is None

    def test_protected_string(self):
        vis, readonly, static, doc_type = parse_property_doc(
            "$email", ["```php\nprotected string $email\n```"]
        )
        assert vis == "protected"
        assert doc_type == "string"

    def test_type_without_namespace(self):
        vis, readonly, static, doc_type = parse_property_doc(
            "$count", ["```php\nprivate int $count\n```"]
        )
        assert vis == "private"
        assert doc_type == "int"
