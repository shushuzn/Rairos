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
import threading
import time
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


# Tool definitions moved to mcp/tools_defs.py
from mcp.tools_defs import get_tools


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


def tool_paper_recommend(
    limit: int = 5,
    focus_tags: Optional[List[str]] = None,
    exclude_read: bool = True,
    strategy: str = "similar_tags",
) -> Dict:
    """Recommend papers based on reading history and collaborative filtering."""
    try:
        # Defensive: if limit arrives as dict (positional dict arg dispatch bug), unpack it
        if isinstance(limit, dict):
            _args = limit
            limit = _args.get("limit", 5)
            focus_tags = _args.get("focus_tags", focus_tags)
            exclude_read = _args.get("exclude_read", exclude_read)
            strategy = _args.get("strategy", strategy)
        limit = max(1, min(int(limit), 100)) if limit else 5

        from db.database import Database

        db = Database()
        db.init()

        # Get user's reading history — completed and in-progress papers
        read_status = (
            ["completed", "reading"] if exclude_read else ["completed", "reading", "unread"]
        )
        history = []
        for status in read_status:
            rows = db.get_papers_by_reading_status(status)
            history.extend(rows)

        if not history:
            return error_response("NO_HISTORY", "No reading history found. Read some papers first.")

        # Extract tags from read papers (weighted by recency and completion)
        tag_weights: Dict[str, float] = {}
        for paper in history:
            tags = db.get_tags(getattr(paper, "paper_id", None) or getattr(paper, "id", None))
            weight = 1.0 if getattr(paper, "reading_status", None) == "completed" else 0.5
            for tag in tags:
                tag_weights[tag] = tag_weights.get(tag, 0) + weight

        # Apply focus_tags boost
        if focus_tags:
            for tag in focus_tags:
                tag_weights[tag] = tag_weights.get(tag, 0) + 3.0

        # Get top-weighted tags
        top_tags = sorted(tag_weights.items(), key=lambda x: -x[1])[:10]
        top_tag_names = [t for t, _ in top_tags]

        # Build candidate pool: all papers not in history (or all if !exclude_read)
        read_ids = {getattr(p, "paper_id", None) or getattr(p, "id", None) for p in history}
        all_rows, _ = db.list_papers(limit=500)  # get all papers
        if exclude_read:
            candidates = [
                r
                for r in all_rows
                if ((getattr(r, "paper_id", None) or getattr(r, "id", None)) not in read_ids)
            ]
        else:
            candidates = list(all_rows)

        if not candidates:
            return error_response("NO_CANDIDATES", "No candidate papers to recommend from.")

        # Score candidates by tag overlap
        scored: List[tuple] = []
        for paper in candidates:
            pid = getattr(paper, "paper_id", None) or getattr(paper, "id", None)
            paper_tags = db.get_tags(pid)
            score = sum(tag_weights.get(t, 0) for t in paper_tags)

            # Strategy adjustments
            if strategy == "complementary":
                # Prefer papers with new tags not yet read
                new_tags = [t for t in paper_tags if t not in tag_weights]
                score += len(new_tags) * 2.0
            elif strategy == "influential":
                # Boost by citation count
                score += (getattr(paper, "citation_count", 0) or 0) * 0.01
            elif strategy == "diverse":
                # Penalize if too many same tags as top recommendation
                pass

            scored.append((pid, score, paper))

        # Sort and dedupe by paper_id
        seen: set = set()
        recommendations = []
        for pid, score, paper in sorted(scored, key=lambda x: -x[1]):
            if pid in seen:
                continue
            seen.add(pid)
            recommendations.append(
                {
                    "paper_id": pid,
                    "title": getattr(paper, "title", ""),
                    "authors": getattr(paper, "authors", ""),
                    "published": getattr(paper, "published", "")[:10]
                    if getattr(paper, "published", None)
                    else "",
                    "score": round(score, 2),
                    "reason": f"tag_match:{','.join(paper_tags[:3])}"
                    if paper_tags
                    else "content_similarity",
                }
            )
            if len(recommendations) >= limit:
                break

        db.close()

        return success_response(
            {
                "strategy": strategy,
                "history_count": len(history),
                "top_tags": top_tag_names,
                "recommendations": recommendations,
            }
        )

    except Exception as e:
        logger.error(f"paper_recommend error: {e}")
        return error_response("RECOMMEND_ERROR", str(e))


def tool_pdf_download(arxiv_id: str, out_path: Optional[str] = None) -> Dict:
    """Download PDF for a paper from DB pdf_url or arXiv fallback."""
    try:
        from pathlib import Path
        from db.database import Database
        from pdf.extract import download_pdf

        db = Database()
        db.init()
        paper = db.get_paper(arxiv_id)

        if paper and getattr(paper, "pdf_url", ""):
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
                    for b in content.blocks
                ],
                "tables": [
                    {
                        "headers": t.headers,
                        "rows": t.rows,
                        "page": t.page,
                    }
                    for t in content.tables
                ],
                "math_count": len(content.math_blocks),
                "pages_extracted": max_pages or "all",
            }
        )

    except Exception as e:
        logger.error(f"pdf_extract_structured error: {e}")
        return error_response("PDF_EXTRACT_ERROR", str(e))


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


def tool_kg_paper_subgraph(paper_id: str, depth: int = 2) -> Dict:
    """Get ego graph around a paper as JSON (nodes + edges)."""
    try:
        from kg.manager import KGManager
        from kg.queries import KGQueries

        kg = KGManager()
        q = KGQueries(kg)
        subgraph = q.get_paper_subgraph(paper_id, depth=depth)

        return success_response(
            {
                "paper_id": paper_id,
                "depth": depth,
                "nodes": [
                    {
                        "id": n["id"],
                        "entity_type": n.get("entity_type"),
                        "entity_id": n.get("entity_id"),
                        "label": n.get("label"),
                    }
                    for n in subgraph.get("nodes", [])
                ],
                "edges": [
                    {
                        "id": e["id"],
                        "source_id": e["source_id"],
                        "target_id": e["target_id"],
                        "relation": e.get("relation"),
                    }
                    for e in subgraph.get("edges", [])
                ],
                "center": subgraph.get("center"),
            }
        )

    except Exception as e:
        logger.error(f"kg_paper_subgraph error: {e}")
        return error_response("KG_ERROR", str(e))


def tool_kg_tag_graph(tag: str) -> Dict:
    """Get all papers and notes related to a tag as graph JSON."""
    try:
        from kg.manager import KGManager
        from kg.queries import KGQueries

        kg = KGManager()
        q = KGQueries(kg)
        ecosystem = q.get_tag_ecosystem(tag)

        return success_response(
            {
                "tag": tag,
                "nodes": [
                    {
                        "id": n["id"],
                        "entity_type": n.get("entity_type"),
                        "entity_id": n.get("entity_id"),
                        "label": n.get("label"),
                    }
                    for n in ecosystem.get("nodes", [])
                ],
                "edges": [
                    {
                        "id": e["id"],
                        "source_id": e["source_id"],
                        "target_id": e["target_id"],
                        "relation": e.get("relation"),
                    }
                    for e in ecosystem.get("edges", [])
                ],
            }
        )

    except Exception as e:
        logger.error(f"kg_tag_graph error: {e}")
        return error_response("KG_ERROR", str(e))


def tool_kg_full_graph(max_nodes: int = 500) -> Dict:
    """Get global KG graph as JSON (up to max_nodes)."""
    try:
        from kg.manager import KGManager
        from kg.queries import KGQueries

        kg = KGManager()
        q = KGQueries(kg)
        export = q.export_graph_json()

        nodes = export["nodes"][:max_nodes]
        nids = {n["id"] for n in nodes}
        edges = [e for e in export["edges"] if e["source_id"] in nids and e["target_id"] in nids]

        return success_response(
            {
                "total_nodes": len(export["nodes"]),
                "total_edges": len(export["edges"]),
                "returned_nodes": len(nodes),
                "returned_edges": len(edges),
                "nodes": [
                    {
                        "id": n["id"],
                        "entity_type": n.get("entity_type"),
                        "entity_id": n.get("entity_id"),
                        "label": n.get("label"),
                    }
                    for n in nodes
                ],
                "edges": [
                    {
                        "id": e["id"],
                        "source_id": e["source_id"],
                        "target_id": e["target_id"],
                        "relation": e.get("relation"),
                    }
                    for e in edges
                ],
            }
        )

    except Exception as e:
        logger.error(f"kg_full_graph error: {e}")
        return error_response("KG_ERROR", str(e))


def tool_tag_add(paper_id: str, tag: str) -> Dict:
    """Add a tag to a paper."""
    try:
        from db.database import Database

        db = Database()
        db.init()
        db.add_tag(paper_id, tag)
        return success_response({"paper_id": paper_id, "tag": tag, "added": True})
    except Exception as e:
        logger.error(f"tag_add error: {e}")
        return error_response("TAG_ERROR", str(e))


def tool_tag_remove(paper_id: str, tag: str) -> Dict:
    """Remove a tag from a paper."""
    try:
        from db.database import Database

        db = Database()
        db.init()
        db.remove_tag(paper_id, tag)
        return success_response({"paper_id": paper_id, "tag": tag, "removed": True})
    except Exception as e:
        logger.error(f"tag_remove error: {e}")
        return error_response("TAG_ERROR", str(e))


def tool_tag_list(paper_id: str) -> Dict:
    """List all tags for a paper."""
    try:
        from db.database import Database

        db = Database()
        db.init()
        tags = db.get_tags(paper_id)
        return success_response({"paper_id": paper_id, "tags": tags, "count": len(tags)})
    except Exception as e:
        logger.error(f"tag_list error: {e}")
        return error_response("TAG_ERROR", str(e))


def tool_tag_all() -> Dict:
    """List all tags in the system."""
    try:
        from db.database import Database

        db = Database()
        db.init()
        cur = db.conn.cursor()
        cur.execute("SELECT name FROM tags ORDER BY name")
        tags = [row["name"] for row in cur.fetchall()]
        return success_response({"tags": tags, "count": len(tags)})
    except Exception as e:
        logger.error(f"tag_all error: {e}")
        return error_response("TAG_ERROR", str(e))


def tool_trends_detect_trending(threshold: float = 0.5) -> Dict:
    """Detect trending tags based on radar snapshots."""
    try:
        from trends.forecaster import TrendForecaster

        f = TrendForecaster()
        results = f.detect_trending(threshold=threshold)
        return success_response({"trending": results, "count": len(results)})
    except Exception as e:
        logger.error(f"trends_detect_trending error: {e}")
        return error_response("TRENDS_ERROR", str(e))


def tool_trends_predict_next(tag: str) -> Dict:
    """Predict next value for a tag."""
    try:
        from trends.forecaster import TrendForecaster

        f = TrendForecaster()
        prediction = f.predict_next(tag)
        return success_response({"tag": tag, "prediction": prediction})
    except Exception as e:
        logger.error(f"trends_predict_next error: {e}")
        return error_response("TRENDS_ERROR", str(e))


def tool_trends_top_predictions(top_k: int = 5) -> Dict:
    """Get top-K predicted trending tags."""
    try:
        from trends.forecaster import TrendForecaster

        f = TrendForecaster()
        predictions = f.get_top_predictions(top_k=top_k)
        return success_response({"predictions": predictions, "count": len(predictions)})
    except Exception as e:
        logger.error(f"trends_top_predictions error: {e}")
        return error_response("TRENDS_ERROR", str(e))


def tool_trends_compare_tags(tag_a: str, tag_b: str) -> Dict:
    """Compare two tags by their trajectories."""
    try:
        from trends.forecaster import TrendForecaster

        f = TrendForecaster()
        comparison = f.compare_tags(tag_a, tag_b)
        return success_response({"tag_a": tag_a, "tag_b": tag_b, "comparison": comparison})
    except Exception as e:
        logger.error(f"trends_compare_tags error: {e}")
        return error_response("TRENDS_ERROR", str(e))


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
        from llm.slides import SlidesGenerator

        if not output_path:
            output_path = str(PROJECT_ROOT / f"{paper_id}_slides.pptx")

        gen = SlidesGenerator()
        gen.generate(paper_id, output_path)

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


def tool_paper2code_run(
    arxiv_id: str,
    framework: str = "pytorch",
    skip_gene_pool: bool = False,
    continuous: bool = False,
    interval_minutes: int = 15,
) -> Dict:
    """Run full paper2code pipeline: download → parse → generate → test → benchmark → Gene Pool.

    If continuous=True, starts a background thread that polls ArXiv subscriptions
    every interval_minutes and runs the pipeline for each new paper discovered.
    """
    try:
        from research_loop.paper2code_integration import PaperPipeline
        import tempfile
        from llm.subscription_monitor import SubscriptionMonitor
        from db.database import Database

        if not continuous:
            pipeline = PaperPipeline(work_dir=tempfile.mkdtemp())
            result = pipeline.run(
                arxiv_id=arxiv_id,
                mode="minimal",
                framework=framework,
                skip_gene_pool=skip_gene_pool,
            )
            return success_response(
                {
                    "arxiv_id": result["arxiv_id"],
                    "paper_dir": result["paper_dir"],
                    "src_dir": result["src_dir"],
                    "test_dir": result["test_dir"],
                    "readme": result["readme"],
                    "benchmark": result.get("benchmark"),
                }
            )

        # Continuous mode: daemon thread polling ArXiv channels
        def _continuous_loop(stop_event):
            db = Database()
            db.init()
            monitor = SubscriptionMonitor(db)
            seen = set()
            while not stop_event.is_set():
                try:
                    results = monitor.check_all()
                    total_new = 0
                    for _sub_id, papers in results.items():
                        for paper in papers:
                            paper_id = paper.get("arxiv_id", "")
                            if paper_id and paper_id not in seen:
                                seen.add(paper_id)
                                total_new += 1
                                logger.info(f"[paper2code continuous] New paper: {paper_id}")
                                p = PaperPipeline(work_dir=tempfile.mkdtemp())
                                p.run(
                                    arxiv_id=paper_id,
                                    mode="minimal",
                                    framework=framework,
                                    skip_gene_pool=skip_gene_pool,
                                )
                    if total_new > 0:
                        logger.info(f"[paper2code continuous] Processed {total_new} new papers")
                except Exception as e:
                    logger.error(f"[paper2code continuous] Cycle error: {e}")
                stop_event.wait(timeout=interval_minutes * 60)

        stop_event = threading.Event()
        thread = threading.Thread(
            target=_continuous_loop, args=(stop_event,), daemon=True, name="paper2code-continuous"
        )
        thread.start()
        return success_response(
            {
                "status": "started",
                "mode": "continuous",
                "interval_minutes": interval_minutes,
                "message": f"paper2code continuous mode started. Polling every {interval_minutes}min.",
            }
        )

    except Exception as e:
        logger.error(f"paper2code_run error: {e}")
        return error_response("PAPER2CODE_ERROR", str(e))


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
                        "type": str(g.gap_type),
                        "description": g.description,
                        "evidence": g.evidence_papers,
                        "confidence": g.confidence,
                        "severity": str(g.severity),
                    }
                    for g in result.gaps
                ],
                "questions": [
                    {
                        "question": q.question,
                        "gap_type": str(q.gap.gap_type) if q.gap else "",
                        "verifiability": q.feasibility,
                    }
                    for q in result.questions
                ],
            }
        )

    except Exception as e:
        logger.error(f"gap_detect error: {e}")
        return error_response("GAP_ERROR", str(e))


def tool_gap_submit(
    topic: str, gap_type: str, title: str, description: str, success_score: float = 0.8
) -> Dict:
    """Submit a new research gap directly to the Gene Pool as a CapsuleGene."""
    try:
        from llm.insight.tracker import EvolutionTracker

        tracker = EvolutionTracker()
        capsule = tracker.encode_capsule(
            topic=topic,
            gap_type=gap_type,
            gap_title=title,
            gap_description=description,
            success_score=success_score,
        )

        return success_response(
            {
                "capsule_id": capsule.capsule_id,
                "topic": topic,
                "gap_type": gap_type,
                "title": title,
                "status": capsule.status,
                "message": f"Gap '{title}' submitted to Gene Pool successfully",
            }
        )

    except Exception as e:
        logger.error(f"gap_submit error: {e}")
        return error_response("GAP_SUBMIT_ERROR", str(e))


def tool_gap_evolve(topic: str, gap_type: Optional[str] = None) -> Dict:
    """Run Gene Pool evolution cycle for a topic — audit, propose, evaluate, apply."""
    try:
        from llm.insight.tracker import EvolutionTracker
        from llm.insight.evolution import InsightEvolution

        tracker = EvolutionTracker()
        evo = InsightEvolution(tracker)
        result = evo.evolve(topic=topic, gap_type=gap_type or "")

        audit = result["audit"]
        ev_result = result["result"]

        return success_response(
            {
                "topic": topic,
                "gap_type": gap_type,
                "audit": {
                    "total_capsules": audit["total"],
                    "avg_quality": round(audit["avg_quality"], 3),
                    "candidates": audit["candidates"],
                    "to_retire": audit["to_retire"],
                },
                "proposed": result["proposed"],
                "evaluated": result["evaluations"],
                "result": {
                    "added": ev_result["added"],
                    "retired": ev_result["retired"],
                    "total_capsules": ev_result["total_capsules"],
                    "avg_quality": round(ev_result["avg_quality"], 3),
                },
            }
        )

    except Exception as e:
        logger.error(f"gap_evolve error: {e}")
        return error_response("GAP_EVOLVE_ERROR", str(e))


def tool_research_agent_start(interval_minutes: int = 30) -> Dict:
    """Start the autonomous research agent in background watch mode."""
    try:
        from research_loop.orchestrator import AutonomousOrchestrator

        orch = AutonomousOrchestrator(webhook_enabled=True)
        orch.start_watch(interval_minutes=interval_minutes)
        status = orch.get_status()
        return success_response(
            {
                "status": "started",
                "interval_minutes": interval_minutes,
                "running": status["running"],
                "message": f"Autonomous research agent started. Will check subscriptions every {interval_minutes} minutes.",
            }
        )
    except Exception as e:
        logger.error(f"research_agent_start error: {e}")
        return error_response("AGENT_ERROR", str(e))


def tool_research_agent_stop() -> Dict:
    """Stop the autonomous research agent watch loop."""
    try:
        from research_loop.orchestrator import AutonomousOrchestrator

        orch = AutonomousOrchestrator()
        orch.stop_watch()
        return success_response(
            {"status": "stopped", "message": "Autonomous research agent stopped."}
        )
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
        return success_response(
            {"status": status, "recent_alerts": [a.to_dict() for a in recent_alerts]}
        )
    except Exception as e:
        logger.error(f"research_agent_status error: {e}")
        return error_response("AGENT_ERROR", str(e))


def tool_research_agent_trigger(topic: Optional[str] = None) -> Dict:
    """Manually trigger one cycle of the autonomous research agent."""
    try:
        from research_loop.orchestrator import AutonomousOrchestrator

        orch = AutonomousOrchestrator(webhook_enabled=True)
        alerts = orch.run_cycle()
        return success_response(
            {
                "status": "cycle_complete",
                "alerts_generated": len(alerts),
                "alerts": [a.to_dict() for a in alerts],
            }
        )
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

        return success_response(
            {
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
                            "technical": h.risk_assessment.technical_risk.value
                            if h.risk_assessment
                            else "unknown",
                            "hypothesis": h.risk_assessment.hypothesis_risk.value
                            if h.risk_assessment
                            else "unknown",
                        }
                        if h.risk_assessment
                        else None,
                    }
                    for h in result.hypotheses
                ],
            }
        )

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

        return success_response(
            {
                "experiment_id": exp_id,
                "hypothesis_id": hypothesis_id,
                "status": status.value,
                "message": f"Experiment recorded: {name} → {result}",
            }
        )

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
            return success_response(
                {
                    "topic": topic,
                    "total_papers": result.review.total_papers if result.review else 0,
                    "sections_count": len(result.review.sections) if result.review else 0,
                    "markdown": result.markdown,
                    "generated_at": result.review.generated_at if result.review else "",
                }
            )
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
                reviews.append(
                    {
                        "filename": f.name,
                        "topic": title,
                        "date": date,
                        "size_bytes": f.stat().st_size,
                    }
                )

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
        return success_response(
            {
                "stance_id": s.stance_id,
                "topic": s.topic,
                "stance": s.stance.value,
                "claim": s.claim[:80],
                "created_at": datetime.fromtimestamp(s.created_at).isoformat(),
            }
        )

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

        return success_response(
            {
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
            }
        )

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
            return success_response(
                {
                    "arxiv_id": arxiv_id,
                    "anomalies_found": 0,
                    "message": "No contradictions detected",
                }
            )

        return success_response(
            {
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
            }
        )

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

        return success_response(
            {
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
            }
        )

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

        return success_response(
            {
                "review_id": review.review_id,
                "persona": review.persona,
                "overall_score": review.overall_score,
                "recommendation": review.recommendation,
                "summary": review.summary,
                "strengths": review.strengths,
                "weaknesses": review.weaknesses,
                "annotation_count": len(review.annotations),
                "annotations": [a.to_dict() for a in review.annotations[:6]],
            }
        )

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


def tool_routeplan_create(
    hypothesis: str,
    goal: str,
    known_papers: Optional[List[Dict[str, str]]] = None,
) -> Dict:
    """Create a research route plan from a hypothesis."""
    try:
        from llm.route_planner import RoutePlanner

        planner = RoutePlanner()
        plan = planner.create_plan(
            hypothesis=hypothesis,
            goal=goal,
            known_papers=known_papers,
        )

        progress = plan.get_progress()
        return success_response(
            {
                "plan_id": plan.plan_id,
                "hypothesis": plan.hypothesis,
                "goal": plan.goal,
                "step_count": len(plan.steps),
                "estimated_hours": progress["estimated_hours"],
                "progress_pct": progress["progress_pct"],
                "steps": [s.to_dict() for s in plan.steps],
                "created_at": datetime.fromtimestamp(plan.created_at).isoformat(),
            }
        )

    except Exception as e:
        logger.error(f"routeplan_create error: {e}")
        return error_response("PLAN_ERROR", str(e))


def tool_routeplan_list() -> Dict:
    """List all research plans."""
    try:
        from llm.route_planner import RoutePlanner, PlanStatus

        planner = RoutePlanner()
        plans = planner.list_plans(limit=20)

        return success_response(
            {
                "plans": [
                    {
                        "plan_id": p.plan_id,
                        "hypothesis": p.hypothesis[:80],
                        "goal": p.goal[:80],
                        "status": p.status.value,
                        "step_count": len(p.steps),
                        "progress": p.get_progress()["progress_pct"],
                        "revision_count": p.revision_count,
                        "created_at": datetime.fromtimestamp(p.created_at).isoformat(),
                        "updated_at": datetime.fromtimestamp(p.updated_at).isoformat(),
                    }
                    for p in plans
                ],
                "count": len(plans),
            }
        )

    except Exception as e:
        logger.error(f"routeplan_list error: {e}")
        return error_response("PLAN_ERROR", str(e))


def tool_routeplan_update_step(
    plan_id: str,
    step_id: str,
    status: str,
    result: str = "",
    notes: str = "",
) -> Dict:
    """Update a step status in a research plan."""
    try:
        from llm.route_planner import RoutePlanner, StepStatus

        status_enum = StepStatus(status.lower())
        planner = RoutePlanner()
        plan = planner.update_step(
            plan_id=plan_id,
            step_id=step_id,
            status=status_enum,
            result=result,
            notes=notes,
        )

        if not plan:
            return error_response("NOT_FOUND", f"Plan {plan_id} or step {step_id} not found")

        return success_response(
            {
                "plan_id": plan.plan_id,
                "step_id": step_id,
                "status": status_enum.value,
                "progress": plan.get_progress(),
                "ready_steps": [
                    {"step_id": s.step_id, "description": s.description}
                    for s in plan.get_ready_steps()
                ],
            }
        )

    except Exception as e:
        logger.error(f"routeplan_update_step error: {e}")
        return error_response("PLAN_ERROR", str(e))


def tool_routeplan_revise(plan_id: str, reason: str) -> Dict:
    """Revise a plan when dead ends are hit."""
    try:
        from llm.route_planner import RoutePlanner

        planner = RoutePlanner()
        new_plan = planner.revise_plan(plan_id=plan_id, reason=reason)

        if not new_plan:
            return error_response("NOT_FOUND", f"Plan {plan_id} not found")

        return success_response(
            {
                "new_plan_id": new_plan.plan_id,
                "old_plan_id": plan_id,
                "revision_count": new_plan.revision_count,
                "step_count": len(new_plan.steps),
                "progress": new_plan.get_progress(),
                "steps": [s.to_dict() for s in new_plan.steps],
            }
        )

    except Exception as e:
        logger.error(f"routeplan_revise error: {e}")
        return error_response("PLAN_ERROR", str(e))


def tool_briefing_generate(arxiv_id: str, use_llm: bool = True) -> Dict:
    """Generate a research briefing for a paper."""
    try:
        from llm.briefing_generator import BriefingGenerator

        generator = BriefingGenerator()
        result = generator.generate(
            arxiv_id=arxiv_id,
            use_llm=use_llm,
            output_dir=PROJECT_ROOT / "data" / "briefings",
        )

        if result.success:
            return success_response(
                {
                    "arxiv_id": arxiv_id,
                    "title": result.briefing.paper_title,
                    "verdict": result.briefing.verdict,
                    "verdict_reason": result.briefing.verdict_reason,
                    "sections_count": len(result.briefing.sections),
                    "gene_pool_matches": len(result.briefing.gene_pool_matches),
                    "memory_stances": len(result.briefing.memory_stances),
                    "markdown": result.markdown,
                    "generated_at": result.briefing.generated_at,
                }
            )
        else:
            return error_response("BRIEFING_ERROR", result.error)

    except Exception as e:
        logger.error(f"briefing_generate error: {e}")
        return error_response("BRIEFING_ERROR", str(e))


def tool_replication_check(arxiv_id: str, include_abstract: bool = True) -> Dict:
    """Check paper reproducibility."""
    try:
        from llm.replication_checker import ReplicationChecker
        from db.database import Database
        from parsers.semantic_scholar import get_paper_by_id

        paper = get_paper_by_id(arxiv_id)
        if not paper:
            return error_response("NOT_FOUND", f"Paper not found: {arxiv_id}")

        title = paper.title or arxiv_id
        abstract = paper.abstract or "" if include_abstract else ""

        checker = ReplicationChecker()
        report = checker.check_paper(
            paper_id=arxiv_id,
            title=title,
            abstract=abstract,
        )

        return success_response(
            {
                **report.to_dict(),
                "rendered": checker.render_report(report),
            }
        )
    except Exception as e:
        logger.error(f"replication_check error: {e}")
        return error_response("REPLICATION_ERROR", str(e))


def tool_replication_compare(arxiv_id_1: str, arxiv_id_2: str) -> Dict:
    """Compare reproducibility of two papers."""
    try:
        from llm.replication_checker import ReplicationChecker
        from parsers.semantic_scholar import get_paper_by_id

        checker = ReplicationChecker()

        paper1 = get_paper_by_id(arxiv_id_1)
        paper2 = get_paper_by_id(arxiv_id_2)

        report1 = checker.check_paper(
            paper_id=arxiv_id_1,
            title=paper1.title if paper1 else arxiv_id_1,
            abstract=paper1.abstract if paper1 else "",
        )
        report2 = checker.check_paper(
            paper_id=arxiv_id_2,
            title=paper2.title if paper2 else arxiv_id_2,
            abstract=paper2.abstract if paper2 else "",
        )

        easier = report1 if report1.difficulty_score < report2.difficulty_score else report2

        return success_response(
            {
                "paper_1": report1.to_dict(),
                "paper_2": report2.to_dict(),
                "easier_to_reproduce": easier.paper_id,
                "comparison": {
                    "difficulty_diff": round(
                        abs(report1.difficulty_score - report2.difficulty_score), 1
                    ),
                    "report_1": checker.render_report(report1),
                    "report_2": checker.render_report(report2),
                },
            }
        )
    except Exception as e:
        logger.error(f"replication_compare error: {e}")
        return error_response("REPLICATION_ERROR", str(e))


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
        elif name == "paper_recommend":
            result = tool_paper_recommend(
                limit=arguments.get("limit", 5),
                focus_tags=arguments.get("focus_tags"),
                exclude_read=arguments.get("exclude_read", True),
                strategy=arguments.get("strategy", "similar_tags"),
            )
        elif name == "pdf_download":
            result = tool_pdf_download(
                arxiv_id=arguments.get("arxiv_id"), out_path=arguments.get("out_path")
            )
        elif name == "pdf_extract_text":
            result = tool_pdf_extract_text(
                pdf_path=arguments.get("pdf_path"),
                max_pages=arguments.get("max_pages"),
                ocr=arguments.get("ocr", False),
                use_pdfminer_fallback=arguments.get("use_pdfminer_fallback", True),
            )
        elif name == "pdf_extract_structured":
            result = tool_pdf_extract_structured(
                pdf_path=arguments.get("pdf_path"), max_pages=arguments.get("max_pages")
            )
        elif name == "kg_query":
            result = tool_kg_query(
                query=arguments.get("query"),
                entity_id=arguments.get("entity_id"),
                tag=arguments.get("tag"),
            )
        elif name == "kg_paper_subgraph":
            result = tool_kg_paper_subgraph(
                paper_id=arguments.get("paper_id"), depth=arguments.get("depth", 2)
            )
        elif name == "kg_tag_graph":
            result = tool_kg_tag_graph(tag=arguments.get("tag"))
        elif name == "kg_full_graph":
            result = tool_kg_full_graph(max_nodes=arguments.get("max_nodes", 500))
        elif name == "tag_add":
            result = tool_tag_add(paper_id=arguments.get("paper_id"), tag=arguments.get("tag"))
        elif name == "tag_remove":
            result = tool_tag_remove(paper_id=arguments.get("paper_id"), tag=arguments.get("tag"))
        elif name == "tag_list":
            result = tool_tag_list(paper_id=arguments.get("paper_id"))
        elif name == "tag_all":
            result = tool_tag_all()
        elif name == "trends_detect_trending":
            result = tool_trends_detect_trending(threshold=arguments.get("threshold", 0.5))
        elif name == "trends_predict_next":
            result = tool_trends_predict_next(tag=arguments.get("tag"))
        elif name == "trends_top_predictions":
            result = tool_trends_top_predictions(top_k=arguments.get("top_k", 5))
        elif name == "trends_compare_tags":
            result = tool_trends_compare_tags(
                tag_a=arguments.get("tag_a"), tag_b=arguments.get("tag_b")
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
        elif name == "paper2code_run":
            result = tool_paper2code_run(
                arxiv_id=arguments.get("arxiv_id"),
                framework=arguments.get("framework", "pytorch"),
                skip_gene_pool=arguments.get("skip_gene_pool", False),
                continuous=arguments.get("continuous", False),
                interval_minutes=arguments.get("interval_minutes", 15),
            )
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
        elif name == "gap_submit":
            result = tool_gap_submit(
                topic=arguments.get("topic"),
                gap_type=arguments.get("gap_type"),
                title=arguments.get("title"),
                description=arguments.get("description"),
                success_score=arguments.get("success_score", 0.8),
            )
        elif name == "gap_evolve":
            result = tool_gap_evolve(
                topic=arguments.get("topic"), gap_type=arguments.get("gap_type")
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
            result = tool_research_agent_trigger(topic=arguments.get("topic"))
        elif name == "hypothesis_generate":
            result = tool_hypothesis_generate(
                topic=arguments.get("topic"),
                gap_context=arguments.get("gap_context", ""),
                gap_type=arguments.get("gap_type", ""),
                creative=arguments.get("creative", False),
            )
        elif name == "hypothesis_list":
            result = tool_hypothesis_list()
        elif name == "experiment_record":
            result = tool_experiment_record(
                hypothesis_id=arguments.get("hypothesis_id"),
                name=arguments.get("name"),
                result=arguments.get("result"),
                metrics=arguments.get("metrics"),
            )
        elif name == "litreview_generate":
            result = tool_litreview_generate(
                topic=arguments.get("topic"),
                limit=arguments.get("limit", 30),
                use_llm=arguments.get("use_llm", True),
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
        elif name == "routeplan_create":
            result = tool_routeplan_create(
                hypothesis=arguments.get("hypothesis"),
                goal=arguments.get("goal"),
                known_papers=arguments.get("known_papers"),
            )
        elif name == "routeplan_list":
            result = tool_routeplan_list()
        elif name == "routeplan_update_step":
            result = tool_routeplan_update_step(
                plan_id=arguments.get("plan_id"),
                step_id=arguments.get("step_id"),
                status=arguments.get("status"),
                result=arguments.get("result", ""),
                notes=arguments.get("notes", ""),
            )
        elif name == "routeplan_revise":
            result = tool_routeplan_revise(
                plan_id=arguments.get("plan_id"),
                reason=arguments.get("reason"),
            )
        elif name == "briefing_generate":
            result = tool_briefing_generate(
                arxiv_id=arguments.get("arxiv_id"),
                use_llm=arguments.get("use_llm", True),
            )
        elif name == "citation_chain_build":
            result = tool_citation_chain_build(
                arxiv_id=arguments.get("arxiv_id"),
                max_depth=arguments.get("max_depth", 2),
            )
        elif name == "citation_chain_families":
            result = tool_citation_chain_families(
                arxiv_id=arguments.get("arxiv_id"),
            )
        elif name == "citation_chain_silent":
            result = tool_citation_chain_silent(
                arxiv_id=arguments.get("arxiv_id"),
            )
        elif name == "citation_chain_render":
            result = tool_citation_chain_render(
                arxiv_id=arguments.get("arxiv_id"),
                format=arguments.get("format", "text"),
            )
        elif name == "impact_rank":
            result = tool_impact_rank(
                topic=arguments.get("topic", ""),
                top_k=arguments.get("top_k", 10),
                min_citations=arguments.get("min_citations", 0),
            )
        elif name == "impact_score_paper":
            result = tool_impact_score_paper(
                arxiv_id=arguments.get("arxiv_id"),
            )
        elif name == "impact_leaderboard":
            result = tool_impact_leaderboard(
                limit=arguments.get("limit", 20),
                year_min=arguments.get("year_min", 2020),
            )
        elif name == "replication_check":
            result = tool_replication_check(
                arxiv_id=arguments.get("arxiv_id"),
                include_abstract=arguments.get("include_abstract", True),
            )
        elif name == "replication_compare":
            result = tool_replication_compare(
                arxiv_id_1=arguments.get("arxiv_id_1"),
                arxiv_id_2=arguments.get("arxiv_id_2"),
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


def tool_citation_chain_build(arxiv_id: str, max_depth: int = 2) -> Dict:
    """Build a citation chain using Semantic Scholar API."""
    try:
        from llm.citation_chain import CitationChainBuilder

        builder = CitationChainBuilder()
        _chain = builder.build_chain(seed_arxiv_id=arxiv_id, max_depth=max_depth)

        return success_response(
            {
                "arxiv_id": arxiv_id,
                "nodes_count": len(_chain.nodes),
                "edges_count": len(_chain.edges),
                "nodes": [
                    {
                        "paper_id": n.paper_id,
                        "title": n.title,
                        "year": n.year,
                        "citations": n.citations,
                        "cited_by": n.cited_by,
                        "citation_count": n.citation_count,
                    }
                    for n in _chain.nodes
                ],
                "edges": [{"from": e[0], "to": e[1]} for e in _chain.edges],
            }
        )
    except Exception as e:
        logger.error(f"citation_chain_build error: {e}")
        return error_response("CHAIN_ERROR", str(e))


def tool_citation_chain_families(arxiv_id: str) -> Dict:
    """Cluster papers in a citation chain into research families."""
    try:
        from llm.citation_chain import CitationChainBuilder

        builder = CitationChainBuilder()
        _chain = builder.build_chain(seed_arxiv_id=arxiv_id, max_depth=2)
        families = builder.cluster_families()

        return success_response(
            {
                "arxiv_id": arxiv_id,
                "families_count": len(families),
                "families": [f.to_dict() for f in families],
            }
        )
    except Exception as e:
        logger.error(f"citation_chain_families error: {e}")
        return error_response("CHAIN_ERROR", str(e))


def tool_citation_chain_silent(arxiv_id: str) -> Dict:
    """Detect potential silent citations in a citation chain."""
    try:
        from llm.citation_chain import CitationChainBuilder

        builder = CitationChainBuilder()
        _chain = builder.build_chain(seed_arxiv_id=arxiv_id, max_depth=2)
        silent = builder.detect_silent_citations()

        return success_response(
            {
                "arxiv_id": arxiv_id,
                "silent_count": len(silent),
                "silent_citations": silent,
            }
        )
    except Exception as e:
        logger.error(f"citation_chain_silent error: {e}")
        return error_response("CHAIN_ERROR", str(e))


def tool_citation_chain_render(arxiv_id: str, format: str = "text") -> Dict:
    """Render a citation chain in text, mermaid, or graphviz format."""
    try:
        from llm.citation_chain import CitationChainBuilder

        builder = CitationChainBuilder()
        chain = builder.build_chain(seed_arxiv_id=arxiv_id, max_depth=2)

        if format == "mermaid":
            rendered = builder.render_mermaid(chain)
        elif format == "graphviz":
            rendered = builder.render_graphviz(chain)
        else:
            rendered = builder.render_text(chain)

        return success_response(
            {
                "arxiv_id": arxiv_id,
                "format": format,
                "rendered": rendered,
                "nodes_count": len(chain.nodes),
                "edges_count": len(chain.edges),
            }
        )
    except Exception as e:
        logger.error(f"citation_chain_render error: {e}")
        return error_response("CHAIN_ERROR", str(e))


def tool_impact_rank(topic: str, top_k: int = 10, min_citations: int = 0) -> Dict:
    """Rank papers by composite impact score."""
    try:
        from llm.impact_scorer import ImpactScorer
        from db.database import Database

        db = Database()
        db.init()

        rows, _ = db.search_papers(topic, limit=top_k * 3)
        if not rows:
            return error_response("NOT_FOUND", f"No papers found for topic: {topic}")

        papers = []
        for r in rows:
            cid = getattr(r, "citation_count", 0) or 0
            if cid < min_citations:
                continue
            papers.append(
                {
                    "paper_id": getattr(r, "paper_id", "") or getattr(r, "arxiv_id", ""),
                    "title": getattr(r, "title", ""),
                    "year": getattr(r, "year", 2020) or 2020,
                    "citation_count": cid,
                }
            )

        scorer = ImpactScorer(db=db)
        ranking = scorer.rank_papers(papers, top_k=top_k)

        return success_response(
            {
                "topic": topic,
                "total_ranked": len(ranking),
                "ranking": ranking,
                "rendered": scorer.render_ranking(ranking),
            }
        )
    except Exception as e:
        logger.error(f"impact_rank error: {e}")
        return error_response("IMPACT_ERROR", str(e))


def tool_impact_score_paper(arxiv_id: str) -> Dict:
    """Get detailed impact score for a specific paper."""
    try:
        from llm.impact_scorer import ImpactScorer
        from parsers.semantic_scholar import get_paper_by_id

        paper = get_paper_by_id(arxiv_id)
        if not paper:
            return error_response("NOT_FOUND", f"Paper not found: {arxiv_id}")

        scorer = ImpactScorer()
        score = scorer.score_paper(
            paper_id=arxiv_id,
            title=paper.title or arxiv_id,
            year=paper.year or 2020,
            raw_citations=paper.citation_count or 0,
        )

        return success_response(
            {
                **score.to_dict(),
                "explanation": scorer._explain_score(score),
            }
        )
    except Exception as e:
        logger.error(f"impact_score_paper error: {e}")
        return error_response("IMPACT_ERROR", str(e))


def tool_impact_leaderboard(limit: int = 20, year_min: int = 2020) -> Dict:
    """Get overall impact leaderboard from local database."""
    try:
        from llm.impact_scorer import ImpactScorer
        from db.database import Database

        db = Database()
        db.init()

        rows, _ = db.list_papers(limit=limit * 5, sort_by="citation_count", sort_order="desc")

        papers = []
        for r in rows:
            year = getattr(r, "year", 2020) or 2020
            if year < year_min:
                continue
            papers.append(
                {
                    "paper_id": getattr(r, "paper_id", "") or getattr(r, "arxiv_id", ""),
                    "title": getattr(r, "title", ""),
                    "year": year,
                    "citation_count": getattr(r, "citation_count", 0) or 0,
                }
            )

        scorer = ImpactScorer(db=db)
        ranking = scorer.rank_papers(papers, top_k=limit)

        return success_response(
            {
                "limit": limit,
                "year_min": year_min,
                "total_ranked": len(ranking),
                "ranking": ranking,
                "rendered": scorer.render_ranking(ranking),
            }
        )
    except Exception as e:
        logger.error(f"impact_leaderboard error: {e}")
        return error_response("IMPACT_ERROR", str(e))
