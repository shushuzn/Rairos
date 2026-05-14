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


def _record_gap_addressing(paper_id: str, title: str, abstract: str) -> None:
    """Check if a newly ingested paper addresses any known gaps and record the event.

    Uses semantic gap extraction to determine gap type and confidence,
    then calls impact_tracker.record_addressing_event().
    """
    try:
        from llm.research.gap_extract import extract_gap_from_paper
        from llm.research.impact_tracker import record_addressing_event

        gap_info = extract_gap_from_paper(
            paper_id=paper_id,
            title=title,
            abstract=abstract,
        )
        if not gap_info or not gap_info.get("gap_type"):
            return

        gap_type = gap_info.get("gap_type", "")
        gap_title = gap_info.get("gap_title", "")
        confidence = gap_info.get("confidence", 0.5)

        # Compute gap_hash for lookup in gap_history
        import hashlib

        topic = ""  # topic unknown at this level — use empty, filtered by hash
        gap_hash = hashlib.sha256(f"{topic}{gap_type}{gap_title}".encode()).hexdigest()[:16]

        record_addressing_event(
            gap_hash=gap_hash,
            topic=topic,
            gap_type=gap_type,
            paper_id=paper_id,
            paper_title=title,
            confidence=confidence,
            event_type="addresses",
            first_identified=None,
        )
    except Exception:
        pass  # Non-critical — don't fail paper ingestion


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
    """Run research loop - search arXiv, save to DB, generate report."""
    try:
        from research_loop.core import search_arxiv
        from db.database import Database

        db = Database()
        db.init()
        import time

        # Search arXiv (with rate-limit respect)
        papers = []
        try:
            time.sleep(3)
            papers = search_arxiv(topic, max_results=limit)
        except Exception as e:
            logger.warning(f"arXiv search failed: {e}, trying local DB instead")
            results, total = db.search_papers(topic, limit=limit)
            from core import Paper

            papers = [
                Paper(
                    source="arxiv",
                    uid=r.paper_id,
                    title=r.title,
                    authors=[],
                    abstract=getattr(r, "abstract", ""),
                    published=getattr(r, "published", ""),
                )  # type: ignore[call-arg]
                for r in results
            ]

        saved = 0
        for p in papers:
            uid = getattr(p, "uid", "") or getattr(p, "id", "")
            if uid:
                db.upsert_paper(
                    paper_id=uid,
                    source="arxiv",
                    title=getattr(p, "title", "") or "",
                    authors=getattr(p, "authors", []) or [],
                    abstract=getattr(p, "abstract", "") or "",
                    published=getattr(p, "published", "") or "",
                    abs_url=getattr(p, "abs_url", "") or "",
                    primary_category=getattr(p, "primary_category", "") or "",
                    categories=getattr(p, "categories", "") or "",
                )
                saved += 1

        return success_response(
            {
                "topic": topic,
                "papers_found": len(papers),
                "papers_saved": saved,
                "status": "completed",
            }
        )

    except Exception as e:
        logger.error(f"research_run error: {e}")
        return error_response("RESEARCH_ERROR", str(e))


def tool_cite_fetch(paper_id: str, direction: str = "both") -> Dict:
    """Fetch citations."""
    try:
        from llm.citation_chain import CitationChainBuilder

        builder = CitationChainBuilder()
        result: Any = (
            builder.build_chain(paper_id)
            if hasattr(builder, "build_chain")
            else {"cited": [], "citing": []}
        )

        cited = [n.arxiv_id for n in getattr(result, "nodes", []) if hasattr(n, "arxiv_id")]
        citing: List[Any] = []
        return success_response(
            {
                "paper_id": paper_id,
                "cited": cited[:10],
                "citing": citing,
                "count": len(cited),
            }
        )

    except Exception as e:
        logger.error(f"cite_fetch error: {e}")
        return error_response("CITE_ERROR", str(e))


def tool_paper_analyze(paper_id: str) -> Dict:
    """Analyze paper."""
    try:
        from llm.research.paper_analyzer import PaperAnalyzer, PaperAnalysisResult
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
                    "ruff_diagnostics": result.get("ruff_diagnostics", []),
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
    """Run Gene Pool evolution cycle for a topic - audit, propose, evaluate, apply."""
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


def tool_gene_pool_decay(
    action: str = "status",
    min_impact: float = 0.1,
    lambda_: float = 0.01,
) -> Dict:
    """Time-weighted impact scoring and auto-archive for Gene Pool capsules."""
    try:
        from llm.gene_pool_decay import gene_pool_decay_action

        result = gene_pool_decay_action(
            action=action,
            min_impact=min_impact,
            lambda_=lambda_,
        )
        return success_response(result)
    except Exception as e:
        logger.error(f"gene_pool_decay error: {e}")
        return error_response("DECAY_ERROR", str(e))


def tool_crossover(
    action: str = "evolve",
    offspring_count: int = 5,
    capsule_id: Optional[str] = None,
) -> Dict:
    """Run CapsuleGene genetic algorithm: select parents, crossover, mutate, encode V3."""
    try:
        from llm.crossover import crossover_action

        result = crossover_action(
            action=action,
            offspring_count=offspring_count,
            capsule_id=capsule_id,
        )
        return success_response(result)
    except Exception as e:
        logger.error(f"crossover error: {e}")
        return error_response("CROSSOVER_ERROR", str(e))


def tool_leaderboard(
    action: str = "status",
    arxiv_id: Optional[str] = None,
    sort_by: str = "combined",
    limit: int = 20,
) -> Dict:
    """Benchmark Leaderboard: ranked paper2code implementations by pass_rate + coverage."""
    try:
        from research_loop.leaderboard import leaderboard_action

        result = leaderboard_action(
            action=action,
            arxiv_id=arxiv_id,
            sort_by=sort_by,
            limit=limit,
        )
        return success_response(result)
    except Exception as e:
        logger.error(f"leaderboard error: {e}")
        return error_response("LEADERBOARD_ERROR", str(e))


def tool_gene_pool_watcher(
    action: str = "status",
    interval_minutes: int = 60,
    min_diversity_score: float = 50.0,
) -> Dict:
    """Manage GenePoolWatcher: check diversity gaps and auto-subscribe to fill them."""
    try:
        from llm.gene_pool_watcher import GenePoolWatcher

        watcher = GenePoolWatcher(
            interval_minutes=interval_minutes,
            min_diversity_score=min_diversity_score,
        )

        if action == "start":
            watcher.start()
            return success_response(
                {
                    "status": "started",
                    "message": f"GenePoolWatcher started. Will check diversity every {interval_minutes}min.",
                    "diversity_score": watcher.state.diversity_score,
                    "underrepresented_families": watcher.state.underrepresented_families,
                }
            )
        elif action == "stop":
            watcher.stop()
            return success_response({"status": "stopped", "message": "GenePoolWatcher stopped."})
        elif action == "trigger_now":
            summary = watcher.trigger_now()
            return success_response(
                {
                    "status": "checked",
                    "diversity_score": summary["diversity_score"],
                    "total_capsules": summary["total_capsules"],
                    "underrepresented_families": summary["underrepresented_families"],
                    "gap_subscriptions_added": summary["gap_subscriptions_added"],
                    "gap_subscriptions_removed": summary["gap_subscriptions_removed"],
                    "triggered": summary["triggered"],
                }
            )
        else:  # status
            from llm.gene_pool_io import get_gene_pool_diversity

            diversity = get_gene_pool_diversity()
            return success_response(
                {
                    "status": "ok",
                    "diversity_score": diversity.get("diversity_score", 0),
                    "total_capsules": diversity.get("capsule_count", 0),
                    "underrepresented_families": diversity.get("underrepresented_families", []),
                    "overrepresented_families": diversity.get("overrepresented_families", []),
                    "gap_subscriptions": [
                        {
                            "family": gs.family,
                            "enabled": gs.enabled,
                            "keywords": gs.keywords,
                        }
                        for gs in watcher.state.gap_subscriptions
                    ],
                }
            )
    except Exception as e:
        logger.error(f"gene_pool_watcher error: {e}")
        return error_response("WATCHER_ERROR", str(e))


def tool_claim_graph(
    action: str = "status",
    paper_id: Optional[str] = None,
    claim_type: Optional[str] = None,
    value: Optional[float] = None,
    source_text: Optional[str] = None,
    from_paper: Optional[str] = None,
    to_paper: Optional[str] = None,
    improvement_ratio: Optional[float] = None,
) -> Dict:
    """Manage cross-paper claim graph: status, add claims, find contradictions, render HTML."""
    try:
        from rairos_claimgraph_py import claim_graph_action_py as claim_graph_action

        result = claim_graph_action(
            action=action,
            paper_id=paper_id,
            claim_type=claim_type,
            value=value,
            source_text=source_text,
            from_paper=from_paper,
            to_paper=to_paper,
            improvement_ratio=improvement_ratio,
        )
        return success_response(result)
    except Exception as e:
        logger.error(f"claim_graph error: {e}")
        return error_response("CLAIM_GRAPH_ERROR", str(e))


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
        from llm.research.hypothesis_generator import HypothesisGenerator

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
        import uuid
        from llm.experiment_tracker import ExperimentTracker, ExperimentStatus, Experiment

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
        tracker.run(experiment)

        return success_response(
            {
                "experiment_id": exp_id,
                "hypothesis_id": hypothesis_id,
                "status": status.value,
                "message": "Experiment recorded: " + name + " -> " + result,
            }
        )

    except Exception as e:
        logger.error(f"experiment_record error: {e}")
        return error_response("EXPERIMENT_ERROR", str(e))


def tool_litreview_list() -> Dict:
    """List all saved literature reviews."""
    try:
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
        stance_enum = (
            StanceType(str(stance or "").lower())
            if str(stance or "").lower() in [e.value for e in StanceType.__members__.values()]
            else StanceType.DEFERRED
        )
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
                "created_at": datetime.datetime.fromtimestamp(s.created_at).isoformat(),
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
                        "created_at": datetime.datetime.fromtimestamp(s.created_at).isoformat(),
                        "updated_at": datetime.datetime.fromtimestamp(s.updated_at).isoformat(),
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
                        "created_at": datetime.datetime.fromtimestamp(a.created_at).isoformat(),
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
                        "created_at": datetime.datetime.fromtimestamp(a.created_at).isoformat(),
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
        from llm.review_simulator import ReviewSimulator, _REVIEW_PERSONAS

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
    hypothesis: str = "",
    goal: str = "",
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
                "created_at": datetime.datetime.fromtimestamp(plan.created_at).isoformat(),
            }
        )

    except Exception as e:
        logger.error(f"routeplan_create error: {e}")
        return error_response("PLAN_ERROR", str(e))


def tool_routeplan_list() -> Dict:
    """List all research plans."""
    try:
        from llm.route_planner import RoutePlanner

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
                        "created_at": datetime.datetime.fromtimestamp(p.created_at).isoformat(),
                        "updated_at": datetime.datetime.fromtimestamp(p.updated_at).isoformat(),
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

        try:
            status_enum = StepStatus(status.lower())
        except Exception:
            from llm.route_planner import StepStatus

            status_enum = StepStatus.PLANNED
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

        if name == "tag_all":
            # type: ignore[arg-type]
            result = tool_tag_all()
        elif name == "chart_query":
            result = tool_chart_query(  # type: ignore[arg-type]
                paper_id=arguments.get("paper_id"),
                action=arguments.get("action"),
                label=arguments.get("label"),
            )
        elif name == "gene_pool_decay":
            # type: ignore[arg-type]
            result = tool_gene_pool_decay(
                action=arguments.get("action", "status"),
                min_impact=arguments.get("min_impact", 0.1),
                lambda_=arguments.get("lambda", 0.01),
            )
        elif name == "crossover":
            # type: ignore[arg-type]
            result = tool_crossover(
                action=arguments.get("action", "evolve"),
                offspring_count=arguments.get("offspring_count", 5),
                capsule_id=arguments.get("capsule_id"),
            )
        elif name == "leaderboard":
            # type: ignore[arg-type]
            result = tool_leaderboard(
                action=arguments.get("action", "status"),
                arxiv_id=arguments.get("arxiv_id"),
                sort_by=arguments.get("sort_by", "combined"),
                limit=arguments.get("limit", 20),
            )
        elif name == "gene_pool_watcher":
            # type: ignore[arg-type]
            result = tool_gene_pool_watcher(
                action=arguments.get("action", "status"),
                interval_minutes=arguments.get("interval_minutes", 60),
                min_diversity_score=arguments.get("min_diversity_score", 50.0),
            )
        elif name == "claim_graph":
            # type: ignore[arg-type]
            result = tool_claim_graph(
                action=arguments.get("action", "status"),
                paper_id=arguments.get("paper_id"),
                claim_type=arguments.get("claim_type"),
                value=arguments.get("value"),
                source_text=arguments.get("source_text"),
                from_paper=arguments.get("from_paper"),
                to_paper=arguments.get("to_paper"),
                improvement_ratio=arguments.get("improvement_ratio"),
            )
        elif name == "research_run":
            result = tool_research_run(  # type: ignore[arg-type]
                topic=arguments.get("topic"), limit=arguments.get("limit", 5)
            )
        elif name == "cite_fetch":
            result = tool_cite_fetch(  # type: ignore[arg-type]
                paper_id=arguments.get("paper_id"), direction=arguments.get("direction", "both")
            )
        elif name == "paper_analyze":
            result = tool_paper_analyze(paper_id=arguments.get("paper_id"))  # type: ignore[arg-type]
        elif name == "paper2code_run":
            result = tool_paper2code_run(  # type: ignore[arg-type]
                arxiv_id=arguments.get("arxiv_id"),
                framework=arguments.get("framework", "pytorch"),
                skip_gene_pool=arguments.get("skip_gene_pool", False),
                continuous=arguments.get("continuous", False),
                interval_minutes=arguments.get("interval_minutes", 15),
            )
        elif name == "gap_submit":
            result = tool_gap_submit(  # type: ignore[arg-type]
                topic=arguments.get("topic"),
                gap_type=arguments.get("gap_type"),
                title=arguments.get("title"),
                description=arguments.get("description"),
                success_score=arguments.get("success_score", 0.8),
            )
        elif name == "gap_evolve":
            result = tool_gap_evolve(  # type: ignore[arg-type]
                topic=arguments.get("topic"), gap_type=arguments.get("gap_type")
            )
        elif name == "research_agent_start":
            # type: ignore[arg-type]
            result = tool_research_agent_start(
                interval_minutes=arguments.get("interval_minutes", 30)
            )
        elif name == "research_agent_stop":
            # type: ignore[arg-type]
            result = tool_research_agent_stop()
        elif name == "research_agent_status":
            # type: ignore[arg-type]
            result = tool_research_agent_status()
        elif name == "research_agent_trigger":
            # type: ignore[arg-type]
            result = tool_research_agent_trigger(topic=arguments.get("topic"))
        elif name == "hypothesis_generate":
            result = tool_hypothesis_generate(  # type: ignore[arg-type]
                topic=arguments.get("topic"),
                gap_context=arguments.get("gap_context", ""),
                gap_type=arguments.get("gap_type", ""),
                creative=arguments.get("creative", False),
            )
        elif name == "hypothesis_list":
            # type: ignore[arg-type]
            result = tool_hypothesis_list()
        elif name == "experiment_record":
            result = tool_experiment_record(  # type: ignore[arg-type]
                hypothesis_id=arguments.get("hypothesis_id"),
                name=arguments.get("name"),
                result=arguments.get("result"),
                metrics=arguments.get("metrics"),
            )
        elif name == "litreview_list":
            # type: ignore[arg-type]
            result = tool_litreview_list()
        elif name == "research_memory_add_stance":
            result = tool_research_memory_add_stance(  # type: ignore[arg-type]
                topic=arguments.get("topic"),
                claim=arguments.get("claim"),
                stance=arguments.get("stance"),
                evidence_refs=arguments.get("evidence_refs"),
                reasoning=arguments.get("reasoning", ""),
                confidence=arguments.get("confidence", 0.5),
            )
        elif name == "research_memory_list_stances":
            # type: ignore[arg-type]
            result = tool_research_memory_list_stances()
        elif name == "research_memory_check_paper":
            result = tool_research_memory_check_paper(  # type: ignore[arg-type]
                arxiv_id=arguments.get("arxiv_id"),
                use_llm=arguments.get("use_llm", True),
            )
        elif name == "research_memory_anomalies":
            # type: ignore[arg-type]
            result = tool_research_memory_anomalies()
        elif name == "review_simulate":
            result = tool_review_simulate(  # type: ignore[arg-type]
                arxiv_id=arguments.get("arxiv_id"),
                persona=arguments.get("persona", "all"),
                use_llm=arguments.get("use_llm", True),
            )
        elif name == "review_list":
            # type: ignore[arg-type]
            result = tool_review_list()
        elif name == "routeplan_create":
            result = tool_routeplan_create(  # type: ignore[arg-type]
                hypothesis=arguments.get("hypothesis"),
                goal=arguments.get("goal"),
                known_papers=arguments.get("known_papers"),
            )
        elif name == "routeplan_list":
            # type: ignore[arg-type]
            result = tool_routeplan_list()
        elif name == "routeplan_update_step":
            result = tool_routeplan_update_step(  # type: ignore[arg-type]
                plan_id=arguments.get("plan_id"),
                step_id=arguments.get("step_id"),
                status=arguments.get("status"),
                result=arguments.get("result", ""),
                notes=arguments.get("notes", ""),
            )
        elif name == "routeplan_revise":
            result = tool_routeplan_revise(  # type: ignore[arg-type]
                plan_id=arguments.get("plan_id"),
                reason=arguments.get("reason"),
            )
        elif name == "replication_compare":
            # type: ignore[arg-type]
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


def _handle_sampling(params: Dict) -> dict:
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
        return success_response(
            {
                "model": result.get("model", "mcp"),
                "role": "assistant",
                "content": result.get("content", ""),
            }
        )
    except Exception as e:
        logger.warning(f"LLM via key failed, using MCP degraded: {e}")
        last = messages[-1].get("content", "") if messages else ""
        return success_response(
            {
                "model": "mcp-degraded",
                "role": "assistant",
                "content": f"[MCP degraded - no API key needed] Query: {last[:200]}",
            }
        )
