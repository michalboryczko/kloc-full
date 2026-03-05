"""Tests for JSON output formatters."""

from __future__ import annotations

from src.models.node import NodeData
from src.output.json_formatter import (
    usages_tree_to_dict,
    deps_tree_to_dict,
    _count_tree_nodes,
    _entry_to_dict,
)


class TestCountTreeNodes:
    """Test _count_tree_nodes."""

    def test_empty_list(self):
        assert _count_tree_nodes([]) == 0

    def test_flat_entries(self):
        entries = [
            {"depth": 1, "fqn": "A", "children": []},
            {"depth": 1, "fqn": "B", "children": []},
        ]
        assert _count_tree_nodes(entries) == 2

    def test_nested_entries(self):
        entries = [
            {"depth": 1, "fqn": "A", "children": [
                {"depth": 2, "fqn": "A1", "children": []},
                {"depth": 2, "fqn": "A2", "children": []},
            ]},
            {"depth": 1, "fqn": "B", "children": []},
        ]
        assert _count_tree_nodes(entries) == 4


class TestEntryToDict:
    """Test _entry_to_dict."""

    def test_basic_entry(self):
        entry = {
            "depth": 1,
            "fqn": "App\\Entity\\User",
            "file": "src/Entity/User.php",
            "line": 10,
            "children": [],
        }
        d = _entry_to_dict(entry)
        assert d["depth"] == 1
        assert d["fqn"] == "App\\Entity\\User"
        assert d["file"] == "src/Entity/User.php"
        assert d["line"] == 11  # 0-based to 1-based
        assert d["children"] == []

    def test_entry_with_none_line(self):
        entry = {
            "depth": 1,
            "fqn": "App\\Entity\\User",
            "file": "src/Entity/User.php",
            "children": [],
        }
        d = _entry_to_dict(entry)
        assert d["line"] is None

    def test_entry_with_children(self):
        entry = {
            "depth": 1,
            "fqn": "A",
            "file": "a.php",
            "line": 5,
            "children": [
                {"depth": 2, "fqn": "B", "file": "b.php", "line": 10, "children": []},
            ],
        }
        d = _entry_to_dict(entry)
        assert len(d["children"]) == 1
        assert d["children"][0]["fqn"] == "B"
        assert d["children"][0]["line"] == 11  # 0-based to 1-based


class TestUsagesTreeToDict:
    """Test usages_tree_to_dict."""

    def _make_target(self):
        return NodeData(
            node_id="node:123",
            kind="Class",
            name="User",
            fqn="App\\Entity\\User",
            symbol="scip:User",
            file="src/Entity/User.php",
            start_line=5,
        )

    def test_basic_tree(self):
        target = self._make_target()
        tree = [
            {
                "depth": 1,
                "fqn": "App\\Service\\UserService",
                "file": "src/Service/UserService.php",
                "line": 20,
                "children": [],
            },
        ]
        d = usages_tree_to_dict(target, 1, tree)
        assert d["target"]["fqn"] == "App\\Entity\\User"
        assert d["target"]["file"] == "src/Entity/User.php"
        assert "id" not in d["target"]  # No id in target
        assert "kind" not in d["target"]  # No kind in target
        assert d["max_depth"] == 1
        assert d["total"] == 1
        assert len(d["tree"]) == 1
        assert d["tree"][0]["line"] == 21  # 0-based to 1-based

    def test_empty_tree(self):
        target = self._make_target()
        d = usages_tree_to_dict(target, 1, [])
        assert d["total"] == 0
        assert d["tree"] == []


class TestDepsTreeToDict:
    """Test deps_tree_to_dict."""

    def _make_target(self):
        return NodeData(
            node_id="node:456",
            kind="Method",
            name="getId",
            fqn="App\\Entity\\User::getId()",
            symbol="scip:getId",
            file="src/Entity/User.php",
            start_line=15,
        )

    def test_basic_tree(self):
        target = self._make_target()
        tree = [
            {
                "depth": 1,
                "fqn": "App\\VO\\UuidVO",
                "file": "src/Entity/User.php",
                "line": 10,
                "children": [],
            },
        ]
        d = deps_tree_to_dict(target, 1, tree)
        assert d["target"]["fqn"] == "App\\Entity\\User::getId()"
        assert d["target"]["file"] == "src/Entity/User.php"
        assert d["max_depth"] == 1
        assert d["total"] == 1
        assert d["tree"][0]["line"] == 11  # 0-based to 1-based
