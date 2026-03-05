"""Tests for inherit query functions."""

from __future__ import annotations

from tests.conftest import requires_neo4j
from src.db.query_runner import QueryRunner
from src.db.queries.inherit import inherit_tree


@requires_neo4j
class TestInheritUp:
    """Test inheritance upward (ancestors) queries."""

    def test_inherit_class_up(self, loaded_database):
        """Class inheriting from abstract class -- up direction."""
        runner = QueryRunner(loaded_database)
        record = runner.execute_single(
            "MATCH (n:Node {fqn: $fqn}) RETURN n.node_id AS nid",
            fqn="App\\Application\\EventListener\\Estate\\EstateCreatedEventListener",
        )
        node_id = record["nid"]

        result = inherit_tree(runner, node_id, direction="up", depth=1)
        assert result["direction"] == "up"
        assert result["max_depth"] == 1
        assert result["root"].fqn == "App\\Application\\EventListener\\Estate\\EstateCreatedEventListener"
        assert len(result["tree"]) == 1
        assert result["tree"][0]["fqn"] == "App\\Application\\EventListener\\AbstractEventListener"
        assert result["tree"][0]["kind"] == "Class"

    def test_inherit_interface_up_empty(self, loaded_database):
        """Interface with no parents -- up returns empty tree."""
        runner = QueryRunner(loaded_database)
        record = runner.execute_single(
            "MATCH (n:Node {fqn: $fqn}) RETURN n.node_id AS nid",
            fqn="App\\Domain\\Repository\\EstateRepository",
        )
        node_id = record["nid"]

        result = inherit_tree(runner, node_id, direction="up", depth=1)
        assert result["tree"] == []

    def test_inherit_enum_up_empty(self, loaded_database):
        """Enum with no parents -- up returns empty tree."""
        runner = QueryRunner(loaded_database)
        record = runner.execute_single(
            "MATCH (n:Node {fqn: $fqn}) RETURN n.node_id AS nid",
            fqn="App\\Domain\\Model\\DepositPolicyCharge\\ChargeType",
        )
        node_id = record["nid"]

        result = inherit_tree(runner, node_id, direction="up", depth=1)
        assert result["tree"] == []

    def test_inherit_class_depth2(self, loaded_database):
        """Multi-level inheritance at depth 2."""
        runner = QueryRunner(loaded_database)
        record = runner.execute_single(
            "MATCH (n:Node {fqn: $fqn}) RETURN n.node_id AS nid",
            fqn="App\\Application\\EventListener\\Estate\\EstateCreatedEventListener",
        )
        node_id = record["nid"]

        result = inherit_tree(runner, node_id, direction="up", depth=2)
        assert result["max_depth"] == 2
        # At depth 2, might find grandparent if exists
        assert len(result["tree"]) >= 1


@requires_neo4j
class TestInheritDown:
    """Test inheritance downward (descendants) queries."""

    def test_inherit_class_down(self, loaded_database):
        """Abstract class descendants -- down direction."""
        runner = QueryRunner(loaded_database)
        record = runner.execute_single(
            "MATCH (n:Node {fqn: $fqn}) RETURN n.node_id AS nid",
            fqn="App\\Application\\EventListener\\AbstractEventListener",
        )
        node_id = record["nid"]

        result = inherit_tree(runner, node_id, direction="down", depth=1)
        assert result["direction"] == "down"
        assert result["max_depth"] == 1
        assert len(result["tree"]) == 19  # 19 event listener subclasses

        # All should be Classes at depth 1
        for entry in result["tree"]:
            assert entry["depth"] == 1
            assert entry["kind"] == "Class"
            assert entry["children"] == []

    def test_inherit_interface_down(self, loaded_database):
        """Interface implementors -- down direction."""
        runner = QueryRunner(loaded_database)
        record = runner.execute_single(
            "MATCH (n:Node {fqn: $fqn}) RETURN n.node_id AS nid",
            fqn="App\\Domain\\Repository\\EstateRepository",
        )
        node_id = record["nid"]

        result = inherit_tree(runner, node_id, direction="down", depth=1)
        assert len(result["tree"]) == 1
        assert "Doctrine" in result["tree"][0]["fqn"]

    def test_inherit_respects_limit(self, loaded_database):
        """Inherit tree respects the limit parameter."""
        runner = QueryRunner(loaded_database)
        record = runner.execute_single(
            "MATCH (n:Node {fqn: $fqn}) RETURN n.node_id AS nid",
            fqn="App\\Application\\EventListener\\AbstractEventListener",
        )
        node_id = record["nid"]

        result = inherit_tree(runner, node_id, direction="down", depth=1, limit=5)
        assert len(result["tree"]) <= 5

    def test_inherit_visited_prevents_revisit(self, loaded_database):
        """BFS visited set prevents re-visiting nodes."""
        runner = QueryRunner(loaded_database)
        record = runner.execute_single(
            "MATCH (n:Node {fqn: $fqn}) RETURN n.node_id AS nid",
            fqn="App\\Application\\EventListener\\AbstractEventListener",
        )
        node_id = record["nid"]

        result = inherit_tree(runner, node_id, direction="down", depth=2, limit=50)
        all_ids = set()

        def collect_ids(entries):
            for e in entries:
                assert e["node_id"] not in all_ids, f"Duplicate: {e['fqn']}"
                all_ids.add(e["node_id"])
                collect_ids(e.get("children", []))

        collect_ids(result["tree"])


@requires_neo4j
class TestInheritValidation:
    """Test inherit query validation."""

    def test_inherit_nonexistent_raises(self, loaded_database):
        """Nonexistent node raises ValueError."""
        runner = QueryRunner(loaded_database)
        import pytest
        with pytest.raises(ValueError, match="Node not found"):
            inherit_tree(runner, "nonexistent-id")

    def test_inherit_method_raises(self, loaded_database):
        """Method node raises ValueError (only Class/Interface/Trait/Enum allowed)."""
        runner = QueryRunner(loaded_database)
        record = runner.execute_single(
            "MATCH (n:Node {fqn: $fqn}) RETURN n.node_id AS nid",
            fqn="App\\Domain\\Model\\Estate\\Estate::getId()",
        )
        node_id = record["nid"]

        import pytest
        with pytest.raises(ValueError, match="must be Class/Interface/Trait/Enum"):
            inherit_tree(runner, node_id)
