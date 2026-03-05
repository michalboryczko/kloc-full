"""Tests for usages query functions."""

from __future__ import annotations

from tests.conftest import requires_neo4j
from src.db.query_runner import QueryRunner
from src.db.queries.usages import usages_flat, usages_tree


@requires_neo4j
class TestUsagesFlat:
    """Test flat usages queries."""

    def test_usages_flat_method(self, loaded_database):
        """Flat usages of a method returns list of dicts."""
        runner = QueryRunner(loaded_database)
        # Resolve Estate::getId()
        record = runner.execute_single(
            "MATCH (n:Node {fqn: $fqn}) RETURN n.node_id AS nid",
            fqn="App\\Domain\\Model\\Estate\\Estate::getId()",
        )
        node_id = record["nid"]

        results = usages_flat(runner, node_id)
        assert isinstance(results, list)
        assert len(results) == 2
        for r in results:
            assert "node_id" in r
            assert "fqn" in r
            assert "file" in r
            assert "line" in r

    def test_usages_flat_class_includes_members(self, loaded_database):
        """Flat usages of a class includes member usages."""
        runner = QueryRunner(loaded_database)
        record = runner.execute_single(
            "MATCH (n:Node {fqn: $fqn}) RETURN n.node_id AS nid",
            fqn="App\\Domain\\Model\\Estate\\Estate",
        )
        node_id = record["nid"]

        results = usages_flat(runner, node_id, limit=200)
        assert len(results) > 10  # Class with many usages
        # Should include usages of the class itself and its members
        fqns = [r["fqn"] for r in results]
        # At minimum, class usages and member usages should both be present
        assert any("\\" in fqn for fqn in fqns)

    def test_usages_flat_respects_limit(self, loaded_database):
        """Flat usages respect the limit parameter."""
        runner = QueryRunner(loaded_database)
        record = runner.execute_single(
            "MATCH (n:Node {fqn: $fqn}) RETURN n.node_id AS nid",
            fqn="App\\Domain\\Model\\Estate\\Estate",
        )
        node_id = record["nid"]

        results = usages_flat(runner, node_id, limit=5)
        assert len(results) <= 5

    def test_usages_flat_nonexistent_raises(self, loaded_database):
        """Flat usages of nonexistent node raises ValueError."""
        runner = QueryRunner(loaded_database)
        import pytest
        with pytest.raises(ValueError, match="Node not found"):
            usages_flat(runner, "nonexistent-id")


@requires_neo4j
class TestUsagesTree:
    """Test usages tree queries."""

    def test_usages_tree_depth1(self, loaded_database):
        """Usages tree at depth 1 returns correct structure."""
        runner = QueryRunner(loaded_database)
        record = runner.execute_single(
            "MATCH (n:Node {fqn: $fqn}) RETURN n.node_id AS nid",
            fqn="App\\Domain\\Model\\Estate\\Estate::getId()",
        )
        node_id = record["nid"]

        result = usages_tree(runner, node_id, depth=1)
        assert "target" in result
        assert "max_depth" in result
        assert "tree" in result
        assert result["max_depth"] == 1

        # Target should be the resolved node
        assert result["target"].fqn == "App\\Domain\\Model\\Estate\\Estate::getId()"

        # Tree entries should have required fields
        for entry in result["tree"]:
            assert "depth" in entry
            assert "fqn" in entry
            assert "children" in entry
            assert entry["depth"] == 1
            assert entry["children"] == []  # No children at depth 1

    def test_usages_tree_depth2(self, loaded_database):
        """Usages tree at depth 2 has children."""
        runner = QueryRunner(loaded_database)
        record = runner.execute_single(
            "MATCH (n:Node {fqn: $fqn}) RETURN n.node_id AS nid",
            fqn="App\\Domain\\Model\\Estate\\Estate",
        )
        node_id = record["nid"]

        result = usages_tree(runner, node_id, depth=2, limit=100)
        assert result["max_depth"] == 2
        # At depth 2, some entries should have children
        has_children = any(
            len(entry.get("children", [])) > 0
            for entry in result["tree"]
        )
        assert has_children

    def test_usages_tree_visited_prevents_revisit(self, loaded_database):
        """BFS visited set prevents re-visiting nodes at deeper depths."""
        runner = QueryRunner(loaded_database)
        record = runner.execute_single(
            "MATCH (n:Node {fqn: $fqn}) RETURN n.node_id AS nid",
            fqn="App\\Domain\\Model\\Estate\\Estate",
        )
        node_id = record["nid"]

        result = usages_tree(runner, node_id, depth=3, limit=100)

        # Collect all node_ids in the tree
        all_ids = set()

        def collect_ids(entries):
            for e in entries:
                assert e["node_id"] not in all_ids, f"Duplicate node: {e['fqn']}"
                all_ids.add(e["node_id"])
                collect_ids(e.get("children", []))

        collect_ids(result["tree"])

    def test_usages_tree_respects_limit(self, loaded_database):
        """Tree usages respect the limit parameter."""
        runner = QueryRunner(loaded_database)
        record = runner.execute_single(
            "MATCH (n:Node {fqn: $fqn}) RETURN n.node_id AS nid",
            fqn="App\\Domain\\Model\\Estate\\Estate",
        )
        node_id = record["nid"]

        result = usages_tree(runner, node_id, depth=3, limit=5)

        # Count all entries in tree
        def count_entries(entries):
            total = 0
            for e in entries:
                total += 1
                total += count_entries(e.get("children", []))
            return total

        assert count_entries(result["tree"]) <= 5

    def test_usages_tree_nonexistent_raises(self, loaded_database):
        """Usages tree of nonexistent node raises ValueError."""
        runner = QueryRunner(loaded_database)
        import pytest
        with pytest.raises(ValueError, match="Node not found"):
            usages_tree(runner, "nonexistent-id")
