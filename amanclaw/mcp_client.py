"""
MCP Client Manager -- connects to MCP servers and exposes their tools
as OpenAI-compatible tool definitions alongside built-in skills.
"""

import os
import re
import logging
from typing import Any

logger = logging.getLogger("amanclaw.mcp_client")


class MCPManager:
    """Manages connections to MCP servers and their tools."""

    def __init__(self, config: dict):
        self._server_configs: dict[str, dict] = {}
        self._connections: dict[str, Any] = {}  # name -> (client, session)
        self._tools: dict[str, dict] = {}  # prefixed_name -> {server, tool_def}

        raw = config.get("mcp_servers") or {}
        for name, server_cfg in raw.items():
            # Expand ${VAR} env vars in config values
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
            # stdio transport
            env = {**os.environ, **(cfg.get("env") or {})}
            server_params = StdioServerParameters(
                command=cfg["command"],
                args=cfg.get("args", []),
                env=env,
            )
            transport = stdio_client(server_params)
        elif "url" in cfg:
            # SSE transport
            transport = sse_client(cfg["url"])
        else:
            logger.warning(f"MCP server '{name}': no 'command' or 'url' specified, skipping")
            return

        read_stream, write_stream = await transport.__aenter__()
        session = ClientSession(read_stream, write_stream)
        await session.__aenter__()
        await session.initialize()

        # Discover tools
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
        """Execute an MCP tool by its prefixed name. Returns result as string."""
        if tool_name not in self._tools:
            return f"Error: Unknown MCP tool '{tool_name}'"

        info = self._tools[tool_name]
        session = info["session"]
        original_name = info["original_name"]

        try:
            result = await session.call_tool(original_name, arguments=tool_input)
            # Flatten result content to string
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
