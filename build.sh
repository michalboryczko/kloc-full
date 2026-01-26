#!/usr/bin/env bash
set -euo pipefail

# Build all KLOC binaries and copy to bin/
# Run setup.sh first to ensure repos are cloned

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

BIN_DIR="$SCRIPT_DIR/bin"
mkdir -p "$BIN_DIR"

echo "Building all KLOC binaries..."
echo ""

# Build kloc-cli
if [ -d "kloc-cli" ]; then
    echo "=== Building kloc-cli ==="
    (cd kloc-cli && ./build.sh)
    cp kloc-cli/dist/kloc-cli "$BIN_DIR/"
    echo "Copied kloc-cli to bin/"
    echo ""
else
    echo "Warning: kloc-cli/ not found. Run ./setup.sh first."
fi

# Build kloc-mapper
if [ -d "kloc-mapper" ]; then
    echo "=== Building kloc-mapper ==="
    (cd kloc-mapper && ./build.sh)
    cp kloc-mapper/dist/kloc-mapper "$BIN_DIR/"
    echo "Copied kloc-mapper to bin/"
    echo ""
else
    echo "Warning: kloc-mapper/ not found. Run ./setup.sh first."
fi

# Build scip-php (when available)
# if [ -d "scip-php" ]; then
#     echo "=== Building scip-php ==="
#     (cd scip-php && ./build.sh)
#     cp scip-php/dist/scip-php "$BIN_DIR/"
#     echo "Copied scip-php to bin/"
#     echo ""
# fi

echo "=== Build complete ==="
echo ""
echo "Binaries available in $BIN_DIR:"
ls -la "$BIN_DIR"
