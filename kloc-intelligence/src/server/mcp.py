"""MCP (Model Context Protocol) server for kloc-intelligence.

Implements JSON-RPC 2.0 based MCP protocol for Claude and other MCP clients.
Backed by Neo4j instead of in-memory sot.json.

Usage:
    kloc-intelligence mcp-server
    kloc-intelligence mcp-server --config /path/to/kloc.json

Config file format:
    {
        "projects": [
            {"name": "my-app", "database": "myapp"},
            {"name": "payments", "database": "payments"}
        ]
    }
"""

from __future__ import annotations

import atexit
import json
import sys
from typing import Any, Optional

from ..config import Neo4jConfig
from ..db.connection import Neo4jConnection, Neo4jConnectionError
from ..db.query_runner import QueryRunner


class MCPServer:
    """MCP server for kloc-intelligence with multi-project support.

    Each project maps to a Neo4j database name. Connections are
    lazily established on first query.
    """

    def __init__(
        self,
        config_path: Optional[str] = None,
        database: Optional[str] = None,
    ):
        """Initialize server.

        Args:
            config_path: Path to JSON config file with multiple projects.
            database: Single database name (creates default project).
        """
        self._projects: dict[str, str] = {}  # name -> database
        self._connections: dict[str, Neo4jConnection] = {}
        self._runners: dict[str, QueryRunner] = {}

        if config_path:
            self._load_config(config_path)
        else:
            self._projects["default"] = database or "neo4j"

    def _load_config(self, config_path: str) -> None:
        """Load projects from config file."""
        with open(config_path, "r") as f:
            config = json.load(f)

        projects = config.get("projects", [])
        if not projects:
            raise ValueError("Config file must contain at least one project")

        for proj in projects:
            name = proj.get("name")
            db = proj.get("database")
            if not name or not db:
                raise ValueError("Each project must have 'name' and 'database' fields")
            self._projects[name] = db

    def _get_runner(self, project: Optional[str] = None) -> QueryRunner:
        """Get query runner for a project (lazy connection).

        Args:
            project: Project name. If None, uses default (only if single project).
        """
        if project is None:
            if len(self._projects) == 1:
                project = list(self._projects.keys())[0]
            else:
                raise ValueError(
                    f"Multiple projects configured. Specify 'project' parameter. "
                    f"Available: {list(self._projects.keys())}"
                )

        if project not in self._projects:
            raise ValueError(
                f"Unknown project: {project}. "
                f"Available: {list(self._projects.keys())}"
            )

        if project not in self._runners:
            neo4j_config = Neo4jConfig.from_env()
            neo4j_config.database = self._projects[project]
            conn = Neo4jConnection(neo4j_config)
            conn.verify_connectivity()
            self._connections[project] = conn
            self._runners[project] = QueryRunner(conn)

        return self._runners[project]

    def shutdown(self) -> None:
        """Clean shutdown: close all connections."""
        for conn in self._connections.values():
            try:
                conn.close()
            except Exception:
                pass
        self._connections.clear()
        self._runners.clear()

    def get_projects(self) -> list[dict]:
        """Return list of configured projects."""
        return [
            {"name": name, "database": db}
            for name, db in self._projects.items()
        ]

    def get_tools(self) -> list[dict]:
        """Return list of available MCP tools."""
        project_prop = {
            "type": "string",
            "description": "Project name (required if multiple projects configured)",
        }

        return [
            {
                "name": "kloc_projects",
                "description": "List all configured projects. Use this to discover available projects before querying.",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": [],
                },
            },
            {
                "name": "kloc_resolve",
                "description": "Resolve a symbol to its definition location. Supports FQN, partial match, or method syntax.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "symbol": {"type": "string", "description": "Symbol to resolve"},
                        "project": project_prop,
                    },
                    "required": ["symbol"],
                },
            },
            {
                "name": "kloc_usages",
                "description": "Find all usages of a symbol with depth expansion.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "symbol": {"type": "string", "description": "Symbol to find usages for"},
                        "depth": {"type": "integer", "description": "BFS depth (default: 1)", "default": 1},
                        "limit": {"type": "integer", "description": "Max results (default: 50)", "default": 50},
                        "project": project_prop,
                    },
                    "required": ["symbol"],
                },
            },
            {
                "name": "kloc_deps",
                "description": "Find all dependencies of a symbol with depth expansion.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "symbol": {"type": "string", "description": "Symbol to find dependencies for"},
                        "depth": {"type": "integer", "description": "BFS depth (default: 1)", "default": 1},
                        "limit": {"type": "integer", "description": "Max results (default: 50)", "default": 50},
                        "project": project_prop,
                    },
                    "required": ["symbol"],
                },
            },
            {
                "name": "kloc_context",
                "description": "Get bidirectional context: what uses it and what it uses. With include_impl, shows implementations/overrides.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "symbol": {"type": "string", "description": "Symbol to get context for"},
                        "depth": {"type": "integer", "description": "BFS depth (default: 1)", "default": 1},
                        "limit": {"type": "integer", "description": "Max results per direction (default: 50)", "default": 50},
                        "include_impl": {"type": "boolean", "description": "Include implementations/overrides", "default": False},
                        "project": project_prop,
                    },
                    "required": ["symbol"],
                },
            },
            {
                "name": "kloc_owners",
                "description": "Show structural containment chain (Method -> Class -> File).",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "symbol": {"type": "string", "description": "Symbol to find ownership for"},
                        "project": project_prop,
                    },
                    "required": ["symbol"],
                },
            },
            {
                "name": "kloc_inherit",
                "description": "Show inheritance tree for a class.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "symbol": {"type": "string", "description": "Class to show inheritance for"},
                        "direction": {"type": "string", "enum": ["up", "down"], "description": "up=ancestors, down=descendants", "default": "up"},
                        "depth": {"type": "integer", "description": "BFS depth (default: 1)", "default": 1},
                        "limit": {"type": "integer", "description": "Max results (default: 100)", "default": 100},
                        "project": project_prop,
                    },
                    "required": ["symbol"],
                },
            },
            {
                "name": "kloc_overrides",
                "description": "Show override tree for a method.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "symbol": {"type": "string", "description": "Method to show overrides for"},
                        "direction": {"type": "string", "enum": ["up", "down"], "description": "up=overridden, down=overriding", "default": "up"},
                        "depth": {"type": "integer", "description": "BFS depth (default: 1)", "default": 1},
                        "limit": {"type": "integer", "description": "Max results (default: 100)", "default": 100},
                        "project": project_prop,
                    },
                    "required": ["symbol"],
                },
            },
        ]

    def call_tool(self, name: str, arguments: dict) -> Any:
        """Call a tool by name."""
        handlers = {
            "kloc_projects": self._handle_projects,
            "kloc_resolve": self._handle_resolve,
            "kloc_usages": self._handle_usages,
            "kloc_deps": self._handle_deps,
            "kloc_context": self._handle_context,
            "kloc_owners": self._handle_owners,
            "kloc_inherit": self._handle_inherit,
            "kloc_overrides": self._handle_overrides,
        }
        handler = handlers.get(name)
        if not handler:
            raise ValueError(f"Unknown tool: {name}")
        return handler(arguments)

    def _resolve_symbol(self, symbol: str, project: Optional[str] = None):
        """Resolve symbol and return node or raise error."""
        from ..db.queries.resolve import resolve_symbol

        runner = self._get_runner(project)
        candidates = resolve_symbol(runner, symbol)

        if not candidates:
            raise ValueError(f"Symbol not found: {symbol}")

        if len(candidates) > 1:
            cands = [{"id": c.id, "kind": c.kind, "fqn": c.fqn} for c in candidates]
            raise ValueError(f"Ambiguous: {len(candidates)} candidates: {cands}")

        return candidates[0], runner

    def _handle_projects(self, args: dict) -> dict:
        """List all configured projects."""
        return {"projects": self.get_projects()}

    def _handle_resolve(self, args: dict) -> dict:
        project = args.get("project")
        node, _ = self._resolve_symbol(args["symbol"], project)
        result: dict = {
            "id": node.id,
            "kind": node.kind,
            "name": node.name,
            "fqn": node.fqn,
            "file": node.file,
            "line": node.start_line + 1 if node.start_line is not None else None,
        }
        if node.signature:
            result["signature"] = node.signature
        return result

    def _handle_usages(self, args: dict) -> dict:
        from ..db.queries.usages import usages_tree
        from ..output.json_formatter import usages_tree_to_dict

        project = args.get("project")
        node, runner = self._resolve_symbol(args["symbol"], project)
        result = usages_tree(
            runner, node.node_id,
            depth=args.get("depth", 1),
            limit=args.get("limit", 50),
        )
        return usages_tree_to_dict(result["target"], result["max_depth"], result["tree"])

    def _handle_deps(self, args: dict) -> dict:
        from ..db.queries.deps import deps_tree
        from ..output.json_formatter import deps_tree_to_dict

        project = args.get("project")
        node, runner = self._resolve_symbol(args["symbol"], project)
        result = deps_tree(
            runner, node.node_id,
            depth=args.get("depth", 1),
            limit=args.get("limit", 50),
        )
        return deps_tree_to_dict(result["target"], result["max_depth"], result["tree"])

    def _handle_context(self, args: dict) -> dict:
        from ..orchestration.context import ContextOrchestrator
        from ..models.output import ContextOutput

        project = args.get("project")
        node, runner = self._resolve_symbol(args["symbol"], project)
        orchestrator = ContextOrchestrator(runner)
        result = orchestrator.execute(
            node.node_id,
            depth=args.get("depth", 1),
            limit=args.get("limit", 50),
            include_impl=args.get("include_impl", False),
        )
        output = ContextOutput.from_result(result)
        return output.to_dict()

    def _handle_owners(self, args: dict) -> dict:
        from ..db.queries.owners import owners_chain
        from ..output.json_formatter import owners_chain_to_dict

        project = args.get("project")
        node, runner = self._resolve_symbol(args["symbol"], project)
        result = owners_chain(runner, node.node_id)
        return owners_chain_to_dict(result["chain"])

    def _handle_inherit(self, args: dict) -> dict:
        from ..db.queries.inherit import inherit_tree
        from ..output.json_formatter import inherit_tree_to_dict

        project = args.get("project")
        node, runner = self._resolve_symbol(args["symbol"], project)
        if node.kind not in ("Class", "Interface", "Trait", "Enum"):
            raise ValueError(f"Symbol must be a class/interface, got: {node.kind}")
        result = inherit_tree(
            runner, node.node_id,
            direction=args.get("direction", "up"),
            depth=args.get("depth", 1),
            limit=args.get("limit", 100),
        )
        return inherit_tree_to_dict(
            result["root"], result["direction"],
            result["max_depth"], result["tree"],
        )

    def _handle_overrides(self, args: dict) -> dict:
        from ..db.queries.overrides import overrides_tree
        from ..output.json_formatter import overrides_tree_to_dict

        project = args.get("project")
        node, runner = self._resolve_symbol(args["symbol"], project)
        if node.kind != "Method":
            raise ValueError(f"Symbol must be a method, got: {node.kind}")
        result = overrides_tree(
            runner, node.node_id,
            direction=args.get("direction", "up"),
            depth=args.get("depth", 1),
            limit=args.get("limit", 100),
        )
        return overrides_tree_to_dict(
            result["root"], result["direction"],
            result["max_depth"], result["tree"],
        )


def run_mcp_server(
    config_path: Optional[str] = None,
    database: Optional[str] = None,
) -> None:
    """Run the MCP server using stdio with JSON-RPC 2.0 protocol.

    Args:
        config_path: Path to JSON config file with multiple projects.
        database: Single database name (creates default project).
    """
    server = MCPServer(config_path=config_path, database=database)
    atexit.register(server.shutdown)

    def send_response(req_id: Any, result: Any = None, error: Any = None) -> None:
        response: dict = {"jsonrpc": "2.0", "id": req_id}
        if error is not None:
            response["error"] = {"code": -32000, "message": str(error)}
        else:
            response["result"] = result
        print(json.dumps(response), flush=True)

    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue

        try:
            request = json.loads(line)
        except json.JSONDecodeError as e:
            send_response(None, error=f"Parse error: {e}")
            continue

        req_id = request.get("id")
        method = request.get("method", "")
        params = request.get("params", {})

        try:
            if method == "initialize":
                send_response(req_id, {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {"tools": {}},
                    "serverInfo": {"name": "kloc-intelligence", "version": "0.1.0"},
                })
            elif method == "notifications/initialized":
                pass  # No response needed for notifications
            elif method == "tools/list":
                send_response(req_id, {"tools": server.get_tools()})
            elif method == "tools/call":
                tool_name = params.get("name", "")
                arguments = params.get("arguments", {})
                result = server.call_tool(tool_name, arguments)
                send_response(req_id, {
                    "content": [{"type": "text", "text": json.dumps(result, indent=2)}],
                })
            elif method == "ping":
                send_response(req_id, {})
            else:
                send_response(req_id, error=f"Method not found: {method}")
        except Neo4jConnectionError as e:
            send_response(req_id, error=f"Neo4j connection error: {e}")
        except Exception as e:
            send_response(req_id, error=str(e))
