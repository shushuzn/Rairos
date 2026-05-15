"""Rairos MCP protocol handlers (lightweight — tool implementations are Rust-native).

This module provides the MCP protocol handler functions needed by tests.
Actual tool implementations are handled by the Rust MCP server (rairos_mcp_py).
"""

import json
from typing import Any, Dict

MCP_VERSION = "2024-11-05"


class MCPError(Exception):
    """MCP protocol-level error."""

    def __init__(self, code: str, message: str):
        self.code = code
        self.message = message
        super().__init__(f"[{code}] {message}")


def error_response(code: str, message: str) -> dict:
    return {"error": {"code": code, "message": message}}


def success_response(result: Any) -> dict:
    return {"result": result}


# Lazy import of tool definitions to avoid circular deps
_TOOLS_CACHE: list[dict] | None = None


def _get_tools() -> list[dict]:
    global _TOOLS_CACHE
    if _TOOLS_CACHE is None:
        from mcp.tools_defs import get_tools
        _TOOLS_CACHE = get_tools()
    return _TOOLS_CACHE


def handle_initialize() -> dict:
    return success_response({
        "protocolVersion": MCP_VERSION,
        "serverInfo": {"name": "rairos", "version": "0.1.0"},
        "capabilities": {"tools": True},
    })


def handle_list_tools() -> dict:
    return success_response({"tools": _get_tools()})


def handle_call_tool(name: str, arguments: Dict[str, Any]) -> dict:
    # All tools are Rust-native — route to rairos_mcp_py
    try:
        from rairos_mcp_py import call_tool_rs
        arguments_json = json.dumps(arguments)
        result = call_tool_rs(name, arguments_json)
        if result is None:
            return error_response("UNKNOWN_TOOL", f"Unknown tool: {name}")
        # Result is a JSON string from Rust — parse it
        parsed = json.loads(result)
        if "error" in parsed:
            return parsed
        return success_response(parsed)
    except ImportError:
        return error_response("UNKNOWN_TOOL", f"Tool '{name}' not found (Rust MCP not available)")
    except json.JSONDecodeError:
        return error_response("TOOL_ERROR", f"Failed to parse result for tool '{name}'")
    except Exception as e:
        return error_response("TOOL_ERROR", str(e))


def handle_request(method: str, params: Dict) -> dict:
    if method == "initialize":
        return handle_initialize()
    elif method == "tools/list":
        return handle_list_tools()
    elif method == "tools/call":
        name = params.get("name")
        args = params.get("arguments", {})
        return handle_call_tool(name, args)
    else:
        return error_response("UNKNOWN_METHOD", f"Unknown method: {method}")


def main() -> None:
    """Entry point for MCP stdio transport."""
    import sys
    for line in sys.stdin:
        try:
            msg = json.loads(line.strip())
            req_id = msg.get("id")
            method = msg.get("method", "")
            params = msg.get("params", {})
            resp = handle_request(method, params)
            resp["id"] = req_id
            sys.stdout.write(json.dumps(resp) + "\n")
            sys.stdout.flush()
        except (json.JSONDecodeError, KeyError) as e:
            err = error_response("PARSE_ERROR", str(e))
            sys.stdout.write(json.dumps(err) + "\n")
            sys.stdout.flush()


if __name__ == "__main__":
    main()
