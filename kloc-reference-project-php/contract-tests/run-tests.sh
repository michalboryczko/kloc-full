#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}=== Contract Tests Runner ===${NC}"

# Check for scip-php binary
SCIP_PHP="${SCIP_PHP_BINARY:-../../scip-php/build/scip-php}"
if [[ ! -x "$SCIP_PHP" ]]; then
    echo -e "${RED}Error: scip-php binary not found at: $SCIP_PHP${NC}"
    echo "Set SCIP_PHP_BINARY environment variable or build scip-php first."
    exit 1
fi

# Step 1: Generate index
echo -e "${YELLOW}Step 1: Generating index with scip-php...${NC}"
mkdir -p output

# Run scip-php on the parent project
"$SCIP_PHP" -d ..

# Move calls.json to output directory
if [[ -f "calls.json" ]]; then
    mv calls.json output/
    echo -e "${GREEN}  ✓ calls.json generated${NC}"
else
    echo -e "${RED}Error: scip-php did not generate calls.json${NC}"
    exit 1
fi

# Clean up other generated files
rm -f index.scip index.kloc

# Step 2: Build Docker image (if needed)
echo -e "${YELLOW}Step 2: Building Docker image...${NC}"
docker compose build --quiet
echo -e "${GREEN}  ✓ Docker image ready${NC}"

# Step 3: Run tests
echo -e "${YELLOW}Step 3: Running contract tests...${NC}"
echo ""

docker compose run --rm -e SKIP_INDEX_GENERATION=1 contract-tests

echo ""
echo -e "${GREEN}=== Done ===${NC}"
