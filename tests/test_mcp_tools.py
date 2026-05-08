"""Tests for mcp/tools_defs.py and cli/__main__.py."""
import pytest
from mcp.tools_defs import get_tools


class TestMcpTools:
    def test_get_tools(self):
        tools = get_tools()
        assert isinstance(tools, list)
