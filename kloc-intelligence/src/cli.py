"""kloc-intelligence CLI interface."""

import typer
from rich.console import Console
from rich.table import Table

app = typer.Typer(name="kloc-intelligence", help="Graph-native code intelligence platform")
schema_app = typer.Typer(name="schema", help="Schema management commands")
app.add_typer(schema_app, name="schema")

console = Console()


@schema_app.command("ensure")
def schema_ensure():
    """Create all constraints and indexes."""
    from .config import Neo4jConfig
    from .db.connection import Neo4jConnection
    from .db.schema import ensure_schema

    config = Neo4jConfig.from_env()
    with Neo4jConnection(config) as conn:
        result = ensure_schema(conn)
        console.print(f"[green]Schema ensured:[/green] {result['constraints']} constraints, "
                       f"{result['indexes']} indexes")


@schema_app.command("reset")
def schema_reset():
    """Drop all data and recreate schema."""
    from .config import Neo4jConfig
    from .db.connection import Neo4jConnection
    from .db.schema import drop_all, ensure_schema

    config = Neo4jConfig.from_env()
    with Neo4jConnection(config) as conn:
        drop_all(conn)
        result = ensure_schema(conn)
        console.print(f"[green]Schema reset:[/green] {result['constraints']} constraints, "
                       f"{result['indexes']} indexes")


@schema_app.command("verify")
def schema_verify():
    """Verify schema state."""
    from .config import Neo4jConfig
    from .db.connection import Neo4jConnection
    from .db.schema import verify_schema, get_node_count, get_edge_count

    config = Neo4jConfig.from_env()
    with Neo4jConnection(config) as conn:
        result = verify_schema(conn)
        nodes = get_node_count(conn)
        edges = get_edge_count(conn)

        table = Table(title="Schema Status")
        table.add_column("Metric", style="cyan")
        table.add_column("Value", style="green")
        table.add_row("Constraints", str(result["constraints"]))
        table.add_row("Indexes", str(result["indexes"]))
        table.add_row("Nodes", str(nodes))
        table.add_row("Edges", str(edges))
        console.print(table)


@app.command("import")
def import_sot(
    sot_path: str = typer.Argument(..., help="Path to sot.json file"),
    clear: bool = typer.Option(True, help="Clear database before import"),
    validate: bool = typer.Option(True, help="Validate after import"),
):
    """Import a sot.json file into Neo4j."""
    import time
    from .config import Neo4jConfig
    from .db.connection import Neo4jConnection
    from .db.schema import ensure_schema, drop_all
    from .db.importer import parse_sot, import_nodes, import_edges, validate_import

    config = Neo4jConfig.from_env()
    conn = Neo4jConnection(config)
    conn.verify_connectivity()
    start = time.perf_counter()

    console.print(f"Parsing {sot_path}...")
    nodes, edges = parse_sot(sot_path)
    console.print(f"  Parsed {len(nodes):,} nodes, {len(edges):,} edges")

    if clear:
        console.print("Clearing database...")
        drop_all(conn)
        ensure_schema(conn)

    console.print("Importing nodes...")
    import_nodes(conn, nodes)
    console.print("Importing edges...")
    import_edges(conn, edges)

    if validate:
        report = validate_import(conn, len(nodes), len(edges))
        node_status = "OK" if report["node_match"] else "MISMATCH"
        edge_status = "OK" if report["edge_match"] else "MISMATCH"
        console.print(
            f"  Nodes: {report['node_count']:,} / {report['expected_nodes']:,}  {node_status}"
        )
        console.print(
            f"  Edges: {report['edge_count']:,} / {report['expected_edges']:,}  {edge_status}"
        )

    total = time.perf_counter() - start
    console.print(f"\nImport complete in {total:.1f}s")
    conn.close()


@app.command()
def resolve(
    query: str = typer.Argument(..., help="Symbol to resolve"),
    output_json: bool = typer.Option(False, "--json", "-j", help="Output as JSON"),
):
    """Resolve a symbol to its definition(s)."""
    import json as json_mod

    from .config import Neo4jConfig
    from .db.connection import Neo4jConnection
    from .db.query_runner import QueryRunner
    from .db.queries.resolve import resolve_symbol as do_resolve

    config = Neo4jConfig.from_env()
    conn = Neo4jConnection(config)
    runner = QueryRunner(conn)
    candidates = do_resolve(runner, query)

    if output_json:
        result = {
            "query": query,
            "candidates": [
                {
                    "id": n.node_id,
                    "kind": n.kind,
                    "name": n.name,
                    "fqn": n.fqn,
                    "file": n.file,
                    "line": n.start_line + 1 if n.start_line is not None else None,
                }
                for n in candidates
            ],
        }
        console.print(json_mod.dumps(result, indent=2))
    else:
        if not candidates:
            console.print(f"No matches for: {query}")
        else:
            table = Table(title=f"Resolve: {query}")
            table.add_column("Kind", style="cyan")
            table.add_column("FQN", style="green")
            table.add_column("File", style="yellow")
            table.add_column("Line")
            for n in candidates:
                table.add_row(
                    n.kind,
                    n.fqn,
                    n.file or "",
                    str(n.start_line + 1) if n.start_line is not None else "",
                )
            console.print(table)
    conn.close()


if __name__ == "__main__":
    app()
