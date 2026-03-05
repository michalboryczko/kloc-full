"""Tests for JSON output formatters."""

from __future__ import annotations

from src.models.node import NodeData
from src.output.json_formatter import (
    usages_tree_to_dict,
    deps_tree_to_dict,
    owners_chain_to_dict,
    inherit_tree_to_dict,
    overrides_tree_to_dict,
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


class TestOwnersChainToDict:
    """Test owners_chain_to_dict."""

    def test_basic_chain(self):
        chain = [
            NodeData(
                node_id="n1", kind="Method", name="getId",
                fqn="App\\Entity\\User::getId()", symbol="s1",
                file="src/Entity/User.php", start_line=109,
            ),
            NodeData(
                node_id="n2", kind="Class", name="User",
                fqn="App\\Entity\\User", symbol="s2",
                file="src/Entity/User.php", start_line=12,
            ),
            NodeData(
                node_id="n3", kind="File", name="User.php",
                fqn="src/Entity/User.php", symbol="s3",
                file="src/Entity/User.php", start_line=None,
            ),
        ]
        d = owners_chain_to_dict(chain)
        assert len(d["chain"]) == 3
        assert d["chain"][0]["kind"] == "Method"
        assert d["chain"][0]["fqn"] == "App\\Entity\\User::getId()"
        assert d["chain"][0]["line"] == 110  # 0-based to 1-based
        assert d["chain"][1]["kind"] == "Class"
        assert d["chain"][1]["line"] == 13
        assert d["chain"][2]["kind"] == "File"
        assert d["chain"][2]["line"] is None

    def test_empty_chain(self):
        d = owners_chain_to_dict([])
        assert d["chain"] == []


class TestInheritTreeToDict:
    """Test inherit_tree_to_dict."""

    def _make_root(self):
        return NodeData(
            node_id="n1", kind="Class", name="UserService",
            fqn="App\\Service\\UserService", symbol="s1",
            file="src/Service/UserService.php", start_line=10,
        )

    def test_basic_tree(self):
        root = self._make_root()
        tree = [
            {
                "depth": 1,
                "kind": "Class",
                "fqn": "App\\Service\\AbstractService",
                "file": "src/Service/AbstractService.php",
                "line": 5,
                "children": [],
            },
        ]
        d = inherit_tree_to_dict(root, "up", 1, tree)
        assert d["root"]["fqn"] == "App\\Service\\UserService"
        assert d["root"]["file"] == "src/Service/UserService.php"
        assert d["direction"] == "up"
        assert d["max_depth"] == 1
        assert d["total"] == 1
        assert d["tree"][0]["kind"] == "Class"
        assert d["tree"][0]["line"] == 6  # 0-based to 1-based

    def test_empty_tree(self):
        root = self._make_root()
        d = inherit_tree_to_dict(root, "up", 1, [])
        assert d["total"] == 0
        assert d["tree"] == []

    def test_nested_tree(self):
        root = self._make_root()
        tree = [
            {
                "depth": 1,
                "kind": "Class",
                "fqn": "Parent",
                "file": "parent.php",
                "line": 5,
                "children": [
                    {
                        "depth": 2,
                        "kind": "Interface",
                        "fqn": "GrandParent",
                        "file": "gp.php",
                        "line": 3,
                        "children": [],
                    },
                ],
            },
        ]
        d = inherit_tree_to_dict(root, "up", 2, tree)
        assert d["total"] == 2
        assert d["tree"][0]["children"][0]["kind"] == "Interface"
        assert d["tree"][0]["children"][0]["line"] == 4


class TestOverridesTreeToDict:
    """Test overrides_tree_to_dict."""

    def _make_root(self):
        return NodeData(
            node_id="n1", kind="Method", name="handle",
            fqn="App\\Handler\\UserHandler::handle()", symbol="s1",
            file="src/Handler/UserHandler.php", start_line=20,
        )

    def test_basic_tree(self):
        root = self._make_root()
        tree = [
            {
                "depth": 1,
                "fqn": "App\\Handler\\AbstractHandler::handle()",
                "file": "src/Handler/AbstractHandler.php",
                "line": 10,
                "children": [],
            },
        ]
        d = overrides_tree_to_dict(root, "up", 1, tree)
        assert d["root"]["fqn"] == "App\\Handler\\UserHandler::handle()"
        assert d["direction"] == "up"
        assert d["max_depth"] == 1
        assert d["total"] == 1
        assert d["tree"][0]["line"] == 11  # 0-based to 1-based
        # Overrides entries do NOT have 'kind'
        assert "kind" not in d["tree"][0]

    def test_empty_tree(self):
        root = self._make_root()
        d = overrides_tree_to_dict(root, "up", 1, [])
        assert d["total"] == 0
        assert d["tree"] == []

    def test_down_direction(self):
        root = self._make_root()
        tree = [
            {
                "depth": 1,
                "fqn": "App\\Handler\\ChildA::handle()",
                "file": "a.php",
                "line": 5,
                "children": [],
            },
            {
                "depth": 1,
                "fqn": "App\\Handler\\ChildB::handle()",
                "file": "b.php",
                "line": 8,
                "children": [],
            },
        ]
        d = overrides_tree_to_dict(root, "down", 1, tree)
        assert d["direction"] == "down"
        assert d["total"] == 2
        assert len(d["tree"]) == 2
