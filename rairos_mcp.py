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

import datetime
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
                        "description": "arXiv ID (e.g. '2601.00155'), DOI, or path to PDF file"
                    },
                    "tags": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Topic tags for the paper"
                    }
                },
                "required": ["identifier"]
            }
        },
        {
            "name": "paper_search",
            "description": "Search papers by keyword (local DB or web)",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {"type": "string", "description": "Search query"},
                    "tag": {"type": "string", "description": "Filter by tag (local only)"},
                    "limit": {"type": "integer", "default": 10, "description": "Max results per source"},
                    "source": {"type": "string", "enum": ["local", "web", "both"], "default": "local", "description": "Search source: local DB, web (arXiv+SemanticScholar), or both"}
                },
                "required": ["query"]
            }
        },
        {
            "name": "paper_chat",
            "description": "Ask a question about papers using RAG",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "question": {"type": "string", "description": "Question to ask"},
                    "paper_id": {"type": "string", "description": "Specific paper ID (optional)"}
                },
                "required": ["question"]
            }
        },
        {
            "name": "kg_query",
            "description": "Query the knowledge graph",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {"type": "string", "description": "Query type: 'stats', 'papers', 'tags', 'neighbors'"},
                    "entity_id": {"type": "string", "description": "Entity ID for neighbor queries"},
                    "tag": {"type": "string", "description": "Tag to filter papers"}
                },
                "required": ["query"]
            }
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
                        "description": "Action: list all, query figure, or query table"
                    },
                    "label": {"type": "string", "description": "Figure/Table label like 'Figure 3'"}
                },
                "required": ["paper_id", "action"]
            }
        },
        {
            "name": "research_run",
            "description": "Run autonomous research loop on a topic",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "topic": {"type": "string", "description": "Research topic"},
                    "limit": {"type": "integer", "default": 5, "description": "Max papers to process"}
                },
                "required": ["topic"]
            }
        },
        {
            "name": "slides_generate",
            "description": "Generate slides from a paper",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "paper_id": {"type": "string", "description": "Paper ID"},
                    "output_path": {"type": "string", "description": "Output PPTX file path"}
                },
                "required": ["paper_id"]
            }
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
                        "default": "both"
                    }
                },
                "required": ["paper_id"]
            }
        },
        {
            "name": "paper_analyze",
            "description": "Get analysis of a paper including summary, novelty, evidence scores",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "paper_id": {"type": "string", "description": "Paper ID"}
                },
                "required": ["paper_id"]
            }
        },
        {
            "name": "citation_graph",
            "description": "Get citation graph data for a paper (forward + backward citations)",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "paper_id": {"type": "string", "description": "Paper ID (arXiv ID)"},
                    "depth": {"type": "integer", "default": 2, "description": "Depth: 1=direct, 2=2-hop"},
                    "max_nodes": {"type": "integer", "default": 100, "description": "Max nodes per direction"}
                },
                "required": ["paper_id"]
            }
        },
        {
            "name": "gap_detect",
            "description": "Detect research gaps in a topic using your paper corpus",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "topic": {"type": "string", "description": "Research topic or keyword to analyze"},
                    "use_llm": {"type": "boolean", "default": true, "description": "Use LLM for deep analysis"}
                },
                "required": ["topic"]
            }
        },
        {
            "name": "research_agent_start",
            "description": "Start the autonomous research agent in background watch mode",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "interval_minutes": {"type": "integer", "default": 30, "description": "Check interval in minutes"}
                }
            }
        },
        {
            "name": "research_agent_stop",
            "description": "Stop the autonomous research agent watch loop"
        },
        {
            "name": "research_agent_status",
            "description": "Get status of the autonomous research agent (running state, alerts, last check)"
        },
        {
            "name": "research_agent_trigger",
            "description": "Manually trigger one cycle of the autonomous research agent",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "topic": {"type": "string", "description": "Specific topic to analyze (optional, analyzes all subscriptions if omitted)"}
                }
            }
        },
        {
            "name": "hypothesis_generate",
            "description": "Generate testable research hypotheses from a gap + topic",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "topic": {"type": "string", "description": "Research topic"},
                    "gap_context": {"type": "string", "description": "Gap description from gap detection"},
                    "gap_type": {"type": "string", "description": "Gap type: method_limitation, unexplored_application, contradiction, evaluation_gap, scalability_issue"},
                    "creative": {"type": "boolean", "default": false, "description": "Include creative cross-domain hypotheses"}
                },
                "required": ["topic"]
            }
        },
        {
            "name": "hypothesis_list",
            "description": "List all tracked hypotheses with their verdict status"
        },
        {
            "name": "experiment_record",
            "description": "Record an experiment result for a hypothesis",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "hypothesis_id": {"type": "string", "description": "Hypothesis ID to link the experiment to"},
                    "name": {"type": "string", "description": "Experiment name"},
                    "result": {"type": "string", "description": "Experiment result: validated, rejected, failed"},
                    "metrics": {"type": "object", "description": "Key metrics as key-value pairs, e.g. {\"accuracy\": 0.92}"}
                },
                "required": ["hypothesis_id", "name", "result"]
            }
        },
        {
            "name": "litreview_generate",
            "description": "Generate a narrative literature review for a research topic",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "topic": {"type": "string", "description": "Research topic to review"},
                    "limit": {"type": "integer", "default": 30, "description": "Max papers to include (default 30)"},
                    "use_llm": {"type": "boolean", "default": true, "description": "Use LLM for generation (default true)"}
                },
                "required": ["topic"]
            }
        },
        {
            "name": "litreview_list",
            "description": "List all saved literature reviews"
        },
        {
            "name": "research_memory_add_stance",
            "description": "Record a research stance (supported/rejected/deferred) on a claim",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "topic": {"type": "string", "description": "Research topic or question"},
                    "claim": {"type": "string", "description": "The specific claim or hypothesis you took a stance on"},
                    "stance": {"type": "string", "description": "Your stance: supported, rejected, deferred, qualified"},
                    "evidence_refs": {"type": "array", "items": {"type": "string"}, "description": "arXiv IDs that support this stance"},
                    "reasoning": {"type": "string", "description": "Why you hold this stance"},
                    "confidence": {"type": "number", "description": "Confidence 0.0–1.0"}
                },
                "required": ["topic", "claim", "stance"]
            }
        },
        {
            "name": "research_memory_list_stances",
            "description": "List all research stances in your memory"
        },
        {
            "name": "research_memory_check_paper",
            "description": "Check a paper against your research memory for anomalies",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "arxiv_id": {"type": "string", "description": "arXiv ID of the paper to check"},
                    "use_llm": {"type": "boolean", "default": true, "description": "Use LLM for deep anomaly detection"}
                },
                "required": ["arxiv_id"]
            }
        },
        {
            "name": "research_memory_anomalies",
            "description": "List recent anomaly alerts — papers that contradict your stances"
        },
        {
            "name": "review_simulate",
            "description": "Simulate adversarial peer reviewers on a paper",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "arxiv_id": {"type": "string", "description": "arXiv ID of paper to review"},
                    "persona": {"type": "string", "description": "Reviewer persona: methodology, contributions, clarity, ethics, or all (default: all)"},
                    "use_llm": {"type": "boolean", "default": true}
                },
                "required": ["arxiv_id"]
            }
        },
        {
            "name": "review_list",
            "description": "List saved simulated reviews"
        }
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
            return success_response({
                "paper_id": arxiv_id,
                "status": "already_exists",
                "title": existing.title if hasattr(existing, 'title') else arxiv_id
            })

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

        return success_response({
            "paper_id": arxiv_id,
            "status": "imported",
            "title": paper.title
        })

    except Exception as e:
        logger.error(f"paper_ingest error: {e}")
        return error_response("INGEST_ERROR", str(e))


def tool_paper_search(query: str, tag: Optional[str] = None, limit: int = 10, source: str = "local") -> Dict:
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
            results.extend([
                {
                    "paper_id": r.paper_id,
                    "title": r.title,
                    "authors": r.authors,
                    "published": r.published,
                    "source": "local"
                }
                for r in local_results[:limit]
            ])
            sources_used.append(f"local({total})")

        if source in ("web", "both"):
            try:
                from parsers.cross_search import search_papers_multi
                web_papers = search_papers_multi(query, max_per_source=limit)
                results.extend([
                    {
                        "paper_id": p.uid,
                        "title": p.title,
                        "authors": p.authors,
                        "published": p.published[:10] if p.published else "",
                        "source": getattr(p, "source", "web")
                    }
                    for p in web_papers
                ])
                sources_used.append(f"web({len(web_papers)})")
            except Exception as e:
                logger.warning(f"Web search failed: {e}")

        return success_response({
            "query": query,
            "sources": sources_used,
            "count": len(results),
            "results": results[:limit * 2] if source == "both" else results[:limit]
        })

    except Exception as e:
        logger.error(f"paper_search error: {e}")
        return error_response("SEARCH_ERROR", str(e))


def tool_paper_chat(question: str, paper_id: Optional[str] = None) -> Dict:
    """Chat about papers using RAG."""
    try:
        from llm.research_chat import research_chat

        answer = research_chat(question, paper_id=paper_id)

        return success_response({
            "question": question,
            "answer": answer,
            "paper_id": paper_id
        })

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
            return success_response({
                "count": len(nodes),
                "papers": [{"id": n["entity_id"], "label": n["label"]} for n in nodes[:50]]
            })

        elif query == "tags":
            nodes = kg.get_all_nodes("Tag")
            return success_response({
                "count": len(nodes),
                "tags": [n["label"] for n in nodes]
            })

        elif query == "neighbors" and entity_id:
            parts = entity_id.split(":", 1)
            if len(parts) != 2:
                return error_response("INVALID_ENTITY", "entity_id must be 'type:id' format")
            node_type, entity = parts
            node = kg.get_node_by_entity(node_type, entity)
            if not node:
                return error_response("NOT_FOUND", f"Entity not found: {entity_id}")
            neighbors = kg.find_neighbors(node["id"], depth=2)
            return success_response({
                "entity": entity_id,
                "neighbors": [
                    {"node": n[0], "edge": n[1], "depth": n[2]}
                    for n in neighbors[:20]
                ]
            })

        elif query == "papers" and tag:
            nodes = kg.find_papers_by_tag(tag)
            return success_response({
                "tag": tag,
                "count": len(nodes),
                "papers": [{"id": n["entity_id"], "label": n["label"]} for n in nodes]
            })

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
            return success_response({
                "paper_id": paper_id,
                "figures": [
                    {
                        "label": f["label"],
                        "page": f.get("properties", {}).get("page", 0) + 1,
                        "description": f.get("properties", {}).get("description", "")
                    }
                    for f in figures
                ],
                "tables": [
                    {
                        "label": t["label"],
                        "page": t.get("properties", {}).get("page", 0) + 1,
                        "description": t.get("properties", {}).get("description", "")
                    }
                    for t in tables
                ]
            })

        elif action == "figure" and label:
            fig = extractor.query_figure(paper_id, label)
            if not fig:
                return error_response("NOT_FOUND", f"Figure not found: {label}")
            props = fig.get("properties", {})
            return success_response({
                "paper_id": paper_id,
                "type": "figure",
                "label": fig["label"],
                "page": props.get("page", 0) + 1,
                "caption": props.get("caption", ""),
                "description": props.get("description", ""),
                "image_path": props.get("image_path", "")
            })

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
            return success_response({
                "paper_id": paper_id,
                "type": "table",
                "label": tbl["label"],
                "page": props.get("page", 0) + 1,
                "caption": props.get("caption", ""),
                "description": props.get("description", ""),
                "markdown": props.get("markdown", "")
            })

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

        return success_response({
            "topic": topic,
            "papers_found": len(results.get("papers", [])),
            "status": "completed"
        })

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

        return success_response({
            "paper_id": paper_id,
            "output_path": output_path
        })

    except Exception as e:
        logger.error(f"slides_generate error: {e}")
        return error_response("SLIDES_ERROR", str(e))


def tool_cite_fetch(paper_id: str, direction: str = "both") -> Dict:
    """Fetch citations."""
    try:
        from llm.citation_chain import CitationChainBuilder

        builder = CitationChainBuilder()
        result = builder.fetch_citations(paper_id, direction=direction)

        return success_response({
            "paper_id": paper_id,
            "cited": result.get("cited", []),
            "citing": result.get("citing", []),
            "count": len(result.get("cited", [])) + len(result.get("citing", []))
        })

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
            return success_response({
                "paper_id": result.paper_id,
                "sections": result.sections,
                "rubric": result.rubric,
                "extracted_methods": result.extracted_methods,
                "extracted_datasets": result.extracted_datasets,
                "extracted_metrics": result.extracted_metrics,
                "llm_used": result.llm_used,
            })
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
            nodes.append({
                "id": nid,
                "entity_id": pid,
                "label": label[:60] if label else pid,
                "type": "Paper",
                "is_root": is_root,
            })

        add_node(root.paper_id, root.title, is_root=True)

        citing = get_citations(root.paper_id, limit=max_nodes)
        for p in citing[:max_nodes]:
            add_node(p.paper_id, p.title)
            links.append({"source": f"s2:{p.paper_id}", "target": f"s2:{root.paper_id}", "relation": "cited_by"})

        return success_response({
            "paper_id": paper_id,
            "root": f"s2:{root.paper_id}",
            "nodes": nodes,
            "links": links,
            "count": len(nodes)
        })

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

        return success_response({
            "topic": topic,
            "analyzed_papers_count": result.analyzed_papers_count,
            "coverage_score": result.coverage_score,
            "gaps": [
                {
                    "type": g.gap_type,
                    "description": g.description,
                    "evidence": g.evidence,
                    "confidence": g.confidence,
                    "severity": g.severity
                }
                for g in result.gaps
            ],
            "questions": [
                {
                    "question": q.question,
                    "gap_type": q.gap_type,
                    "verifiability": q.verifiability
                }
                for q in result.questions
            ]
        })

    except Exception as e:
        logger.error(f"gap_detect error: {e}")
        return error_response("GAP_ERROR", str(e))


def tool_research_agent_start(interval_minutes: int = 30) -> Dict:
    """Start the autonomous research agent in background watch mode."""
    try:
        from research_loop.orchestrator import AutonomousOrchestrator
        orch = AutonomousOrchestrator(webhook_enabled=True)
        orch.start_watch(interval_minutes=interval_minutes)
        status = orch.get_status()
        return success_response({
            "status": "started",
            "interval_minutes": interval_minutes,
            "running": status["running"],
            "message": f"Autonomous research agent started. Will check subscriptions every {interval_minutes} minutes."
        })
    except Exception as e:
        logger.error(f"research_agent_start error: {e}")
        return error_response("AGENT_ERROR", str(e))


def tool_research_agent_stop() -> Dict:
    """Stop the autonomous research agent watch loop."""
    try:
        from research_loop.orchestrator import AutonomousOrchestrator
        orch = AutonomousOrchestrator()
        orch.stop_watch()
        return success_response({
            "status": "stopped",
            "message": "Autonomous research agent stopped."
        })
    except Exception as e:
        logger.error(f"research_agent_stop error: {e}")
        return error_response("AGENT_ERROR", str(e))


def tool_research_agent_status() -> Dict:
    """Get status of the autonomous research agent."""
    try:
        from research_loop.orchestrator import AutonomousOrchestrator
        orch = AutonomousOrchestrator()
        status = orch.get_status()
        recent_alerts = orch.get_recent_alerts(limit=10)
        return success_response({
            "status": status,
            "recent_alerts": [a.to_dict() for a in recent_alerts]
        })
    except Exception as e:
        logger.error(f"research_agent_status error: {e}")
        return error_response("AGENT_ERROR", str(e))


def tool_research_agent_trigger(topic: Optional[str] = None) -> Dict:
    """Manually trigger one cycle of the autonomous research agent."""
    try:
        from research_loop.orchestrator import AutonomousOrchestrator
        orch = AutonomousOrchestrator(webhook_enabled=True)
        alerts = orch.run_cycle()
        return success_response({
            "status": "cycle_complete",
            "alerts_generated": len(alerts),
            "alerts": [a.to_dict() for a in alerts]
        })
    except Exception as e:
        logger.error(f"research_agent_trigger error: {e}")
        return error_response("AGENT_ERROR", str(e))


def tool_hypothesis_generate(
    topic: str,
    gap_context: str = "",
    gap_type: str = "",
    creative: bool = False,
) -> Dict:
    """Generate testable research hypotheses from gap + topic."""
    try:
        from llm.hypothesis_generator import HypothesisGenerator

        gen = HypothesisGenerator()
        result = gen.generate(
            topic=topic,
            gap_context=gap_context,
            use_llm=True,
            creative=creative,
        )

        return success_response({
            "topic": topic,
            "summary": result.summary,
            "hypotheses": [
                {
                    "id": h.id,
                    "title": h.title,
                    "type": h.hypothesis_type.value,
                    "core_statement": h.core_statement,
                    "based_on": h.based_on,
                    "novelty_score": h.novelty_score,
                    "feasibility_score": h.feasibility_score,
                    "experiment_design": {
                        "baseline": h.experiment_design.baseline,
                        "variables": h.experiment_design.variables,
                        "controls": h.experiment_design.controls,
                        "evaluation_metrics": h.experiment_design.evaluation_metrics,
                        "expected_results": h.experiment_design.expected_results,
                    },
                    "risk": {
                        "technical": h.risk_assessment.technical_risk.value if h.risk_assessment else "unknown",
                        "hypothesis": h.risk_assessment.hypothesis_risk.value if h.risk_assessment else "unknown",
                    } if h.risk_assessment else None,
                }
                for h in result.hypotheses
            ]
        })

    except Exception as e:
        logger.error(f"hypothesis_generate error: {e}")
        return error_response("HYPOTHESIS_ERROR", str(e))


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
            rows.append({
                "hypothesis_id": hid,
                "verdict": verdict,
                "detail": detail,
                "linked_experiments": len(linked),
                "experiments": [
                    {"id": e.id, "name": e.name, "status": e.status}
                    for e in linked
                ]
            })

        return success_response({
            "total": len(rows),
            "hypotheses": rows
        })

    except Exception as e:
        logger.error(f"hypothesis_list error: {e}")
        return error_response("HYPOTHESIS_ERROR", str(e))


def _compute_verdict(events):
    """Compute verdict from hypothesis events."""
    if not events:
        return "INCONCLUSIVE", "no experiments recorded"
    action_vals = {e.action.value if hasattr(e.action, 'value') else str(e.action) for e in events}
    has_completed = "validated" in action_vals
    has_failed = "rejected" in action_vals
    if has_completed and has_failed:
        return "MIXED", "both validated and rejected experiments exist"
    if has_completed:
        return "VALIDATED", "all experiments succeeded"
    if has_failed:
        return "REJECTED", "all experiments failed"
    return "INCONCLUSIVE", "no completed experiments yet"


def tool_experiment_record(
    hypothesis_id: str,
    name: str,
    result: str,
    metrics: Optional[Dict] = None,
) -> Dict:
    """Record an experiment result for a hypothesis."""
    try:
        from llm.experiment_tracker import ExperimentTracker, ExperimentStatus

        tracker = ExperimentTracker()
        status_map = {
            "validated": ExperimentStatus.COMPLETED,
            "rejected": ExperimentStatus.FAILED,
            "failed": ExperimentStatus.FAILED,
            "running": ExperimentStatus.RUNNING,
            "completed": ExperimentStatus.COMPLETED,
        }
        status = status_map.get(result.lower(), ExperimentStatus.COMPLETED)

        exp_id = str(uuid.uuid4())[:8]
        experiment = Experiment(
            id=exp_id,
            name=name,
            hypothesis_id=hypothesis_id,
            status=status.value,
            results={"verdict": result, "metrics": metrics or {}},
        )
        tracker.save_experiment(experiment)

        return success_response({
            "experiment_id": exp_id,
            "hypothesis_id": hypothesis_id,
            "status": status.value,
            "message": f"Experiment recorded: {name} → {result}"
        })

    except Exception as e:
        logger.error(f"experiment_record error: {e}")
        return error_response("EXPERIMENT_ERROR", str(e))


def tool_litreview_generate(topic: str, limit: int = 30, use_llm: bool = True) -> Dict:
    """Generate a literature review for a topic."""
    try:
        from llm.litreview_generator import LitReviewGenerator

        generator = LitReviewGenerator()
        result = generator.generate(
            topic=topic,
            limit=limit,
            use_llm=use_llm,
            output_dir=PROJECT_ROOT / "data" / "litreviews",
        )

        if result.success:
            return success_response({
                "topic": topic,
                "total_papers": result.review.total_papers if result.review else 0,
                "sections_count": len(result.review.sections) if result.review else 0,
                "markdown": result.markdown,
                "generated_at": result.review.generated_at if result.review else "",
            })
        else:
            return error_response("LITREVIEW_ERROR", result.error)

    except Exception as e:
        logger.error(f"litreview_generate error: {e}")
        return error_response("LITREVIEW_ERROR", str(e))


def tool_litreview_list() -> Dict:
    """List all saved literature reviews."""
    try:
        from pathlib import Path

        litreview_dir = PROJECT_ROOT / "data" / "litreviews"
        reviews = []
        if litreview_dir.exists():
            for f in sorted(litreview_dir.glob("litreview_*.md"), reverse=True)[:20]:
                content = f.read_text(encoding="utf-8")
                lines = content.split("\n")
                title = lines[0].lstrip("# ").strip() if lines else f.stem
                date = ""
                for line in lines[1:5]:
                    if "Generated:" in line:
                        date = line.split("Generated:")[-1].strip()
                        break
                reviews.append({
                    "filename": f.name,
                    "topic": title,
                    "date": date,
                    "size_bytes": f.stat().st_size,
                })

        return success_response({"reviews": reviews, "count": len(reviews)})

    except Exception as e:
        logger.error(f"litreview_list error: {e}")
        return error_response("LITREVIEW_ERROR", str(e))


def tool_research_memory_add_stance(
    topic: str,
    claim: str,
    stance: str,
    evidence_refs: Optional[List[str]] = None,
    reasoning: str = "",
    confidence: float = 0.5,
) -> Dict:
    """Add a research stance to memory."""
    try:
        from llm.research_memory import ResearchMemory, StanceType

        memory = ResearchMemory()
        stance_enum = StanceType(stance.lower())
        s = memory.add_stance(
            topic=topic,
            claim=claim,
            stance=stance_enum,
            evidence_refs=evidence_refs or [],
            reasoning=reasoning,
            confidence=confidence,
        )
        return success_response({
            "stance_id": s.stance_id,
            "topic": s.topic,
            "stance": s.stance.value,
            "claim": s.claim[:80],
            "created_at": datetime.fromtimestamp(s.created_at).isoformat(),
        })

    except Exception as e:
        logger.error(f"research_memory_add_stance error: {e}")
        return error_response("MEMORY_ERROR", str(e))


def tool_research_memory_list_stances() -> Dict:
    """List all stances in research memory."""
    try:
        from llm.research_memory import ResearchMemory

        memory = ResearchMemory()
        summary = memory.get_summary()
        stances = memory.get_stances()

        return success_response({
            "summary": summary,
            "stances": [
                {
                    "stance_id": s.stance_id,
                    "topic": s.topic,
                    "claim": s.claim[:100],
                    "stance": s.stance.value,
                    "confidence": s.confidence,
                    "evidence_count": len(s.evidence_refs),
                    "created_at": datetime.fromtimestamp(s.created_at).isoformat(),
                    "updated_at": datetime.fromtimestamp(s.updated_at).isoformat(),
                }
                for s in stances
            ],
        })

    except Exception as e:
        logger.error(f"research_memory_list_stances error: {e}")
        return error_response("MEMORY_ERROR", str(e))


def tool_research_memory_check_paper(arxiv_id: str, use_llm: bool = True) -> Dict:
    """Check a paper against research memory for anomalies."""
    try:
        from llm.research_memory import ResearchMemory

        memory = ResearchMemory()

        # Fetch paper from DB
        from db.database import Database
        db = Database()
        db.init()
        rows, _ = db.search_papers(arxiv_id, limit=1)
        if not rows:
            return error_response("NOT_FOUND", f"Paper {arxiv_id} not found in database")

        row = rows[0]
        paper = {
            "arxiv_id": getattr(row, "paper_id", "") or getattr(row, "arxiv_id", ""),
            "title": getattr(row, "title", ""),
            "abstract": getattr(row, "abstract", "") or "",
        }

        anomalies = memory.check_paper_against_stances(paper, use_llm=use_llm)

        if not anomalies:
            return success_response({
                "arxiv_id": arxiv_id,
                "anomalies_found": 0,
                "message": "No contradictions detected",
            })

        return success_response({
            "arxiv_id": arxiv_id,
            "anomalies_found": len(anomalies),
            "anomalies": [
                {
                    "anomaly_id": a.anomaly_id,
                    "stance_id": a.stance_id,
                    "topic": a.topic,
                    "stance_claim": a.stance_claim[:80],
                    "paper_title": a.paper_title,
                    "anomaly_type": a.anomaly_type,
                    "severity": a.severity.value,
                    "description": a.description,
                    "created_at": datetime.fromtimestamp(a.created_at).isoformat(),
                }
                for a in anomalies
            ],
        })

    except Exception as e:
        logger.error(f"research_memory_check_paper error: {e}")
        return error_response("MEMORY_ERROR", str(e))


def tool_research_memory_anomalies() -> Dict:
    """List recent anomaly alerts."""
    try:
        from llm.research_memory import ResearchMemory

        memory = ResearchMemory()
        anomalies = memory.get_recent_anomalies(limit=20)
        summary = memory.get_summary()

        return success_response({
            "summary": summary,
            "anomalies": [
                {
                    "anomaly_id": a.anomaly_id,
                    "stance_id": a.stance_id,
                    "topic": a.topic,
                    "paper_title": a.paper_title,
                    "paper_arxiv_id": a.paper_arxiv_id,
                    "anomaly_type": a.anomaly_type,
                    "severity": a.severity.value,
                    "description": a.description,
                    "created_at": datetime.fromtimestamp(a.created_at).isoformat(),
                }
                for a in anomalies
            ],
        })

    except Exception as e:
        logger.error(f"research_memory_anomalies error: {e}")
        return error_response("MEMORY_ERROR", str(e))


def tool_review_simulate(arxiv_id: str, persona: str = "all", use_llm: bool = True) -> Dict:
    """Simulate adversarial peer reviewers on a paper."""
    try:
        from llm.review_simulator import ReviewSimulator, ReviewPersona, _REVIEW_PERSONAS

        # Fetch paper
        from db.database import Database
        db = Database()
        db.init()
        rows, _ = db.search_papers(arxiv_id, limit=1)
        if not rows:
            return error_response("NOT_FOUND", f"Paper {arxiv_id} not found")

        row = rows[0]
        paper_text = getattr(row, "abstract", "") or ""
        title = getattr(row, "title", "")

        # Build paper text for review
        full_text = f"{title}\n\n{paper_text}"

        simulator = ReviewSimulator()

        if persona != "all":
            persona_map = {p.name.lower().split()[0]: p for p in _REVIEW_PERSONAS}
            selected = persona_map.get(persona.lower())
            if not selected:
                return error_response("INVALID_PERSONA", f"Unknown persona: {persona}")
            review = simulator.review(full_text, title, persona=selected)
        else:
            review = simulator.review(full_text, title)

        # Save review
        from llm.review_simulator import save_review
        save_review(review)

        return success_response({
            "review_id": review.review_id,
            "persona": review.persona,
            "overall_score": review.overall_score,
            "recommendation": review.recommendation,
            "summary": review.summary,
            "strengths": review.strengths,
            "weaknesses": review.weaknesses,
            "annotation_count": len(review.annotations),
            "annotations": [a.to_dict() for a in review.annotations[:6]],
        })

    except Exception as e:
        logger.error(f"review_simulate error: {e}")
        return error_response("REVIEW_ERROR", str(e))


def tool_review_list() -> Dict:
    """List saved simulated reviews."""
    try:
        from llm.review_simulator import list_reviews
        reviews = list_reviews(limit=20)
        return success_response({"reviews": reviews, "count": len(reviews)})
    except Exception as e:
        logger.error(f"review_list error: {e}")
        return error_response("REVIEW_ERROR", str(e))


# ─── MCP Protocol Handlers ──────────────────────────────────────────


def handle_initialize() -> dict:
    """Handle initialize request."""
    return success_response({
        "protocolVersion": MCP_VERSION,
        "serverInfo": {
            "name": "rairos",
            "version": "1.5.4"
        },
        "capabilities": {
            "tools": True
        }
    })


def handle_list_tools() -> dict:
    """Handle list_tools request."""
    return success_response({"tools": get_tools()})


def handle_call_tool(name: str, arguments: Dict) -> dict:
    """Handle call_tool request."""
    try:
        if name == "paper_ingest":
            result = tool_paper_ingest(
                identifier=arguments.get("identifier"),
                tags=arguments.get("tags")
            )
        elif name == "paper_search":
            result = tool_paper_search(
                query=arguments.get("query"),
                tag=arguments.get("tag"),
                limit=arguments.get("limit", 10),
                source=arguments.get("source", "local")
            )
        elif name == "paper_chat":
            result = tool_paper_chat(
                question=arguments.get("question"),
                paper_id=arguments.get("paper_id")
            )
        elif name == "kg_query":
            result = tool_kg_query(
                query=arguments.get("query"),
                entity_id=arguments.get("entity_id"),
                tag=arguments.get("tag")
            )
        elif name == "chart_query":
            result = tool_chart_query(
                paper_id=arguments.get("paper_id"),
                action=arguments.get("action"),
                label=arguments.get("label")
            )
        elif name == "research_run":
            result = tool_research_run(
                topic=arguments.get("topic"),
                limit=arguments.get("limit", 5)
            )
        elif name == "slides_generate":
            result = tool_slides_generate(
                paper_id=arguments.get("paper_id"),
                output_path=arguments.get("output_path")
            )
        elif name == "cite_fetch":
            result = tool_cite_fetch(
                paper_id=arguments.get("paper_id"),
                direction=arguments.get("direction", "both")
            )
        elif name == "paper_analyze":
            result = tool_paper_analyze(
                paper_id=arguments.get("paper_id")
            )
        elif name == "citation_graph":
            result = tool_citation_graph(
                paper_id=arguments.get("paper_id"),
                depth=arguments.get("depth", 2),
                max_nodes=arguments.get("max_nodes", 100)
            )
        elif name == "gap_detect":
            result = tool_gap_detect(
                topic=arguments.get("topic"),
                use_llm=arguments.get("use_llm", True)
            )
        elif name == "research_agent_start":
            result = tool_research_agent_start(
                interval_minutes=arguments.get("interval_minutes", 30)
            )
        elif name == "research_agent_stop":
            result = tool_research_agent_stop()
        elif name == "research_agent_status":
            result = tool_research_agent_status()
        elif name == "research_agent_trigger":
            result = tool_research_agent_trigger(
                topic=arguments.get("topic")
            )
        elif name == "hypothesis_generate":
            result = tool_hypothesis_generate(
                topic=arguments.get("topic"),
                gap_context=arguments.get("gap_context", ""),
                gap_type=arguments.get("gap_type", ""),
                creative=arguments.get("creative", False)
            )
        elif name == "hypothesis_list":
            result = tool_hypothesis_list()
        elif name == "experiment_record":
            result = tool_experiment_record(
                hypothesis_id=arguments.get("hypothesis_id"),
                name=arguments.get("name"),
                result=arguments.get("result"),
                metrics=arguments.get("metrics")
            )
        elif name == "litreview_generate":
            result = tool_litreview_generate(
                topic=arguments.get("topic"),
                limit=arguments.get("limit", 30),
                use_llm=arguments.get("use_llm", True)
            )
        elif name == "litreview_list":
            result = tool_litreview_list()
        elif name == "research_memory_add_stance":
            result = tool_research_memory_add_stance(
                topic=arguments.get("topic"),
                claim=arguments.get("claim"),
                stance=arguments.get("stance"),
                evidence_refs=arguments.get("evidence_refs"),
                reasoning=arguments.get("reasoning", ""),
                confidence=arguments.get("confidence", 0.5),
            )
        elif name == "research_memory_list_stances":
            result = tool_research_memory_list_stances()
        elif name == "research_memory_check_paper":
            result = tool_research_memory_check_paper(
                arxiv_id=arguments.get("arxiv_id"),
                use_llm=arguments.get("use_llm", True),
            )
        elif name == "research_memory_anomalies":
            result = tool_research_memory_anomalies()
        elif name == "review_simulate":
            result = tool_review_simulate(
                arxiv_id=arguments.get("arxiv_id"),
                persona=arguments.get("persona", "all"),
                use_llm=arguments.get("use_llm", True),
            )
        elif name == "review_list":
            result = tool_review_list()
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
        return handle_call_tool(
            name=params.get("name"),
            arguments=params.get("arguments", {})
        )
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
