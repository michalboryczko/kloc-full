#!/bin/bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

echo "=== kloc-intelligence setup ==="

# 1. Start Neo4j
echo "Starting Neo4j..."
cd "$PROJECT_DIR"
docker compose up -d

# 2. Wait for Neo4j to be healthy
echo "Waiting for Neo4j..."
for i in $(seq 1 30); do
    if docker compose exec neo4j neo4j status 2>/dev/null | grep -q "running"; then
        echo "Neo4j is running."
        break
    fi
    if [ "$i" -eq 30 ]; then
        echo "ERROR: Neo4j did not start within 60 seconds."
        exit 1
    fi
    sleep 2
done

# 3. Install Python dependencies
echo "Installing dependencies..."
cd "$PROJECT_DIR"
uv pip install -e ".[dev]"

# 4. Apply schema
echo "Applying schema..."
kloc-intelligence schema ensure

echo "=== Setup complete ==="
echo "Neo4j Browser: http://localhost:7474"
echo "Bolt URI: bolt://localhost:7687"
