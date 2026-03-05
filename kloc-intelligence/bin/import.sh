#!/bin/bash
set -euo pipefail

if [ -z "${1:-}" ]; then
    echo "Usage: bin/import.sh <path-to-sot.json>"
    exit 1
fi

SOT_PATH="$1"

if [ ! -f "$SOT_PATH" ]; then
    echo "ERROR: File not found: $SOT_PATH"
    exit 1
fi

echo "Importing $SOT_PATH..."
kloc-intelligence import "$SOT_PATH"
echo "Import complete."
