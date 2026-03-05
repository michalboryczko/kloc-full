"""Node data model adapted from kloc-cli's NodeSpec.

Key difference: constructed from Neo4j node properties instead of msgspec struct.
"""

from __future__ import annotations

import re
from dataclasses import dataclass, field
from typing import Optional

_RE_VISIBILITY = re.compile(
    r"^(?:public\s+|protected\s+|private\s+|static\s+|final\s+|abstract\s+)*function\s+"
)
_RE_ATTRIBUTES = re.compile(r"#\[[^\]]*\]\s*")
_RE_WHITESPACE = re.compile(r"\s+")


@dataclass
class NodeData:
    """Node data model compatible with kloc-cli's NodeSpec.

    The `id` property returns `node_id` for backward compatibility with
    kloc-cli code that accesses `node.id`.
    """

    node_id: str
    kind: str
    name: str
    fqn: str
    symbol: str
    file: Optional[str] = None
    start_line: Optional[int] = None
    start_col: Optional[int] = None
    end_line: Optional[int] = None
    end_col: Optional[int] = None
    documentation: list[str] = field(default_factory=list)
    value_kind: Optional[str] = None
    type_symbol: Optional[str] = None
    call_kind: Optional[str] = None

    @property
    def id(self) -> str:
        """Compatibility alias for node_id (kloc-cli uses node.id)."""
        return self.node_id

    @property
    def location_str(self) -> str:
        """Return file:line string (1-based line numbers)."""
        if self.file and self.start_line is not None:
            return f"{self.file}:{self.start_line + 1}"
        elif self.file:
            return self.file
        return "<unknown>"

    @property
    def signature(self) -> Optional[str]:
        """Extract method/function signature from documentation.

        Ported from kloc-cli NodeSpec.signature property.
        """
        if not self.documentation or self.kind not in ("Method", "Function"):
            return None

        for doc in self.documentation:
            clean = doc.replace("```php", "").replace("```", "").strip()
            if "function " in clean:
                sig_lines = []
                capturing = False
                for line in clean.split("\n"):
                    line = line.strip()
                    if "function " in line:
                        capturing = True
                    if capturing:
                        sig_lines.append(line)
                        if ")" in line:
                            break

                if not sig_lines:
                    continue

                full_sig = " ".join(sig_lines)
                full_sig = _RE_VISIBILITY.sub("", full_sig)
                full_sig = _RE_ATTRIBUTES.sub("", full_sig)
                full_sig = _RE_WHITESPACE.sub(" ", full_sig).strip()

                if "(" in full_sig and ")" in full_sig:
                    return full_sig
                if "(" in full_sig:
                    method_name = full_sig.split("(")[0]
                    return f"{method_name}(...)"
                return full_sig
        return None

    @property
    def display_name(self) -> str:
        """Return display name - signature for methods, FQN otherwise."""
        if self.kind in ("Method", "Function") and self.signature:
            if "::" in self.fqn:
                class_part = self.fqn.rsplit("::", 1)[0]
                return f"{class_part}::{self.signature}"
            return self.signature
        return self.fqn
