"""Jin10 Financial Data MCP Client — real-time quotes, news, calendar."""

from __future__ import annotations

import json
import logging
import os
import time
from pathlib import Path
from typing import Any, Dict, List, Optional

# Load .env if present (before any other imports)
_env = Path(__file__).parent.parent / ".env"
if _env.exists():
    with open(_env) as _f:
        for _line in _f:
            _line = _line.strip()
            if _line and not _line.startswith("#") and "=" in _line:
                _k, _, _v = _line.partition("=")
                os.environ.setdefault(_k.strip(), _v.strip())

import requests

logger = logging.getLogger(__name__)

MCP_URL = os.getenv("JIN10_MCP_URL", "https://mcp.jin10.com/mcp")
MCP_TOKEN = os.getenv("JIN10_MCP_TOKEN", "")
MCP_VERSION = "2025-11-25"


class MCPError(Exception):
    """MCP protocol or business error."""

    pass


def _fix_mojibake(text: str) -> str:
    """Fix UTF-8 bytes misinterpreted as Latin-1 (mojibake).

    Jin10 MCP server returns UTF-8 bytes as Latin-1, causing CJK text
    like '阿联酋' to appear as 'é¿èé'. This reverses that.
    """
    if not text or not any(ord(c) > 0x7F for c in text):
        return text
    try:
        fixed = text.encode("latin-1", errors="replace").decode("utf-8", errors="replace")
        if fixed != text:
            has_cjk = any(ord(c) > 0x4E00 for c in fixed)
            more_alpha = sum(1 for c in fixed if c.isalpha()) > sum(1 for c in text if c.isalpha())
            if has_cjk or more_alpha:
                return fixed
    except Exception:
        pass
    return text


class Jin10Client:
    """MCP client for Jin10 financial data service."""

    def __init__(self, url: str = MCP_URL, token: str = MCP_TOKEN):
        self.url = url
        self.token = token
        self.headers = {
            "Content-Type": "application/json",
            "Authorization": f"Bearer {token}",
        }
        self._session = requests.Session()
        self._session.headers.update(self.headers)
        self._initialized = False
        self._session_id: str = ""
        self._tools: Dict[str, Any] = {}
        self._resources: Dict[str, Any] = {}

    def _fix_encoding(self, obj):
        """Recursively fix mojibake in all string values of a result dict/list."""
        if isinstance(obj, str):
            return _fix_mojibake(obj)
        elif isinstance(obj, dict):
            return {k: self._fix_encoding(v) for k, v in obj.items()}
        elif isinstance(obj, list):
            return [self._fix_encoding(item) for item in obj]
        return obj

    def _call(self, method: str, params: Any = None, id: int = 0) -> Dict[str, Any]:
        """Send a JSON-RPC request. Handles SSE (Streamable HTTP) responses."""
        if not id:
            id = int(time.time() * 1000) % 10**6
        payload = {
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
        }
        if params is not None:
            payload["params"] = params

        headers = dict(self.headers)
        if self._session_id:
            headers["Mcp-Session-Id"] = self._session_id

        try:
            r = self._session.post(self.url, json=payload, headers=headers, timeout=30)
            r.raise_for_status()

            # Handle SSE (text/event-stream) responses
            ct = r.headers.get("Content-Type", "")
            if "event-stream" in ct:
                body = r.text
                sid = r.headers.get("Mcp-Session-Id", "")
                if sid:
                    self._session_id = sid
                for line in body.split("\n"):
                    line = line.strip()
                    if line.startswith("data: "):
                        data = json.loads(line[6:])
                        if "error" in data:
                            err = data["error"]
                            raise MCPError(
                                f"JSON-RPC error [{err.get('code')}]: {err.get('message')}"
                            )
                        if data.get("id") == id:
                            return self._fix_encoding(data.get("result", {}))  # type: ignore[no-any-return]  # type: ignore[no-any-return]

                raise MCPError(f"No matching response in SSE for method '{method}'")

            # Plain JSON response
            data = r.json()
            if "error" in data:
                err = data["error"]
                raise MCPError(f"JSON-RPC error [{err.get('code')}]: {err.get('message')}")
            return self._fix_encoding(data.get("result", {}))  # type: ignore[no-any-return]

        except requests.RequestException as e:
            raise MCPError(f"HTTP error: {e}") from e
        except json.JSONDecodeError as e:
            raise MCPError(f"Invalid JSON-RPC response: {e}") from e

    def initialize(self) -> Dict[str, Any]:
        """Initialize MCP session (Streamable HTTP)."""
        result = self._call(
            "initialize",
            {
                "protocolVersion": MCP_VERSION,
                "capabilities": {},
                "clientInfo": {"name": "rairos", "version": "1.0"},
            },
        )
        self._initialized = True
        return result

    def tools_list(self) -> List[Dict[str, Any]]:
        """List available MCP tools."""
        result = self._call("tools/list")
        self._tools = {t["name"]: t for t in result.get("tools", [])}
        return list(self._tools.values())

    def resources_list(self) -> List[Dict[str, Any]]:
        """List available MCP resources."""
        result = self._call("resources/list")
        self._resources = {r["name"]: r for r in result.get("resources", [])}
        return list(self._resources.values())

    def call_tool(self, name: str, arguments: Dict[str, Any]) -> Dict[str, Any]:
        """Call an MCP tool and return structured content."""
        if not self._initialized:
            self.initialize()
        result = self._call("tools/call", {"name": name, "arguments": arguments})
        return result

    def read_resource(self, uri: str) -> Any:
        """Read an MCP resource."""
        if not self._initialized:
            self.initialize()
        result = self._call("resources/read", {"uri": uri})
        return result

    # ── High-level helpers ────────────────────────────────────────────

    def ensure_init(self):
        if not self._initialized:
            self.initialize()
            self.tools_list()

    def get_quote(self, code: str) -> Dict[str, Any]:
        """Get real-time quote for a symbol."""
        self.ensure_init()
        result = self.call_tool("get_quote", {"code": code})
        return result.get("structuredContent", result.get("content", {}))  # type: ignore[return-value,no-any-return]

    def get_kline(self, code: str, time: int = 1, count: int = 10) -> List[Dict]:
        """Get K-line data. time in minutes (1, 5, 15, 60, 240, 1440)."""
        self.ensure_init()
        result = self.call_tool("get_kline", {"code": code, "time": time, "count": count})
        return result.get("structuredContent", result.get("content", {}))  # type: ignore[return-value,no-any-return]

    def list_flash(self, cursor: str = "") -> Dict[str, Any]:
        """Get latest flash news."""
        self.ensure_init()
        params = {}
        if cursor:
            params["cursor"] = cursor
        result = self.call_tool("list_flash", params)
        return result.get("structuredContent", result.get("content", {}))  # type: ignore[return-value,no-any-return]

    def search_flash(self, keyword: str) -> List[Dict]:
        """Search flash news by keyword."""
        self.ensure_init()
        result = self.call_tool("search_flash", {"keyword": keyword})
        return result.get("structuredContent", result.get("content", {}))  # type: ignore[return-value,no-any-return]

    def list_news(self, cursor: str = "") -> Dict[str, Any]:
        """Get latest news list."""
        self.ensure_init()
        params = {}
        if cursor:
            params["cursor"] = cursor
        result = self.call_tool("list_news", params)
        return result.get("structuredContent", result.get("content", {}))  # type: ignore[return-value,no-any-return]

    def search_news(self, keyword: str, cursor: str = "") -> Dict[str, Any]:
        """Search news by keyword."""
        self.ensure_init()
        params = {"keyword": keyword}
        if cursor:
            params["cursor"] = cursor
        result = self.call_tool("search_news", params)
        return result.get("structuredContent", result.get("content", {}))  # type: ignore[return-value,no-any-return]

    def get_news(self, id: str) -> Dict[str, Any]:
        """Get single news article detail."""
        self.ensure_init()
        result = self.call_tool("get_news", {"id": id})
        return result.get("structuredContent", result.get("content", {}))  # type: ignore[return-value,no-any-return]

    def list_calendar(self) -> List[Dict]:
        """Get economic calendar data."""
        self.ensure_init()
        result = self.call_tool("list_calendar", {})
        return result.get("structuredContent", result.get("content", {}))  # type: ignore[return-value,no-any-return]

    def list_symbols(self) -> List[Dict]:
        """Get supported quote symbols."""
        self.ensure_init()
        result = self.read_resource("quote://codes")
        # The resource returns contents[0].text containing JSON
        contents = result.get("contents", [])
        if contents:
            text = contents[0].get("text", "{}")
            try:
                parsed = json.loads(text)  # type: ignore[no-any-return]
                return parsed.get("data", [])  # type: ignore[no-any-return]
            except json.JSONDecodeError:
                pass
        return []


# ── Quick-access functions ───────────────────────────────────────────

_default_client: Optional[Jin10Client] = None


def _get_client() -> Jin10Client:
    global _default_client
    if _default_client is None:
        _default_client = Jin10Client()
        _default_client.initialize()
        _default_client.tools_list()
    return _default_client


def quote(code: str) -> Dict[str, Any]:
    return _get_client().get_quote(code)


def kline(code: str, time: str = "1m", count: int = 10) -> Dict:
    time_int = int(time.rstrip("m")) if isinstance(time, str) else time  # type: ignore[assignment]
    return _get_client().get_kline(code, time_int, count)  # type: ignore[return-value]


def flash(cursor: str = "") -> Dict[str, Any]:
    return _get_client().list_flash(cursor)


def search_flash(keyword: str) -> List[Dict]:
    return _get_client().search_flash(keyword)


def news_list(cursor: str = "") -> Dict[str, Any]:
    return _get_client().list_news(cursor)


def search_news(keyword: str, cursor: str = "") -> Dict[str, Any]:
    return _get_client().search_news(keyword, cursor)


def news_detail(id: str) -> Dict[str, Any]:
    return _get_client().get_news(id)


def calendar() -> List[Dict]:
    return _get_client().list_calendar()


def symbols() -> List[Dict]:
    return _get_client().list_symbols()
