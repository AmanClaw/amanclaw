"""
MCP Client Manager -- connects to MCP servers and exposes their tools
as OpenAI-compatible tool definitions alongside built-in skills.

Runs MCP connections in a dedicated background thread with its own event loop
to keep async context managers alive for the process lifetime.
"""

import os
import re
import asyncio
import logging
import threading
from pathlib import Path
from typing import Any

import yaml

logger = logging.getLogger("amanclaw.mcp_client")

CONFIG_PATH = Path(os.environ.get("CONFIG_PATH", "config.yaml"))


class MCPManager:
    """Manages connections to MCP servers and their tools."""

    def __init__(self, config: dict):
        self._server_configs: dict[str, dict] = {}
        self._tools: dict[str, dict] = {}  # prefixed_name -> {server, tool_def, session}
        self._connected_servers: set[str] = set()

        # Dedicated event loop in background thread for MCP connections
        self._loop = asyncio.new_event_loop()
        self._thread = threading.Thread(target=self._run_loop, daemon=True)
        self._thread.start()

        raw = config.get("mcp_servers") or {}
        for name, server_cfg in raw.items():
            resolved = self._resolve_env_vars(server_cfg)
            self._server_configs[name] = resolved

    def _run_loop(self):
        """Run the dedicated event loop in a background thread."""
        asyncio.set_event_loop(self._loop)
        self._loop.run_forever()

    def _run_in_loop(self, coro, timeout=30):
        """Submit a coroutine to the background loop and wait for result."""
        future = asyncio.run_coroutine_threadsafe(coro, self._loop)
        return future.result(timeout=timeout)

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

    def start(self):
        """Connect to all configured MCP servers (called from main thread)."""
        for name, cfg in self._server_configs.items():
            try:
                self._run_in_loop(self._connect_server(name, cfg), timeout=60)
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

            # Create a long-lived task that keeps the context managers open
            connected = asyncio.get_event_loop().create_future()

            async def _keep_alive():
                async with stdio_client(server_params) as (read_stream, write_stream):
                    async with ClientSession(read_stream, write_stream) as session:
                        await session.initialize()
                        result = await session.list_tools()

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

                        self._connected_servers.add(name)
                        logger.info(f"MCP server '{name}': connected, {len(result.tools)} tools discovered")
                        connected.set_result(True)

                        # Block forever to keep the connection alive
                        await asyncio.Future()

            task = asyncio.ensure_future(_keep_alive())
            # Wait for connection to be established (or fail)
            try:
                await asyncio.wait_for(connected, timeout=30)
            except Exception:
                task.cancel()
                raise

        elif "url" in cfg:
            connected = asyncio.get_event_loop().create_future()

            async def _keep_alive_sse():
                async with sse_client(cfg["url"]) as (read_stream, write_stream):
                    async with ClientSession(read_stream, write_stream) as session:
                        await session.initialize()
                        result = await session.list_tools()

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

                        self._connected_servers.add(name)
                        logger.info(f"MCP server '{name}': connected, {len(result.tools)} tools discovered")
                        connected.set_result(True)
                        await asyncio.Future()

            task = asyncio.ensure_future(_keep_alive_sse())
            try:
                await asyncio.wait_for(connected, timeout=30)
            except Exception:
                task.cancel()
                raise
        else:
            logger.warning(f"MCP server '{name}': no 'command' or 'url' specified, skipping")

    def add_server(self, name: str, cfg: dict) -> str:
        """Add and connect a new MCP server at runtime. Persists to config."""
        if name in self._connected_servers:
            return f"Server '{name}' is already connected. Remove it first."

        resolved = self._resolve_env_vars(cfg)
        self._server_configs[name] = resolved

        try:
            self._run_in_loop(self._connect_server(name, resolved), timeout=60)
        except Exception as e:
            self._server_configs.pop(name, None)
            return f"Failed to connect '{name}': {e}"

        self._save_to_config()
        tool_count = sum(1 for t in self._tools.values() if t["server"] == name)
        return f"Server '{name}' connected — {tool_count} tools discovered."

    def remove_server(self, name: str) -> str:
        """Disconnect and remove an MCP server."""
        if name not in self._connected_servers:
            return f"Server '{name}' is not connected."

        # Remove tools belonging to this server
        to_remove = [k for k, v in self._tools.items() if v["server"] == name]
        for k in to_remove:
            del self._tools[k]

        self._connected_servers.discard(name)
        self._server_configs.pop(name, None)
        self._save_to_config()
        return f"Server '{name}' removed."

    def list_servers(self) -> str:
        """List all configured MCP servers and their status."""
        if not self._server_configs:
            return "No MCP servers configured."

        lines = []
        for name, cfg in self._server_configs.items():
            connected = name in self._connected_servers
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

            full_config["mcp_servers"] = self._server_configs
            with open(CONFIG_PATH, "w") as f:
                yaml.dump(full_config, f, default_flow_style=False, sort_keys=False)
            logger.info("MCP config saved to config.yaml")
        except Exception as e:
            logger.warning(f"Could not save MCP config: {e}")

    def stop(self):
        """Stop the background event loop."""
        self._loop.call_soon_threadsafe(self._loop.stop)
        self._thread.join(timeout=5)
        self._connected_servers.clear()
        self._tools.clear()

    def get_tool_definitions(self) -> list[dict]:
        """Return all MCP tools as OpenAI-compatible tool definitions."""
        return [info["definition"] for info in self._tools.values()]

    def execute(self, tool_name: str, tool_input: dict) -> str:
        """Execute an MCP tool by its prefixed name (sync, dispatches to background loop)."""
        if tool_name not in self._tools:
            return f"Error: Unknown MCP tool '{tool_name}'"

        info = self._tools[tool_name]
        session = info["session"]
        original_name = info["original_name"]

        async def _call():
            result = await session.call_tool(original_name, arguments=tool_input)
            parts = []
            for block in result.content:
                if hasattr(block, "text"):
                    parts.append(block.text)
                else:
                    parts.append(str(block))
            return "\n".join(parts) if parts else "(empty result)"

        try:
            future = asyncio.run_coroutine_threadsafe(_call(), self._loop)
            return future.result(timeout=30)
        except Exception as e:
            error_msg = f"MCP tool '{tool_name}' failed: {type(e).__name__}: {e}"
            logger.error(error_msg)
            return error_msg

    def has_tool(self, tool_name: str) -> bool:
        """Check if a tool name belongs to MCP."""
        return tool_name in self._tools
