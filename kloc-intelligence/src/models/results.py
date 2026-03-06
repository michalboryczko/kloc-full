"""Result models for usages and deps commands."""

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
