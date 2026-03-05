"""kloc-intelligence CLI entry point.

Typer-based CLI with subcommands for schema management, import,
and all 8 query commands.
"""

from __future__ import annotations

import typer
from rich.console import Console

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


if __name__ == "__main__":
    app()
