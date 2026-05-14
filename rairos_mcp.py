"""Rairos MCP Server - Provides research tools to Claude Code.

Usage as MCP server:
    python -m .claude.plugins.rairos.server

Or configure in Claude Code settings.json:
{
  "mcpServers": {
    "rairos": {
      "command": "python",
      "args": ["-m", ".claude.plugins.rairos.server"]
    }
  }
}
"""

import sys
from pathlib import Path

# Add project root to path
PROJECT_ROOT = Path(__file__).parent.parent.parent.parent
sys.path.insert(0, str(PROJECT_ROOT))

import datetime
import json
import logging
import threading
from typing import Any, Dict, List, Optional

# Configure logging
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

# MCP protocol
MCP_VERSION = "2024-11-05"


class MCPError(Exception):
    def __init__(self, code: str, message: str):
        self.code = code
        self.message = message
        super().__init__(message)


def error_response(code: str, message: str) -> dict:
    return {"error": {"code": code, "message": message}}


def success_response(result: Any) -> dict:
    return {"result": result}


# Tool definitions moved to mcp/tools_defs.py
from mcp.tools_defs import get_tools


# ─── Tool Implementations ────────────────────────────────────────────


def _ensure_data_dir():
    """Ensure data directory exists."""
    data_dir = PROJECT_ROOT / "data"
    data_dir.mkdir(exist_ok=True)
    return data_dir


def tool_pdf_download(arxiv_id: str, out_path: Optional[str] = None) -> Dict:
    """Download PDF for a paper from DB pdf_url or arXiv fallback."""
    try:
        from pathlib import Path
        from db.database import Database
        from pdf.extract import download_pdf

        db = Database()
        db.init()
        paper = db.get_paper(arxiv_id)

        if paper and getattr(paper, "pdf_url", ""):  # type: ignore[arg-type]
            pdf_url = paper.pdf_url
        else:
            pdf_url = f"https://arxiv.org/pdf/{arxiv_id}.pdf"

        if out_path:
            target = Path(out_path)
        else:
            import tempfile

            tmp_dir = Path(tempfile.gettempdir()) / "rairos_pdfs"
            target = tmp_dir / f"{arxiv_id}.pdf"

        download_pdf(pdf_url, target)

        return success_response(
            {
                "arxiv_id": arxiv_id,
                "pdf_url": pdf_url,
                "saved_path": str(target),
                "size_bytes": target.stat().st_size,
            }
        )

    except Exception as e:
        logger.error(f"pdf_download error: {e}")
        return error_response("PDF_DOWNLOAD_ERROR", str(e))


def tool_pdf_extract_text(
    pdf_path: str,
    max_pages: Optional[int] = None,
    ocr: bool = False,
    use_pdfminer_fallback: bool = True,
) -> Dict:
    """Extract plain text from a PDF file."""
    try:
        from pathlib import Path
        from pdf.extract import extract_pdf_text_hybrid

        path = Path(pdf_path)
        if not path.exists():
            return error_response("FILE_NOT_FOUND", f"PDF not found: {pdf_path}")

        text = extract_pdf_text_hybrid(
            path, max_pages=max_pages, ocr=ocr, use_pdfminer_fallback=use_pdfminer_fallback
        )

        return success_response(
            {
                "pdf_path": pdf_path,
                "text": text,
                "char_count": len(text),
                "pages_extracted": max_pages or "all",
            }
        )

    except Exception as e:
        logger.error(f"pdf_extract_text error: {e}")
        return error_response("PDF_EXTRACT_ERROR", str(e))


def tool_pdf_extract_structured(pdf_path: str, max_pages: Optional[int] = None) -> Dict:
    """Extract structured content from a PDF: text blocks, tables, math."""
    try:
        from pathlib import Path
        from pdf.extract import extract_pdf_structured

        path = Path(pdf_path)
        if not path.exists():
            return error_response("FILE_NOT_FOUND", f"PDF not found: {pdf_path}")

        content = extract_pdf_structured(path, max_pages=max_pages)

        return success_response(
            {
                "pdf_path": pdf_path,
                "blocks": [
                    {
                        "type": b.type.value if hasattr(b.type, "value") else str(b.type),
                        "text": b.text,
                        "page": b.page,
                    }
                    for b in content.blocks  # type: ignore[attr-defined]
                ],
                "tables": [
                    {
                        "headers": t.headers,  # type: ignore[attr-defined]
                        "rows": t.rows,  # type: ignore[attr-defined]
                        "page": t.page,
                    }
                    for t in content.tables  # type: ignore[attr-defined]
                ],
                "math_count": len(content.math_blocks),
                "pages_extracted": max_pages or "all",
            }
        )

    except Exception as e:
        logger.error(f"pdf_extract_structured error: {e}")
        return error_response("PDF_EXTRACT_ERROR", str(e))






def tool_chart_query(paper_id: str, action: str, label: Optional[str] = None) -> Dict:
    """Query figures and tables."""
    try:
        from kg.manager import KGManager
        from pdf.chart_kg import ChartKGExtractor

        kg = KGManager()
        extractor = ChartKGExtractor(kg)

        if action == "list":
            figures = extractor.get_paper_figures(paper_id)
            tables = extractor.get_paper_tables(paper_id)
            return success_response(
                {
                    "paper_id": paper_id,
                    "figures": [
                        {
                            "label": f["label"],
                            "page": f.get("properties", {}).get("page", 0) + 1,
                            "description": f.get("properties", {}).get("description", ""),
                        }
                        for f in figures
                    ],
                    "tables": [
                        {
                            "label": t["label"],
                            "page": t.get("properties", {}).get("page", 0) + 1,
                            "description": t.get("properties", {}).get("description", ""),
                        }
                        for t in tables
                    ],
                }
            )

        elif action == "figure" and label:
            fig = extractor.query_figure(paper_id, label)
            if not fig:
                return error_response("NOT_FOUND", f"Figure not found: {label}")
            props = fig.get("properties", {})
            return success_response(
                {
                    "paper_id": paper_id,
                    "type": "figure",
                    "label": fig["label"],
                    "page": props.get("page", 0) + 1,
                    "caption": props.get("caption", ""),
                    "description": props.get("description", ""),
                    "image_path": props.get("image_path", ""),
                }
            )

        elif action == "table" and label:
            tables = extractor.get_paper_tables(paper_id)
            tbl = None
            for t in tables:
                if label.lower() in t["label"].lower():
                    tbl = t
                    break
            if not tbl:
                return error_response("NOT_FOUND", f"Table not found: {label}")
            props = tbl.get("properties", {})
            return success_response(
                {
                    "paper_id": paper_id,
                    "type": "table",
                    "label": tbl["label"],
                    "page": props.get("page", 0) + 1,
                    "caption": props.get("caption", ""),
                    "description": props.get("description", ""),
                    "markdown": props.get("markdown", ""),
                }
            )

        else:
            return error_response("INVALID_ACTION", f"Unknown action: {action}")

    except Exception as e:
        logger.error(f"chart_query error: {e}")
        return error_response("CHART_ERROR", str(e))


def tool_hypothesis_list() -> Dict:
    """List all tracked hypotheses with verdict status."""
    try:
        from llm.insight.evolution import EvolutionTracker
        from llm.experiment_tracker import ExperimentTracker

        ev = EvolutionTracker()
        tracker = ExperimentTracker()

        events = ev.get_recent_events(limit=10000)
        hypothesis_ids = set()
        for e in events:
            if e.hypothesis_id:
                hypothesis_ids.add(e.hypothesis_id)

        experiments = tracker.list_experiments()
        exp_by_hid: Dict[str, List] = {}
        for e in experiments:
            if e.hypothesis_id:
                exp_by_hid.setdefault(e.hypothesis_id, []).append(e)

        rows = []
        for hid in sorted(hypothesis_ids):
            evts = ev.get_hypothesis_events(hid)
            verdict, detail = _compute_verdict(evts)
            linked = exp_by_hid.get(hid, [])
            rows.append(
                {
                    "hypothesis_id": hid,
                    "verdict": verdict,
                    "detail": detail,
                    "linked_experiments": len(linked),
                    "experiments": [
                        {"id": e.id, "name": e.name, "status": e.status} for e in linked
                    ],
                }
            )

        return success_response({"total": len(rows), "hypotheses": rows})

    except Exception as e:
        logger.error(f"hypothesis_list error: {e}")
        return error_response("HYPOTHESIS_ERROR", str(e))


def _compute_verdict(events):
    """Compute verdict from hypothesis events."""
    if not events:
        return "INCONCLUSIVE", "no experiments recorded"
    action_vals = {e.action.value if hasattr(e.action, "value") else str(e.action) for e in events}
    has_completed = "validated" in action_vals
    has_failed = "rejected" in action_vals
    if has_completed and has_failed:
        return "MIXED", "both validated and rejected experiments exist"
    if has_completed:
        return "VALIDATED", "all experiments succeeded"
    if has_failed:
        return "REJECTED", "all experiments failed"
    return "INCONCLUSIVE", "no completed experiments yet"


def handle_initialize() -> dict:
    """Handle initialize request."""
    return success_response(
        {
            "protocolVersion": MCP_VERSION,
            "serverInfo": {"name": "rairos", "version": "1.5.4"},
            "capabilities": {"tools": True},
        }
    )


def handle_list_tools() -> dict:
    """Handle list_tools request — merge Python + Rust tools."""
    import json

    # Get Rust tool definitions
    rust_json = ""
    try:
        from rairos_mcp_py import list_tools_detailed_rs

        rust_json = list_tools_detailed_rs()
    except Exception:
        pass

    # Get Python tool definitions
    py_tools = get_tools()
    py_names = {t["name"] for t in py_tools}

    # Merge: Python tools first, then Rust tools not already in Python
    if rust_json:
        try:
            rust_tools = json.loads(rust_json)
            for rt in rust_tools:
                if rt["name"] not in py_names:
                    py_tools.append(rt)
        except Exception:
            pass

    return success_response({"tools": py_tools})


def handle_call_tool(name: str, arguments: Dict[str, Any]) -> dict:  # type: ignore[arg-type]
    """Handle call_tool request with schema validation."""
    try:
        # ── Schema validation ────────────────────────────────────────────────
        from mcp.tools_defs import get_tools

        tools = {t["name"]: t for t in get_tools()}
        tool_def = tools.get(name)
        if tool_def:
            schema = tool_def.get("inputSchema", {})
            # Check required fields
            required = schema.get("required", [])
            for field in required:
                val = arguments.get(field)
                if val is None or (isinstance(val, str) and not val.strip()):
                    return {
                        "content": [
                            {
                                "type": "text",
                                "text": f"Missing or empty required field: '{field}'",
                            }
                        ],
                        "isError": True,
                    }
            # Type validation for known fields
            props = schema.get("properties", {})
            for field, val in arguments.items():
                if field not in props or val is None:
                    continue
                expected = props[field].get("type", "string")
                actual = type(val).__name__
                # Coerce common mismatches
                if expected == "integer" and isinstance(val, str):
                    try:
                        arguments[field] = int(val)
                    except (ValueError, TypeError):
                        return {
                            "content": [
                                {
                                    "type": "text",
                                    "text": f"Field '{field}' must be integer, got: {actual}",
                                }
                            ],
                            "isError": True,
                        }
                elif expected == "number" and isinstance(val, str):
                    try:
                        arguments[field] = float(val)
                    except (ValueError, TypeError):
                        return {
                            "content": [
                                {
                                    "type": "text",
                                    "text": f"Field '{field}' must be number, got: {actual}",
                                }
                            ],
                            "isError": True,
                        }
                elif expected == "boolean" and not isinstance(val, bool):
                    if isinstance(val, str):
                        arguments[field] = val.lower() in ("true", "1", "yes")
                    elif isinstance(val, (int, float)):
                        arguments[field] = bool(val)
                elif expected == "array" and not isinstance(val, (list, tuple)):
                    return {
                        "content": [
                            {
                                "type": "text",
                                "text": f"Field '{field}' must be array, got: {actual}",
                            }
                        ],
                        "isError": True,
                    }

        # ── Try Rust MCP server first (faster, no dynamic import) ────────────
        try:
            import json as _json
            from rairos_mcp_py import call_tool_rs

            _result_json = call_tool_rs(name, _json.dumps(arguments))
            if _result_json is not None:
                _parsed = _json.loads(_result_json)
                # Rust MCP returns {"content": [{"type": "text", "text": "..."}]}
                _content = _parsed.get("content", [])
                if _content and isinstance(_content, list):
                    _text = _content[0].get("text", "{}")
                    return success_response(_json.loads(_text))
                return success_response(_parsed)
        except Exception:
            pass

        if name == "chart_query":
            result = tool_chart_query(  # type: ignore[arg-type]
                paper_id=arguments.get("paper_id"),
                action=arguments.get("action"),
                label=arguments.get("label"),
            )
        else:
            result = error_response("UNKNOWN_TOOL", f"Unknown tool: {name}")

        return result

    except Exception as e:
        logger.error(f"call_tool {name} error: {e}")
        return error_response("TOOL_ERROR", str(e))


def handle_request(method: str, params: Dict) -> dict:
    """Route MCP request to handler."""
    if method == "initialize":
        return handle_initialize()
    elif method == "tools/list":
        return handle_list_tools()
    elif method == "tools/call":
        return handle_call_tool(name=params.get("name"), arguments=params.get("arguments", {}))  # type: ignore[arg-type]
    elif method == "sampling/createMessage":
        # MCP sampling: server requests LLM generation from client
        return handle_sampling(params)
    else:
        return error_response("UNKNOWN_METHOD", f"Unknown method: {method}")


def handle_sampling(params: Dict) -> dict:
    """Handle MCP sampling/createMessage via protocol (no API key needed)."""
    messages = params.get("messages", [])
    if not messages:
        return error_response("INVALID_PARAMS", "No messages provided")
    system_prompt = params.get("systemPrompt", "")
    try:
        from llm.client import call_llm_chat_completions

        result = call_llm_chat_completions(
            messages=messages,
            model="minimax-m2.7-highspeed",
            system_prompt=system_prompt,
            timeout=60,
        )
        content = result if isinstance(result, str) else str(result)
        return success_response({"model": "mcp", "role": "assistant", "content": content})
    except Exception as e:
        logger.warning(f"MCP degraded (no LLM key): {e}")
        last = messages[-1].get("content", "") if messages else ""
        return success_response(
            {
                "model": "mcp-degraded",
                "role": "assistant",
                "content": f"[MCP degraded - no API key needed] Query: {last[:200]}",
            }
        )


def main():
    """Run as stdio MCP server."""
    while True:
        try:
            line = sys.stdin.readline()
            if not line:
                break

            request = json.loads(line.strip())
            method = request.get("method", "")
            params = request.get("params", {})
            req_id = request.get("id")

            response = handle_request(method, params)

            if req_id is not None:
                response["id"] = req_id

            print(json.dumps(response))
            sys.stdout.flush()

        except json.JSONDecodeError as e:
            error_resp = error_response("PARSE_ERROR", f"Invalid JSON: {e}")
            print(json.dumps(error_resp))
            sys.stdout.flush()
        except Exception as e:
            logger.error(f"Server error: {e}")
            error_resp = error_response("SERVER_ERROR", str(e))
            print(json.dumps(error_resp))
            sys.stdout.flush()


if __name__ == "__main__":
    main()
