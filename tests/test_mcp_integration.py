# tests/test_mcp_integration.py
"""Tests for MCP integration with skill registry."""
import pytest
from unittest.mock import AsyncMock, MagicMock
from amanclaw.skills import get_tool_definitions, execute, set_mcp_manager


class TestMCPIntegration:
    def test_get_tool_definitions_includes_mcp(self):
        """Tool definitions should include MCP tools when manager is set."""
        mock_mgr = MagicMock()
        mock_mgr.get_tool_definitions.return_value = [
            {"name": "mcp_test_greet", "description": "Say hello", "input_schema": {"type": "object", "properties": {}}}
        ]
        set_mcp_manager(mock_mgr)

        defs = get_tool_definitions()
        names = [d["name"] for d in defs]
        assert "mcp_test_greet" in names

        # Cleanup
        set_mcp_manager(None)

    def test_get_tool_definitions_without_mcp(self):
        """Tool definitions should work fine without MCP manager."""
        set_mcp_manager(None)
        defs = get_tool_definitions()
        # Should still have built-in skills
        assert isinstance(defs, list)
