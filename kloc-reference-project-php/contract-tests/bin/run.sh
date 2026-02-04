#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR/.."

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

show_help() {
    echo -e "${CYAN}Contract Tests Runner${NC}"
    echo ""
    echo "Usage: bin/run.sh <command> [options]"
    echo ""
    echo "Commands:"
    echo "  test                    Run all tests (generates fresh index)"
    echo "  test --filter <name>    Run specific test by name/pattern"
    echo "  test --suite <name>     Run specific test suite"
    echo "  test --experimental     Include experimental tests and index experimental kinds"
    echo "  docs                    Generate markdown documentation"
    echo "  docs --format=json      Generate JSON documentation"
    echo "  docs --format=csv       Generate CSV documentation"
    echo "  docs --output=FILE      Write documentation to file"
    echo ""
    echo "Test suites: smoke, integrity, reference, chain, argument"
    echo ""
    echo "Examples:"
    echo "  bin/run.sh test"
    echo "  bin/run.sh test --experimental"
    echo "  bin/run.sh test --filter testOrderRepository"
    echo "  bin/run.sh test --suite smoke"
    echo "  bin/run.sh docs"
    echo "  bin/run.sh docs --format=json --output=tests.json"
}

generate_index() {
    local experimental="${1:-}"

    # Path to scip-php.sh wrapper
    SCIP_PHP="${SCIP_PHP_BINARY:-../../scip-php/bin/scip-php.sh}"

    if [[ ! -x "$SCIP_PHP" ]]; then
        echo -e "${RED}Error: scip-php.sh not found at: $SCIP_PHP${NC}"
        echo "Make sure scip-php Docker image is built: cd ../../scip-php && ./build/build.sh"
        exit 1
    fi

    echo -e "${YELLOW}Generating index with scip-php...${NC}"
    mkdir -p output

    # Build scip-php command
    local scip_cmd="$SCIP_PHP -d $(cd .. && pwd) -o $(pwd)/output"
    if [[ "$experimental" == "1" ]]; then
        scip_cmd="$scip_cmd --experimental"
        echo -e "${CYAN}  (with --experimental flag)${NC}"
    fi

    # Run scip-php on the parent project (kloc-reference-project-php)
    eval "$scip_cmd"

    # Check if calls.json was generated
    if [[ -f "output/calls.json" ]]; then
        echo -e "${GREEN}  calls.json generated${NC}"
    else
        echo -e "${RED}Error: scip-php did not generate calls.json${NC}"
        exit 1
    fi

}

build_docker() {
    echo -e "${YELLOW}Building Docker image...${NC}"
    docker compose build --quiet
    echo -e "${GREEN}  Docker image ready${NC}"
}

convert_scip_to_json() {
    # Check if index.scip was generated and convert to JSON
    if [[ -f "output/index.scip" ]]; then
        echo -e "${YELLOW}Converting SCIP to JSON...${NC}"
        docker compose run --rm contract-tests scip print --json /app/contract-tests/output/index.scip > output/index.scip.json 2>/dev/null
        if [[ -f "output/index.scip.json" && -s "output/index.scip.json" ]]; then
            echo -e "${GREEN}  index.scip.json generated${NC}"
        else
            echo -e "${YELLOW}  Warning: index.scip.json generation failed (non-fatal)${NC}"
        fi
    fi
}

run_tests() {
    local filter="$1"
    local suite="$2"
    local experimental="$3"

    echo -e "${GREEN}=== Contract Tests ===${NC}"
    if [[ "$experimental" == "1" ]]; then
        echo -e "${CYAN}(experimental mode)${NC}"
    fi
    echo ""

    generate_index "$experimental"
    build_docker
    convert_scip_to_json

    echo -e "${YELLOW}Running tests...${NC}"
    echo ""

    local cmd="vendor/bin/phpunit --display-skipped"
    if [[ -n "$filter" ]]; then
        cmd="$cmd --filter $filter"
    fi
    if [[ -n "$suite" ]]; then
        cmd="$cmd --testsuite=$suite"
    fi

    # Set experimental env var for PHPUnit
    local env_args=""
    if [[ "$experimental" == "1" ]]; then
        env_args="-e CONTRACT_TESTS_EXPERIMENTAL=1"
    fi

    docker compose run --rm $env_args contract-tests $cmd

    echo ""
    echo -e "${GREEN}=== Done ===${NC}"
}

run_docs() {
    local format="$1"
    local output="$2"

    echo -e "${GREEN}=== Generate Documentation ===${NC}"
    echo ""

    generate_index
    build_docker
    convert_scip_to_json

    echo -e "${YELLOW}Generating documentation...${NC}"
    echo ""

    local cmd="php bin/generate-docs.php"
    if [[ -n "$format" ]]; then
        cmd="$cmd --format=$format"
    fi
    if [[ -n "$output" ]]; then
        cmd="$cmd --output=$output"
    fi

    docker compose run --rm contract-tests $cmd

    echo ""
    echo -e "${GREEN}=== Done ===${NC}"
}

# Parse command
COMMAND="${1:-}"

case "$COMMAND" in
    test)
        shift
        FILTER=""
        SUITE=""
        EXPERIMENTAL=""
        while [[ $# -gt 0 ]]; do
            case "$1" in
                --filter)
                    FILTER="$2"
                    shift 2
                    ;;
                --suite)
                    SUITE="$2"
                    shift 2
                    ;;
                --experimental)
                    EXPERIMENTAL="1"
                    shift
                    ;;
                *)
                    echo -e "${RED}Unknown option: $1${NC}"
                    show_help
                    exit 1
                    ;;
            esac
        done
        run_tests "$FILTER" "$SUITE" "$EXPERIMENTAL"
        ;;
    docs)
        shift
        FORMAT=""
        OUTPUT=""
        while [[ $# -gt 0 ]]; do
            case "$1" in
                --format=*)
                    FORMAT="${1#*=}"
                    shift
                    ;;
                --output=*)
                    OUTPUT="${1#*=}"
                    shift
                    ;;
                *)
                    echo -e "${RED}Unknown option: $1${NC}"
                    show_help
                    exit 1
                    ;;
            esac
        done
        run_docs "$FORMAT" "$OUTPUT"
        ;;
    help|--help|-h)
        show_help
        ;;
    *)
        if [[ -n "$COMMAND" ]]; then
            echo -e "${RED}Unknown command: $COMMAND${NC}"
            echo ""
        fi
        show_help
        exit 1
        ;;
esac
