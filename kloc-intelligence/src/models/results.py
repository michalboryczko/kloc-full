"""Result models for usages, deps, owners, inherit, and overrides commands."""

from dataclasses import dataclass, field
from typing import Optional

from .node import NodeData


@dataclass
class UsageEntry:
    """Single usage entry with tree support."""

    depth: int
    node_id: str
    fqn: str
    file: Optional[str] = None
    line: Optional[int] = None
    children: list["UsageEntry"] = field(default_factory=list)

    def to_dict(self) -> dict:
        d = {"depth": self.depth, "node_id": self.node_id, "fqn": self.fqn}
        if self.file is not None:
            d["file"] = self.file
        if self.line is not None:
            d["line"] = self.line + 1  # 0-based to 1-based for output
        if self.children:
            d["children"] = [c.to_dict() for c in self.children]
        return d


@dataclass
class UsagesTreeResult:
    """Result of usages query with tree structure."""

    target: NodeData
    max_depth: int
    tree: list[UsageEntry] = field(default_factory=list)

    def to_dict(self) -> dict:
        return {
            "target": {
                "id": self.target.node_id,
                "kind": self.target.kind,
                "fqn": self.target.fqn,
                "file": self.target.file,
                "line": self.target.start_line + 1
                if self.target.start_line is not None
                else None,
            },
            "max_depth": self.max_depth,
            "tree": [e.to_dict() for e in self.tree],
        }


@dataclass
class DepsEntry:
    """Single dependency entry with tree support."""

    depth: int
    node_id: str
    fqn: str
    file: Optional[str] = None
    line: Optional[int] = None
    children: list["DepsEntry"] = field(default_factory=list)

    def to_dict(self) -> dict:
        d = {"depth": self.depth, "node_id": self.node_id, "fqn": self.fqn}
        if self.file is not None:
            d["file"] = self.file
        if self.line is not None:
            d["line"] = self.line + 1  # 0-based to 1-based for output
        if self.children:
            d["children"] = [c.to_dict() for c in self.children]
        return d


@dataclass
class DepsTreeResult:
    """Result of deps query with tree structure."""

    target: NodeData
    max_depth: int
    tree: list[DepsEntry] = field(default_factory=list)

    def to_dict(self) -> dict:
        return {
            "target": {
                "id": self.target.node_id,
                "kind": self.target.kind,
                "fqn": self.target.fqn,
                "file": self.target.file,
                "line": self.target.start_line + 1
                if self.target.start_line is not None
                else None,
            },
            "max_depth": self.max_depth,
            "tree": [e.to_dict() for e in self.tree],
        }


@dataclass
class OwnersResult:
    """Result of owners query -- containment chain from target up to File."""

    chain: list[NodeData]

    def to_dict(self) -> dict:
        return {
            "chain": [
                {
                    "id": n.node_id,
                    "kind": n.kind,
                    "fqn": n.fqn,
                    "file": n.file,
                    "line": n.start_line + 1
                    if n.start_line is not None
                    else None,
                }
                for n in self.chain
            ]
        }


@dataclass
class InheritEntry:
    """Single entry in an inheritance tree."""

    depth: int
    node_id: str
    fqn: str
    kind: str
    file: Optional[str] = None
    line: Optional[int] = None
    children: list["InheritEntry"] = field(default_factory=list)

    def to_dict(self) -> dict:
        d: dict = {
            "depth": self.depth,
            "node_id": self.node_id,
            "fqn": self.fqn,
            "kind": self.kind,
        }
        if self.file is not None:
            d["file"] = self.file
        if self.line is not None:
            d["line"] = self.line + 1  # 0-based to 1-based for output
        if self.children:
            d["children"] = [c.to_dict() for c in self.children]
        return d


@dataclass
class InheritTreeResult:
    """Result of inherit query with tree structure."""

    root: NodeData
    direction: str
    max_depth: int
    tree: list[InheritEntry] = field(default_factory=list)

    def to_dict(self) -> dict:
        return {
            "root": {
                "id": self.root.node_id,
                "kind": self.root.kind,
                "fqn": self.root.fqn,
                "file": self.root.file,
                "line": self.root.start_line + 1
                if self.root.start_line is not None
                else None,
            },
            "direction": self.direction,
            "max_depth": self.max_depth,
            "tree": [e.to_dict() for e in self.tree],
        }


@dataclass
class OverrideEntry:
    """Single entry in an overrides tree."""

    depth: int
    node_id: str
    fqn: str
    file: Optional[str] = None
    line: Optional[int] = None
    children: list["OverrideEntry"] = field(default_factory=list)

    def to_dict(self) -> dict:
        d: dict = {
            "depth": self.depth,
            "node_id": self.node_id,
            "fqn": self.fqn,
        }
        if self.file is not None:
            d["file"] = self.file
        if self.line is not None:
            d["line"] = self.line + 1  # 0-based to 1-based for output
        if self.children:
            d["children"] = [c.to_dict() for c in self.children]
        return d


@dataclass
class OverridesTreeResult:
    """Result of overrides query with tree structure."""

    root: NodeData
    direction: str
    max_depth: int
    tree: list[OverrideEntry] = field(default_factory=list)

    def to_dict(self) -> dict:
        return {
            "root": {
                "id": self.root.node_id,
                "kind": self.root.kind,
                "fqn": self.root.fqn,
                "file": self.root.file,
                "line": self.root.start_line + 1
                if self.root.start_line is not None
                else None,
            },
            "direction": self.direction,
            "max_depth": self.max_depth,
            "tree": [e.to_dict() for e in self.tree],
        }
