"""Tests for symbol resolution (resolve_symbol cascade).

These tests require Neo4j with uestate data loaded.
The loaded_database fixture handles data import once per session.
"""

from __future__ import annotations

from tests.conftest import requires_neo4j
from src.db.query_runner import QueryRunner
from src.db.queries.resolve import resolve_symbol


@requires_neo4j
class TestResolve:
    """Test symbol resolution against uestate dataset."""

    def test_exact_class_fqn(self, loaded_database):
        """Exact FQN resolves a single class."""
        runner = QueryRunner(loaded_database)
        results = resolve_symbol(runner, "App\\Domain\\Model\\Estate\\Estate")
        assert len(results) == 1
        assert results[0].kind == "Class"
        assert results[0].fqn == "App\\Domain\\Model\\Estate\\Estate"
        assert results[0].name == "Estate"

    def test_exact_method_fqn(self, loaded_database):
        """Exact FQN resolves a method."""
        runner = QueryRunner(loaded_database)
        results = resolve_symbol(runner, "App\\Domain\\Model\\Estate\\Estate::getId()")
        assert len(results) == 1
        assert results[0].kind == "Method"
        assert results[0].fqn == "App\\Domain\\Model\\Estate\\Estate::getId()"

    def test_exact_interface_fqn(self, loaded_database):
        """Exact FQN resolves an interface."""
        runner = QueryRunner(loaded_database)
        results = resolve_symbol(runner, "App\\Domain\\Repository\\EstateRepository")
        assert len(results) == 1
        assert results[0].kind == "Interface"

    def test_exact_property_fqn(self, loaded_database):
        """Exact FQN resolves a property."""
        runner = QueryRunner(loaded_database)
        results = resolve_symbol(runner, "App\\Domain\\Model\\Estate\\Estate::$id")
        assert len(results) == 1
        assert results[0].kind == "Property"

    def test_exact_const_fqn(self, loaded_database):
        """Exact FQN resolves a constant."""
        runner = QueryRunner(loaded_database)
        results = resolve_symbol(
            runner,
            "App\\Infrastructure\\Persistence\\DataFixtures\\PropertyFixtures::PROPERTY_4_ID",
        )
        assert len(results) == 1
        assert results[0].kind == "Const"

    def test_exact_enum_fqn(self, loaded_database):
        """Exact FQN resolves an enum."""
        runner = QueryRunner(loaded_database)
        results = resolve_symbol(
            runner, "App\\Domain\\Model\\DepositPolicyCharge\\ChargeType"
        )
        assert len(results) == 1
        assert results[0].kind == "Enum"

    def test_case_insensitive(self, loaded_database):
        """Case-insensitive search finds the right node."""
        runner = QueryRunner(loaded_database)
        results = resolve_symbol(runner, "app\\domain\\model\\estate\\estate")
        assert len(results) >= 1
        assert results[0].fqn == "App\\Domain\\Model\\Estate\\Estate"

    def test_suffix_match(self, loaded_database):
        """Suffix match resolves when FQN ends with query."""
        runner = QueryRunner(loaded_database)
        results = resolve_symbol(runner, "Estate\\Estate")
        assert len(results) >= 1
        # Should find the Estate class via suffix
        fqns = [r.fqn for r in results]
        assert "App\\Domain\\Model\\Estate\\Estate" in fqns

    def test_no_match(self, loaded_database):
        """Non-existent symbol returns empty list."""
        runner = QueryRunner(loaded_database)
        results = resolve_symbol(runner, "ThisClassDoesNotExist12345")
        assert len(results) == 0

    def test_leading_backslash_stripped(self, loaded_database):
        """Leading backslash is stripped from the query."""
        runner = QueryRunner(loaded_database)
        results = resolve_symbol(runner, "\\App\\Domain\\Model\\Estate\\Estate")
        assert len(results) == 1
        assert results[0].fqn == "App\\Domain\\Model\\Estate\\Estate"

    def test_node_has_file_and_line(self, loaded_database):
        """Resolved node has file and line information."""
        runner = QueryRunner(loaded_database)
        results = resolve_symbol(runner, "App\\Domain\\Model\\Estate\\Estate")
        assert len(results) == 1
        node = results[0]
        assert node.file is not None
        assert node.start_line is not None
        assert node.file.endswith(".php")

    def test_node_id_property(self, loaded_database):
        """NodeData.id returns node_id."""
        runner = QueryRunner(loaded_database)
        results = resolve_symbol(runner, "App\\Domain\\Model\\Estate\\Estate")
        assert len(results) == 1
        node = results[0]
        assert node.id == node.node_id
        assert node.id.startswith("node:")

    def test_location_str(self, loaded_database):
        """NodeData.location_str returns file:line (1-based)."""
        runner = QueryRunner(loaded_database)
        results = resolve_symbol(runner, "App\\Domain\\Model\\Estate\\Estate")
        assert len(results) == 1
        node = results[0]
        loc = node.location_str
        assert ":" in loc
        # Line should be 1-based (start_line is 0-based, location_str adds 1)
        file_part, line_part = loc.rsplit(":", 1)
        assert int(line_part) > 0

    def test_searchable_kinds_filter(self, loaded_database):
        """Resolve should not return Call, Value, or Argument nodes."""
        runner = QueryRunner(loaded_database)
        # Even if we search for something broad, internal kinds should be excluded
        results = resolve_symbol(runner, "App\\Domain\\Model\\Estate\\Estate")
        for node in results:
            assert node.kind not in ("Call", "Value", "Argument")
