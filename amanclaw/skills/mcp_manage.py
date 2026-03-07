"""
MCP server management skill — add, remove, list MCP servers at runtime.
Admin-only. Changes persist to config.yaml.
"""

import json
import logging
from amanclaw.skills import skill

logger = logging.getLogger("amanclaw.skills.mcp_manage")


def _get_manager():
    """Get the active MCP manager."""
    from amanclaw.skills import _mcp_manager
    if not _mcp_manager:
        return None
    return _mcp_manager


@skill(
    name="mcp_list_servers",
    description="List all configured MCP servers and their connection status, including how many tools each provides.",
    parameters={},
    timeout=10,
)
def mcp_list_servers() -> str:
    """List MCP servers."""
    mgr = _get_manager()
    if not mgr:
        return "MCP is not initialized."
    return mgr.list_servers()


@skill(
    name="mcp_add_server",
    description=(
        "Add a new MCP server. Supports two types:\n"
        "- stdio: runs a local command (e.g., npx, python). Provide 'command' and optional 'args'.\n"
        "- sse: connects to a remote URL. Provide 'url'.\n"
        "The server connects immediately and its tools become available.\n"
        "Example stdio: name='filesystem', command='npx', args='-y @modelcontextprotocol/server-filesystem /home'\n"
        "Example sse: name='my_api', url='http://localhost:8080/sse'"
    ),
    parameters={
        "name": {
            "type": "string",
            "description": "Unique name for this MCP server (e.g., 'filesystem', 'weather')",
        },
        "command": {
            "type": "string",
            "description": "For stdio servers: the command to run (e.g., 'npx', 'python', 'node')",
            "optional": True,
        },
        "args": {
            "type": "string",
            "description": "For stdio servers: space-separated arguments (e.g., '-y @modelcontextprotocol/server-filesystem /home')",
            "optional": True,
        },
        "url": {
            "type": "string",
            "description": "For SSE servers: the server URL (e.g., 'http://localhost:8080/sse')",
            "optional": True,
        },
        "env": {
            "type": "string",
            "description": "Optional environment variables as JSON object (e.g., '{\"API_KEY\": \"abc123\"}')",
            "optional": True,
        },
    },
    timeout=30,
)
def mcp_add_server(name: str, command: str = None, args: str = None, url: str = None, env: str = None) -> str:
    """Add an MCP server at runtime."""
    mgr = _get_manager()
    if not mgr:
        return "MCP is not initialized."

    if not command and not url:
        return "Error: Provide either 'command' (for stdio) or 'url' (for SSE)."

    cfg = {}
    if command:
        cfg["command"] = command
        if args:
            cfg["args"] = args.split()
    if url:
        cfg["url"] = url
    if env:
        try:
            cfg["env"] = json.loads(env)
        except json.JSONDecodeError:
            return "Error: 'env' must be valid JSON (e.g., '{\"KEY\": \"value\"}')"

    return mgr.add_server(name, cfg)


@skill(
    name="mcp_remove_server",
    description="Remove and disconnect an MCP server. Its tools will no longer be available.",
    parameters={
        "name": {
            "type": "string",
            "description": "Name of the MCP server to remove",
        },
    },
    timeout=15,
)
def mcp_remove_server(name: str) -> str:
    """Remove an MCP server."""
    mgr = _get_manager()
    if not mgr:
        return "MCP is not initialized."
    return mgr.remove_server(name)
