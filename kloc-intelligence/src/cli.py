"""kloc-intelligence CLI entry point.

Typer-based CLI with subcommands for schema management, import,
and all 8 query commands.
"""

from __future__ import annotations

import time
from pathlib import Path

import typer
from rich.console import Console
from rich.progress import Progress, SpinnerColumn, TimeElapsedColumn

from .config import Neo4jConfig
from .db.connection import Neo4jConnection, Neo4jConnectionError
from .db.schema import ensure_schema, verify_schema, drop_all, get_node_count, get_edge_count

app = typer.Typer(
    name="kloc-intelligence",
    help="Graph-native code intelligence platform backed by Neo4j.",
    no_args_is_help=True,
)

schema_app = typer.Typer(help="Manage Neo4j schema (constraints and indexes).")
app.add_typer(schema_app, name="schema")

console = Console()


def _get_connection() -> Neo4jConnection:
    """Create a Neo4j connection from environment variables."""
    config = Neo4jConfig.from_env()
    return Neo4jConnection(config)


@schema_app.command("ensure")
def schema_ensure() -> None:
    """Create all constraints and indexes (idempotent)."""
    try:
        with _get_connection() as conn:
            conn.verify_connectivity()
            result = ensure_schema(conn)
            console.print(
                f"[green]Schema applied:[/green] "
                f"{result['constraints']} constraints, "
                f"{result['indexes']} indexes"
            )
    except Neo4jConnectionError as e:
        console.print(f"[red]Error:[/red] {e}")
        raise typer.Exit(code=1)


@schema_app.command("verify")
def schema_verify() -> None:
    """Verify all expected constraints and indexes exist."""
    try:
        with _get_connection() as conn:
            conn.verify_connectivity()
            result = verify_schema(conn)
            console.print(
                f"[green]Schema verified:[/green] "
                f"{result['constraints']} constraints, "
                f"{result['indexes']} indexes"
            )
            node_count = get_node_count(conn)
            edge_count = get_edge_count(conn)
            console.print(f"Database: {node_count} nodes, {edge_count} edges")
    except Neo4jConnectionError as e:
        console.print(f"[red]Error:[/red] {e}")
        raise typer.Exit(code=1)


@schema_app.command("reset")
def schema_reset() -> None:
    """Drop all data, constraints, and indexes.

    WARNING: Destroys all data in the database.
    """
    try:
        with _get_connection() as conn:
            conn.verify_connectivity()
            drop_all(conn)
            console.print("[yellow]Database reset:[/yellow] All data and schema removed.")
    except Neo4jConnectionError as e:
        console.print(f"[red]Error:[/red] {e}")
        raise typer.Exit(code=1)


@app.command("import")
def import_sot(
    sot_path: str = typer.Argument(..., help="Path to sot.json file"),
    clear: bool = typer.Option(True, help="Clear database before import (default: True)"),
    validate: bool = typer.Option(True, help="Run validation after import (default: True)"),
    batch_size: int = typer.Option(5000, help="Batch size for UNWIND operations"),
) -> None:
    """Import a sot.json file into Neo4j."""
    from .db.importer import parse_sot, import_nodes, import_edges, validate_import

    sot_file = Path(sot_path)
    if not sot_file.exists():
        console.print(f"[red]Error:[/red] File not found: {sot_path}")
        raise typer.Exit(code=1)

    try:
        conn = _get_connection()
        conn.verify_connectivity()
    except Neo4jConnectionError as e:
        console.print(f"[red]Error:[/red] {e}")
        raise typer.Exit(code=1)

    start = time.perf_counter()

    # 1. Parse
    console.print(f"Parsing {sot_path}...")
    nodes, edges = parse_sot(sot_path)
    parse_time = time.perf_counter() - start
    console.print(f"  Parsed {len(nodes):,} nodes, {len(edges):,} edges ({parse_time:.1f}s)")

    # 2. Clear + schema
    if clear:
        console.print("Clearing database...")
        drop_all(conn)
        ensure_schema(conn)

    # 3. Import nodes
    console.print("Importing nodes...")
    node_start = time.perf_counter()
    with Progress(
        SpinnerColumn(), *Progress.get_default_columns(), TimeElapsedColumn()
    ) as progress:
        task = progress.add_task("Nodes", total=len(nodes))

        def node_progress(count):
            progress.advance(task, count)

        import_nodes(
            conn, nodes, batch_size=batch_size, progress_callback=node_progress
        )
    node_time = time.perf_counter() - node_start

    # 4. Import edges
    console.print("Importing edges...")
    edge_start = time.perf_counter()
    with Progress(
        SpinnerColumn(), *Progress.get_default_columns(), TimeElapsedColumn()
    ) as progress:
        task = progress.add_task("Edges", total=len(edges))

        def edge_progress(count):
            progress.advance(task, count)

        import_edges(
            conn, edges, batch_size=batch_size, progress_callback=edge_progress
        )
    edge_time = time.perf_counter() - edge_start

    # 5. Validate
    if validate:
        console.print("Validating...")
        try:
            report = validate_import(conn, expected_nodes=len(nodes), expected_edges=len(edges))
            status_nodes = "[green]OK[/green]" if report["node_match"] else "[red]MISMATCH[/red]"
            status_edges = "[green]OK[/green]" if report["edge_match"] else "[red]MISMATCH[/red]"
            console.print(
                f"  Nodes: {report['node_count']:,} / {report['expected_nodes']:,}  {status_nodes}"
            )
            console.print(
                f"  Edges: {report['edge_count']:,} / {report['expected_edges']:,}  {status_edges}"
            )
        except Exception as e:
            console.print(f"[red]Validation failed:[/red] {e}")
            conn.close()
            raise typer.Exit(code=1)

    total_time = time.perf_counter() - start
    console.print(
        f"\nImport complete in {total_time:.1f}s "
        f"(parse: {parse_time:.1f}s, nodes: {node_time:.1f}s, edges: {edge_time:.1f}s)"
    )

    conn.close()


# =============================================================================
# Query Commands
# =============================================================================


@app.command()
def resolve(
    query: str = typer.Argument(..., help="Symbol to resolve (FQN, partial, or short name)"),
    json_output: bool = typer.Option(False, "--json", "-j", help="Output as JSON"),
) -> None:
    """Resolve a symbol to its definition location."""
    import json as json_mod

    from .db.query_runner import QueryRunner
    from .db.queries.resolve import resolve_symbol

    try:
        conn = _get_connection()
        conn.verify_connectivity()
    except Neo4jConnectionError as e:
        console.print(f"[red]Error:[/red] {e}")
        raise typer.Exit(code=1)

    runner = QueryRunner(conn)
    candidates = resolve_symbol(runner, query)

    if not candidates:
        if json_output:
            print(json_mod.dumps({"error": "Symbol not found", "query": query}, indent=2, ensure_ascii=False))
        else:
            console.print(f"[red]Symbol not found: {query}[/red]")
        conn.close()
        raise typer.Exit(code=1)

    if len(candidates) == 1:
        # Single match: print_node format
        node = candidates[0]
        if json_output:
            print(json_mod.dumps({
                "id": node.id,
                "kind": node.kind,
                "name": node.name,
                "fqn": node.fqn,
                "file": node.file,
                "line": node.start_line + 1 if node.start_line is not None else None,
            }, indent=2, ensure_ascii=False))
        else:
            console.print(f"[bold]{node.kind}[/bold]: {node.fqn}")
            console.print(f"  File: {node.location_str}")
            if node.documentation:
                doc = node.documentation[0][:100]
                console.print(f"  Doc: {doc}...")
    else:
        # Multiple matches: print_candidates format
        if json_output:
            print(json_mod.dumps([
                {
                    "id": n.id,
                    "kind": n.kind,
                    "fqn": n.fqn,
                    "file": n.file,
                    "line": n.start_line + 1 if n.start_line is not None else None,
                }
                for n in candidates
            ], indent=2, ensure_ascii=False))
        else:
            console.print(f"[yellow]Found {len(candidates)} candidates:[/yellow]")
            for i, node in enumerate(candidates, 1):
                console.print(f"  [{i}] {node.kind}: {node.fqn}")
                console.print(f"      {node.location_str}")

    conn.close()


@app.command()
def usages(
    query: str = typer.Argument(..., help="Symbol to find usages for"),
    depth: int = typer.Option(1, "--depth", "-d", help="BFS depth for expansion"),
    limit: int = typer.Option(100, "--limit", "-l", help="Maximum results"),
    json_output: bool = typer.Option(False, "--json", "-j", help="Output as JSON"),
) -> None:
    """Find all usages of a symbol with depth expansion."""
    import json as json_mod

    from .db.query_runner import QueryRunner
    from .db.queries.resolve import resolve_symbol
    from .db.queries.usages import usages_tree
    from .output.json_formatter import usages_tree_to_dict

    try:
        conn = _get_connection()
        conn.verify_connectivity()
    except Neo4jConnectionError as e:
        console.print(f"[red]Error:[/red] {e}")
        raise typer.Exit(code=1)

    runner = QueryRunner(conn)
    candidates = resolve_symbol(runner, query)

    if not candidates:
        if json_output:
            print(json_mod.dumps({"error": "Symbol not found", "query": query}, indent=2, ensure_ascii=False))
        else:
            console.print(f"[red]Symbol not found: {query}[/red]")
        conn.close()
        raise typer.Exit(code=1)

    if len(candidates) > 1:
        if json_output:
            print(json_mod.dumps([
                {
                    "id": n.id,
                    "kind": n.kind,
                    "fqn": n.fqn,
                    "file": n.file,
                    "line": n.start_line + 1 if n.start_line is not None else None,
                }
                for n in candidates
            ], indent=2, ensure_ascii=False))
        else:
            console.print(f"[yellow]Found {len(candidates)} candidates:[/yellow]")
            for i, node in enumerate(candidates, 1):
                console.print(f"  [{i}] {node.kind}: {node.fqn}")
                console.print(f"      {node.location_str}")
        conn.close()
        raise typer.Exit(code=1)

    node = candidates[0]
    result = usages_tree(runner, node.node_id, depth=depth, limit=limit)

    if json_output:
        output = usages_tree_to_dict(result["target"], result["max_depth"], result["tree"])
        print(json_mod.dumps(output, indent=2, ensure_ascii=False))
    else:
        console.print(f"[bold]Usages of {node.fqn} (depth={depth}):[/bold]")
        for entry in result["tree"]:
            _print_tree_entry(entry)

    conn.close()


@app.command()
def deps(
    query: str = typer.Argument(..., help="Symbol to find dependencies for"),
    depth: int = typer.Option(1, "--depth", "-d", help="BFS depth for expansion"),
    limit: int = typer.Option(100, "--limit", "-l", help="Maximum results"),
    json_output: bool = typer.Option(False, "--json", "-j", help="Output as JSON"),
) -> None:
    """Find all dependencies of a symbol with depth expansion."""
    import json as json_mod

    from .db.query_runner import QueryRunner
    from .db.queries.resolve import resolve_symbol
    from .db.queries.deps import deps_tree
    from .output.json_formatter import deps_tree_to_dict

    try:
        conn = _get_connection()
        conn.verify_connectivity()
    except Neo4jConnectionError as e:
        console.print(f"[red]Error:[/red] {e}")
        raise typer.Exit(code=1)

    runner = QueryRunner(conn)
    candidates = resolve_symbol(runner, query)

    if not candidates:
        if json_output:
            print(json_mod.dumps({"error": "Symbol not found", "query": query}, indent=2, ensure_ascii=False))
        else:
            console.print(f"[red]Symbol not found: {query}[/red]")
        conn.close()
        raise typer.Exit(code=1)

    if len(candidates) > 1:
        if json_output:
            print(json_mod.dumps([
                {
                    "id": n.id,
                    "kind": n.kind,
                    "fqn": n.fqn,
                    "file": n.file,
                    "line": n.start_line + 1 if n.start_line is not None else None,
                }
                for n in candidates
            ], indent=2, ensure_ascii=False))
        else:
            console.print(f"[yellow]Found {len(candidates)} candidates:[/yellow]")
            for i, node in enumerate(candidates, 1):
                console.print(f"  [{i}] {node.kind}: {node.fqn}")
                console.print(f"      {node.location_str}")
        conn.close()
        raise typer.Exit(code=1)

    node = candidates[0]
    result = deps_tree(runner, node.node_id, depth=depth, limit=limit)

    if json_output:
        output = deps_tree_to_dict(result["target"], result["max_depth"], result["tree"])
        print(json_mod.dumps(output, indent=2, ensure_ascii=False))
    else:
        console.print(f"[bold]Dependencies of {node.fqn} (depth={depth}):[/bold]")
        for entry in result["tree"]:
            _print_tree_entry(entry)

    conn.close()


@app.command()
def owners(
    query: str = typer.Argument(..., help="Symbol to find ownership chain for"),
    json_output: bool = typer.Option(False, "--json", "-j", help="Output as JSON"),
) -> None:
    """Find the structural containment chain (Method -> Class -> File)."""
    import json as json_mod

    from .db.query_runner import QueryRunner
    from .db.queries.resolve import resolve_symbol
    from .db.queries.owners import owners_chain
    from .output.json_formatter import owners_chain_to_dict

    try:
        conn = _get_connection()
        conn.verify_connectivity()
    except Neo4jConnectionError as e:
        console.print(f"[red]Error:[/red] {e}")
        raise typer.Exit(code=1)

    runner = QueryRunner(conn)
    candidates = resolve_symbol(runner, query)

    if not candidates:
        if json_output:
            print(json_mod.dumps({"error": "Symbol not found", "query": query}, indent=2, ensure_ascii=False))
        else:
            console.print(f"[red]Symbol not found: {query}[/red]")
        conn.close()
        raise typer.Exit(code=1)

    if len(candidates) > 1:
        if json_output:
            print(json_mod.dumps([
                {
                    "id": n.id,
                    "kind": n.kind,
                    "fqn": n.fqn,
                    "file": n.file,
                    "line": n.start_line + 1 if n.start_line is not None else None,
                }
                for n in candidates
            ], indent=2, ensure_ascii=False))
        else:
            console.print(f"[yellow]Found {len(candidates)} candidates:[/yellow]")
            for i, node in enumerate(candidates, 1):
                console.print(f"  [{i}] {node.kind}: {node.fqn}")
                console.print(f"      {node.location_str}")
        conn.close()
        raise typer.Exit(code=1)

    node = candidates[0]
    result = owners_chain(runner, node.node_id)

    if json_output:
        output = owners_chain_to_dict(result["chain"])
        print(json_mod.dumps(output, indent=2, ensure_ascii=False))
    else:
        console.print(f"[bold]Ownership chain for {node.fqn}:[/bold]")
        for n in result["chain"]:
            line_str = f":{n.start_line + 1}" if n.start_line is not None else ""
            console.print(f"  {n.kind}: {n.fqn} ({n.file}{line_str})")

    conn.close()


@app.command("inherit")
def inherit_cmd(
    query: str = typer.Argument(..., help="Symbol to find inheritance for"),
    direction: str = typer.Option("up", "--direction", "-r", help="'up' for ancestors, 'down' for descendants"),
    depth: int = typer.Option(1, "--depth", "-d", help="BFS depth for expansion"),
    limit: int = typer.Option(100, "--limit", "-l", help="Maximum results"),
    json_output: bool = typer.Option(False, "--json", "-j", help="Output as JSON"),
) -> None:
    """Find inheritance tree (ancestors or descendants)."""
    import json as json_mod

    from .db.query_runner import QueryRunner
    from .db.queries.resolve import resolve_symbol
    from .db.queries.inherit import inherit_tree
    from .output.json_formatter import inherit_tree_to_dict

    try:
        conn = _get_connection()
        conn.verify_connectivity()
    except Neo4jConnectionError as e:
        console.print(f"[red]Error:[/red] {e}")
        raise typer.Exit(code=1)

    runner = QueryRunner(conn)
    candidates = resolve_symbol(runner, query)

    if not candidates:
        if json_output:
            print(json_mod.dumps({"error": "Symbol not found", "query": query}, indent=2, ensure_ascii=False))
        else:
            console.print(f"[red]Symbol not found: {query}[/red]")
        conn.close()
        raise typer.Exit(code=1)

    if len(candidates) > 1:
        if json_output:
            print(json_mod.dumps([
                {
                    "id": n.id,
                    "kind": n.kind,
                    "fqn": n.fqn,
                    "file": n.file,
                    "line": n.start_line + 1 if n.start_line is not None else None,
                }
                for n in candidates
            ], indent=2, ensure_ascii=False))
        else:
            console.print(f"[yellow]Found {len(candidates)} candidates:[/yellow]")
            for i, node in enumerate(candidates, 1):
                console.print(f"  [{i}] {node.kind}: {node.fqn}")
                console.print(f"      {node.location_str}")
        conn.close()
        raise typer.Exit(code=1)

    node = candidates[0]

    try:
        result = inherit_tree(runner, node.node_id, direction=direction, depth=depth, limit=limit)
    except ValueError as e:
        if json_output:
            print(json_mod.dumps({"error": str(e)}, indent=2, ensure_ascii=False))
        else:
            console.print(f"[red]Error:[/red] {e}")
        conn.close()
        raise typer.Exit(code=1)

    if json_output:
        output = inherit_tree_to_dict(
            result["root"], result["direction"], result["max_depth"], result["tree"]
        )
        print(json_mod.dumps(output, indent=2, ensure_ascii=False))
    else:
        console.print(f"[bold]Inheritance ({direction}) for {node.fqn} (depth={depth}):[/bold]")
        for entry in result["tree"]:
            _print_inherit_entry(entry)

    conn.close()


@app.command("overrides")
def overrides_cmd(
    query: str = typer.Argument(..., help="Method to find overrides for"),
    direction: str = typer.Option("up", "--direction", "-r", help="'up' for overridden, 'down' for overriding"),
    depth: int = typer.Option(1, "--depth", "-d", help="BFS depth for expansion"),
    limit: int = typer.Option(100, "--limit", "-l", help="Maximum results"),
    json_output: bool = typer.Option(False, "--json", "-j", help="Output as JSON"),
) -> None:
    """Find override chain/tree for a method."""
    import json as json_mod

    from .db.query_runner import QueryRunner
    from .db.queries.resolve import resolve_symbol
    from .db.queries.overrides import overrides_tree
    from .output.json_formatter import overrides_tree_to_dict

    try:
        conn = _get_connection()
        conn.verify_connectivity()
    except Neo4jConnectionError as e:
        console.print(f"[red]Error:[/red] {e}")
        raise typer.Exit(code=1)

    runner = QueryRunner(conn)
    candidates = resolve_symbol(runner, query)

    if not candidates:
        if json_output:
            print(json_mod.dumps({"error": "Symbol not found", "query": query}, indent=2, ensure_ascii=False))
        else:
            console.print(f"[red]Symbol not found: {query}[/red]")
        conn.close()
        raise typer.Exit(code=1)

    if len(candidates) > 1:
        if json_output:
            print(json_mod.dumps([
                {
                    "id": n.id,
                    "kind": n.kind,
                    "fqn": n.fqn,
                    "file": n.file,
                    "line": n.start_line + 1 if n.start_line is not None else None,
                }
                for n in candidates
            ], indent=2, ensure_ascii=False))
        else:
            console.print(f"[yellow]Found {len(candidates)} candidates:[/yellow]")
            for i, node in enumerate(candidates, 1):
                console.print(f"  [{i}] {node.kind}: {node.fqn}")
                console.print(f"      {node.location_str}")
        conn.close()
        raise typer.Exit(code=1)

    node = candidates[0]

    try:
        result = overrides_tree(runner, node.node_id, direction=direction, depth=depth, limit=limit)
    except ValueError as e:
        if json_output:
            print(json_mod.dumps({"error": str(e)}, indent=2, ensure_ascii=False))
        else:
            console.print(f"[red]Error:[/red] {e}")
        conn.close()
        raise typer.Exit(code=1)

    if json_output:
        output = overrides_tree_to_dict(
            result["root"], result["direction"], result["max_depth"], result["tree"]
        )
        print(json_mod.dumps(output, indent=2, ensure_ascii=False))
    else:
        console.print(f"[bold]Overrides ({direction}) for {node.fqn} (depth={depth}):[/bold]")
        for entry in result["tree"]:
            _print_tree_entry(entry)

    conn.close()


def _print_tree_entry(entry: dict, indent: str = "  ") -> None:
    """Print a tree entry to console with indentation."""
    file_info = ""
    if entry.get("file"):
        file_info = f" ({entry['file']}"
        if entry.get("line") is not None:
            file_info += f":{entry['line'] + 1}"
        file_info += ")"
    console.print(f"{indent}[{entry['depth']}] {entry['fqn']}{file_info}")
    for child in entry.get("children", []):
        _print_tree_entry(child, indent + "  ")


def _print_inherit_entry(entry: dict, indent: str = "  ") -> None:
    """Print an inherit tree entry (includes kind) to console."""
    file_info = ""
    if entry.get("file"):
        file_info = f" ({entry['file']}"
        if entry.get("line") is not None:
            file_info += f":{entry['line'] + 1}"
        file_info += ")"
    console.print(f"{indent}[{entry['depth']}] {entry.get('kind', '')}: {entry['fqn']}{file_info}")
    for child in entry.get("children", []):
        _print_inherit_entry(child, indent + "  ")


if __name__ == "__main__":
    app()
