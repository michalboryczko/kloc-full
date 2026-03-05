"""Tests for reference type inference engine."""

from __future__ import annotations

from src.logic.reference_types import (
    CHAINABLE_REFERENCE_TYPES,
    REF_TYPE_PRIORITY,
    infer_reference_type,
    get_reference_type_from_call_kind,
    sort_entries_by_priority,
    sort_entries_by_location,
)


class TestChainableReferenceTypes:
    """Test CHAINABLE_REFERENCE_TYPES constant."""

    def test_has_four_types(self):
        assert len(CHAINABLE_REFERENCE_TYPES) == 4

    def test_method_call_is_chainable(self):
        assert "method_call" in CHAINABLE_REFERENCE_TYPES

    def test_property_access_is_chainable(self):
        assert "property_access" in CHAINABLE_REFERENCE_TYPES

    def test_instantiation_is_chainable(self):
        assert "instantiation" in CHAINABLE_REFERENCE_TYPES

    def test_static_call_is_chainable(self):
        assert "static_call" in CHAINABLE_REFERENCE_TYPES

    def test_type_hint_not_chainable(self):
        assert "type_hint" not in CHAINABLE_REFERENCE_TYPES

    def test_extends_not_chainable(self):
        assert "extends" not in CHAINABLE_REFERENCE_TYPES


class TestRefTypePriority:
    """Test REF_TYPE_PRIORITY constant."""

    def test_has_ten_entries(self):
        assert len(REF_TYPE_PRIORITY) == 10

    def test_instantiation_highest_priority(self):
        assert REF_TYPE_PRIORITY["instantiation"] == 0

    def test_type_hint_lowest_priority(self):
        assert REF_TYPE_PRIORITY["type_hint"] == 6

    def test_extends_and_implements_same_priority(self):
        assert REF_TYPE_PRIORITY["extends"] == REF_TYPE_PRIORITY["implements"]


class TestInferReferenceType:
    """Test infer_reference_type()."""

    # Direct edge type mappings
    def test_extends_edge(self):
        assert infer_reference_type("extends", "Class", "Class") == "extends"

    def test_implements_edge(self):
        assert infer_reference_type("implements", "Interface", "Class") == "implements"

    def test_uses_trait_edge(self):
        assert infer_reference_type("uses_trait", "Trait", "Class") == "uses_trait"

    # Uses edge + target kind
    def test_uses_method_target(self):
        assert infer_reference_type("uses", "Method", "Method") == "method_call"

    def test_uses_property_target(self):
        assert infer_reference_type("uses", "Property", "Method") == "property_access"

    def test_uses_constant_target(self):
        assert infer_reference_type("uses", "Constant", "Method") == "constant_access"

    def test_uses_function_target(self):
        assert infer_reference_type("uses", "Function", "File") == "function_call"

    def test_uses_argument_target(self):
        assert infer_reference_type("uses", "Argument", "Method") == "argument_ref"

    def test_uses_variable_target(self):
        assert infer_reference_type("uses", "Variable", "Method") == "variable_ref"

    # Complex Class/Interface target sub-classification
    def test_argument_source_is_parameter_type(self):
        assert infer_reference_type("uses", "Class", "Argument") == "parameter_type"

    def test_property_source_is_property_type(self):
        assert infer_reference_type("uses", "Class", "Property") == "property_type"

    def test_method_with_arg_type_hint_is_parameter_type(self):
        result = infer_reference_type(
            "uses", "Class", "Method", has_arg_type_hint=True
        )
        assert result == "parameter_type"

    def test_method_with_return_type_hint_is_return_type(self):
        result = infer_reference_type(
            "uses", "Class", "Method", has_return_type_hint=True
        )
        assert result == "return_type"

    def test_method_arg_type_hint_takes_priority_over_return(self):
        result = infer_reference_type(
            "uses", "Class", "Method",
            has_arg_type_hint=True, has_return_type_hint=True,
        )
        assert result == "parameter_type"

    def test_constructor_promoted_property_type(self):
        result = infer_reference_type(
            "uses", "Class", "Method",
            source_name="__construct",
            has_class_property_type_hint=True,
        )
        assert result == "property_type"

    def test_class_property_type_hint(self):
        result = infer_reference_type(
            "uses", "Class", "Class",
            has_class_property_type_hint=True,
        )
        assert result == "property_type"

    def test_class_target_no_hints_is_type_hint(self):
        result = infer_reference_type("uses", "Class", "Method")
        assert result == "type_hint"

    def test_interface_target_no_hints_is_type_hint(self):
        result = infer_reference_type("uses", "Interface", "Method")
        assert result == "type_hint"

    def test_enum_target_no_hints_is_type_hint(self):
        result = infer_reference_type("uses", "Enum", "Method")
        assert result == "type_hint"

    # Fallback
    def test_unknown_edge_type_is_uses(self):
        assert infer_reference_type("unknown", "Class", "Method") == "uses"

    def test_uses_without_target_is_uses(self):
        assert infer_reference_type("uses", None, "Method") == "uses"


class TestGetReferenceTypeFromCallKind:
    """Test get_reference_type_from_call_kind()."""

    def test_method(self):
        assert get_reference_type_from_call_kind("method") == "method_call"

    def test_method_static(self):
        assert get_reference_type_from_call_kind("method_static") == "static_call"

    def test_constructor(self):
        assert get_reference_type_from_call_kind("constructor") == "instantiation"

    def test_access(self):
        assert get_reference_type_from_call_kind("access") == "property_access"

    def test_function(self):
        assert get_reference_type_from_call_kind("function") == "function_call"

    def test_none(self):
        assert get_reference_type_from_call_kind(None) == "unknown"

    def test_unknown_kind(self):
        assert get_reference_type_from_call_kind("xyz") == "unknown"


class TestSortEntriesByPriority:
    """Test sort_entries_by_priority()."""

    def test_sorts_by_ref_type_priority(self):
        entries = [
            {"ref_type": "type_hint", "file": "a.php", "line": 1},
            {"ref_type": "instantiation", "file": "a.php", "line": 1},
            {"ref_type": "method_call", "file": "a.php", "line": 1},
        ]
        result = sort_entries_by_priority(entries)
        assert result[0]["ref_type"] == "instantiation"
        assert result[1]["ref_type"] == "method_call"
        assert result[2]["ref_type"] == "type_hint"

    def test_sorts_by_file_within_priority(self):
        entries = [
            {"ref_type": "method_call", "file": "b.php", "line": 1},
            {"ref_type": "method_call", "file": "a.php", "line": 1},
        ]
        result = sort_entries_by_priority(entries)
        assert result[0]["file"] == "a.php"
        assert result[1]["file"] == "b.php"

    def test_sorts_by_line_within_same_file(self):
        entries = [
            {"ref_type": "method_call", "file": "a.php", "line": 20},
            {"ref_type": "method_call", "file": "a.php", "line": 10},
        ]
        result = sort_entries_by_priority(entries)
        assert result[0]["line"] == 10
        assert result[1]["line"] == 20

    def test_handles_none_values(self):
        entries = [
            {"ref_type": "method_call", "file": None, "line": None},
            {"ref_type": "instantiation", "file": "a.php", "line": 1},
        ]
        result = sort_entries_by_priority(entries)
        assert result[0]["ref_type"] == "instantiation"


class TestSortEntriesByLocation:
    """Test sort_entries_by_location()."""

    def test_sorts_by_file_then_line(self):
        entries = [
            {"file": "b.php", "line": 5},
            {"file": "a.php", "line": 10},
            {"file": "a.php", "line": 5},
        ]
        result = sort_entries_by_location(entries)
        assert result[0]["file"] == "a.php"
        assert result[0]["line"] == 5
        assert result[1]["file"] == "a.php"
        assert result[1]["line"] == 10
        assert result[2]["file"] == "b.php"
