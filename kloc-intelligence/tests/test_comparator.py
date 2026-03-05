"""Tests for the snapshot comparison engine."""

from __future__ import annotations

from tests.snapshot_compare import (
    compare_json,
    compare_snapshot,
    format_diff_report,
    ComparisonResult,
    FieldDiff,
)


class TestCompareJson:
    """Tests for compare_json function."""

    def test_identical_dicts(self):
        """Identical dicts produce no diffs."""
        expected = {"a": 1, "b": "hello", "c": True}
        actual = {"a": 1, "b": "hello", "c": True}
        diffs = compare_json(expected, actual)
        assert diffs == []

    def test_dict_key_order_ignored(self):
        """Key ordering in dicts does not cause failures."""
        expected = {"a": 1, "b": 2, "c": 3}
        actual = {"c": 3, "a": 1, "b": 2}
        diffs = compare_json(expected, actual)
        assert diffs == []

    def test_missing_key(self):
        """Missing key in actual is reported."""
        expected = {"a": 1, "b": 2}
        actual = {"a": 1}
        diffs = compare_json(expected, actual)
        assert len(diffs) == 1
        assert diffs[0].diff_type == "missing"
        assert diffs[0].path == "$.b"

    def test_extra_key(self):
        """Extra key in actual is reported."""
        expected = {"a": 1}
        actual = {"a": 1, "b": 2}
        diffs = compare_json(expected, actual)
        assert len(diffs) == 1
        assert diffs[0].diff_type == "extra"
        assert diffs[0].path == "$.b"

    def test_value_mismatch(self):
        """Value mismatch is reported."""
        expected = {"a": 1}
        actual = {"a": 2}
        diffs = compare_json(expected, actual)
        assert len(diffs) == 1
        assert diffs[0].diff_type == "value_mismatch"

    def test_type_mismatch(self):
        """Type mismatch between incompatible types is reported."""
        expected = {"a": "hello"}
        actual = {"a": [1, 2, 3]}
        diffs = compare_json(expected, actual)
        assert len(diffs) == 1
        assert diffs[0].diff_type == "type_mismatch"

    def test_int_float_comparison(self):
        """Int and float are compared numerically."""
        expected = {"a": 1}
        actual = {"a": 1.0}
        diffs = compare_json(expected, actual)
        assert diffs == []

    def test_float_tolerance(self):
        """Floats within tolerance pass."""
        expected = {"a": 1.0000001}
        actual = {"a": 1.0000002}
        diffs = compare_json(expected, actual)
        assert diffs == []

    def test_float_outside_tolerance(self):
        """Floats outside tolerance fail."""
        expected = {"a": 1.0}
        actual = {"a": 2.0}
        diffs = compare_json(expected, actual)
        assert len(diffs) == 1

    def test_list_order_matters(self):
        """Array order IS checked."""
        expected = [1, 2, 3]
        actual = [1, 3, 2]
        diffs = compare_json(expected, actual)
        assert len(diffs) == 2  # positions 1 and 2 differ

    def test_list_length_mismatch(self):
        """List length mismatch is reported."""
        expected = [1, 2, 3]
        actual = [1, 2]
        diffs = compare_json(expected, actual)
        assert any(d.path.endswith("__len__") for d in diffs)

    def test_nested_structure(self):
        """Deeply nested structures are compared."""
        expected = {"a": {"b": {"c": 1}}}
        actual = {"a": {"b": {"c": 2}}}
        diffs = compare_json(expected, actual)
        assert len(diffs) == 1
        assert diffs[0].path == "$.a.b.c"

    def test_nested_array(self):
        """Arrays inside objects are compared."""
        expected = {"items": [{"id": 1}, {"id": 2}]}
        actual = {"items": [{"id": 1}, {"id": 3}]}
        diffs = compare_json(expected, actual)
        assert len(diffs) == 1
        assert diffs[0].path == "$.items[1].id"

    def test_null_values(self):
        """Null values match exactly."""
        expected = {"a": None}
        actual = {"a": None}
        diffs = compare_json(expected, actual)
        assert diffs == []

    def test_null_vs_value(self):
        """Null vs non-null is a mismatch."""
        expected = {"a": None}
        actual = {"a": 1}
        diffs = compare_json(expected, actual)
        assert len(diffs) == 1

    def test_empty_structures(self):
        """Empty dicts and lists match."""
        assert compare_json({}, {}) == []
        assert compare_json([], []) == []

    def test_identical_complex(self):
        """Complex real-world-like structure matches."""
        expected = {
            "target": {"fqn": "App\\Entity\\Order", "kind": "Class"},
            "tree": [
                {
                    "depth": 1,
                    "fqn": "App\\Repository\\OrderRepository",
                    "children": [
                        {"depth": 2, "fqn": "App\\Service\\OrderService"}
                    ],
                }
            ],
        }
        diffs = compare_json(expected, expected)
        assert diffs == []


class TestCompareSnapshot:
    """Tests for compare_snapshot function."""

    def test_matching_snapshot(self):
        """Matching output passes."""
        golden = {"json_output": {"a": 1, "b": 2}}
        actual = {"a": 1, "b": 2}
        result = compare_snapshot("test-1", golden, actual)
        assert result.passed is True

    def test_mismatching_snapshot(self):
        """Mismatching output fails."""
        golden = {"json_output": {"a": 1}}
        actual = {"a": 2}
        result = compare_snapshot("test-2", golden, actual)
        assert result.passed is False
        assert len(result.diffs) == 1

    def test_no_golden_json(self):
        """Missing json_output in golden fails."""
        golden = {"json_output": None}
        result = compare_snapshot("test-3", golden, {"a": 1})
        assert result.passed is False


class TestFormatDiffReport:
    """Tests for format_diff_report."""

    def test_pass_report(self):
        """Pass report is concise."""
        result = ComparisonResult(query_id="test", passed=True)
        report = format_diff_report(result)
        assert "PASS" in report

    def test_fail_report_shows_diffs(self):
        """Fail report shows diff details."""
        result = ComparisonResult(
            query_id="test",
            passed=False,
            diffs=[FieldDiff("$.a", 1, 2, "value_mismatch")],
        )
        report = format_diff_report(result)
        assert "FAIL" in report
        assert "$.a" in report
        assert "expected" in report

    def test_max_diffs_cap(self):
        """Report caps at max_diffs."""
        diffs = [FieldDiff(f"$.item{i}", i, i + 1, "value_mismatch") for i in range(30)]
        result = ComparisonResult(query_id="test", passed=False, diffs=diffs)
        report = format_diff_report(result, max_diffs=5)
        assert "25 more diffs" in report
