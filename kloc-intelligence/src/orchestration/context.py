"""Main context orchestrator: kind-based dispatch to specialized context builders.

Routes context queries to the appropriate handler based on target node kind:
- Class -> class_context
- Interface -> interface_context
- Method/Function -> method_context
- Property -> property_context
- File -> file_context
- Enum, Trait, Value, Constant -> generic_context

Special cases:
- __construct methods redirect USED BY to containing Class
"""

from __future__ import annotations

from ..db.query_runner import QueryRunner
from ..db.queries.definition import definition_for_node
from ..db.queries.resolve import resolve_symbol
from ..db.result_mapper import records_to_nodes
from ..models.node import NodeData
from ..models.results import ContextEntry, ContextResult


class ContextOrchestrator:
    """Main context orchestrator with kind-based dispatch.

    Usage:
        orchestrator = ContextOrchestrator(runner)
        result = orchestrator.execute(node_id, depth=1, limit=100)
        # or from symbol:
        result = orchestrator.execute_symbol("App\\MyClass", depth=1)
    """

    def __init__(self, runner: QueryRunner):
        self._runner = runner

    def execute(
        self,
        node_id: str,
        depth: int = 1,
        limit: int = 100,
        include_impl: bool = False,
    ) -> ContextResult:
        """Execute context query with kind-based dispatch.

        Args:
            node_id: Target node ID.
            depth: BFS depth for expansion.
            limit: Maximum results per direction.
            include_impl: If True, enables polymorphic analysis.

        Returns:
            ContextResult with tree structures for used_by and uses.

        Raises:
            ValueError: If node_id not found.
        """
        # Resolve target node
        records = self._runner.execute(
            "MATCH (n:Node {node_id: $id}) RETURN n",
            id=node_id,
        )
        if not records:
            raise ValueError(f"Node not found: {node_id}")

        nodes = records_to_nodes(records)
        target = nodes[0]

        # Build definition
        definition = definition_for_node(self._runner, node_id)

        # Dispatch USED BY and USES based on kind
        used_by = self._dispatch_used_by(target, depth, limit, include_impl)
        uses = self._dispatch_uses(target, depth, limit, include_impl)

        return ContextResult(
            target=target,
            max_depth=depth,
            used_by=used_by,
            uses=uses,
            definition=definition,
        )

    def execute_symbol(
        self,
        symbol: str,
        depth: int = 1,
        limit: int = 100,
        include_impl: bool = False,
    ) -> ContextResult:
        """Execute context query by symbol name.

        Resolves the symbol first, then dispatches context query.

        Args:
            symbol: FQN or partial symbol name.
            depth: BFS depth for expansion.
            limit: Maximum results per direction.
            include_impl: If True, enables polymorphic analysis.

        Returns:
            ContextResult with tree structures for used_by and uses.

        Raises:
            ValueError: If symbol not found.
        """
        candidates = resolve_symbol(self._runner, symbol)
        if not candidates:
            raise ValueError(f"Symbol not found: {symbol}")

        node = candidates[0]
        return self.execute(node.node_id, depth=depth, limit=limit, include_impl=include_impl)

    def _dispatch_used_by(
        self,
        target: NodeData,
        depth: int,
        limit: int,
        include_impl: bool,
    ) -> list[ContextEntry]:
        """Dispatch USED BY to appropriate handler based on node kind."""
        kind = target.kind
        node_id = target.node_id

        if kind == "Class":
            from .class_context import build_class_used_by
            return build_class_used_by(self._runner, node_id, depth, limit, include_impl)

        if kind == "Interface":
            from .interface_context import build_interface_used_by
            return build_interface_used_by(self._runner, node_id, depth, limit, include_impl)

        if kind == "Method":
            # ISSUE-A: __construct redirects to containing Class USED BY
            if target.name == "__construct":
                parent = self._runner.execute_single(
                    "MATCH (n:Node {node_id: $id})<-[:CONTAINS]-(p:Node) "
                    "WHERE p.kind IN ['Class', 'Enum'] "
                    "RETURN p.node_id AS parent_id",
                    id=node_id,
                )
                if parent:
                    from .class_context import build_class_used_by
                    return build_class_used_by(
                        self._runner, parent["parent_id"], depth, limit, include_impl
                    )

            from .method_context import build_method_used_by
            return build_method_used_by(self._runner, node_id, depth, limit, include_impl)

        if kind == "Property":
            from .property_context import build_property_used_by
            return build_property_used_by(self._runner, node_id, depth, limit, include_impl)

        if kind == "File":
            from .file_context import build_file_used_by
            return build_file_used_by(self._runner, node_id, depth, limit, include_impl)

        # Enum, Trait, Value, Constant, Function, etc. -> generic
        from .generic_context import build_generic_used_by
        return build_generic_used_by(self._runner, node_id, depth, limit, include_impl)

    def _dispatch_uses(
        self,
        target: NodeData,
        depth: int,
        limit: int,
        include_impl: bool,
    ) -> list[ContextEntry]:
        """Dispatch USES to appropriate handler based on node kind."""
        kind = target.kind
        node_id = target.node_id

        if kind == "Class":
            from .class_context import build_class_uses
            return build_class_uses(self._runner, node_id, depth, limit, include_impl)

        if kind == "Interface":
            from .interface_context import build_interface_uses
            return build_interface_uses(self._runner, node_id, depth, limit, include_impl)

        if kind == "Method":
            from .method_context import build_method_uses
            return build_method_uses(self._runner, node_id, depth, limit, include_impl)

        if kind == "Property":
            from .property_context import build_property_uses
            return build_property_uses(self._runner, node_id, depth, limit, include_impl)

        if kind == "File":
            from .file_context import build_file_uses
            return build_file_uses(self._runner, node_id, depth, limit, include_impl)

        # Enum, Trait, Value, Constant, Function, etc. -> generic
        from .generic_context import build_generic_uses
        return build_generic_uses(self._runner, node_id, depth, limit, include_impl)
