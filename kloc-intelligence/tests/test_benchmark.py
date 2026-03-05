"""Performance benchmarks for kloc-intelligence queries.

Measures query execution time against the uestate test dataset (15K nodes).
Run with: pytest tests/test_benchmark.py -v -s

These benchmarks use the same Neo4j connection as snapshot tests.
"""

from __future__ import annotations

import time
from dataclasses import dataclass

import pytest

# Symbols for benchmarking (chosen for representative complexity)
BENCH_CLASS = "App\\Domain\\Model\\Estate\\Estate"
BENCH_METHOD = "App\\Domain\\Model\\Estate\\Estate::create()"
BENCH_INTERFACE = "App\\Domain\\Repository\\EstateRepository"
BENCH_PROPERTY = "App\\Domain\\Model\\Estate\\Estate::$id"
BENCH_ENUM = "App\\Domain\\Model\\DepositPolicyCharge\\ChargeType"
BENCH_TRAIT = "App\\Domain\\Model\\EnumHelper"
BENCH_FILE = "src/Domain/Model/Estate/Estate.php"


@dataclass
class BenchResult:
    name: str
    min_ms: float
    max_ms: float
    mean_ms: float
    rounds: int


def _bench(fn, rounds: int = 5, warmup: int = 1) -> BenchResult:
    """Run a benchmark function multiple times and return stats."""
    # Warmup
    for _ in range(warmup):
        fn()

    times = []
    for _ in range(rounds):
        start = time.perf_counter()
        fn()
        elapsed = (time.perf_counter() - start) * 1000
        times.append(elapsed)

    return BenchResult(
        name="",
        min_ms=min(times),
        max_ms=max(times),
        mean_ms=sum(times) / len(times),
        rounds=rounds,
    )


@pytest.mark.benchmark
class TestBenchmark:
    """Performance benchmarks against uestate dataset (15K nodes)."""

    @pytest.fixture(autouse=True)
    def setup(self, loaded_database):
        """Use the shared loaded database fixture."""
        self.conn = loaded_database

    def _get_runner(self):
        from src.db.query_runner import QueryRunner
        return QueryRunner(self.conn)

    def test_resolve_exact(self):
        """Benchmark exact FQN resolution."""
        from src.db.queries.resolve import resolve_symbol

        runner = self._get_runner()
        result = _bench(lambda: resolve_symbol(runner, BENCH_CLASS), rounds=10)
        print(f"\n  resolve (exact):  min={result.min_ms:.1f}ms  max={result.max_ms:.1f}ms  mean={result.mean_ms:.1f}ms")
        assert result.mean_ms < 50, f"resolve too slow: {result.mean_ms:.1f}ms"

    def test_usages_flat(self):
        """Benchmark flat usages query."""
        from src.db.queries.resolve import resolve_symbol
        from src.db.queries.usages import usages_tree

        runner = self._get_runner()
        nodes = resolve_symbol(runner, BENCH_CLASS)
        node = nodes[0]
        result = _bench(lambda: usages_tree(runner, node.node_id, depth=1), rounds=5)
        print(f"\n  usages (flat):    min={result.min_ms:.1f}ms  max={result.max_ms:.1f}ms  mean={result.mean_ms:.1f}ms")
        assert result.mean_ms < 2000, f"usages too slow: {result.mean_ms:.1f}ms"

    def test_deps_flat(self):
        """Benchmark flat deps query."""
        from src.db.queries.resolve import resolve_symbol
        from src.db.queries.deps import deps_tree

        runner = self._get_runner()
        nodes = resolve_symbol(runner, BENCH_CLASS)
        node = nodes[0]
        result = _bench(lambda: deps_tree(runner, node.node_id, depth=1), rounds=5)
        print(f"\n  deps (flat):      min={result.min_ms:.1f}ms  max={result.max_ms:.1f}ms  mean={result.mean_ms:.1f}ms")
        assert result.mean_ms < 1000, f"deps too slow: {result.mean_ms:.1f}ms"

    def test_context_class_d1(self):
        """Benchmark class context at depth 1."""
        from src.orchestration.context import ContextOrchestrator

        runner = self._get_runner()
        orchestrator = ContextOrchestrator(runner)
        result = _bench(lambda: orchestrator.execute_symbol(BENCH_CLASS, depth=1), rounds=5)
        print(f"\n  context class d1: min={result.min_ms:.1f}ms  max={result.max_ms:.1f}ms  mean={result.mean_ms:.1f}ms")
        assert result.mean_ms < 500, f"context d1 too slow: {result.mean_ms:.1f}ms"

    def test_context_method_d1(self):
        """Benchmark method context at depth 1."""
        from src.orchestration.context import ContextOrchestrator

        runner = self._get_runner()
        orchestrator = ContextOrchestrator(runner)
        result = _bench(lambda: orchestrator.execute_symbol(BENCH_METHOD, depth=1), rounds=5)
        print(f"\n  context method d1: min={result.min_ms:.1f}ms  max={result.max_ms:.1f}ms  mean={result.mean_ms:.1f}ms")
        assert result.mean_ms < 500, f"context method d1 too slow: {result.mean_ms:.1f}ms"

    def test_context_class_d2(self):
        """Benchmark class context at depth 2."""
        from src.orchestration.context import ContextOrchestrator

        runner = self._get_runner()
        orchestrator = ContextOrchestrator(runner)
        result = _bench(lambda: orchestrator.execute_symbol(BENCH_CLASS, depth=2), rounds=3)
        print(f"\n  context class d2: min={result.min_ms:.1f}ms  max={result.max_ms:.1f}ms  mean={result.mean_ms:.1f}ms")
        assert result.mean_ms < 2000, f"context d2 too slow: {result.mean_ms:.1f}ms"

    def test_owners(self):
        """Benchmark owners chain query."""
        from src.db.queries.resolve import resolve_symbol
        from src.db.queries.owners import owners_chain

        runner = self._get_runner()
        nodes = resolve_symbol(runner, BENCH_METHOD)
        node = nodes[0]
        result = _bench(lambda: owners_chain(runner, node.node_id), rounds=10)
        print(f"\n  owners:           min={result.min_ms:.1f}ms  max={result.max_ms:.1f}ms  mean={result.mean_ms:.1f}ms")
        assert result.mean_ms < 50, f"owners too slow: {result.mean_ms:.1f}ms"

    def test_inherit(self):
        """Benchmark inherit tree query."""
        from src.db.queries.resolve import resolve_symbol
        from src.db.queries.inherit import inherit_tree

        runner = self._get_runner()
        nodes = resolve_symbol(runner, BENCH_CLASS)
        node = nodes[0]
        result = _bench(lambda: inherit_tree(runner, node.node_id, direction="up", depth=2), rounds=10)
        print(f"\n  inherit:          min={result.min_ms:.1f}ms  max={result.max_ms:.1f}ms  mean={result.mean_ms:.1f}ms")
        assert result.mean_ms < 50, f"inherit too slow: {result.mean_ms:.1f}ms"

    def test_overrides(self):
        """Benchmark overrides tree query."""
        from src.db.queries.resolve import resolve_symbol
        from src.db.queries.overrides import overrides_tree

        runner = self._get_runner()
        nodes = resolve_symbol(runner, BENCH_METHOD)
        node = nodes[0]
        result = _bench(lambda: overrides_tree(runner, node.node_id, direction="up", depth=2), rounds=10)
        print(f"\n  overrides:        min={result.min_ms:.1f}ms  max={result.max_ms:.1f}ms  mean={result.mean_ms:.1f}ms")
        assert result.mean_ms < 50, f"overrides too slow: {result.mean_ms:.1f}ms"

    def test_context_file(self):
        """Benchmark file context (most complex query)."""
        from src.orchestration.context import ContextOrchestrator

        runner = self._get_runner()
        orchestrator = ContextOrchestrator(runner)
        result = _bench(lambda: orchestrator.execute_symbol(BENCH_FILE, depth=1), rounds=3)
        print(f"\n  context file d1:  min={result.min_ms:.1f}ms  max={result.max_ms:.1f}ms  mean={result.mean_ms:.1f}ms")
        assert result.mean_ms < 2000, f"context file too slow: {result.mean_ms:.1f}ms"

    def test_full_output_pipeline(self):
        """Benchmark full output pipeline: execute + serialize."""
        from src.orchestration.context import ContextOrchestrator
        from src.models.output import ContextOutput

        runner = self._get_runner()
        orchestrator = ContextOrchestrator(runner)

        def full_pipeline():
            result = orchestrator.execute_symbol(BENCH_CLASS, depth=1)
            output = ContextOutput.from_result(result)
            return output.to_dict()

        result = _bench(full_pipeline, rounds=5)
        print(f"\n  full pipeline:    min={result.min_ms:.1f}ms  max={result.max_ms:.1f}ms  mean={result.mean_ms:.1f}ms")
        assert result.mean_ms < 1000, f"full pipeline too slow: {result.mean_ms:.1f}ms"
