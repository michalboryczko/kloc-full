"""Tests for owners query functions."""

from __future__ import annotations

from tests.conftest import requires_neo4j
from src.db.query_runner import QueryRunner
from src.db.queries.owners import owners_chain


@requires_neo4j
class TestOwnersChain:
    """Test ownership chain queries."""

    def test_owners_method(self, loaded_database):
        """Method -> Class -> File ownership chain."""
        runner = QueryRunner(loaded_database)
        record = runner.execute_single(
            "MATCH (n:Node {fqn: $fqn}) RETURN n.node_id AS nid",
            fqn="App\\Domain\\Model\\Estate\\Estate::getId()",
        )
        node_id = record["nid"]

        result = owners_chain(runner, node_id)
        chain = result["chain"]
        assert len(chain) == 3
        assert chain[0].kind == "Method"
        assert chain[0].fqn == "App\\Domain\\Model\\Estate\\Estate::getId()"
        assert chain[1].kind == "Class"
        assert chain[1].fqn == "App\\Domain\\Model\\Estate\\Estate"
        assert chain[2].kind == "File"

    def test_owners_property(self, loaded_database):
        """Property -> Class -> File ownership chain."""
        runner = QueryRunner(loaded_database)
        record = runner.execute_single(
            "MATCH (n:Node {fqn: $fqn}) RETURN n.node_id AS nid",
            fqn="App\\Domain\\Model\\Estate\\Estate::$id",
        )
        node_id = record["nid"]

        result = owners_chain(runner, node_id)
        chain = result["chain"]
        assert len(chain) == 3
        assert chain[0].kind == "Property"
        assert chain[1].kind == "Class"
        assert chain[2].kind == "File"

    def test_owners_class(self, loaded_database):
        """Class -> File ownership chain (short chain)."""
        runner = QueryRunner(loaded_database)
        record = runner.execute_single(
            "MATCH (n:Node {fqn: $fqn}) RETURN n.node_id AS nid",
            fqn="App\\Domain\\Model\\Estate\\Estate",
        )
        node_id = record["nid"]

        result = owners_chain(runner, node_id)
        chain = result["chain"]
        assert len(chain) == 2
        assert chain[0].kind == "Class"
        assert chain[1].kind == "File"

    def test_owners_const(self, loaded_database):
        """Const -> Class -> File ownership chain."""
        runner = QueryRunner(loaded_database)
        record = runner.execute_single(
            "MATCH (n:Node {fqn: $fqn}) RETURN n.node_id AS nid",
            fqn="App\\Infrastructure\\Persistence\\DataFixtures\\PropertyFixtures::PROPERTY_4_ID",
        )
        node_id = record["nid"]

        result = owners_chain(runner, node_id)
        chain = result["chain"]
        assert len(chain) == 3
        assert chain[0].kind == "Const"
        assert chain[1].kind == "Class"
        assert chain[2].kind == "File"

    def test_owners_nonexistent_raises(self, loaded_database):
        """Nonexistent node raises ValueError."""
        runner = QueryRunner(loaded_database)
        import pytest
        with pytest.raises(ValueError, match="Node not found"):
            owners_chain(runner, "nonexistent-id")

    def test_owners_file_has_no_parent(self, loaded_database):
        """File node at root has only itself in chain."""
        runner = QueryRunner(loaded_database)
        record = runner.execute_single(
            "MATCH (n:Node {fqn: $fqn}) RETURN n.node_id AS nid",
            fqn="src/Domain/Model/Estate/Estate.php",
        )
        node_id = record["nid"]

        result = owners_chain(runner, node_id)
        chain = result["chain"]
        assert len(chain) == 1
        assert chain[0].kind == "File"
