#!/usr/bin/env python3
"""Validate JSON data files against kloc-contracts schemas.

Usage:
    python validate.py sot-json <path-to-sot.json>
    python validate.py scip-php-output <path-to-output.json>
"""

import json
import sys
from pathlib import Path

try:
    from jsonschema import validate, ValidationError, Draft202012Validator
except ImportError:
    print("Error: jsonschema package is required. Install with: pip install jsonschema", file=sys.stderr)
    sys.exit(1)

SCHEMAS_DIR = Path(__file__).parent

SCHEMA_MAP = {
    "sot-json": "sot-json.json",
    "scip-php-output": "scip-php-output.json",
}


def load_schema(schema_name: str) -> dict:
    schema_file = SCHEMAS_DIR / SCHEMA_MAP[schema_name]
    with open(schema_file) as f:
        return json.load(f)


def validate_file(schema_name: str, data_path: str) -> bool:
    schema = load_schema(schema_name)
    with open(data_path) as f:
        data = json.load(f)

    validator = Draft202012Validator(schema)
    errors = list(validator.iter_errors(data))

    if not errors:
        print(f"OK: {data_path} is valid against {SCHEMA_MAP[schema_name]}")
        return True

    print(f"FAIL: {data_path} has {len(errors)} validation error(s):", file=sys.stderr)
    for i, error in enumerate(errors, 1):
        path = " -> ".join(str(p) for p in error.absolute_path) if error.absolute_path else "(root)"
        print(f"  {i}. [{path}] {error.message}", file=sys.stderr)
    return False


def main() -> int:
    if len(sys.argv) != 3:
        print(__doc__.strip(), file=sys.stderr)
        print(f"\nAvailable schemas: {', '.join(sorted(SCHEMA_MAP.keys()))}", file=sys.stderr)
        return 1

    schema_name = sys.argv[1]
    data_path = sys.argv[2]

    if schema_name not in SCHEMA_MAP:
        print(f"Error: Unknown schema '{schema_name}'. Available: {', '.join(sorted(SCHEMA_MAP.keys()))}", file=sys.stderr)
        return 1

    if not Path(data_path).exists():
        print(f"Error: File not found: {data_path}", file=sys.stderr)
        return 1

    return 0 if validate_file(schema_name, data_path) else 1


if __name__ == "__main__":
    sys.exit(main())
