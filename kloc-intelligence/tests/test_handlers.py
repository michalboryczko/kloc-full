"""Tests for USED BY edge handlers."""

from __future__ import annotations

from src.logic.handlers import (
    EdgeContext,
    EntryBucket,
    InstantiationHandler,
    ExtendsHandler,
    ImplementsHandler,
    PropertyTypeHandler,
    MethodCallHandler,
    PropertyAccessHandler,
    ParamReturnHandler,
    USED_BY_HANDLERS,
)


def _make_ctx(**overrides) -> EdgeContext:
    """Create an EdgeContext with sensible defaults."""
    defaults = {
        "start_id": "target-1",
        "source_id": "source-1",
        "source_kind": "Method",
        "source_fqn": "App\\Foo::bar()",
        "source_name": "bar",
        "source_file": "src/Foo.php",
        "source_start_line": 10,
        "source_signature": None,
        "target_kind": "Class",
        "target_fqn": "App\\Bar",
        "target_name": "Bar",
        "ref_type": "method_call",
        "file": "src/Foo.php",
        "line": 15,
        "call_node_id": None,
        "classes_with_injection": frozenset(),
    }
    defaults.update(overrides)
    return EdgeContext(**defaults)


class TestInstantiationHandler:
    """Test InstantiationHandler."""

    def test_creates_entry_with_containing_method(self):
        ctx = _make_ctx(
            ref_type="instantiation",
            containing_method_id="method-1",
            containing_method_fqn="App\\Service::create",
            containing_method_kind="Method",
        )
        bucket = EntryBucket()
        InstantiationHandler().handle(ctx, bucket)
        assert len(bucket.instantiation) == 1
        entry = bucket.instantiation[0]
        assert entry["fqn"] == "App\\Service::create()"  # Appends ()
        assert entry["ref_type"] == "instantiation"
        assert entry["node_id"] == "method-1"

    def test_deduplicates_by_method(self):
        ctx = _make_ctx(
            ref_type="instantiation",
            containing_method_id="method-1",
            containing_method_fqn="App\\Service::create()",
            containing_method_kind="Method",
        )
        bucket = EntryBucket()
        InstantiationHandler().handle(ctx, bucket)
        InstantiationHandler().handle(ctx, bucket)
        assert len(bucket.instantiation) == 1

    def test_uses_source_when_no_containing_method(self):
        ctx = _make_ctx(ref_type="instantiation")
        bucket = EntryBucket()
        InstantiationHandler().handle(ctx, bucket)
        assert len(bucket.instantiation) == 1
        assert bucket.instantiation[0]["node_id"] == "source-1"

    def test_includes_arguments(self):
        ctx = _make_ctx(
            ref_type="instantiation",
            arguments=[{"position": 0, "param_name": "$id"}],
        )
        bucket = EntryBucket()
        InstantiationHandler().handle(ctx, bucket)
        assert bucket.instantiation[0]["arguments"] == [{"position": 0, "param_name": "$id"}]


class TestExtendsHandler:
    """Test ExtendsHandler."""

    def test_creates_extends_entry(self):
        ctx = _make_ctx(
            ref_type="extends",
            source_kind="Class",
            source_fqn="App\\ChildClass",
        )
        bucket = EntryBucket()
        ExtendsHandler().handle(ctx, bucket)
        assert len(bucket.extends) == 1
        entry = bucket.extends[0]
        assert entry["ref_type"] == "extends"
        assert entry["fqn"] == "App\\ChildClass"


class TestImplementsHandler:
    """Test ImplementsHandler."""

    def test_creates_entry_in_extends_list(self):
        ctx = _make_ctx(
            ref_type="implements",
            source_kind="Class",
            source_fqn="App\\ConcreteClass",
        )
        bucket = EntryBucket()
        ImplementsHandler().handle(ctx, bucket)
        # Implements entries go into extends list
        assert len(bucket.extends) == 1
        assert bucket.extends[0]["ref_type"] == "implements"


class TestPropertyTypeHandler:
    """Test PropertyTypeHandler."""

    def test_direct_property_source(self):
        ctx = _make_ctx(
            ref_type="property_type",
            source_kind="Property",
            source_id="prop-1",
            source_fqn="App\\Foo::$bar",
            source_file="src/Foo.php",
            source_start_line=5,
        )
        bucket = EntryBucket()
        PropertyTypeHandler().handle(ctx, bucket)
        assert len(bucket.property_type) == 1
        entry = bucket.property_type[0]
        assert entry["fqn"] == "App\\Foo::$bar"
        assert entry["kind"] == "Property"

    def test_method_source_resolves_property(self):
        ctx = _make_ctx(
            ref_type="property_type",
            source_kind="Method",
            resolved_property_id="prop-1",
            resolved_property_fqn="App\\Foo::$bar",
            resolved_property_file="src/Foo.php",
            resolved_property_line=5,
        )
        bucket = EntryBucket()
        PropertyTypeHandler().handle(ctx, bucket)
        assert len(bucket.property_type) == 1
        assert bucket.property_type[0]["fqn"] == "App\\Foo::$bar"

    def test_deduplicates_by_prop_fqn(self):
        ctx = _make_ctx(
            ref_type="property_type",
            source_kind="Property",
            source_fqn="App\\Foo::$bar",
        )
        bucket = EntryBucket()
        PropertyTypeHandler().handle(ctx, bucket)
        PropertyTypeHandler().handle(ctx, bucket)
        assert len(bucket.property_type) == 1

    def test_no_property_found_skips(self):
        ctx = _make_ctx(
            ref_type="property_type",
            source_kind="Method",
            # No resolved_property_fqn
        )
        bucket = EntryBucket()
        PropertyTypeHandler().handle(ctx, bucket)
        assert len(bucket.property_type) == 0


class TestMethodCallHandler:
    """Test MethodCallHandler."""

    def test_creates_entry_with_callee(self):
        ctx = _make_ctx(
            ref_type="method_call",
            target_kind="Method",
            target_name="save",
            containing_method_id="method-1",
            containing_method_fqn="App\\Service::handle",
            containing_method_kind="Method",
        )
        bucket = EntryBucket()
        MethodCallHandler().handle(ctx, bucket)
        assert len(bucket.method_call) == 1
        entry = bucket.method_call[0]
        assert entry["callee"] == "save()"
        assert entry["fqn"] == "App\\Service::handle()"

    def test_suppresses_when_injection_present(self):
        ctx = _make_ctx(
            ref_type="method_call",
            containing_method_id="method-1",
            containing_method_fqn="App\\Foo::bar()",
            containing_method_kind="Method",
            containing_class_id="class-1",
            classes_with_injection=frozenset({"class-1"}),
        )
        bucket = EntryBucket()
        MethodCallHandler().handle(ctx, bucket)
        assert len(bucket.method_call) == 0

    def test_includes_on_expr(self):
        ctx = _make_ctx(
            ref_type="method_call",
            target_kind="Method",
            target_name="save",
            on_expr="$this->repository",
            on_kind="property",
        )
        bucket = EntryBucket()
        MethodCallHandler().handle(ctx, bucket)
        assert bucket.method_call[0]["on"] == "$this->repository"
        assert bucket.method_call[0]["on_kind"] == "property"


class TestPropertyAccessHandler:
    """Test PropertyAccessHandler."""

    def test_groups_by_fqn_and_method(self):
        ctx = _make_ctx(
            ref_type="property_access",
            target_fqn="App\\Foo::$bar",
            containing_method_id="method-1",
            containing_method_fqn="App\\Service::handle()",
            containing_method_kind="Method",
        )
        bucket = EntryBucket()
        PropertyAccessHandler().handle(ctx, bucket)
        assert "App\\Foo::$bar" in bucket.property_access_groups
        assert len(bucket.property_access_groups["App\\Foo::$bar"]) == 1

    def test_merges_lines_for_same_group(self):
        ctx1 = _make_ctx(
            ref_type="property_access",
            target_fqn="App\\Foo::$bar",
            line=10,
            containing_method_id="m-1",
            containing_method_fqn="App\\Service::handle()",
            containing_method_kind="Method",
        )
        ctx2 = _make_ctx(
            ref_type="property_access",
            target_fqn="App\\Foo::$bar",
            line=20,
            containing_method_id="m-1",
            containing_method_fqn="App\\Service::handle()",
            containing_method_kind="Method",
        )
        bucket = EntryBucket()
        PropertyAccessHandler().handle(ctx1, bucket)
        PropertyAccessHandler().handle(ctx2, bucket)
        groups = bucket.property_access_groups["App\\Foo::$bar"]
        assert len(groups) == 1
        assert groups[0]["lines"] == [10, 20]


class TestParamReturnHandler:
    """Test ParamReturnHandler."""

    def test_return_type_shows_method_fqn(self):
        ctx = _make_ctx(
            ref_type="return_type",
            source_kind="Method",
            source_fqn="App\\Foo::bar",
        )
        bucket = EntryBucket()
        ParamReturnHandler().handle(ctx, bucket)
        assert len(bucket.param_return) == 1
        assert bucket.param_return[0]["fqn"] == "App\\Foo::bar()"

    def test_return_type_deduplicates(self):
        ctx = _make_ctx(
            ref_type="return_type",
            source_kind="Method",
            source_fqn="App\\Foo::bar()",
        )
        bucket = EntryBucket()
        ParamReturnHandler().handle(ctx, bucket)
        ParamReturnHandler().handle(ctx, bucket)
        assert len(bucket.param_return) == 1

    def test_parameter_type_groups_by_class(self):
        ctx = _make_ctx(
            ref_type="parameter_type",
            source_kind="Class",
            source_id="cls-1",
            source_fqn="App\\Service",
        )
        bucket = EntryBucket()
        ParamReturnHandler().handle(ctx, bucket)
        assert len(bucket.param_return) == 1
        assert bucket.param_return[0]["fqn"] == "App\\Service"

    def test_type_hint_from_file_is_skipped(self):
        ctx = _make_ctx(
            ref_type="type_hint",
            source_kind="File",
            source_fqn="src/Foo.php",
        )
        bucket = EntryBucket()
        ParamReturnHandler().handle(ctx, bucket)
        assert len(bucket.param_return) == 0


class TestUsedByHandlersRegistry:
    """Test USED_BY_HANDLERS registry."""

    def test_has_nine_entries(self):
        assert len(USED_BY_HANDLERS) == 9

    def test_all_ref_types_mapped(self):
        expected = {
            "instantiation", "extends", "implements", "property_type",
            "method_call", "property_access", "parameter_type",
            "return_type", "type_hint",
        }
        assert set(USED_BY_HANDLERS.keys()) == expected

    def test_param_return_shared_handler(self):
        # parameter_type, return_type, and type_hint share the same handler instance
        assert USED_BY_HANDLERS["parameter_type"] is USED_BY_HANDLERS["return_type"]
        assert USED_BY_HANDLERS["return_type"] is USED_BY_HANDLERS["type_hint"]
