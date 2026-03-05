"""Tests for sot.json parser."""

from __future__ import annotations

import os

import pytest

from src.db.importer import (
    NodeSpec,
    EdgeSpec,
    node_to_props,
    edge_to_props,
    parse_sot,
)


class TestNodeToProps:
    """Tests for node_to_props conversion."""

    def test_basic_node(self):
        """Basic node fields are mapped correctly."""
        node = NodeSpec(
            id="abc123",
            kind="Class",
            name="OrderRepository",
            fqn="App\\Repository\\OrderRepository",
            symbol="scip-php composer ...",
        )
        props = node_to_props(node)
        assert props["node_id"] == "abc123"
        assert props["kind"] == "Class"
        assert props["name"] == "OrderRepository"
        assert props["fqn"] == "App\\Repository\\OrderRepository"
        assert props["symbol"] == "scip-php composer ..."
        assert "id" not in props  # id renamed to node_id

    def test_node_with_file(self):
        """File property is included when present."""
        node = NodeSpec(
            id="a", kind="Class", name="A", fqn="A", symbol="s",
            file="src/A.php",
        )
        props = node_to_props(node)
        assert props["file"] == "src/A.php"

    def test_node_with_range(self):
        """Range dict is flattened to individual properties."""
        node = NodeSpec(
            id="a", kind="Class", name="A", fqn="A", symbol="s",
            range={"start_line": 10, "start_col": 0, "end_line": 50, "end_col": 0},
        )
        props = node_to_props(node)
        assert props["start_line"] == 10
        assert props["start_col"] == 0
        assert props["end_line"] == 50
        assert props["end_col"] == 0

    def test_node_with_documentation(self):
        """Documentation list is preserved."""
        node = NodeSpec(
            id="a", kind="Method", name="getId", fqn="A::getId()", symbol="s",
            documentation=["```php\nfunction getId(): int\n```"],
        )
        props = node_to_props(node)
        assert props["documentation"] == ["```php\nfunction getId(): int\n```"]

    def test_node_without_optional_fields(self):
        """Optional fields are omitted when not present."""
        node = NodeSpec(
            id="a", kind="Class", name="A", fqn="A", symbol="s",
        )
        props = node_to_props(node)
        assert "file" not in props
        assert "start_line" not in props
        assert "documentation" not in props
        assert "value_kind" not in props
        assert "call_kind" not in props

    def test_value_node_fields(self):
        """Value node fields (value_kind, type_symbol) are mapped."""
        node = NodeSpec(
            id="v1", kind="Value", name="$user", fqn="method::$user", symbol="s",
            value_kind="parameter", type_symbol="App\\Entity\\User",
        )
        props = node_to_props(node)
        assert props["value_kind"] == "parameter"
        assert props["type_symbol"] == "App\\Entity\\User"

    def test_call_node_fields(self):
        """Call node fields (call_kind) are mapped."""
        node = NodeSpec(
            id="c1", kind="Call", name="findById", fqn="method::findById()", symbol="s",
            call_kind="method",
        )
        props = node_to_props(node)
        assert props["call_kind"] == "method"


class TestEdgeToProps:
    """Tests for edge_to_props conversion."""

    def test_basic_edge(self):
        """Basic edge fields are mapped correctly."""
        edge = EdgeSpec(type="uses", source="abc", target="def")
        props = edge_to_props(edge)
        assert props["type"] == "uses"
        assert props["source_id"] == "abc"
        assert props["target_id"] == "def"

    def test_edge_with_location(self):
        """Location dict is flattened to loc_file, loc_line."""
        edge = EdgeSpec(
            type="uses", source="abc", target="def",
            location={"file": "src/A.php", "line": 25},
        )
        props = edge_to_props(edge)
        assert props["loc_file"] == "src/A.php"
        assert props["loc_line"] == 25

    def test_edge_without_location(self):
        """Missing location produces null values."""
        edge = EdgeSpec(type="contains", source="abc", target="def")
        props = edge_to_props(edge)
        assert props["loc_file"] is None
        assert props["loc_line"] is None

    def test_argument_edge_fields(self):
        """Argument edge fields (position, expression, parameter) are mapped."""
        edge = EdgeSpec(
            type="argument", source="v1", target="c1",
            location={"file": "src/A.php", "line": 30},
            position=0, expression="$user", parameter="App\\Service::save()::$entity",
        )
        props = edge_to_props(edge)
        assert props["position"] == 0
        assert props["expression"] == "$user"
        assert props["parameter"] == "App\\Service::save()::$entity"


class TestParseSot:
    """Tests for parse_sot function."""

    def test_file_not_found(self):
        """FileNotFoundError for missing file."""
        with pytest.raises(FileNotFoundError):
            parse_sot("/nonexistent/sot.json")

    @pytest.mark.skipif(
        not os.path.exists("/Users/michal/dev/ai/kloc/data/uestate/sot.json"),
        reason="Test data not available",
    )
    def test_parse_uestate(self):
        """Parse uestate dataset (15K nodes)."""
        nodes, edges = parse_sot("/Users/michal/dev/ai/kloc/data/uestate/sot.json")
        assert len(nodes) > 10000
        assert len(edges) > 10000
        # Check first node has required fields
        assert "node_id" in nodes[0]
        assert "kind" in nodes[0]
        assert "fqn" in nodes[0]
