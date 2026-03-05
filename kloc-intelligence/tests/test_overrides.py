"""Tests for overrides query functions."""

from __future__ import annotations

from tests.conftest import requires_neo4j
from src.db.query_runner import QueryRunner
from src.db.queries.overrides import overrides_tree


@requires_neo4j
class TestOverridesUp:
    """Test overrides upward (overridden methods) queries."""

    def test_overrides_method_up(self, loaded_database):
        """Method overriding parent -- up direction."""
        runner = QueryRunner(loaded_database)
        record = runner.execute_single(
            "MATCH (n:Node {fqn: $fqn}) RETURN n.node_id AS nid",
            fqn="App\\Application\\EventListener\\Estate\\EstateCreatedEventListener::__construct()",
        )
        node_id = record["nid"]

        result = overrides_tree(runner, node_id, direction="up", depth=1)
        assert result["direction"] == "up"
        assert result["max_depth"] == 1
        assert result["root"].fqn == "App\\Application\\EventListener\\Estate\\EstateCreatedEventListener::__construct()"
        assert len(result["tree"]) == 1
        assert result["tree"][0]["fqn"] == "App\\Application\\EventListener\\AbstractEventListener::__construct()"

    def test_overrides_no_match(self, loaded_database):
        """Method with no overrides -- empty tree."""
        runner = QueryRunner(loaded_database)
        record = runner.execute_single(
            "MATCH (n:Node {fqn: $fqn}) RETURN n.node_id AS nid",
            fqn="App\\Domain\\Model\\Estate\\Estate::getId()",
        )
        node_id = record["nid"]

        result = overrides_tree(runner, node_id, direction="up", depth=1)
        assert result["tree"] == []

    def test_overrides_interface_method_up(self, loaded_database):
        """Method implementing interface method -- up direction."""
        runner = QueryRunner(loaded_database)
        record = runner.execute_single(
            "MATCH (n:Node {fqn: $fqn}) RETURN n.node_id AS nid",
            fqn="App\\Application\\Dto\\Request\\ListPaginationDto::getLimit()",
        )
        node_id = record["nid"]

        result = overrides_tree(runner, node_id, direction="up", depth=1)
        assert len(result["tree"]) == 1
        assert "ListPagination::getLimit()" in result["tree"][0]["fqn"]


@requires_neo4j
class TestOverridesDown:
    """Test overrides downward (overriding methods) queries."""

    def test_overrides_method_down(self, loaded_database):
        """Parent method overridden by children -- down direction."""
        runner = QueryRunner(loaded_database)
        record = runner.execute_single(
            "MATCH (n:Node {fqn: $fqn}) RETURN n.node_id AS nid",
            fqn="App\\Application\\EventListener\\AbstractEventListener::__construct()",
        )
        node_id = record["nid"]

        result = overrides_tree(runner, node_id, direction="down", depth=1)
        assert result["direction"] == "down"
        assert len(result["tree"]) == 2
        fqns = [e["fqn"] for e in result["tree"]]
        assert any("EstateCreatedEventListener" in fqn for fqn in fqns)
        assert any("EstateUpdatedEventListener" in fqn for fqn in fqns)

    def test_overrides_visited_prevents_revisit(self, loaded_database):
        """BFS visited set prevents duplicate nodes."""
        runner = QueryRunner(loaded_database)
        record = runner.execute_single(
            "MATCH (n:Node {fqn: $fqn}) RETURN n.node_id AS nid",
            fqn="App\\Application\\EventListener\\AbstractEventListener::__construct()",
        )
        node_id = record["nid"]

        result = overrides_tree(runner, node_id, direction="down", depth=2, limit=50)
        all_ids = set()

        def collect_ids(entries):
            for e in entries:
                assert e["node_id"] not in all_ids, f"Duplicate: {e['fqn']}"
                all_ids.add(e["node_id"])
                collect_ids(e.get("children", []))

        collect_ids(result["tree"])

    def test_overrides_respects_limit(self, loaded_database):
        """Overrides tree respects the limit parameter."""
        runner = QueryRunner(loaded_database)
        record = runner.execute_single(
            "MATCH (n:Node {fqn: $fqn}) RETURN n.node_id AS nid",
            fqn="App\\Application\\EventListener\\AbstractEventListener::__construct()",
        )
        node_id = record["nid"]

        result = overrides_tree(runner, node_id, direction="down", depth=1, limit=1)
        assert len(result["tree"]) <= 1


@requires_neo4j
class TestOverridesValidation:
    """Test overrides query validation."""

    def test_overrides_nonexistent_raises(self, loaded_database):
        """Nonexistent node raises ValueError."""
        runner = QueryRunner(loaded_database)
        import pytest
        with pytest.raises(ValueError, match="Node not found"):
            overrides_tree(runner, "nonexistent-id")

    def test_overrides_class_raises(self, loaded_database):
        """Class node raises ValueError (only Method allowed)."""
        runner = QueryRunner(loaded_database)
        record = runner.execute_single(
            "MATCH (n:Node {fqn: $fqn}) RETURN n.node_id AS nid",
            fqn="App\\Domain\\Model\\Estate\\Estate",
        )
        node_id = record["nid"]

        import pytest
        with pytest.raises(ValueError, match="must be Method"):
            overrides_tree(runner, node_id)
