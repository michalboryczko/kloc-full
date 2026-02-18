#!/usr/bin/env bash
set -euo pipefail

# Build KLOC binaries using repos.yml config
# Run setup.sh first to ensure repos are cloned
#
# Usage:
#   ./build.sh              # Build all repos
#   ./build.sh kloc-cli     # Build only kloc-cli
#   ./build.sh --force      # Clean build dirs first, then build all
#   ./build.sh --force kloc-cli  # Clean + build specific repo

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Parse flags
FORCE=false
TARGET=""
for arg in "$@"; do
    case "$arg" in
        --force) FORCE=true ;;
        *) TARGET="$arg" ;;
    esac
done

BIN_DIR="$SCRIPT_DIR/bin"

# Force clean: remove build artifacts
if [ "$FORCE" = true ]; then
    echo "Force mode: cleaning build directories..."
    rm -rf "$BIN_DIR"
    # Clean each repo's build artifacts
    for dir in kloc-cli kloc-mapper scip-php; do
        if [ -d "$dir" ]; then
            rm -rf "$dir/dist" "$dir/build" "$dir/__pycache__" "$dir/.pyinstaller"
            echo "  Cleaned $dir/dist, $dir/build"
        fi
    done
    echo ""
fi

mkdir -p "$BIN_DIR"

echo "Building KLOC binaries..."
[ -n "$TARGET" ] && echo "Target: $TARGET"
echo ""

# Parse repos.yml and build each repo
name=""
build_script=""
binary_path=""

while IFS= read -r line; do
    if [[ "$line" =~ ^[[:space:]]*-[[:space:]]*name:[[:space:]]*(.+)$ ]]; then
        name="${BASH_REMATCH[1]}"
        build_script=""
        binary_path=""
    elif [[ "$line" =~ ^[[:space:]]*build:[[:space:]]*(.+)$ ]]; then
        build_script="${BASH_REMATCH[1]}"
    elif [[ "$line" =~ ^[[:space:]]*binary:[[:space:]]*(.+)$ ]]; then
        binary_path="${BASH_REMATCH[1]}"

        # Skip if target specified and doesn't match
        if [ -n "$TARGET" ] && [ "$TARGET" != "$name" ]; then
            continue
        fi

        # We have all info, build this repo
        if [ -n "$name" ] && [ -n "$build_script" ] && [ -n "$binary_path" ]; then
            if [ -d "$name" ]; then
                echo "=== Building $name ==="
                (cd "$name" && $build_script)

                if [ -f "$name/$binary_path" ]; then
                    cp "$name/$binary_path" "$BIN_DIR/"
                    echo "Copied $(basename "$binary_path") to bin/"
                else
                    echo "Warning: Binary not found at $name/$binary_path"
                fi
                echo ""
            else
                echo "Warning: $name/ not found. Run ./setup.sh first."
                echo ""
            fi
        fi
    fi
done < repos.yml

echo "=== Build complete ==="
echo ""
echo "Binaries in $BIN_DIR:"
ls -la "$BIN_DIR" 2>/dev/null || echo "(empty)"
