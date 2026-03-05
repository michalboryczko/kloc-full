#!/bin/bash
set -euo pipefail

echo "=== kloc-intelligence status ==="

# Check Docker
if docker compose ps --format json 2>/dev/null | grep -q "neo4j"; then
    echo "Neo4j container: RUNNING"
else
    echo "Neo4j container: NOT RUNNING"
    exit 1
fi

# Check connectivity
kloc-intelligence schema verify
