"""Rich console formatters for usages, deps, owners, inherit, and overrides output."""

from rich.console import Console
from rich.table import Table
from rich.tree import Tree

from ..models.results import (
    UsagesTreeResult,
    DepsTreeResult,
    UsageEntry,
    DepsEntry,
    OwnersResult,
    InheritTreeResult,
    InheritEntry,
    OverridesTreeResult,
    OverrideEntry,
)

console = Console()


def _format_location(file: str | None, line: int | None) -> str:
    """Format file:line location string (1-based line numbers)."""
    if file and line is not None:
        return f"{file}:{line + 1}"
    elif file:
        return file
    return "<unknown>"


def print_usages_result(result: UsagesTreeResult) -> None:
    """Print usages result as a Rich table (flat) or tree (depth > 1)."""
    console.print(f"[bold]Usages of[/bold] {result.target.fqn}")
    console.print(f"  defined at: {result.target.location_str}")
    console.print()

    if not result.tree:
        console.print("[dim]No usages found[/dim]")
        return

    if result.max_depth == 1:
        _print_usages_flat(result)
    else:
        _print_usages_tree(result)


def _print_usages_flat(result: UsagesTreeResult) -> None:
    """Print flat usages as a table."""
    table = Table(title=f"Usages ({len(result.tree)} results)")
    table.add_column("File", style="yellow")
    table.add_column("Line")
    table.add_column("Referrer", style="green")

    for entry in result.tree:
        table.add_row(
            entry.file or "<unknown>",
            str(entry.line + 1) if entry.line is not None else "",
            entry.fqn,
        )

    console.print(table)


def _print_usages_tree(result: UsagesTreeResult) -> None:
    """Print usages as a Rich tree for depth > 1."""
    root = Tree(f"[bold]{result.target.fqn}[/bold]")
    _add_usage_children(root, result.tree)
    console.print(root)


def _add_usage_children(parent: Tree, entries: list[UsageEntry]) -> None:
    """Recursively add usage entries to a Rich tree."""
    for entry in entries:
        loc = _format_location(entry.file, entry.line)
        label = f"[{entry.depth}] {entry.fqn} ({loc})"
        node = parent.add(label)
        if entry.children:
            _add_usage_children(node, entry.children)


def print_deps_result(result: DepsTreeResult) -> None:
    """Print deps result as a Rich table (flat) or tree (depth > 1)."""
    console.print(f"[bold]Dependencies of[/bold] {result.target.fqn}")
    console.print(f"  defined at: {result.target.location_str}")
    console.print()

    if not result.tree:
        console.print("[dim]No dependencies found[/dim]")
        return

    if result.max_depth == 1:
        _print_deps_flat(result)
    else:
        _print_deps_tree(result)


def _print_deps_flat(result: DepsTreeResult) -> None:
    """Print flat deps as a table."""
    table = Table(title=f"Dependencies ({len(result.tree)} results)")
    table.add_column("File", style="yellow")
    table.add_column("Line")
    table.add_column("Dependency", style="green")

    for entry in result.tree:
        table.add_row(
            entry.file or "<unknown>",
            str(entry.line + 1) if entry.line is not None else "",
            entry.fqn,
        )

    console.print(table)


def _print_deps_tree(result: DepsTreeResult) -> None:
    """Print deps as a Rich tree for depth > 1."""
    root = Tree(f"[bold]{result.target.fqn}[/bold]")
    _add_deps_children(root, result.tree)
    console.print(root)


def _add_deps_children(parent: Tree, entries: list[DepsEntry]) -> None:
    """Recursively add deps entries to a Rich tree."""
    for entry in entries:
        loc = _format_location(entry.file, entry.line)
        label = f"[{entry.depth}] {entry.fqn} ({loc})"
        node = parent.add(label)
        if entry.children:
            _add_deps_children(node, entry.children)


# --- Owners ---


def print_owners_result(result: OwnersResult) -> None:
    """Print owners result as a flat chain display."""
    if not result.chain:
        console.print("[dim]No containment chain found[/dim]")
        return

    target = result.chain[0]
    console.print(f"[bold]Owners of[/bold] {target.fqn}")
    console.print()

    table = Table(title=f"Containment chain ({len(result.chain)} levels)")
    table.add_column("Level", style="dim")
    table.add_column("Kind", style="cyan")
    table.add_column("FQN", style="green")
    table.add_column("Location", style="yellow")

    for i, node in enumerate(result.chain):
        loc = _format_location(node.file, node.start_line)
        table.add_row(str(i), node.kind, node.fqn, loc)

    console.print(table)


# --- Inherit ---


def print_inherit_result(result: InheritTreeResult) -> None:
    """Print inherit result as a Rich tree."""
    direction_label = "ancestors" if result.direction == "up" else "descendants"
    console.print(
        f"[bold]Inheritance {direction_label} of[/bold] {result.root.fqn}"
    )
    console.print(f"  defined at: {result.root.location_str}")
    console.print()

    if not result.tree:
        console.print(f"[dim]No {direction_label} found[/dim]")
        return

    root = Tree(f"[bold]{result.root.fqn}[/bold] ({result.root.kind})")
    _add_inherit_children(root, result.tree)
    console.print(root)


def _add_inherit_children(
    parent: Tree, entries: list[InheritEntry]
) -> None:
    """Recursively add inherit entries to a Rich tree."""
    for entry in entries:
        loc = _format_location(entry.file, entry.line)
        label = f"[{entry.depth}] {entry.fqn} ({entry.kind}) ({loc})"
        node = parent.add(label)
        if entry.children:
            _add_inherit_children(node, entry.children)


# --- Overrides ---


def print_overrides_result(result: OverridesTreeResult) -> None:
    """Print overrides result as a Rich tree."""
    direction_label = (
        "overridden by" if result.direction == "down" else "overrides"
    )
    console.print(
        f"[bold]Method {direction_label}[/bold] {result.root.fqn}"
    )
    console.print(f"  defined at: {result.root.location_str}")
    console.print()

    if not result.tree:
        console.print("[dim]No overrides found[/dim]")
        return

    root = Tree(f"[bold]{result.root.fqn}[/bold]")
    _add_override_children(root, result.tree)
    console.print(root)


def _add_override_children(
    parent: Tree, entries: list[OverrideEntry]
) -> None:
    """Recursively add override entries to a Rich tree."""
    for entry in entries:
        loc = _format_location(entry.file, entry.line)
        label = f"[{entry.depth}] {entry.fqn} ({loc})"
        node = parent.add(label)
        if entry.children:
            _add_override_children(node, entry.children)
