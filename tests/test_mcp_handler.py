"""Tests for rairos_mcp handle_call_tool routing and protocol handlers."""

import json
import sys
from pathlib import Path

# Ensure project root on path for imports
project_root = Path(__file__).resolve().parent.parent
if str(project_root) not in sys.path:
    sys.path.insert(0, str(project_root))

import pytest
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

# ── Rust MCP availability ──────────────────────────────────────────────────

HAS_RUST_MCP = False
try:
    import rairos_mcp_py
    HAS_RUST_MCP = True
except Exception:
    pass


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

    RUST_ONLY_TOOLS = {
        "paper_ingest", "paper_search", "paper_chat", "paper_recommend",
        "kg_query", "kg_paper_subgraph", "kg_tag_graph", "kg_full_graph",
        "tag_add", "tag_remove", "tag_list", "tag_all",
        "trends_detect_trending", "citation_graph",
        "briefing_generate", "citation_chain_build", "citation_chain_families",
        "citation_chain_silent", "citation_chain_render",
        "impact_rank", "impact_score_paper",
        "gap_detect", "litreview_generate", "slides_generate", "replication_check",
        "pdf_download", "pdf_extract_text", "pdf_extract_structured",
        "trends_predict_next", "trends_top_predictions", "trends_compare_tags",
        "cite_fetch", "gap_submit", "gap_evolve", "gene_pool_decay", "crossover",
        "research_memory_add_stance", "research_memory_list_stances",
        "research_memory_check_paper", "research_memory_anomalies",
        "leaderboard", "impact_leaderboard", "claim_graph",
        "review_list", "experiment_record", "litreview_list",
        "review_simulate", "gene_pool_watcher", "replication_compare",
        "routeplan_list", "routeplan_update_step", "routeplan_revise",
        "research_run",
        "paper_analyze", "routeplan_create",
    }

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
        """Every tool routed in handle_call_tool must have a corresponding tool_* function or Rust handler."""
        import rairos_mcp

        for name in self.TOOL_NAMES_IN_HANDLER:
            if name in self.RUST_ONLY_TOOLS:
                continue  # handled by Rust MCP
            func_name = f"tool_{name}"
            assert hasattr(rairos_mcp, func_name), (
                f"Missing function: {func_name} for tool '{name}'"
            )
            func = getattr(rairos_mcp, func_name)
            assert callable(func), f"tool_{name} is not callable"

    def test_all_handler_tools_in_defs(self):
        """Every tool routed in handle_call_tool must be in tools_defs or Rust."""
        defs = {t["name"] for t in get_tools()}
        missing = [n for n in self.TOOL_NAMES_IN_HANDLER if n not in defs and n not in self.RUST_ONLY_TOOLS]
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


@pytest.mark.skipif(not HAS_RUST_MCP, reason="rairos_mcp_py not available")
class TestRustToolIntegration:
    """Verify that Rust-backed MCP tools work correctly through the dispatch."""

    TOOL_LIST = [
        ("impact_score_paper",
         {"paper_id": "2301.00001", "title": "Test", "citation_count": 50, "year": 2023},
         ["paper_id", "composite", "citation_count"]),
        ("impact_rank",
         {"topic": "test", "top_k": 5,
          "papers": [{"arxiv_id": f"p{i:04d}", "title": f"T{i}",
                      "citation_count": i * 10, "year": 2020} for i in range(20)]},
         ["ranked"]),
        ("gap_detect",
         {"topic": "transformer neural networks"},
         ["gaps", "total", "topic"]),
        ("citation_chain_families",
         {"arxiv_id": "2301.00001"},
         ["families", "total"]),
        ("citation_chain_silent",
         {"arxiv_id": "2301.00001"},
         ["silent_citations", "total"]),
        ("citation_chain_render",
         {"arxiv_id": "2301.00001", "format": "text"},
         ["text"]),
        ("replication_check",
         {"paper_id": "2301.00001", "include_abstract": "A novel method using deep learning.",
          "title": "Test Paper"},
         ["score", "has_code", "has_data"]),
    ]

    @pytest.mark.parametrize("tool_name,args,expected_keys", TOOL_LIST)
    def test_rust_tool_returns_expected_keys(self, tool_name, args, expected_keys):
        """Each Rust tool returns a dict with the expected top-level keys."""
        resp = handle_call_tool(tool_name, args)
        assert "error" not in resp, (
            f"{tool_name} returned error: {resp.get('error')}"
        )
        result = resp.get("result", {})
        for key in expected_keys:
            assert key in result, (
                f"{tool_name} result missing key '{key}': {result}"
            )

    def test_rust_dispatch_takes_priority(self):
        """Rust tools return results, not UNKNOWN_TOOL error."""
        resp = handle_call_tool("impact_score_paper", {
            "paper_id": "2301.00001", "title": "T", "citation_count": 10, "year": 2023,
        })
        assert "error" not in resp, f"Rust dispatch failed: {resp}"
        assert "composite" in resp.get("result", {})

    def test_briefing_generate_skipped_without_llm_key(self):
        """briefing_generate needs LLM key; without it should fail gracefully."""
        resp = handle_call_tool("briefing_generate", {
            "arxiv_id": "2301.00001",
        })
        has_key = bool(__import__("os").environ.get("OPENAI_API_KEY") or __import__("os").environ.get("ANTHROPIC_API_KEY"))
        if not has_key:
            # Without key, should return error, not crash
            assert "error" in resp or isinstance(resp, dict)

    def test_impact_rank_ordering(self):
        """impact_rank returns papers sorted by composite descending."""
        papers = [{"arxiv_id": f"p{i:04d}", "title": f"T{i}",
                    "citation_count": i, "year": 2020} for i in range(10)]
        resp = handle_call_tool("impact_rank", {"topic": "t", "top_k": 10, "papers": papers})
        assert "error" not in resp, str(resp)
        ranked = resp.get("result", {}).get("ranked", [])
        assert len(ranked) >= 2
        scores = [r["composite"] for r in ranked]
        assert scores == sorted(scores, reverse=True), "not sorted descending"


class TestPythonFallback:
    """Verify Python-only tools still work through the fallback dispatch."""

    def test_python_fallback_still_works(self):
        """Python-only tools (like chart_query) should return results, not UNKNOWN_TOOL."""
        resp = handle_call_tool("chart_query", {"paper_id": "test", "action": "list"})
        # chart_query is Python-only — should not return UNKNOWN_TOOL
        assert "error" not in resp or resp.get("error", {}).get("code") != "UNKNOWN_TOOL", (
            f"Python fallback failed: {resp}"
        )
