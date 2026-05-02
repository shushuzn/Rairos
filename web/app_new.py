"""
Rairos — FastAPI + Jinja2 Hand-Drawn UI.
Run: uvicorn web.app_new:app --reload --port 8501
"""
from __future__ import annotations

import sys
from pathlib import Path

# Add project root to path
PROJECT_ROOT = Path(__file__).parent.parent.parent
sys.path.insert(0, str(PROJECT_ROOT))

from typing import Optional
from fastapi import FastAPI, Request, Form
from fastapi.responses import RedirectResponse
from fastapi.staticfiles import StaticFiles
from fastapi.templating import Jinja2Templates

app = FastAPI(title="Rairos", description="AI Research OS — Hand-drawn UI")

# Static files + templates
WEB_DIR = Path(__file__).parent
app.mount("/static", StaticFiles(directory=str(WEB_DIR / "static")), name="static")
templates = Jinja2Templates(directory=str(WEB_DIR / "templates"))

# Jinja filters
def _jinja_truncate(value, length=80):
    s = str(value)
    return s[:length] + "…" if len(s) > length else s

def _jinja_timestamp(value):
    from datetime import datetime
    try:
        return datetime.fromtimestamp(float(value)).strftime("%H:%M:%S")
    except Exception:
        return str(value)[:8]

templates.env.filters["truncate"] = _jinja_truncate
templates.env.filters["timestamp"] = _jinja_timestamp


# ════════════════════════════════════════════
# Database helper
# ════════════════════════════════════════════

def _get_db():
    from db.database import Database
    db = Database()
    db.init()
    return db


# ════════════════════════════════════════════
# Pages
# ════════════════════════════════════════════

@app.get("/")
async def dashboard(request: Request):
    """Dashboard — stats, charts, recent papers."""
    db = _get_db()
    stats = db.get_stats()

    rows, _ = db.list_papers(limit=8, sort_by="added_at", sort_order="desc")
    recent = []
    for r in rows:
        authors = ", ".join((r.authors or [])[:3])
        year = (r.published or "")[:4] if r.published else "?"
        pid = r.paper_id if hasattr(r, 'paper_id') and r.paper_id else (r.id or "")
        recent.append((pid, r.title, authors, year, r.source, f"/paper/{pid}"))

    # Queue jobs + papers being parsed
    queue_jobs = db.get_queue_jobs(limit=10)
    queue_list = []
    for row in queue_jobs:
        pid = row.paper_id or ""
        title = db.get_paper_title(pid) if pid else ""
        queue_list.append((
            pid,
            title[:70] if title else "(unknown)",
            row.status or "queued",
            row.job_type or "parse",
        ))

    # Papers currently parsing
    parsing_rows, _ = db.list_papers(limit=10, parse_status="running")
    parsing = [(r.id, r.title[:60], r.source) for r in parsing_rows]

    # Category distribution
    cur = db.conn.cursor()
    cur.execute("SELECT primary_category, COUNT(*) FROM papers WHERE primary_category != '' GROUP BY primary_category ORDER BY COUNT(*) DESC LIMIT 8")
    by_category = tuple((r[0] or "uncategorized", r[1]) for r in cur.fetchall())

    # Activity: papers added in last 7 days
    cur.execute("SELECT id, title, added_at FROM papers WHERE added_at != '' ORDER BY added_at DESC LIMIT 7")
    activity = []
    for r in cur.fetchall():
        pid = getattr(r, 'paper_id', None) or r.id if hasattr(r, 'id') else r[0]
        added = r[2] or ""
        date_str = added[:10] if added else "?"
        activity.append((r[0], r[1][:60], date_str))

    # Flatten stats — convert unhashable dicts to tuples so Jinja2 LRU cache works
    stats_flat = {
        "total_papers": stats.get("total_papers", 0),
        "parsed": stats.get("by_status", {}).get("parsed", 0),
        "idle": stats.get("by_status", {}).get("idle", 0),
        "pending": stats.get("by_status", {}).get("pending", 0),
        "failed": stats.get("by_status", {}).get("failed", 0),
        "queued": stats.get("queue_queued", 0) + stats.get("queue_running", 0),
        "cache_entries": stats.get("cache_entries", 0),
        "by_source": tuple((k, v) for k, v in stats.get("by_source", {}).items()),
        "by_status": tuple((k, v) for k, v in stats.get("by_status", {}).items()),
    }

    return templates.TemplateResponse(
        request,
        "dashboard.html",
        {"page": "dashboard", "stats": stats_flat, "recent": recent,
         "queue_list": queue_list, "parsing": parsing,
         "by_category": by_category, "activity": activity},
    )


@app.get("/papers")
async def papers(request: Request, q: str = "", source: str = "", page: int = 1,
                 year_from: str = "", year_to: str = ""):
    """Papers — search and list with pagination."""
    db = _get_db()
    limit = 20
    offset = (max(1, page) - 1) * limit

    # Build date filters from year range
    date_from = f"{year_from}-01-01" if year_from else None
    date_to = f"{year_to}-12-31" if year_to else None

    if q:
        rows, total = db.search_papers(q, limit=limit, offset=offset)
    else:
        rows, total = db.list_papers(
            limit=limit, offset=offset,
            source=source if source else None,
            date_from=date_from, date_to=date_to,
        )

    papers_list = []
    for r in rows:
        authors = ", ".join((r.authors or [])[:3])
        year = (r.published or "")[:4] if r.published else "?"
        snippet = getattr(r, 'snippet', '') or (r.abstract or "")[:150]
        pid = getattr(r, 'paper_id', None) or getattr(r, 'id', '') or ''
        papers_list.append((
            pid, r.title, authors, year, r.source,
            getattr(r, 'primary_category', '') or "",
            snippet, f"/paper/{pid}",
        ))

    total_pages = max(1, (total + limit - 1) // limit)

    return templates.TemplateResponse(request, "papers.html", {
        "page": "papers",
        "papers": papers_list,
        "query": q,
        "total": total,
        "page": page,
        "total_pages": total_pages,
        "year_from": year_from,
        "year_to": year_to,
    })


@app.get("/paper/{paper_id}")
async def paper_detail(request: Request, paper_id: str):
    """Paper detail — full metadata."""
    db = _get_db()
    paper = db.get_paper(paper_id)

    if not paper:
        return templates.TemplateResponse(request, "paper_detail.html", {
            "page": "paper",
            "paper": None,
            "paper_id": paper_id,
            "error": f"Paper '{paper_id}' not found.",
        })

    authors = ", ".join((paper.authors or [])[:10])
    year = (paper.published or "")[:4] if paper.published else "?"
    all_categories = (paper.categories or "").split(",")[:8]

    # Gene Pool matches
    from llm.briefing_generator import _match_gene_pool
    gene_matches_raw = _match_gene_pool(paper.id, paper.title, paper.abstract or "")
    gene_matches = tuple(
        (m["gap_title"], m["gap_type"], m["outcome_score"], m["match_reason"])
        for m in gene_matches_raw
    )

    # Flatten to hashable tuple
    paper_tuple = (
        paper.id,
        paper.title,
        authors,
        year,
        paper.source,
        paper.published or "",
        paper.primary_category or "",
        paper.abstract or "",
        paper.abs_url or "",
        paper.pdf_url or "",
        paper.journal or "",
        paper.doi or "",
        paper.reference_count or 0,
        paper.citation_count if hasattr(paper, 'citation_count') and paper.citation_count else 0,
        paper.page_count or 0,
        paper.word_count or 0,
        paper.parse_status or "unknown",
        paper.reading_status or "unread",
        paper.added_at or "",
        all_categories,
    )

    return templates.TemplateResponse(request, "paper_detail.html", {
        "page": "paper",
        "paper": paper_tuple,
        "paper_id": paper_id,
        "error": None,
        "gene_matches": gene_matches,
    })


@app.get("/paper/{paper_id}/extract-gap")
async def extract_paper_gap(request: Request, paper_id: str):
    """Extract a research gap from a paper using LLM."""
    db = _get_db()
    paper = db.get_paper(paper_id)
    if not paper:
        return {"error": f"Paper '{paper_id}' not found."}

    from llm.paper_gap_extractor import extract_gap_from_paper
    result = extract_gap_from_paper(
        paper_id=paper_id,
        title=paper.title,
        abstract=paper.abstract or "",
        authors=paper.authors,
    )
    return result


@app.post("/paper/{paper_id}/save-gap")
async def save_paper_gap(request: Request, paper_id: str):
    """Save an extracted gap to the Gene Pool."""
    body = await request.json()
    gap_type = body.get("gap_type", "")
    gap_title = body.get("gap_title", "")
    keywords = body.get("keywords", [])
    summary = body.get("summary", "")

    from llm.paper_gap_extractor import save_gap_to_gene_pool
    db = _get_db()
    paper = db.get_paper(paper_id)
    title = paper.title if paper else paper_id

    success = save_gap_to_gene_pool(
        paper_id=paper_id,
        title=title,
        gap_type=gap_type,
        gap_title=gap_title,
        keywords=keywords,
        summary=summary,
    )
    return {"success": success}


@app.get("/briefing")
async def briefing(request: Request, arxiv_id: str = ""):
    """Research Briefing — generate or show."""
    return templates.TemplateResponse(request, "briefing.html", {
        "page": "briefing",
        "arxiv_id": arxiv_id,
        "result": None,
        "error": None,
    })


@app.post("/briefing")
async def briefing_generate(
    request: Request,
    arxiv_id: str = Form(""),
    use_llm: bool = Form(True),
):
    """Generate a research briefing."""
    if not arxiv_id.strip():
        return templates.TemplateResponse(request, "briefing.html", {
            "page": "briefing",
            "arxiv_id": arxiv_id,
            "result": None,
            "error": "Please enter an arXiv ID.",
        })

    try:
        from llm.briefing_generator import BriefingGenerator
        gen = BriefingGenerator()
        result = gen.generate(
            arxiv_id=arxiv_id.strip(),
            use_llm=use_llm,
            output_dir=PROJECT_ROOT / "data" / "briefings",
        )

        if result.success:
            return templates.TemplateResponse(request, "briefing.html", {
                "page": "briefing",
                "arxiv_id": arxiv_id,
                "result": result,
                "error": None,
            })
        else:
            return templates.TemplateResponse(request, "briefing.html", {
                "page": "briefing",
                "arxiv_id": arxiv_id,
                "result": None,
                "error": result.error,
            })
    except Exception as e:
        return templates.TemplateResponse(request, "briefing.html", {
            "page": "briefing",
            "arxiv_id": arxiv_id,
            "result": None,
            "error": str(e),
        })


@app.get("/briefing/history")
async def briefing_history(request: Request):
    """List all previously generated briefings."""
    briefings_dir = PROJECT_ROOT / "data" / "briefings"
    briefings = []

    if briefings_dir.exists():
        for f in sorted(briefings_dir.glob("briefing_*.md"), reverse=True):
            try:
                text = f.read_text(encoding="utf-8")
                lines = text.split("\n", 4)
                # Line 0: # Research Briefing: Title
                title = lines[0][len("# Research Briefing: "):] if len(lines) > 0 and lines[0].startswith("# Research Briefing: ") else f.stem
                # Line 1: metadata — "**arXiv:** [id](url) | **Authors:** ... | **Generated:** ..."
                arxiv_id = ""
                generated = ""
                if len(lines) > 1:
                    import re
                    aid_match = re.search(r'\*\*arXiv:\*\* \[([^\]]+)\]', lines[1])
                    if aid_match:
                        arxiv_id = aid_match.group(1)
                    gen_match = re.search(r'\*\*Generated:\*\* (\S+)', lines[1])
                    if gen_match:
                        generated = gen_match.group(1)
                verdict = ""
                verdict_emoji = ""
                if len(lines) > 2:
                    v_match = re.search(r'\*\*Verdict:\*\* (.) \*\*([A-Z]+)\*\*', lines[2])
                    if v_match:
                        verdict_emoji = v_match.group(1)
                        verdict = v_match.group(2).lower()
                briefings.append((
                    arxiv_id,
                    title[:90],
                    generated,
                    verdict,
                    verdict_emoji,
                ))
            except Exception:
                pass

    return templates.TemplateResponse(request, "briefing_history.html", {
        "page": "briefing-history",
        "briefings": briefings,
    })


@app.get("/citation-chain")
async def citation_chain(request: Request, arxiv_id: str = ""):
    """Citation Chain — build and visualize."""
    return templates.TemplateResponse(request, "citation_chain.html", {
        "page": "citation_chain",
        "arxiv_id": arxiv_id,
        "chain_data": None,
        "error": None,
    })


@app.post("/citation-chain")
async def citation_chain_build(
    request: Request,
    arxiv_id: str = Form(""),
    max_depth: int = Form(2),
    fmt: str = Form("mermaid"),
):
    """Build a citation chain."""
    if not arxiv_id.strip():
        return templates.TemplateResponse(request, "citation_chain.html", {
            "page": "citation_chain",
            "arxiv_id": arxiv_id,
            "chain_data": None,
            "error": "Please enter an arXiv ID.",
        })

    try:
        from llm.citation_chain import CitationChainBuilder
        builder = CitationChainBuilder()
        chain = builder.build_chain(seed_arxiv_id=arxiv_id.strip(), max_depth=max_depth)

        if fmt == "text":
            rendered = builder.render_text(chain)
        elif fmt == "graphviz":
            rendered = builder.render_graphviz(chain)
        else:
            rendered = builder.render_mermaid(chain)

        return templates.TemplateResponse(request, "citation_chain.html", {
            "page": "citation_chain",
            "arxiv_id": arxiv_id,
            "chain_data": {
                "nodes": [(n.paper_id, n.title, n.year, n.citation_count) for n in chain.nodes],
                "edges": chain.edges,
                "rendered": rendered,
                "fmt": fmt,
                "nodes_count": len(chain.nodes),
                "edges_count": len(chain.edges),
            },
            "error": None,
        })
    except Exception as e:
        return templates.TemplateResponse(request, "citation_chain.html", {
            "page": "citation_chain",
            "arxiv_id": arxiv_id,
            "chain_data": None,
            "error": str(e),
        })


# ── Research Loop ────────────────────────────────────────────────────────────────

@app.get("/research-loop")
async def research_loop(request: Request):
    """Research Loop dashboard — status, alerts, subscriptions."""
    try:
        from research_loop.orchestrator import AutonomousOrchestrator
        orch = AutonomousOrchestrator(webhook_enabled=False)
        status = orch.get_status()
        alerts_raw = orch.get_recent_alerts(limit=20)
    except Exception:
        status = {"running": False, "interval_minutes": 30, "last_check": "", "alerts_count": 0, "error": "Orchestrator unavailable"}
        alerts_raw = []

    alerts = []
    for a in alerts_raw:
        created = a.created_at if hasattr(a, 'created_at') else a.get('created_at', '')
        alerts.append((
            a.alert_id,
            a.session_id,
            a.topic,
            a.triggered_by,
            a.trigger_title,
            a.gaps_found,
            a.top_gap_title,
            a.top_gap_type,
            a.severity,
            a.gene_pool_score,
            a.preference_boost,
            created,
        ))

    try:
        db = _get_db()
        subs_raw = db.list_arxiv_subscriptions()
    except Exception:
        subs_raw = []

    subscriptions = []
    for s in subs_raw:
        subscriptions.append({
            "id": s.get("id", ""),
            "topic": s.get("topic", ""),
        })

    return templates.TemplateResponse(request, "research_loop.html", {
        "page": "research-loop",
        "status": status,
        "alerts": alerts,
        "subscriptions": subscriptions,
    })


@app.post("/research-loop/start")
async def research_loop_start(request: Request):
    """Start the autonomous watch loop."""
    try:
        from research_loop.orchestrator import AutonomousOrchestrator
        orch = AutonomousOrchestrator(webhook_enabled=False)
        orch.start_watch(interval_minutes=30)
    except Exception as e:
        import logging
        logging.getLogger(__name__).warning(f"Could not start orchestrator: {e}")
    return RedirectResponse(url="/research-loop", status_code=303)


@app.post("/research-loop/stop")
async def research_loop_stop(request: Request):
    """Stop the autonomous watch loop."""
    try:
        from research_loop.orchestrator import AutonomousOrchestrator
        orch = AutonomousOrchestrator(webhook_enabled=False)
        orch.stop_watch()
    except Exception:
        pass
    return RedirectResponse(url="/research-loop", status_code=303)


@app.post("/research-loop/run-cycle")
async def research_loop_run_cycle(request: Request):
    """Manually trigger one orchestrator cycle."""
    import threading
    def run():
        try:
            from research_loop.orchestrator import AutonomousOrchestrator
            orch = AutonomousOrchestrator(webhook_enabled=False)
            orch.run_cycle()
        except Exception as e:
            import logging
            logging.getLogger(__name__).error(f"Run cycle failed: {e}")
    threading.Thread(target=run, daemon=True).start()
    return RedirectResponse(url="/research-loop", status_code=303)


# ── Squad Coordinator ────────────────────────────────────────────────────────────

@app.get("/research-loop/squad")
async def squad_dashboard(request: Request):
    """Squad dashboard — multi-agent activity stream."""
    try:
        from research_loop.agents.squad import SquadCoordinator
        coord = SquadCoordinator()
        squad_status = coord.get_status()
        activity = coord.get_activity(limit=50)
        alerts = coord.get_alerts(limit=20)
    except Exception as e:
        squad_status = {"running": False, "agents": {}, "error": str(e), "interval_minutes": 30, "last_cycle": ""}
        activity = []
        alerts = []

    return templates.TemplateResponse(request, "squad_dashboard.html", {
        "page": "squad-dashboard",
        "squad_status": squad_status,
        "activity": activity,
        "alerts": alerts,
    })


@app.post("/research-loop/squad/start")
async def squad_start(request: Request):
    """Start the multi-agent squad."""
    try:
        from research_loop.agents.squad import SquadCoordinator
        coord = SquadCoordinator()
        coord.start_watch(interval_minutes=30)
    except Exception as e:
        import logging
        logging.getLogger(__name__).warning(f"Could not start squad: {e}")
    return RedirectResponse(url="/research-loop/squad", status_code=303)


@app.post("/research-loop/squad/stop")
async def squad_stop(request: Request):
    """Stop the multi-agent squad."""
    try:
        from research_loop.agents.squad import SquadCoordinator
        coord = SquadCoordinator()
        coord.stop_watch()
    except Exception:
        pass
    return RedirectResponse(url="/research-loop/squad", status_code=303)


@app.post("/research-loop/squad/run-cycle")
async def squad_run_cycle(request: Request):
    """Manually trigger one squad cycle."""
    import threading
    def run():
        try:
            from research_loop.agents.squad import SquadCoordinator
            coord = SquadCoordinator()
            coord.run_cycle()
        except Exception as e:
            import logging
            logging.getLogger(__name__).error(f"Squad cycle failed: {e}")
    threading.Thread(target=run, daemon=True).start()
    return RedirectResponse(url="/research-loop/squad", status_code=303)


@app.get("/research-loop/squad/activity")
async def squad_activity():
    """JSON endpoint for squad activity stream."""
    try:
        from research_loop.agents.squad import SquadCoordinator
        coord = SquadCoordinator()
        return {
            "activity": coord.get_activity(limit=50),
            "status": coord.get_status(),
        }
    except Exception as e:
        return {"activity": [], "error": str(e)}


@app.get("/impact")
async def impact(request: Request):
    """Impact Ranking — leaderboard."""
    db = _get_db()
    rows, _ = db.list_papers(limit=50, sort_by="citation_count", sort_order="desc")

    papers = []
    for r in rows:
        papers.append({
            "paper_id": getattr(r, 'paper_id', '') or getattr(r, 'id', ''),
            "title": r.title,
            "year": getattr(r, 'year', 2020) or 2020,
            "citation_count": getattr(r, 'citation_count', 0) or 0,
        })

    try:
        from llm.impact_scorer import ImpactScorer
        scorer = ImpactScorer(db=db)
        ranking = scorer.rank_papers(papers, top_k=min(20, len(papers)))
    except Exception:
        ranking = []

    return templates.TemplateResponse(request, "impact.html", {
        "page": "impact",
        "ranking": ranking,
    })


@app.get("/streamlit")
async def streamlit_redirect():
    """Redirect to the full Streamlit app."""
    return RedirectResponse(url="http://localhost:8501", status_code=307)


if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=8501)
