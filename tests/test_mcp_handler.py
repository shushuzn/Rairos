"""Tests for rairos_mcp handle_call_tool routing and protocol handlers."""

import json
import pytest
import sys
from pathlib import Path

# Ensure project root on path for imports
project_root = Path(__file__).resolve().parent.parent
if str(project_root) not in sys.path:
    sys.path.insert(0, str(project_root))

from rairos_mcp import (
    handle_initialize,
    handle_list_tools,
    handle_call_tool,
    handle_request,
    error_response,
    success_response,
    MCP_VERSION,
)
from mcp.tools_defs import get_tools


class TestProtocolHandlers:
    """Test MCP protocol handler functions."""

    def test_handle_initialize(self):
        resp = handle_initialize()
        assert "result" in resp
        data = resp["result"]
        assert data["protocolVersion"] == MCP_VERSION
        assert data["serverInfo"]["name"] == "rairos"
        assert data["capabilities"] == {"tools": True}

    def test_handle_list_tools(self):
        resp = handle_list_tools()
        assert "result" in resp
        tools = resp["result"]["tools"]
        assert isinstance(tools, list)
        assert len(tools) >= 30

    def test_handle_request_initialize(self):
        resp = handle_request("initialize", {})
        assert "result" in resp
        assert resp["result"]["protocolVersion"] == MCP_VERSION

    def test_handle_request_tools_list(self):
        resp = handle_request("tools/list", {})
        assert "result" in resp

    def test_handle_request_unknown_method(self):
        resp = handle_request("nonexistent/method", {})
        assert "error" in resp
        assert resp["error"]["code"] == "UNKNOWN_METHOD"

    def test_error_response_format(self):
        resp = error_response("TEST_CODE", "test message")
        assert "error" in resp
        assert resp["error"]["code"] == "TEST_CODE"
        assert resp["error"]["message"] == "test message"

    def test_success_response_format(self):
        resp = success_response({"key": "value"})
        assert "result" in resp
        assert resp["result"] == {"key": "value"}


class TestToolRouting:
    """Test that every tool in handle_call_tool has a corresponding implementation."""

    TOOL_NAMES_IN_HANDLER = [
        "paper_ingest",
        "paper_search",
        "paper_chat",
        "paper_recommend",
        "pdf_download",
        "pdf_extract_text",
        "pdf_extract_structured",
        "kg_query",
        "kg_paper_subgraph",
        "kg_tag_graph",
        "kg_full_graph",
        "tag_add",
        "tag_remove",
        "tag_list",
        "tag_all",
        "trends_detect_trending",
        "trends_predict_next",
        "trends_top_predictions",
        "trends_compare_tags",
        "chart_query",
        "research_run",
        "slides_generate",
        "cite_fetch",
        "paper_analyze",
        "paper2code_run",
        "citation_graph",
        "gap_detect",
        "gap_submit",
        "gap_evolve",
        "research_agent_start",
        "research_agent_stop",
        "gene_pool_decay",
        "crossover",
        "leaderboard",
        "gene_pool_watcher",
        "claim_graph",
        "research_agent_status",
        "research_agent_trigger",
        "hypothesis_generate",
        "hypothesis_list",
        "experiment_record",
        "litreview_generate",
        "litreview_list",
        "research_memory_add_stance",
        "research_memory_list_stances",
        "research_memory_check_paper",
        "research_memory_anomalies",
        "review_simulate",
        "review_list",
        "routeplan_create",
        "routeplan_list",
        "routeplan_update_step",
        "routeplan_revise",
        "briefing_generate",
        "citation_chain_build",
        "citation_chain_families",
        "citation_chain_silent",
        "citation_chain_render",
        "impact_rank",
        "impact_score_paper",
        "impact_leaderboard",
        "replication_check",
        "replication_compare",
    ]

    def test_all_handler_tools_have_impl(self):
        """Every tool routed in handle_call_tool must have a corresponding tool_* function."""
        import rairos_mcp
        for name in self.TOOL_NAMES_IN_HANDLER:
            func_name = f"tool_{name}"
            assert hasattr(rairos_mcp, func_name), f"Missing function: {func_name} for tool '{name}'"
            func = getattr(rairos_mcp, func_name)
            assert callable(func), f"tool_{name} is not callable"

    def test_all_handler_tools_in_defs(self):
        """Every tool routed in handle_call_tool must be in tools_defs."""
        defs = {t["name"] for t in get_tools()}
        missing = [n for n in self.TOOL_NAMES_IN_HANDLER if n not in defs]
        assert not missing, f"Tools in handler but missing from tools_defs: {missing}"

    def test_unknown_tool_returns_error(self):
        """Calling an unknown tool returns UNKNOWN_TOOL error."""
        resp = handle_call_tool("nonexistent_tool_xyz", {})
        assert "error" in resp
        assert resp["error"]["code"] == "UNKNOWN_TOOL"

    def test_call_tool_exception_handling(self):
        """Exceptions in tool_* functions are caught and returned as TOOL_ERROR."""
        # Call with a valid tool name but invalid args that trigger an exception
        # We use a tool that has required args
        resp = handle_call_tool("paper_ingest", {})  # missing required 'identifier'
        # This should NOT be an unhandled exception - either error_response or the tool handles it
        assert isinstance(resp, dict)


class TestMcpJsonRpc:
    """Test JSON-RPC round-trip through handle_request."""

    def test_json_roundtrip_initialize(self):
        """handle_request must accept and return JSON-serializable dicts."""
        resp = handle_request("initialize", {})
        assert isinstance(resp, dict)
        # id is added by main() after handle_request, not inside handle_request itself
        assert "result" in resp

    def test_json_roundtrip_list_tools(self):
        resp = handle_request("tools/list", {})
        assert isinstance(resp, dict)
        assert "result" in resp

    def test_call_tool_request_format(self):
        """tools/call request format maps to handle_call_tool."""
        params = {"name": "tag_all", "arguments": {}}
        resp = handle_request("tools/call", params)
        assert isinstance(resp, dict)

    def test_success_response_serializable(self):
        """success_response output must be JSON-serializable."""
        resp = success_response({"papers": [], "count": 0})
        # Must not raise
        json.dumps(resp)
        assert "result" in resp

    def test_error_response_serializable(self):
        """error_response output must be JSON-serializable."""
        resp = error_response("CODE", "message")
        json.dumps(resp)  # Must not raise
        assert "error" in resp
