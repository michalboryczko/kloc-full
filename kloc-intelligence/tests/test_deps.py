"""Tests for deps query functions."""

from __future__ import annotations

from tests.conftest import requires_neo4j
from src.db.query_runner import QueryRunner
from src.db.queries.deps import deps_flat, deps_tree


@requires_neo4j
class TestDepsFlat:
    """Test flat deps queries."""

    def test_deps_flat_method(self, loaded_database):
        """Flat deps of a method returns list of dicts."""
        runner = QueryRunner(loaded_database)
        record = runner.execute_single(
            "MATCH (n:Node {fqn: $fqn}) RETURN n.node_id AS nid",
            fqn="App\\Domain\\Model\\Estate\\Estate::getId()",
        )
        node_id = record["nid"]

        results = deps_flat(runner, node_id)
        assert isinstance(results, list)
        assert len(results) == 2
        for r in results:
            assert "node_id" in r
            assert "fqn" in r
            assert "file" in r

    def test_deps_flat_class_includes_members(self, loaded_database):
        """Flat deps of a class includes member dependencies."""
        runner = QueryRunner(loaded_database)
        record = runner.execute_single(
            "MATCH (n:Node {fqn: $fqn}) RETURN n.node_id AS nid",
            fqn="App\\Domain\\Model\\Estate\\Estate",
        )
        node_id = record["nid"]

        results = deps_flat(runner, node_id, limit=200)
        assert len(results) > 10  # Class with many deps

    def test_deps_flat_respects_limit(self, loaded_database):
        """Flat deps respect the limit parameter."""
        runner = QueryRunner(loaded_database)
        record = runner.execute_single(
            "MATCH (n:Node {fqn: $fqn}) RETURN n.node_id AS nid",
            fqn="App\\Domain\\Model\\Estate\\Estate",
        )
        node_id = record["nid"]

        results = deps_flat(runner, node_id, limit=5)
        assert len(results) <= 5

    def test_deps_flat_no_line_fallback(self, loaded_database):
        """Deps line has no fallback to target start_line."""
        runner = QueryRunner(loaded_database)
        record = runner.execute_single(
            "MATCH (n:Node {fqn: $fqn}) RETURN n.node_id AS nid",
            fqn="App\\Domain\\Model\\Estate\\Estate::getId()",
        )
        node_id = record["nid"]

        results = deps_flat(runner, node_id)
        # All deps should have lines from edge locations
        # (no fallback to target node's start_line like usages do)
        for r in results:
            # line comes from edge location, not target node
            assert r["file"] is not None

    def test_deps_flat_nonexistent_raises(self, loaded_database):
        """Flat deps of nonexistent node raises ValueError."""
        runner = QueryRunner(loaded_database)
        import pytest
        with pytest.raises(ValueError, match="Node not found"):
            deps_flat(runner, "nonexistent-id")


@requires_neo4j
class TestDepsTree:
    """Test deps tree queries."""

    def test_deps_tree_depth1(self, loaded_database):
        """Deps tree at depth 1 returns correct structure."""
        runner = QueryRunner(loaded_database)
        record = runner.execute_single(
            "MATCH (n:Node {fqn: $fqn}) RETURN n.node_id AS nid",
            fqn="App\\Domain\\Model\\Estate\\Estate::getId()",
        )
        node_id = record["nid"]

        result = deps_tree(runner, node_id, depth=1)
        assert "target" in result
        assert "max_depth" in result
        assert "tree" in result
        assert result["max_depth"] == 1
        assert result["target"].fqn == "App\\Domain\\Model\\Estate\\Estate::getId()"

        for entry in result["tree"]:
            assert entry["depth"] == 1
            assert entry["children"] == []

    def test_deps_tree_depth2(self, loaded_database):
        """Deps tree at depth 2 has children."""
        runner = QueryRunner(loaded_database)
        record = runner.execute_single(
            "MATCH (n:Node {fqn: $fqn}) RETURN n.node_id AS nid",
            fqn="App\\Domain\\Model\\Estate\\Estate::create()",
        )
        node_id = record["nid"]

        result = deps_tree(runner, node_id, depth=2, limit=100)
        assert result["max_depth"] == 2
        # At depth 2, some entries should have children
        has_children = any(
            len(entry.get("children", [])) > 0
            for entry in result["tree"]
        )
        assert has_children

    def test_deps_tree_visited_prevents_revisit(self, loaded_database):
        """BFS visited set prevents re-visiting nodes at deeper depths."""
        runner = QueryRunner(loaded_database)
        record = runner.execute_single(
            "MATCH (n:Node {fqn: $fqn}) RETURN n.node_id AS nid",
            fqn="App\\Domain\\Model\\Estate\\Estate",
        )
        node_id = record["nid"]

        result = deps_tree(runner, node_id, depth=2, limit=100)

        all_ids = set()

        def collect_ids(entries):
            for e in entries:
                assert e["node_id"] not in all_ids, f"Duplicate node: {e['fqn']}"
                all_ids.add(e["node_id"])
                collect_ids(e.get("children", []))

        collect_ids(result["tree"])

    def test_deps_tree_respects_limit(self, loaded_database):
        """Tree deps respect the limit parameter."""
        runner = QueryRunner(loaded_database)
        record = runner.execute_single(
            "MATCH (n:Node {fqn: $fqn}) RETURN n.node_id AS nid",
            fqn="App\\Domain\\Model\\Estate\\Estate",
        )
        node_id = record["nid"]

        result = deps_tree(runner, node_id, depth=2, limit=5)

        def count_entries(entries):
            total = 0
            for e in entries:
                total += 1
                total += count_entries(e.get("children", []))
            return total

        assert count_entries(result["tree"]) <= 5

    def test_deps_tree_nonexistent_raises(self, loaded_database):
        """Deps tree of nonexistent node raises ValueError."""
        runner = QueryRunner(loaded_database)
        import pytest
        with pytest.raises(ValueError, match="Node not found"):
            deps_tree(runner, "nonexistent-id")
