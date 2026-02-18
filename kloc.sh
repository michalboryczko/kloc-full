#!/bin/bash
set -euo pipefail

# kloc: Full pipeline for any PHP project
# Index a PHP project -> map to sot.json -> query with kloc-cli
#
# Pipeline mode (create index + sot):
#   ./kloc.sh index --project myapp -d /path/to/php-project
#   ./kloc.sh index --project myapp -d /path/to/php-project --internal-all
#
# CLI mode (query existing project data):
#   ./kloc.sh cli --project myapp context "App\Service\OrderService" --depth 2
#   ./kloc.sh cli --project myapp resolve "App\Entity\Order"
#   ./kloc.sh cli --project myapp usages "App\Entity\Order" --depth 1
#   ./kloc.sh cli --project myapp mcp-server
#
# Data is stored in: data/{project_name}/
#   data/{project_name}/index.json  - SCIP index
#   data/{project_name}/sot.json    - Source of Truth graph

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BIN_DIR="$SCRIPT_DIR/bin"
DATA_BASE="$SCRIPT_DIR/data"

# Verify binaries exist
check_binary() {
    if [[ ! -x "$BIN_DIR/$1" ]]; then
        echo "Error: $1 binary not found at $BIN_DIR/$1"
        echo "Run ./build.sh first to build all binaries."
        exit 1
    fi
}

show_help() {
    echo "kloc - PHP code intelligence pipeline"
    echo ""
    echo "Usage:"
    echo "  kloc.sh index --project <name> -d <project-dir> [--internal-all] [--experimental]"
    echo "  kloc.sh cli   --project <name> <command> [args...]"
    echo ""
    echo "Commands:"
    echo "  index    Index a PHP project (scip-php + kloc-mapper)"
    echo "  cli      Run kloc-cli queries against indexed project"
    echo ""
    echo "Options:"
    echo "  --project <name>    Project name (used for data directory)"
    echo "  -d <path>           PHP project directory to index (index mode)"
    echo "  --internal-all      Treat vendor packages as internal (index mode)"
    echo "  --experimental      Include experimental call kinds (index mode)"
    echo "  -h, --help          Show this help"
    echo ""
    echo "Data stored in: data/<project_name>/"
    echo ""
    echo "Examples:"
    echo "  # Index a project"
    echo "  ./kloc.sh index --project myapp -d ~/code/myapp"
    echo ""
    echo "  # Query the indexed project"
    echo "  ./kloc.sh cli --project myapp context \"App\\Service\\OrderService\" --depth 2"
    echo "  ./kloc.sh cli --project myapp resolve \"App\\Entity\\Order\""
    echo "  ./kloc.sh cli --project myapp mcp-server"
}

# Need at least one argument
if [[ $# -eq 0 ]]; then
    show_help
    exit 0
fi

# Extract the command (first arg)
COMMAND="$1"
shift

if [[ "$COMMAND" == "-h" || "$COMMAND" == "--help" ]]; then
    show_help
    exit 0
fi

# Parse common flags
PROJECT=""
PROJECT_DIR=""
INTERNAL_ALL=""
EXPERIMENTAL=""
CLI_ARGS=()
PARSING_PROJECT=false
PARSING_DIR=false

for arg in "$@"; do
    if [[ "$PARSING_PROJECT" == true ]]; then
        PROJECT="$arg"
        PARSING_PROJECT=false
        continue
    fi
    if [[ "$PARSING_DIR" == true ]]; then
        PROJECT_DIR="$arg"
        PARSING_DIR=false
        continue
    fi
    case "$arg" in
        --project)
            PARSING_PROJECT=true
            ;;
        --project=*)
            PROJECT="${arg#--project=}"
            ;;
        -d)
            if [[ "$COMMAND" == "index" ]]; then
                PARSING_DIR=true
            else
                CLI_ARGS+=("$arg")
            fi
            ;;
        --internal-all)
            if [[ "$COMMAND" == "index" ]]; then
                INTERNAL_ALL="--internal-all"
            else
                CLI_ARGS+=("$arg")
            fi
            ;;
        --experimental)
            if [[ "$COMMAND" == "index" ]]; then
                EXPERIMENTAL="--experimental"
            else
                CLI_ARGS+=("$arg")
            fi
            ;;
        *)
            CLI_ARGS+=("$arg")
            ;;
    esac
done

# Validate project name
if [[ -z "$PROJECT" ]]; then
    echo "Error: --project <name> is required"
    echo ""
    show_help
    exit 1
fi

# Project data directory
DATA_DIR="$DATA_BASE/$PROJECT"
INDEX_FILE="$DATA_DIR/index.json"
SOT_FILE="$DATA_DIR/sot.json"

case "$COMMAND" in
    index)
        # --- Index mode: scip-php + kloc-mapper ---
        if [[ -z "$PROJECT_DIR" ]]; then
            echo "Error: -d <project-dir> is required for index command"
            exit 1
        fi

        mkdir -p "$DATA_DIR"

        echo "== kloc index =="
        echo "Project:   $PROJECT"
        echo "Source:    $PROJECT_DIR"
        echo "Data:      $DATA_DIR"
        echo ""

        # Step 1: Generate SCIP index
        echo "[1/2] Generating SCIP index..."
        "$SCRIPT_DIR/scip-php/bin/scip-php.sh" \
            -d "$PROJECT_DIR" \
            -o "$DATA_DIR" \
            $INTERNAL_ALL \
            $EXPERIMENTAL
        echo ""

        if [[ ! -f "$INDEX_FILE" ]]; then
            echo "Error: index.json not found at $INDEX_FILE after indexing"
            exit 1
        fi

        # Step 2: Map to sot.json
        echo "[2/2] Mapping index to sot.json..."
        check_binary kloc-mapper
        "$BIN_DIR/kloc-mapper" map "$INDEX_FILE" -o "$SOT_FILE"
        echo ""

        if [[ ! -f "$SOT_FILE" ]]; then
            echo "Error: sot.json not found at $SOT_FILE after mapping"
            exit 1
        fi

        echo "Done! Project '$PROJECT' indexed."
        echo "  Index: $INDEX_FILE"
        echo "  SoT:   $SOT_FILE"
        echo ""
        echo "Query with: ./kloc.sh cli --project $PROJECT context \"App\\YourClass\""
        ;;

    cli)
        # --- CLI mode: run kloc-cli with project's sot.json ---
        if [[ ! -f "$SOT_FILE" ]]; then
            echo "Error: No data found for project '$PROJECT'"
            echo "Expected: $SOT_FILE"
            echo ""
            echo "Run index first: ./kloc.sh index --project $PROJECT -d /path/to/project"
            exit 1
        fi

        check_binary kloc-cli
        exec "$BIN_DIR/kloc-cli" "${CLI_ARGS[@]}" --sot "$SOT_FILE"
        ;;

    *)
        echo "Unknown command: $COMMAND"
        echo "Use 'index' or 'cli'. Run with --help for usage."
        exit 1
        ;;
esac
