#!/usr/bin/env python3
"""Generate comprehensive test cases for kloc-cli context command regression testing.

Analyzes sot.json, picks random symbols per kind, and generates all
depth × impl combinations as cases.json.

Usage:
    python generate_cases.py <sot-json-path> [--seed SEED] [--count N]
"""

import json
import random
import sys
from collections import defaultdict
from pathlib import Path


# How many symbols to pick per kind (capped at available)
DEFAULT_COUNT = 20

# Symbol kinds to test and how to group them
SYMBOL_GROUPS = {
    "class": {"kind": "Class", "value_kind": None},
    "interface": {"kind": "Interface", "value_kind": None},
    "method": {"kind": "Method", "value_kind": None},
    "property": {"kind": "Property", "value_kind": None},
    "value-parameter": {"kind": "Value", "value_kind": "parameter"},
    "value-local": {"kind": "Value", "value_kind": "local"},
}

DEPTHS = [1, 2, 3, 4, 5]
IMPL_OPTIONS = [False, True]


def load_sot(path: str) -> dict:
    with open(path) as f:
        return json.load(f)


def pick_symbols(sot: dict, count: int, seed: int) -> dict[str, list[dict]]:
    """Pick random symbols for each group. Returns {group_name: [nodes]}."""
    rng = random.Random(seed)
    picked = {}

    for group_name, spec in SYMBOL_GROUPS.items():
        candidates = [
            n for n in sot["nodes"]
            if n["kind"] == spec["kind"]
            and (spec["value_kind"] is None or n.get("value_kind") == spec["value_kind"])
            and n.get("file") is not None  # skip external symbols
        ]

        # Deduplicate by FQN (some nodes share FQNs)
        seen_fqns = set()
        unique = []
        for n in candidates:
            if n["fqn"] not in seen_fqns:
                seen_fqns.add(n["fqn"])
                unique.append(n)
        candidates = unique

        n_pick = min(count, len(candidates))
        selected = rng.sample(candidates, n_pick)
        picked[group_name] = selected
        print(f"  {group_name}: picked {n_pick}/{len(candidates)} symbols", file=sys.stderr)

    return picked


def make_case_name(group: str, fqn: str, depth: int, impl: bool, used_names: set) -> str:
    """Generate a short unique case name."""
    # Extract short name from FQN
    short = fqn.split("\\")[-1] if "\\" in fqn else fqn
    # Clean up for case name
    short = short.replace("::", "-").replace("()", "").replace(".$", "-").replace(".", "-")
    short = short.replace("$", "").replace("@", "at")
    # Truncate long names
    if len(short) > 50:
        short = short[:50]

    suffix = f"-d{depth}"
    if impl:
        suffix += "-impl"
    base = f"{group}/{short}{suffix}"

    # Ensure uniqueness
    name = base
    counter = 2
    while name in used_names:
        name = f"{base}-{counter}"
        counter += 1
    used_names.add(name)
    return name


def generate_cases(picked: dict[str, list[dict]]) -> list[dict]:
    """Generate all case combinations."""
    cases = []
    used_names: set[str] = set()
    for group_name, nodes in picked.items():
        for node in nodes:
            for depth in DEPTHS:
                for impl in IMPL_OPTIONS:
                    name = make_case_name(group_name, node["fqn"], depth, impl, used_names)
                    cases.append({
                        "name": name,
                        "symbol": node["fqn"],
                        "depth": depth,
                        "impl": impl,
                        "category": group_name,
                    })
    return cases


def main():
    import argparse
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("sot_path", help="Path to sot.json")
    parser.add_argument("--seed", type=int, default=42, help="Random seed for reproducibility")
    parser.add_argument("--count", type=int, default=DEFAULT_COUNT, help="Symbols per kind")
    parser.add_argument("--output", default=None, help="Output path (default: cases.json in script dir)")
    args = parser.parse_args()

    sot_path = args.sot_path
    if not Path(sot_path).exists():
        print(f"Error: sot.json not found: {sot_path}", file=sys.stderr)
        sys.exit(1)

    print(f"Loading sot.json: {sot_path}", file=sys.stderr)
    sot = load_sot(sot_path)
    print(f"  {len(sot['nodes'])} nodes, {len(sot['edges'])} edges", file=sys.stderr)

    print(f"\nPicking {args.count} symbols per kind (seed={args.seed}):", file=sys.stderr)
    picked = pick_symbols(sot, args.count, args.seed)

    cases = generate_cases(picked)

    # Summary
    total_symbols = sum(len(v) for v in picked.values())
    print(f"\nGenerated {len(cases)} cases from {total_symbols} symbols", file=sys.stderr)
    print(f"  = {total_symbols} symbols × {len(DEPTHS)} depths × {len(IMPL_OPTIONS)} impl options", file=sys.stderr)

    # Read sot_id from main cases.json if possible
    script_dir = Path(__file__).parent
    main_cases = script_dir.parent / "cases.json"
    sot_id = "context-final"  # default
    if main_cases.exists():
        with open(main_cases) as f:
            sot_id = json.load(f).get("sot_id", sot_id)

    output_data = {
        "sot_id": sot_id,
        "seed": args.seed,
        "count_per_kind": args.count,
        "depths": DEPTHS,
        "summary": {group: len(nodes) for group, nodes in picked.items()},
        "cases": cases,
    }

    output_path = args.output or str(script_dir / "cases.json")
    with open(output_path, "w") as f:
        json.dump(output_data, f, indent=2)

    print(f"\nWrote: {output_path}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
