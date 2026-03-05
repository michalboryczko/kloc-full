"""Generate golden outputs from kloc-cli for snapshot testing.

Usage:
    cd kloc-intelligence
    uv run python tests/snapshots/generate_golden.py

Requires:
    - kloc-cli installed and working (from ../kloc-cli/)
    - data/uestate/sot.json available
"""

from __future__ import annotations

import json
import subprocess
from pathlib import Path

import yaml

CORPUS_PATH = Path(__file__).parent / "corpus.yaml"
GOLDEN_DIR = Path(__file__).parent / "golden"
# kloc-cli working directory (relative to project root)
KLOC_CLI_DIR = Path(__file__).parent.parent.parent.parent / "kloc-cli"


def generate_golden():
    """Run all corpus queries against kloc-cli and save golden outputs."""
    with open(CORPUS_PATH) as f:
        corpus = yaml.safe_load(f)

    dataset = corpus["dataset"]
    # Resolve dataset path relative to repo root
    repo_root = Path(__file__).parent.parent.parent.parent
    dataset_path = str(repo_root / dataset)

    GOLDEN_DIR.mkdir(parents=True, exist_ok=True)

    total = len(corpus["queries"])
    passed = 0
    failed = 0
    errors = 0

    for i, query in enumerate(corpus["queries"], 1):
        query_id = query["id"]
        command = query["command"]
        args = query.get("args", [])
        options = query.get("options", {})

        # Build CLI command
        cmd = ["uv", "run", "kloc-cli", command, *args, "--sot", dataset_path]

        # Add options
        if options.get("json", False):
            cmd.append("--json")
        if "depth" in options:
            cmd.extend(["--depth", str(options["depth"])])
        if "direction" in options:
            cmd.extend(["--direction", options["direction"]])
        if options.get("impl", False):
            cmd.append("--impl")
        if options.get("direct", False):
            cmd.append("--direct")

        print(f"[{i}/{total}] {query_id}: {' '.join(cmd)}")

        try:
            result = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                timeout=60,
                cwd=str(KLOC_CLI_DIR),
            )

            golden = {
                "query_id": query_id,
                "command": command,
                "args": args,
                "options": options,
                "exit_code": result.returncode,
                "stdout": result.stdout,
                "stderr": result.stderr,
            }

            # Try to parse stdout as JSON
            try:
                golden["json_output"] = json.loads(result.stdout)
            except json.JSONDecodeError:
                golden["json_output"] = None

            output_path = GOLDEN_DIR / f"{query_id}.json"
            with open(output_path, "w") as f:
                json.dump(golden, f, indent=2, ensure_ascii=False)

            if result.returncode == 0:
                passed += 1
                print("  OK")
            else:
                failed += 1
                stderr_preview = result.stderr[:200] if result.stderr else ""
                print(f"  EXIT {result.returncode}: {stderr_preview}")

        except subprocess.TimeoutExpired:
            errors += 1
            print("  TIMEOUT (60s)")
        except Exception as e:
            errors += 1
            print(f"  ERROR: {e}")

    print("\n=== Golden Generation Complete ===")
    print(f"Total: {total}, OK: {passed}, Failed: {failed}, Errors: {errors}")


if __name__ == "__main__":
    generate_golden()
