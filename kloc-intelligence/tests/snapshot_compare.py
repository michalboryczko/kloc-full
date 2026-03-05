"""Snapshot comparison engine for behavioral parity validation.

Compares kloc-intelligence JSON output against golden files from kloc-cli.
Handles key ordering differences, float tolerance, and clear diff reporting.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any


@dataclass
class FieldDiff:
    """A single field difference between expected and actual output."""

    path: str  # JSON path, e.g., "tree[0].children[1].fqn"
    expected: Any
    actual: Any
    diff_type: str  # "missing", "extra", "value_mismatch", "type_mismatch"


@dataclass
class ComparisonResult:
    """Result of comparing kloc-intelligence output against a golden file."""

    query_id: str
    passed: bool
    diffs: list[FieldDiff] = field(default_factory=list)

    def summary(self) -> str:
        if self.passed:
            return f"{self.query_id}: PASS"
        return f"{self.query_id}: FAIL ({len(self.diffs)} diffs)"


def compare_json(expected: Any, actual: Any, path: str = "$") -> list[FieldDiff]:
    """Recursively compare two JSON structures.

    Rules:
    - Objects: key ordering is ignored, all keys must match
    - Arrays: order IS significant (tree structures depend on order)
    - Strings: exact match
    - Numbers: integers exact, floats within tolerance (1e-6)
    - Null: exact match
    - Missing keys: reported as diffs
    - Extra keys: reported as diffs
    """
    diffs: list[FieldDiff] = []

    if expected is None and actual is None:
        return diffs

    # Check for type mismatches (using isinstance to satisfy linter)
    expected_type = type(expected)
    actual_type = type(actual)
    if expected_type is not actual_type:
        # Special case: int vs float comparison
        if isinstance(expected, (int, float)) and isinstance(actual, (int, float)):
            if abs(float(expected) - float(actual)) > 1e-6:
                diffs.append(FieldDiff(path, expected, actual, "value_mismatch"))
            return diffs
        # Special case: None vs missing
        if expected is None or actual is None:
            diffs.append(FieldDiff(path, expected, actual, "value_mismatch"))
            return diffs
        diffs.append(FieldDiff(path, expected, actual, "type_mismatch"))
        return diffs

    if isinstance(expected, dict):
        all_keys = set(expected.keys()) | set(actual.keys())
        for key in sorted(all_keys):
            child_path = f"{path}.{key}"
            if key not in expected:
                diffs.append(FieldDiff(child_path, None, actual[key], "extra"))
            elif key not in actual:
                diffs.append(FieldDiff(child_path, expected[key], None, "missing"))
            else:
                diffs.extend(compare_json(expected[key], actual[key], child_path))

    elif isinstance(expected, list):
        if len(expected) != len(actual):
            diffs.append(
                FieldDiff(f"{path}.__len__", len(expected), len(actual), "value_mismatch")
            )
        for i in range(min(len(expected), len(actual))):
            diffs.extend(compare_json(expected[i], actual[i], f"{path}[{i}]"))

    elif isinstance(expected, float):
        if abs(expected - actual) > 1e-6:
            diffs.append(FieldDiff(path, expected, actual, "value_mismatch"))

    elif expected != actual:
        diffs.append(FieldDiff(path, expected, actual, "value_mismatch"))

    return diffs


def _sort_constructor_deps(data: Any) -> Any:
    """Sort constructorDeps arrays by name for order-insensitive comparison.

    Neo4j doesn't preserve sot.json edge order, so constructorDeps ordering
    may differ from kloc-cli. Sort both sides by 'name' before comparison.
    """
    if not isinstance(data, dict):
        return data
    data = dict(data)  # shallow copy
    if "definition" in data and isinstance(data["definition"], dict):
        defn = dict(data["definition"])
        if "constructorDeps" in defn and isinstance(defn["constructorDeps"], list):
            defn["constructorDeps"] = sorted(
                defn["constructorDeps"],
                key=lambda d: d.get("name", "") if isinstance(d, dict) else "",
            )
        data["definition"] = defn
    return data


def _entry_sort_key(entry: dict) -> tuple:
    """Generate a sort key for usedBy/uses entries for order-insensitive comparison.

    For method context entries (identified by entry_type field), sort by line number
    first to preserve execution flow order. For class/interface context, sort by
    (fqn, refType, file, line).
    """
    if entry.get("entry_type") or entry.get("source_call"):
        # Method context: sort by line to preserve execution flow order
        return (
            entry.get("file", ""),
            entry.get("line") or 0,
            entry.get("fqn", ""),
        )
    return (
        entry.get("fqn", ""),
        entry.get("refType", ""),
        entry.get("file", ""),
        entry.get("line") or 0,
    )


def _sort_entries_recursive(entries: list) -> list:
    """Sort a list of entries and their children recursively."""
    sorted_entries = sorted(entries, key=_entry_sort_key)
    for entry in sorted_entries:
        if isinstance(entry, dict) and "children" in entry and isinstance(entry["children"], list):
            entry["children"] = _sort_entries_recursive(entry["children"])
    return sorted_entries


def _sort_context_arrays(data: Any) -> Any:
    """Sort usedBy/uses arrays for order-insensitive comparison.

    Neo4j query execution order is non-deterministic, so context arrays
    may differ in order from kloc-cli. Sort both sides by (fqn, refType, file, line).
    Also recursively sorts children arrays.
    """
    if not isinstance(data, dict):
        return data
    data = dict(data)  # shallow copy
    for key in ("usedBy", "uses"):
        if key in data and isinstance(data[key], list):
            data[key] = _sort_entries_recursive(data[key])
    return data


def compare_snapshot(query_id: str, golden: dict, actual_output: Any) -> ComparisonResult:
    """Compare kloc-intelligence output against golden file.

    Args:
        query_id: Test identifier.
        golden: Parsed golden file (from generate_golden.py).
        actual_output: kloc-intelligence JSON output.

    Returns:
        ComparisonResult with diffs.
    """
    expected = golden.get("json_output")
    if expected is None:
        return ComparisonResult(
            query_id=query_id,
            passed=False,
            diffs=[FieldDiff("$", "JSON output", None, "missing")],
        )

    # Normalize ordering for Neo4j non-deterministic results
    expected = _sort_constructor_deps(expected)
    actual_output = _sort_constructor_deps(actual_output)
    expected = _sort_context_arrays(expected)
    actual_output = _sort_context_arrays(actual_output)

    diffs = compare_json(expected, actual_output)

    return ComparisonResult(
        query_id=query_id,
        passed=len(diffs) == 0,
        diffs=diffs,
    )


def format_diff_report(result: ComparisonResult, max_diffs: int = 20) -> str:
    """Format a human-readable diff report."""
    lines = [result.summary()]

    if not result.passed:
        for diff in result.diffs[:max_diffs]:
            if diff.diff_type == "missing":
                lines.append(f"  MISSING {diff.path}: expected {repr(diff.expected)}")
            elif diff.diff_type == "extra":
                lines.append(f"  EXTRA   {diff.path}: got {repr(diff.actual)}")
            elif diff.diff_type == "value_mismatch":
                lines.append(
                    f"  DIFF    {diff.path}: "
                    f"expected {repr(diff.expected)}, got {repr(diff.actual)}"
                )
            elif diff.diff_type == "type_mismatch":
                lines.append(
                    f"  TYPE    {diff.path}: "
                    f"expected {type(diff.expected).__name__}, "
                    f"got {type(diff.actual).__name__}"
                )

        if len(result.diffs) > max_diffs:
            lines.append(f"  ... and {len(result.diffs) - max_diffs} more diffs")

    return "\n".join(lines)
