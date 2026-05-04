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
import os
from pathlib import Path

# Add project root to path
PROJECT_ROOT = Path(__file__).parent.parent.parent.parent
sys.path.insert(0, str(PROJECT_ROOT))

import json
import logging
from typing import Any, Dict, List, Optional
from dataclasses import asdict

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


# ─── Tool Definitions ────────────────────────────────────────────────


def get_tools() -> List[Dict]:
    """Return list of available tools."""
    return [
        {
            "name": "paper_ingest",
            "description": "Import a paper from arXiv ID, DOI, or PDF file into Rairos",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "identifier": {
                        "type": "string",
                        "description": "arXiv ID (e.g. '2601.00155'), DOI, or path to PDF file",
                    },
                    "tags": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Topic tags for the paper",
                    },
                },
                "required": ["identifier"],
            },
        },
        {
            "name": "paper_search",
            "description": "Search papers by keyword (local DB or web)",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {"type": "string", "description": "Search query"},
                    "tag": {"type": "string", "description": "Filter by tag (local only)"},
                    "limit": {
                        "type": "integer",
                        "default": 10,
                        "description": "Max results per source",
                    },
                    "source": {
                        "type": "string",
                        "enum": ["local", "web", "both"],
                        "default": "local",
                        "description": "Search source: local DB, web (arXiv+SemanticScholar), or both",
                    },
                },
                "required": ["query"],
            },
        },
        {
            "name": "paper_chat",
            "description": "Ask a question about papers using RAG",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "question": {"type": "string", "description": "Question to ask"},
                    "paper_id": {"type": "string", "description": "Specific paper ID (optional)"},
                },
                "required": ["question"],
            },
        },
        {
            "name": "kg_query",
            "description": "Query the knowledge graph",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Query type: 'stats', 'papers', 'tags', 'neighbors'",
                    },
                    "entity_id": {
                        "type": "string",
                        "description": "Entity ID for neighbor queries",
                    },
                    "tag": {"type": "string", "description": "Tag to filter papers"},
                },
                "required": ["query"],
            },
        },
        {
            "name": "chart_query",
            "description": "Query figures and tables from a paper's knowledge graph",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "paper_id": {"type": "string", "description": "Paper ID"},
                    "action": {
                        "type": "string",
                        "enum": ["list", "figure", "table"],
                        "description": "Action: list all, query figure, or query table",
                    },
                    "label": {
                        "type": "string",
                        "description": "Figure/Table label like 'Figure 3'",
                    },
                },
                "required": ["paper_id", "action"],
            },
        },
        {
            "name": "research_run",
            "description": "Run autonomous research loop on a topic",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "topic": {"type": "string", "description": "Research topic"},
                    "limit": {
                        "type": "integer",
                        "default": 5,
                        "description": "Max papers to process",
                    },
                },
                "required": ["topic"],
            },
        },
        {
            "name": "slides_generate",
            "description": "Generate slides from a paper",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "paper_id": {"type": "string", "description": "Paper ID"},
                    "output_path": {"type": "string", "description": "Output PPTX file path"},
                },
                "required": ["paper_id"],
            },
        },
        {
            "name": "cite_fetch",
            "description": "Fetch citation data for a paper",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "paper_id": {"type": "string", "description": "Paper ID"},
                    "direction": {
                        "type": "string",
                        "enum": ["cited", "citing", "both"],
                        "default": "both",
                    },
                },
                "required": ["paper_id"],
            },
        },
        {
            "name": "paper_analyze",
            "description": "Get analysis of a paper including summary, novelty, evidence scores",
            "inputSchema": {
                "type": "object",
                "properties": {"paper_id": {"type": "string", "description": "Paper ID"}},
                "required": ["paper_id"],
            },
        },
        {
            "name": "citation_graph",
            "description": "Get citation graph data for a paper (forward + backward citations)",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "paper_id": {"type": "string", "description": "Paper ID (arXiv ID)"},
                    "depth": {
                        "type": "integer",
                        "default": 2,
                        "description": "Depth: 1=direct, 2=2-hop",
                    },
                    "max_nodes": {
                        "type": "integer",
                        "default": 100,
                        "description": "Max nodes per direction",
                    },
                },
                "required": ["paper_id"],
            },
        },
        {
            "name": "gap_detect",
            "description": "Detect research gaps in a topic using your paper corpus",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "topic": {
                        "type": "string",
                        "description": "Research topic or keyword to analyze",
                    },
                    "use_llm": {
                        "type": "boolean",
                        "default": true,
                        "description": "Use LLM for deep analysis",
                    },
                },
                "required": ["topic"],
            },
        },
    ]


# ─── Tool Implementations ────────────────────────────────────────────


def _ensure_data_dir():
    """Ensure data directory exists."""
    data_dir = PROJECT_ROOT / "data"
    data_dir.mkdir(exist_ok=True)
    return data_dir


def tool_paper_ingest(identifier: str, tags: Optional[List[str]] = None) -> Dict:
    """Ingest a paper."""
    try:
        from parsers.arxiv import arxiv_id_from_input
        from db.database import Database
        from core import Paper

        db = Database()
        db.init()

        # Parse identifier
        arxiv_id = arxiv_id_from_input(identifier)
        if not arxiv_id:
            return error_response("INVALID_INPUT", f"Could not parse identifier: {identifier}")

        # Check if already exists
        existing = db.get_paper(arxiv_id)
        if existing:
            return success_response(
                {
                    "paper_id": arxiv_id,
                    "status": "already_exists",
                    "title": existing.title if hasattr(existing, "title") else arxiv_id,
                }
            )

        # Fetch metadata
        from parsers.arxiv import fetch_arxiv_paper

        paper_data = fetch_arxiv_paper(arxiv_id)
        if not paper_data:
            return error_response("FETCH_FAILED", f"Could not fetch paper: {arxiv_id}")

        paper = Paper(**paper_data)

        # Save to DB
        db.upsert_paper(
            paper_id=arxiv_id,
            source="arxiv",
            title=paper.title,
            abstract=paper.abstract,
            authors=paper.authors,
            published=paper.published,
            pdf_url=paper.pdf_url,
        )

        if tags:
            db.upsert_tags(arxiv_id, tags)

        db.close()

        return success_response({"paper_id": arxiv_id, "status": "imported", "title": paper.title})

    except Exception as e:
        logger.error(f"paper_ingest error: {e}")
        return error_response("INGEST_ERROR", str(e))


def tool_paper_search(
    query: str, tag: Optional[str] = None, limit: int = 10, source: str = "local"
) -> Dict:
    """Search papers from local DB and/or web sources."""
    try:
        results = []
        sources_used = []

        if source in ("local", "both"):
            from db.database import Database

            db = Database()
            db.init()
            local_results, total = db.search_papers(query, limit=limit)
            db.close()
            results.extend(
                [
                    {
                        "paper_id": r.paper_id,
                        "title": r.title,
                        "authors": r.authors,
                        "published": r.published,
                        "source": "local",
                    }
                    for r in local_results[:limit]
                ]
            )
            sources_used.append(f"local({total})")

        if source in ("web", "both"):
            try:
                from parsers.cross_search import search_papers_multi

                web_papers = search_papers_multi(query, max_per_source=limit)
                results.extend(
                    [
                        {
                            "paper_id": p.uid,
                            "title": p.title,
                            "authors": p.authors,
                            "published": p.published[:10] if p.published else "",
                            "source": getattr(p, "source", "web"),
                        }
                        for p in web_papers
                    ]
                )
                sources_used.append(f"web({len(web_papers)})")
            except Exception as e:
                logger.warning(f"Web search failed: {e}")

        return success_response(
            {
                "query": query,
                "sources": sources_used,
                "count": len(results),
                "results": results[: limit * 2] if source == "both" else results[:limit],
            }
        )

    except Exception as e:
        logger.error(f"paper_search error: {e}")
        return error_response("SEARCH_ERROR", str(e))


def tool_paper_chat(question: str, paper_id: Optional[str] = None) -> Dict:
    """Chat about papers using RAG."""
    try:
        from llm.research_chat import research_chat

        answer = research_chat(question, paper_id=paper_id)

        return success_response({"question": question, "answer": answer, "paper_id": paper_id})

    except Exception as e:
        logger.error(f"paper_chat error: {e}")
        return error_response("CHAT_ERROR", str(e))


def tool_kg_query(query: str, entity_id: Optional[str] = None, tag: Optional[str] = None) -> Dict:
    """Query knowledge graph."""
    try:
        from kg.manager import KGManager

        kg = KGManager()

        if query == "stats":
            stats = kg.stats()
            return success_response(stats)

        elif query == "papers":
            nodes = kg.get_all_nodes("Paper")
            return success_response(
                {
                    "count": len(nodes),
                    "papers": [{"id": n["entity_id"], "label": n["label"]} for n in nodes[:50]],
                }
            )

        elif query == "tags":
            nodes = kg.get_all_nodes("Tag")
            return success_response({"count": len(nodes), "tags": [n["label"] for n in nodes]})

        elif query == "neighbors" and entity_id:
            parts = entity_id.split(":", 1)
            if len(parts) != 2:
                return error_response("INVALID_ENTITY", "entity_id must be 'type:id' format")
            node_type, entity = parts
            node = kg.get_node_by_entity(node_type, entity)
            if not node:
                return error_response("NOT_FOUND", f"Entity not found: {entity_id}")
            neighbors = kg.find_neighbors(node["id"], depth=2)
            return success_response(
                {
                    "entity": entity_id,
                    "neighbors": [
                        {"node": n[0], "edge": n[1], "depth": n[2]} for n in neighbors[:20]
                    ],
                }
            )

        elif query == "papers" and tag:
            nodes = kg.find_papers_by_tag(tag)
            return success_response(
                {
                    "tag": tag,
                    "count": len(nodes),
                    "papers": [{"id": n["entity_id"], "label": n["label"]} for n in nodes],
                }
            )

        else:
            return error_response("INVALID_QUERY", f"Unknown query: {query}")

    except Exception as e:
        logger.error(f"kg_query error: {e}")
        return error_response("KG_ERROR", str(e))


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


def tool_research_run(topic: str, limit: int = 5) -> Dict:
    """Run research loop."""
    try:
        from research_loop.core import ResearchLoop

        loop = ResearchLoop()
        results = loop.run(topic=topic, limit=limit)

        return success_response(
            {"topic": topic, "papers_found": len(results.get("papers", [])), "status": "completed"}
        )

    except Exception as e:
        logger.error(f"research_run error: {e}")
        return error_response("RESEARCH_ERROR", str(e))


def tool_slides_generate(paper_id: str, output_path: Optional[str] = None) -> Dict:
    """Generate slides."""
    try:
        from llm.slides import generate_slides

        if not output_path:
            output_path = str(PROJECT_ROOT / f"{paper_id}_slides.pptx")

        generate_slides(paper_id, output_path)

        return success_response({"paper_id": paper_id, "output_path": output_path})

    except Exception as e:
        logger.error(f"slides_generate error: {e}")
        return error_response("SLIDES_ERROR", str(e))


def tool_cite_fetch(paper_id: str, direction: str = "both") -> Dict:
    """Fetch citations."""
    try:
        from llm.citation_chain import CitationChainBuilder

        builder = CitationChainBuilder()
        result = builder.fetch_citations(paper_id, direction=direction)

        return success_response(
            {
                "paper_id": paper_id,
                "cited": result.get("cited", []),
                "citing": result.get("citing", []),
                "count": len(result.get("cited", [])) + len(result.get("citing", [])),
            }
        )

    except Exception as e:
        logger.error(f"cite_fetch error: {e}")
        return error_response("CITE_ERROR", str(e))


def tool_paper_analyze(paper_id: str) -> Dict:
    """Analyze paper."""
    try:
        from llm.paper_analyzer import PaperAnalyzer, PaperAnalysisResult
        from db.database import Database

        db = Database()
        db.init()
        rec = db.get_paper(paper_id)
        db.close()

        if not rec:
            return error_response("NOT_FOUND", f"Paper not found: {paper_id}")

        analyzer = PaperAnalyzer()
        result = analyzer.analyze(
            paper_id=paper_id,
            title=rec.title or "",
            abstract=rec.abstract or "",
            body_text="",
            authors=rec.authors,
        )

        if isinstance(result, PaperAnalysisResult):
            return success_response(
                {
                    "paper_id": result.paper_id,
                    "sections": result.sections,
                    "rubric": result.rubric,
                    "extracted_methods": result.extracted_methods,
                    "extracted_datasets": result.extracted_datasets,
                    "extracted_metrics": result.extracted_metrics,
                    "llm_used": result.llm_used,
                }
            )
        return success_response(result)

    except Exception as e:
        logger.error(f"paper_analyze error: {e}")
        return error_response("ANALYZE_ERROR", str(e))


def tool_citation_graph(paper_id: str, depth: int = 2, max_nodes: int = 30) -> Dict:
    """Get citation graph for a paper using Semantic Scholar API."""
    try:
        from parsers.semantic_scholar import get_paper_by_id, get_citations

        root = get_paper_by_id(paper_id)
        if not root:
            return error_response("NOT_FOUND", f"Paper not found: {paper_id}")

        nodes = []
        links = []
        node_ids = set()

        def add_node(pid: str, label: str, is_root: bool = False):
            nid = f"s2:{pid}"
            if nid in node_ids:
                return
            node_ids.add(nid)
            nodes.append(
                {
                    "id": nid,
                    "entity_id": pid,
                    "label": label[:60] if label else pid,
                    "type": "Paper",
                    "is_root": is_root,
                }
            )

        add_node(root.paper_id, root.title, is_root=True)

        citing = get_citations(root.paper_id, limit=max_nodes)
        for p in citing[:max_nodes]:
            add_node(p.paper_id, p.title)
            links.append(
                {
                    "source": f"s2:{p.paper_id}",
                    "target": f"s2:{root.paper_id}",
                    "relation": "cited_by",
                }
            )

        return success_response(
            {
                "paper_id": paper_id,
                "root": f"s2:{root.paper_id}",
                "nodes": nodes,
                "links": links,
                "count": len(nodes),
            }
        )

    except Exception as e:
        logger.error(f"citation_graph error: {e}")
        return error_response("GRAPH_ERROR", str(e))


def tool_gap_detect(topic: str, use_llm: bool = True) -> Dict:
    """Detect research gaps in a topic using the paper corpus."""
    try:
        from llm.gap_detector import GapDetector
        from db.database import Database

        db = Database()
        db.init()

        detector = GapDetector(db=db)
        result = detector.analyze(topic=topic, use_llm=use_llm)

        db.close()

        return success_response(
            {
                "topic": topic,
                "analyzed_papers_count": result.analyzed_papers_count,
                "coverage_score": result.coverage_score,
                "gaps": [
                    {
                        "type": g.gap_type,
                        "description": g.description,
                        "evidence": g.evidence,
                        "confidence": g.confidence,
                        "severity": g.severity,
                    }
                    for g in result.gaps
                ],
                "questions": [
                    {
                        "question": q.question,
                        "gap_type": q.gap_type,
                        "verifiability": q.verifiability,
                    }
                    for q in result.questions
                ],
            }
        )

    except Exception as e:
        logger.error(f"gap_detect error: {e}")
        return error_response("GAP_ERROR", str(e))


# ─── MCP Protocol Handlers ──────────────────────────────────────────


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
    """Handle list_tools request."""
    return success_response({"tools": get_tools()})


def handle_call_tool(name: str, arguments: Dict) -> dict:
    """Handle call_tool request."""
    try:
        if name == "paper_ingest":
            result = tool_paper_ingest(
                identifier=arguments.get("identifier"), tags=arguments.get("tags")
            )
        elif name == "paper_search":
            result = tool_paper_search(
                query=arguments.get("query"),
                tag=arguments.get("tag"),
                limit=arguments.get("limit", 10),
                source=arguments.get("source", "local"),
            )
        elif name == "paper_chat":
            result = tool_paper_chat(
                question=arguments.get("question"), paper_id=arguments.get("paper_id")
            )
        elif name == "kg_query":
            result = tool_kg_query(
                query=arguments.get("query"),
                entity_id=arguments.get("entity_id"),
                tag=arguments.get("tag"),
            )
        elif name == "chart_query":
            result = tool_chart_query(
                paper_id=arguments.get("paper_id"),
                action=arguments.get("action"),
                label=arguments.get("label"),
            )
        elif name == "research_run":
            result = tool_research_run(
                topic=arguments.get("topic"), limit=arguments.get("limit", 5)
            )
        elif name == "slides_generate":
            result = tool_slides_generate(
                paper_id=arguments.get("paper_id"), output_path=arguments.get("output_path")
            )
        elif name == "cite_fetch":
            result = tool_cite_fetch(
                paper_id=arguments.get("paper_id"), direction=arguments.get("direction", "both")
            )
        elif name == "paper_analyze":
            result = tool_paper_analyze(paper_id=arguments.get("paper_id"))
        elif name == "citation_graph":
            result = tool_citation_graph(
                paper_id=arguments.get("paper_id"),
                depth=arguments.get("depth", 2),
                max_nodes=arguments.get("max_nodes", 100),
            )
        elif name == "gap_detect":
            result = tool_gap_detect(
                topic=arguments.get("topic"), use_llm=arguments.get("use_llm", True)
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
        return handle_call_tool(name=params.get("name"), arguments=params.get("arguments", {}))
    else:
        return error_response("UNKNOWN_METHOD", f"Unknown method: {method}")


# ─── Main Entry Point ────────────────────────────────────────────────


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
