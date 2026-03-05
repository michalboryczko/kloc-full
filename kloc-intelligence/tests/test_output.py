"""Tests for output model serialization."""

from __future__ import annotations

from src.models.node import NodeData
from src.models.results import (
    ArgumentInfo,
    ContextEntry,
    ContextResult,
    DefinitionInfo,
    MemberRef,
)
from src.models.output import (
    ContextOutput,
    OutputArgumentInfo,
    OutputDefinition,
    OutputEntry,
    OutputMemberRef,
    OutputTarget,
    _shorten_param_key,
)


def _make_node(**overrides) -> NodeData:
    """Create a NodeData with defaults."""
    defaults = {
        "node_id": "node-1",
        "kind": "Class",
        "name": "Foo",
        "fqn": "App\\Foo",
        "symbol": "php:App\\Foo",
        "file": "src/Foo.php",
        "start_line": 10,
    }
    defaults.update(overrides)
    return NodeData(**defaults)


def _make_entry(**overrides) -> ContextEntry:
    """Create a ContextEntry with defaults."""
    defaults = {
        "depth": 1,
        "node_id": "n-1",
        "fqn": "App\\Bar::handle()",
        "kind": "Method",
        "file": "src/Bar.php",
        "line": 20,  # 0-based
    }
    defaults.update(overrides)
    return ContextEntry(**defaults)


class TestShortenParamKey:
    """Test _shorten_param_key helper."""

    def test_shortens_fqn(self):
        assert _shorten_param_key("App\\Order\\Order::__construct().$id", None, 0) == "Order::__construct().$id"

    def test_shortens_simple_fqn(self):
        assert _shorten_param_key("App\\Order\\Order::$id", None, 0) == "Order::$id"

    def test_uses_param_name_fallback(self):
        assert _shorten_param_key(None, "$id", 0) == "$id"

    def test_uses_position_fallback(self):
        assert _shorten_param_key(None, None, 2) == "arg[2]"

    def test_simple_class(self):
        assert _shorten_param_key("Order::$id", None, 0) == "Order::$id"


class TestOutputArgumentInfo:
    """Test OutputArgumentInfo serialization."""

    def test_from_info(self):
        info = ArgumentInfo(position=0, param_name="$id", value_expr="$input->id", value_source="parameter")
        output = OutputArgumentInfo.from_info(info)
        assert output.position == 0
        assert output.param_name == "$id"

    def test_to_dict_minimal(self):
        output = OutputArgumentInfo(position=0, param_name="$id", value_expr="$x", value_source="local")
        d = output.to_dict()
        assert d == {"position": 0, "param_name": "$id", "value_expr": "$x", "value_source": "local"}
        assert "value_type" not in d

    def test_to_dict_with_optionals(self):
        output = OutputArgumentInfo(
            position=0, param_name="$id", value_expr="$x", value_source="local",
            value_type="int", param_fqn="Order::$id",
        )
        d = output.to_dict()
        assert d["value_type"] == "int"
        assert d["param_fqn"] == "Order::$id"


class TestOutputMemberRef:
    """Test OutputMemberRef serialization."""

    def test_from_ref_converts_line(self):
        ref = MemberRef(target_name="save()", target_fqn="App\\Repo::save()", line=19)
        output = OutputMemberRef.from_ref(ref)
        assert output.line == 20  # 0-based -> 1-based

    def test_from_ref_on_line_converts(self):
        ref = MemberRef(target_name="save()", target_fqn="App\\Repo::save()", on_line=5)
        output = OutputMemberRef.from_ref(ref)
        assert output.on_line == 6

    def test_to_dict_minimal(self):
        output = OutputMemberRef(
            target_name="save()", target_fqn="App\\Repo::save()",
            target_kind="Method", file="src/Repo.php", line=20,
        )
        d = output.to_dict()
        assert d["target_name"] == "save()"
        assert d["line"] == 20
        assert "reference_type" not in d
        assert "access_chain" not in d

    def test_to_dict_with_optionals(self):
        output = OutputMemberRef(
            target_name="save()", target_fqn="App\\Repo::save()",
            target_kind="Method", file="src/Repo.php", line=20,
            reference_type="method_call", access_chain="$this->repo",
        )
        d = output.to_dict()
        assert d["reference_type"] == "method_call"
        assert d["access_chain"] == "$this->repo"


class TestOutputEntry:
    """Test OutputEntry conversion and serialization."""

    def test_line_number_conversion(self):
        entry = _make_entry(line=19)
        output = OutputEntry.from_entry(entry)
        assert output.line == 20  # 0-based -> 1-based

    def test_none_line_stays_none(self):
        entry = _make_entry(line=None)
        output = OutputEntry.from_entry(entry)
        assert output.line is None

    def test_signature_method_level_no_ref_type(self):
        entry = _make_entry(signature="handle($id)", ref_type=None)
        output = OutputEntry.from_entry(entry, class_level=False)
        assert output.signature == "handle($id)"

    def test_signature_method_level_with_ref_type_suppressed(self):
        entry = _make_entry(signature="handle($id)", ref_type="method_call")
        output = OutputEntry.from_entry(entry, class_level=False)
        assert output.signature is None

    def test_signature_class_level_suppressed(self):
        entry = _make_entry(signature="handle($id)", ref_type=None)
        output = OutputEntry.from_entry(entry, class_level=True)
        assert output.signature is None

    def test_signature_override_preserved_class_level(self):
        entry = _make_entry(signature="handle($id)", ref_type="override")
        output = OutputEntry.from_entry(entry, class_level=True)
        assert output.signature == "handle($id)"

    def test_member_ref_method_level_only(self):
        ref = MemberRef(target_name="save()", target_fqn="App\\Repo::save()")
        entry = _make_entry(member_ref=ref, ref_type=None)
        output_method = OutputEntry.from_entry(entry, class_level=False)
        assert output_method.member_ref is not None
        output_class = OutputEntry.from_entry(entry, class_level=True)
        assert output_class.member_ref is None

    def test_member_ref_suppressed_with_ref_type(self):
        ref = MemberRef(target_name="save()", target_fqn="App\\Repo::save()")
        entry = _make_entry(member_ref=ref, ref_type="method_call")
        output = OutputEntry.from_entry(entry, class_level=False)
        assert output.member_ref is None

    def test_arguments_flat_format_class_level(self):
        args = [ArgumentInfo(position=0, param_name="$id", value_expr="42", param_fqn="Order::$id")]
        entry = _make_entry(arguments=args, ref_type="instantiation")
        output = OutputEntry.from_entry(entry, class_level=True)
        assert output.args is not None
        assert output.args["Order::$id"] == "42"
        assert output.arguments is None

    def test_arguments_rich_format_method_level(self):
        args = [ArgumentInfo(position=0, param_name="$id", value_expr="42", value_source="literal")]
        entry = _make_entry(arguments=args, ref_type=None)
        output = OutputEntry.from_entry(entry, class_level=False)
        assert output.arguments is not None
        assert len(output.arguments) == 1
        assert output.args is None

    def test_arguments_flat_when_ref_type_set(self):
        args = [ArgumentInfo(position=0, param_name="$id", value_expr="42")]
        entry = _make_entry(arguments=args, ref_type="instantiation")
        output = OutputEntry.from_entry(entry, class_level=False)
        assert output.args is not None
        assert output.arguments is None

    def test_callee_only_for_method_call(self):
        entry = _make_entry(callee="save()", ref_type="method_call")
        output = OutputEntry.from_entry(entry)
        assert output.callee == "save()"

        entry2 = _make_entry(callee="save()", ref_type="instantiation")
        output2 = OutputEntry.from_entry(entry2)
        assert output2.callee is None

    def test_crossed_from_suppressed_class_level_depth1(self):
        entry = _make_entry(crossed_from="App\\Foo::$bar", depth=1)
        output = OutputEntry.from_entry(entry, class_level=True)
        assert output.crossed_from is None

    def test_crossed_from_shown_class_level_depth2(self):
        entry = _make_entry(crossed_from="App\\Foo::$bar", depth=2)
        output = OutputEntry.from_entry(entry, class_level=True)
        assert output.crossed_from == "App\\Foo::$bar"

    def test_crossed_from_shown_method_level(self):
        entry = _make_entry(crossed_from="App\\Foo::$bar", depth=1)
        output = OutputEntry.from_entry(entry, class_level=False)
        assert output.crossed_from == "App\\Foo::$bar"

    def test_sites_removes_line_from_dict(self):
        entry = _make_entry(sites=[{"method": "handle()", "line": 20}], line=19)
        output = OutputEntry.from_entry(entry)
        d = output.to_dict()
        assert "sites" in d
        assert "line" not in d

    def test_to_dict_camel_case(self):
        entry = _make_entry(ref_type="method_call", on="$this->repo", on_kind="property")
        output = OutputEntry.from_entry(entry)
        d = output.to_dict()
        assert d["refType"] == "method_call"
        assert d["onKind"] == "property"
        assert d["on"] == "$this->repo"
        # Verify snake_case NOT present
        assert "ref_type" not in d
        assert "on_kind" not in d

    def test_to_dict_property_group(self):
        entry = _make_entry(property_name="$repo", access_count=5, method_count=2)
        output = OutputEntry.from_entry(entry)
        d = output.to_dict()
        assert d["property"] == "$repo"
        assert d["accessCount"] == 5
        assert d["methodCount"] == 2

    def test_children_recursive(self):
        child = _make_entry(depth=2, fqn="App\\Baz::call()", line=30)
        entry = _make_entry(children=[child])
        output = OutputEntry.from_entry(entry)
        assert len(output.children) == 1
        assert output.children[0].fqn == "App\\Baz::call()"
        assert output.children[0].line == 31  # 1-based

    def test_implementations(self):
        impl = _make_entry(depth=2, fqn="App\\ConcreteHandler::handle()")
        entry = _make_entry(implementations=[impl])
        output = OutputEntry.from_entry(entry)
        assert output.implementations is not None
        assert len(output.implementations) == 1

    def test_via_interface_in_dict(self):
        entry = _make_entry(via_interface=True)
        output = OutputEntry.from_entry(entry)
        d = output.to_dict()
        assert d["via_interface"] is True

    def test_via_interface_false_excluded(self):
        entry = _make_entry(via_interface=False)
        output = OutputEntry.from_entry(entry)
        d = output.to_dict()
        assert "via_interface" not in d


class TestOutputTarget:
    """Test OutputTarget."""

    def test_from_node(self):
        node = _make_node(fqn="App\\Foo", file="src/Foo.php", start_line=9)
        target = OutputTarget.from_node(node)
        assert target.fqn == "App\\Foo"
        assert target.line == 10  # 0-based -> 1-based

    def test_to_dict_with_signature(self):
        node = _make_node(
            kind="Method",
            fqn="App\\Foo::bar()",
            documentation=["```php\nfunction bar(): void\n```"],
        )
        target = OutputTarget.from_node(node)
        d = target.to_dict()
        assert d["fqn"] == "App\\Foo::bar()"
        assert "signature" in d

    def test_to_dict_without_signature(self):
        node = _make_node()
        target = OutputTarget.from_node(node)
        d = target.to_dict()
        assert "signature" not in d


class TestOutputDefinition:
    """Test OutputDefinition serialization."""

    def test_line_1_based(self):
        info = DefinitionInfo(fqn="App\\Foo", kind="Class", line=9)
        output = OutputDefinition.from_info(info)
        assert output.line == 10

    def test_property_type_extraction(self):
        info = DefinitionInfo(
            fqn="App\\Foo::$bar", kind="Property",
            return_type={"name": "string", "visibility": "private", "readonly": True, "promoted": True},
        )
        output = OutputDefinition.from_info(info)
        assert output.type_name == "string"
        assert output.visibility == "private"
        assert output.readonly is True
        assert output.promoted is True

    def test_property_to_dict(self):
        info = DefinitionInfo(
            fqn="App\\Foo::$bar", kind="Property", line=5,
            return_type={"name": "string", "visibility": "private"},
        )
        output = OutputDefinition.from_info(info)
        d = output.to_dict()
        assert d["type"] == "string"
        assert d["visibility"] == "private"
        assert "returnType" not in d

    def test_method_return_type(self):
        info = DefinitionInfo(
            fqn="App\\Foo::bar()", kind="Method",
            return_type={"fqn": "App\\Bar", "name": "Bar"},
        )
        output = OutputDefinition.from_info(info)
        d = output.to_dict()
        assert d["returnType"] == {"fqn": "App\\Bar", "name": "Bar"}
        assert "type" not in d

    def test_declared_in_1_based_line(self):
        info = DefinitionInfo(
            fqn="App\\Foo::bar()", kind="Method",
            declared_in={"fqn": "App\\Foo", "file": "src/Foo.php", "line": 5},
        )
        output = OutputDefinition.from_info(info)
        d = output.to_dict()
        assert d["declaredIn"]["line"] == 6  # 0-based -> 1-based

    def test_declared_in_suppressed_for_class(self):
        info = DefinitionInfo(
            fqn="App\\Foo", kind="Class",
            declared_in={"fqn": "src/Foo.php", "file": "src/Foo.php", "line": 0},
        )
        output = OutputDefinition.from_info(info)
        assert output.declared_in is None

    def test_constructor_deps_serialization(self):
        info = DefinitionInfo(
            fqn="App\\Foo", kind="Class",
            constructor_deps=[{"name": "$service", "type": "Service"}],
        )
        output = OutputDefinition.from_info(info)
        d = output.to_dict()
        assert d["constructorDeps"] == [{"name": "$service", "type": "Service"}]

    def test_value_specific_fields(self):
        info = DefinitionInfo(
            fqn="local#1", kind="Value",
            value_kind="local",
            type_info={"fqn": "App\\Order", "name": "Order"},
            source={"call_fqn": "c#1", "method_fqn": "App\\Repo::find()", "method_name": "find()"},
        )
        output = OutputDefinition.from_info(info)
        d = output.to_dict()
        assert d["value_kind"] == "local"
        assert d["type"] == {"fqn": "App\\Order", "name": "Order"}
        assert d["source"]["method_name"] == "find()"

    def test_class_with_structure(self):
        info = DefinitionInfo(
            fqn="App\\Foo", kind="Class",
            properties=[{"name": "$bar", "type": "string"}],
            methods=[{"name": "baz"}],
            extends="App\\Base",
            implements=["App\\FooInterface"],
            uses_traits=["App\\LoggableTrait"],
        )
        output = OutputDefinition.from_info(info)
        d = output.to_dict()
        assert d["properties"] == [{"name": "$bar", "type": "string"}]
        assert d["methods"] == [{"name": "baz"}]
        assert d["extends"] == "App\\Base"
        assert d["implements"] == ["App\\FooInterface"]
        assert d["uses_traits"] == ["App\\LoggableTrait"]

    def test_empty_lists_excluded(self):
        info = DefinitionInfo(fqn="App\\Foo", kind="Class")
        output = OutputDefinition.from_info(info)
        d = output.to_dict()
        assert "properties" not in d
        assert "methods" not in d
        assert "implements" not in d
        assert "uses_traits" not in d
        assert "constructorDeps" not in d


class TestContextOutput:
    """Test ContextOutput conversion."""

    def test_from_result_class_level(self):
        node = _make_node(kind="Class")
        entry = _make_entry(signature="handle($id)")
        result = ContextResult(target=node, max_depth=1, used_by=[entry])
        output = ContextOutput.from_result(result)
        # Class-level: signature suppressed (no ref_type override/inherited)
        assert output.used_by[0].signature is None

    def test_from_result_method_level(self):
        node = _make_node(kind="Method", fqn="App\\Foo::bar()")
        entry = _make_entry(signature="handle($id)")
        result = ContextResult(target=node, max_depth=1, used_by=[entry])
        output = ContextOutput.from_result(result)
        # Method-level: signature shown when no ref_type
        assert output.used_by[0].signature == "handle($id)"

    def test_to_dict_structure(self):
        node = _make_node()
        result = ContextResult(target=node, max_depth=2, used_by=[], uses=[])
        output = ContextOutput.from_result(result)
        d = output.to_dict()
        assert "target" in d
        assert d["maxDepth"] == 2
        assert d["usedBy"] == []
        assert d["uses"] == []

    def test_to_dict_with_definition(self):
        node = _make_node()
        defn = DefinitionInfo(fqn="App\\Foo", kind="Class", line=9)
        result = ContextResult(target=node, max_depth=1, definition=defn)
        output = ContextOutput.from_result(result)
        d = output.to_dict()
        assert "definition" in d
        assert d["definition"]["line"] == 10

    def test_property_is_class_level(self):
        node = _make_node(kind="Property", fqn="App\\Foo::$bar")
        entry = _make_entry(signature="handle($id)")
        result = ContextResult(target=node, max_depth=1, used_by=[entry])
        output = ContextOutput.from_result(result)
        # Property is class-level
        assert output.used_by[0].signature is None
