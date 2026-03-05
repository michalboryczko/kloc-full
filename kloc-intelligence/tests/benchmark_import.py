"""Import benchmark: measures parse, node import, edge import, and validation times.

Usage:
    cd kloc-intelligence
    uv run python tests/benchmark_import.py [sot_path] [runs]

Defaults:
    sot_path = ../data/uestate/sot.json
    runs = 3
"""

from __future__ import annotations

import os
import sys
import time
import statistics

# Add project root to path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

from src.db.connection import Neo4jConnection
from src.db.schema import ensure_schema, drop_all
from src.db.importer import parse_sot, import_nodes, import_edges, validate_import


def run_benchmark(sot_path: str, runs: int = 3) -> dict:
    """Run the import benchmark multiple times and return results."""
    conn = Neo4jConnection()
    conn.verify_connectivity()

    results = []
    for i in range(runs):
        timings: dict = {}

        # Parse
        t0 = time.perf_counter()
        nodes, edges = parse_sot(sot_path)
        timings["parse"] = time.perf_counter() - t0

        # Clear + schema
        t0 = time.perf_counter()
        drop_all(conn)
        ensure_schema(conn)
        timings["clear_schema"] = time.perf_counter() - t0

        # Node import
        t0 = time.perf_counter()
        import_nodes(conn, nodes)
        timings["nodes"] = time.perf_counter() - t0

        # Edge import
        t0 = time.perf_counter()
        import_edges(conn, edges)
        timings["edges"] = time.perf_counter() - t0

        # Validation
        t0 = time.perf_counter()
        validate_import(conn, len(nodes), len(edges))
        timings["validate"] = time.perf_counter() - t0

        timings["total"] = sum(timings.values())
        results.append(timings)
        print(
            f"  Run {i + 1}/{runs}: "
            f"total={timings['total']:.1f}s "
            f"(parse={timings['parse']:.1f}s, "
            f"nodes={timings['nodes']:.1f}s, "
            f"edges={timings['edges']:.1f}s)"
        )

    conn.close()

    # Compute averages
    avg = {
        key: statistics.mean(r[key] for r in results) for key in results[0]
    }

    return {
        "runs": results,
        "average": avg,
        "node_count": len(nodes),
        "edge_count": len(edges),
    }


def main():
    sot_path = sys.argv[1] if len(sys.argv) > 1 else "../data/uestate/sot.json"
    runs = int(sys.argv[2]) if len(sys.argv) > 2 else 3

    if not os.path.exists(sot_path):
        print(f"Error: File not found: {sot_path}")
        sys.exit(1)

    print("\n=== Import Benchmark ===")
    print(f"File: {sot_path}")
    print(f"Runs: {runs}\n")

    result = run_benchmark(sot_path, runs)

    print("\n=== Results ===")
    print(f"Dataset: {result['node_count']:,} nodes, {result['edge_count']:,} edges")
    print(f"Average total: {result['average']['total']:.1f}s")
    print(f"  Parse:       {result['average']['parse']:.1f}s")
    print(f"  Clear/Schema:{result['average']['clear_schema']:.1f}s")
    print(f"  Nodes:       {result['average']['nodes']:.1f}s")
    print(f"  Edges:       {result['average']['edges']:.1f}s")
    print(f"  Validate:    {result['average']['validate']:.1f}s")

    # Check against targets
    if result["node_count"] < 50000:
        target = 5.0
        label = "15K"
    else:
        target = 120.0
        label = "721K"

    avg_total = result["average"]["total"]
    status = "PASS" if avg_total < target else "FAIL"
    print(f"\nTarget ({label}): <{target:.0f}s  Actual: {avg_total:.1f}s  [{status}]")


if __name__ == "__main__":
    main()
