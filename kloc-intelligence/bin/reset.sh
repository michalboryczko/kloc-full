#!/bin/bash
set -euo pipefail

echo "=== Resetting kloc-intelligence database ==="
echo "WARNING: This will delete ALL data in Neo4j."
read -p "Continue? [y/N] " -n 1 -r
echo

if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Aborted."
    exit 0
fi

kloc-intelligence schema reset
kloc-intelligence schema ensure

echo "=== Database reset complete ==="
