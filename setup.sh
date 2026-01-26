#!/usr/bin/env bash
set -euo pipefail

# Setup script for kloc-full meta repository
# Reads repos.yml and clones/updates repositories
#
# Usage:
#   ./setup.sh              # Setup all repos
#   ./setup.sh kloc-cli     # Setup only kloc-cli
#   ./setup.sh scip-php     # Setup only scip-php

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

TARGET="${1:-}"  # Empty means all

echo "Setting up KLOC repositories..."
[ -n "$TARGET" ] && echo "Target: $TARGET"
echo ""

# Parse repos.yml and clone/update each repo
while IFS= read -r line; do
    if [[ "$line" =~ ^[[:space:]]*-[[:space:]]*name:[[:space:]]*(.+)$ ]]; then
        name="${BASH_REMATCH[1]}"
    elif [[ "$line" =~ ^[[:space:]]*url:[[:space:]]*(.+)$ ]]; then
        url="${BASH_REMATCH[1]}"

        # Skip if target specified and doesn't match
        if [ -n "$TARGET" ] && [ "$TARGET" != "$name" ]; then
            continue
        fi

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
echo "Run ./build.sh to build binaries"
