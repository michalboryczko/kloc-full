#!/usr/bin/env bash
set -euo pipefail

# Setup script for kloc-full meta repository
# Clones or updates all related repositories

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "Setting up KLOC repositories..."

clone_or_pull() {
    local name="$1"
    local url="$2"

    if [ -d "$name" ]; then
        echo "Updating $name..."
        (cd "$name" && git pull --ff-only)
    else
        echo "Cloning $name..."
        git clone "$url" "$name"
    fi
}

# Clone/update repositories
clone_or_pull "kloc-cli" "git@github.com:michalboryczko/kloc-cli.git"
clone_or_pull "kloc-mapper" "git@github.com:michalboryczko/kloc-mapper.git"
# clone_or_pull "scip-php" "git@github.com:michalboryczko/scip-php.git"

echo ""
echo "Setup complete!"
echo "Repositories are ready in:"
echo "  - kloc-cli/"
echo "  - kloc-mapper/"
# echo "  - scip-php/"
