#!/usr/bin/env bash
set -euo pipefail

# Setup script for kloc-full meta repository
# Reads repos.yml and clones/updates all repositories

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "Setting up KLOC repositories..."

# Parse repos.yml and clone/update each repo
# Simple parsing without external dependencies
while IFS= read -r line; do
    if [[ "$line" =~ ^[[:space:]]*-[[:space:]]*name:[[:space:]]*(.+)$ ]]; then
        name="${BASH_REMATCH[1]}"
    elif [[ "$line" =~ ^[[:space:]]*url:[[:space:]]*(.+)$ ]]; then
        url="${BASH_REMATCH[1]}"

        if [ -d "$name" ]; then
            echo "Updating $name..."
            (cd "$name" && git pull --ff-only) || echo "Warning: Failed to update $name"
        else
            echo "Cloning $name..."
            git clone "$url" "$name"
        fi
    fi
done < repos.yml

echo ""
echo "Setup complete!"
echo "Run ./build.sh to build all binaries"
