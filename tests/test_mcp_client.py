# tests/test_mcp_client.py
"""Tests for MCP client manager."""
import pytest
from amanclaw.mcp_client import MCPManager


class TestMCPManager:
    def test_init_empty_config(self):
        """MCPManager with no servers configured should work fine."""
        mgr = MCPManager({})
        assert mgr.get_tool_definitions() == []

    def test_init_with_server_config(self):
        """MCPManager should parse server configs."""
        config = {
            "mcp_servers": {
                "test-server": {
                    "command": "echo",
                    "args": ["hello"],
                }
            }
        }
        mgr = MCPManager(config)
        assert "test-server" in mgr._server_configs

    def test_get_tool_definitions_not_connected(self):
        """Before start(), no tools should be available."""
        config = {
            "mcp_servers": {
                "test-server": {
                    "command": "echo",
                    "args": ["hello"],
                }
            }
        }
        mgr = MCPManager(config)
        assert mgr.get_tool_definitions() == []
