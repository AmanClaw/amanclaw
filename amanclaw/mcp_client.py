"""
MCP Client Manager -- connects to MCP servers and exposes their tools
as OpenAI-compatible tool definitions alongside built-in skills.

Supports runtime add/remove of servers (persisted to config).
"""

import os
import re
import logging
from pathlib import Path
from typing import Any

import yaml

logger = logging.getLogger("amanclaw.mcp_client")

CONFIG_PATH = Path(os.environ.get("CONFIG_PATH", "config.yaml"))


class MCPManager:
    """Manages connections to MCP servers and their tools."""

    def __init__(self, config: dict):
        self._server_configs: dict[str, dict] = {}
        self._connections: dict[str, Any] = {}  # name -> (transport, session)
        self._tools: dict[str, dict] = {}  # prefixed_name -> {server, tool_def}

        raw = config.get("mcp_servers") or {}
        for name, server_cfg in raw.items():
            resolved = self._resolve_env_vars(server_cfg)
            self._server_configs[name] = resolved

    def _resolve_env_vars(self, obj):
        """Recursively resolve ${VAR} patterns in config values."""
        if isinstance(obj, str):
            def replacer(match):
                var = match.group(1)
                return os.environ.get(var, match.group(0))
            return re.sub(r"\$\{(\w+)\}", replacer, obj)
        elif isinstance(obj, dict):
            return {k: self._resolve_env_vars(v) for k, v in obj.items()}
        elif isinstance(obj, list):
            return [self._resolve_env_vars(v) for v in obj]
        return obj

    async def start(self):
        """Connect to all configured MCP servers."""
        for name, cfg in self._server_configs.items():
            try:
                await self._connect_server(name, cfg)
            except Exception as e:
                logger.warning(f"Failed to connect MCP server '{name}': {e}")

    async def _connect_server(self, name: str, cfg: dict):
        """Connect to a single MCP server and discover its tools."""
        try:
            from mcp import ClientSession
            from mcp.client.stdio import stdio_client, StdioServerParameters
            from mcp.client.sse import sse_client
        except ImportError:
            logger.error("MCP SDK not installed. Install with: pip install mcp")
            return

        if "command" in cfg:
            env = {**os.environ, **(cfg.get("env") or {})}
            server_params = StdioServerParameters(
                command=cfg["command"],
                args=cfg.get("args", []),
                env=env,
            )
            transport = stdio_client(server_params)
        elif "url" in cfg:
            transport = sse_client(cfg["url"])
        else:
            logger.warning(f"MCP server '{name}': no 'command' or 'url' specified, skipping")
            return

        read_stream, write_stream = await transport.__aenter__()
        session = ClientSession(read_stream, write_stream)
        await session.__aenter__()
        await session.initialize()

        result = await session.list_tools()
        self._connections[name] = (transport, session)

        for tool in result.tools:
            prefixed = f"mcp_{name}_{tool.name}"
            self._tools[prefixed] = {
                "server": name,
                "original_name": tool.name,
                "session": session,
                "definition": {
                    "name": prefixed,
                    "description": f"[MCP:{name}] {tool.description or tool.name}",
                    "input_schema": tool.inputSchema if hasattr(tool, 'inputSchema') else {
                        "type": "object",
                        "properties": {},
                    },
                },
            }
        logger.info(f"MCP server '{name}': connected, {len(result.tools)} tools discovered")

    async def add_server(self, name: str, cfg: dict) -> str:
        """Add and connect a new MCP server at runtime. Persists to config."""
        if name in self._connections:
            return f"Server '{name}' is already connected. Remove it first."

        resolved = self._resolve_env_vars(cfg)
        self._server_configs[name] = resolved

        try:
            await self._connect_server(name, resolved)
        except Exception as e:
            self._server_configs.pop(name, None)
            return f"Failed to connect '{name}': {e}"

        # Persist to config file
        self._save_to_config()

        tool_count = sum(1 for t in self._tools.values() if t["server"] == name)
        return f"Server '{name}' connected — {tool_count} tools discovered."

    async def remove_server(self, name: str) -> str:
        """Disconnect and remove an MCP server. Persists to config."""
        if name not in self._connections:
            return f"Server '{name}' is not connected."

        # Disconnect
        transport, session = self._connections.pop(name)
        try:
            await session.__aexit__(None, None, None)
            await transport.__aexit__(None, None, None)
        except Exception as e:
            logger.warning(f"Error disconnecting '{name}': {e}")

        # Remove tools belonging to this server
        to_remove = [k for k, v in self._tools.items() if v["server"] == name]
        for k in to_remove:
            del self._tools[k]

        self._server_configs.pop(name, None)
        self._save_to_config()
        return f"Server '{name}' disconnected and removed."

    def list_servers(self) -> str:
        """List all configured MCP servers and their status."""
        if not self._server_configs:
            return "No MCP servers configured."

        lines = []
        for name, cfg in self._server_configs.items():
            connected = name in self._connections
            tool_count = sum(1 for t in self._tools.values() if t["server"] == name)
            transport_type = "stdio" if "command" in cfg else "sse" if "url" in cfg else "unknown"
            status = f"connected, {tool_count} tools" if connected else "disconnected"
            target = cfg.get("command", cfg.get("url", "?"))
            lines.append(f"• {name} [{transport_type}] — {status}\n  {target}")
        return "\n".join(lines)

    def _save_to_config(self):
        """Persist current MCP server configs to config.yaml."""
        try:
            if CONFIG_PATH.exists():
                with open(CONFIG_PATH) as f:
                    full_config = yaml.safe_load(f) or {}
            else:
                full_config = {}

            # Build clean config (without resolved env vars — keep originals)
            full_config["mcp_servers"] = self._server_configs
            with open(CONFIG_PATH, "w") as f:
                yaml.dump(full_config, f, default_flow_style=False, sort_keys=False)
            logger.info("MCP config saved to config.yaml")
        except Exception as e:
            logger.warning(f"Could not save MCP config: {e}")

    async def stop(self):
        """Disconnect all MCP servers."""
        for name, (transport, session) in self._connections.items():
            try:
                await session.__aexit__(None, None, None)
                await transport.__aexit__(None, None, None)
                logger.info(f"MCP server '{name}': disconnected")
            except Exception as e:
                logger.warning(f"Error disconnecting MCP server '{name}': {e}")
        self._connections.clear()
        self._tools.clear()

    def get_tool_definitions(self) -> list[dict]:
        """Return all MCP tools as OpenAI-compatible tool definitions."""
        return [info["definition"] for info in self._tools.values()]

    async def execute(self, tool_name: str, tool_input: dict) -> str:
        """Execute an MCP tool by its prefixed name."""
        if tool_name not in self._tools:
            return f"Error: Unknown MCP tool '{tool_name}'"

        info = self._tools[tool_name]
        session = info["session"]
        original_name = info["original_name"]

        try:
            result = await session.call_tool(original_name, arguments=tool_input)
            parts = []
            for block in result.content:
                if hasattr(block, "text"):
                    parts.append(block.text)
                else:
                    parts.append(str(block))
            return "\n".join(parts) if parts else "(empty result)"
        except Exception as e:
            error_msg = f"MCP tool '{tool_name}' failed: {type(e).__name__}: {e}"
            logger.error(error_msg)
            return error_msg

    def has_tool(self, tool_name: str) -> bool:
        """Check if a tool name belongs to MCP."""
        return tool_name in self._tools
