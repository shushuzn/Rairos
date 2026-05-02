"""
Rairos — FastAPI + Jinja2 Hand-Drawn UI.
Run: uvicorn web.app_new:app --reload --port 8501
"""
from __future__ import annotations

import json
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


@app.get("/research-loop/squad/stream")
async def squad_stream():
    """Server-Sent Events — real-time agent activity stream."""
    from fastapi.responses import StreamingResponse
    import asyncio

    async def event_generator():
        import time as _time
        _last_len = 0
        try:
            from research_loop.agents.squad import SquadCoordinator
            coord = SquadCoordinator()
            while True:
                activity = coord.get_activity(limit=50)
                status = coord.get_status()
                alerts = coord.get_alerts(limit=10)

                # Only emit if something changed
                if len(activity) != _last_len:
                    _last_len = len(activity)
                    payload = {
                        "activity": activity[-20:],
                        "agents": status.get("agents", {}),
                        "alerts": alerts,
                        "running": status.get("running", False),
                    }
                    yield f"data: {__import__('json').dumps(payload)}\n\n"

                await asyncio.sleep(3)
        except Exception as e:
            yield f"data: {__import__('json').dumps({'error': str(e)})}\n\n"

    return StreamingResponse(
        event_generator(),
        media_type="text/event-stream",
        headers={
            "Cache-Control": "no-cache",
            "Connection": "keep-alive",
            "X-Accel-Buffering": "no",
        },
    )


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


@app.get("/insights")
async def insights(request: Request):
    """Research Insights — Gene Pool knowledge, user archetype, exploration history."""
    try:
        from llm.insight.tracker import EvolutionTracker
        tracker = EvolutionTracker()

        # Gene Pool capsules
        capsules_path = Path.home() / ".ai_research_os" / "gene_pool" / "capsules.json"
        capsules = []
        if capsules_path.exists():
            data = json.loads(capsules_path.read_text(encoding="utf-8"))
            raw = data.get("capsules", []) if isinstance(data, dict) else data
            for c in raw[-20:]:  # newest 20
                status = c.get("status", "active")
                capsules.append((
                    c.get("capsule_id", "")[:12],
                    c.get("trigger_topic", "")[:60],
                    c.get("action_gap_type", ""),
                    c.get("action_gap_title", "")[:80],
                    c.get("outcome_success_score", 0.0),
                    c.get("created_at", "")[:10],
                    c.get("trigger_keywords", [])[:5],
                    status,
                ))

        # Gene Pool stats from tracker
        stats = tracker.get_gene_pool_stats()

        # User archetype
        archetype = tracker.get_archetype()

        # Top gap type preferences
        profile = tracker.get_profile()
        gap_prefs = dict(sorted(
            (profile.gap_type_preferences or {}).items(),
            key=lambda x: x[1], reverse=True
        ))

        # Top topics
        topic_freq = dict(sorted(
            (profile.topic_frequency or {}).items(),
            key=lambda x: x[1], reverse=True
        )[:8])

        # Recent events (last 15)
        recent_events = tracker.get_recent_events(limit=15)
        events_display = []
        for e in reversed(recent_events):
            ts = e.timestamp[11:16] if e.timestamp else ""
            date = e.timestamp[:10] if e.timestamp else ""
            events_display.append((
                ts, date,
                e.action.value if hasattr(e.action, 'value') else str(e.action),
                e.topic[:40] if e.topic else "—",
                e.gap_type or "—",
                e.gap_title[:50] if e.gap_title else "—",
            ))

        # Exploration stats
        exp_stats = tracker.get_exploration_stats()

        # ── Actionable Project Suggestions ────────────────────────────────────────
        # Analyze Gene Pool patterns to generate concrete next-step suggestions
        suggestions = _generate_suggestions(capsules, gap_prefs, topic_freq, archetype, tracker)

    except Exception as e:
        capsules, stats, archetype, gap_prefs, topic_freq, events_display, exp_stats = [], {}, {}, {}, {}, [], {}
        import logging
        logging.getLogger(__name__).warning(f"Insights unavailable: {e}")

    return templates.TemplateResponse(request, "insights.html", {
        "page": "insights",
        "capsules": capsules,
        "gene_pool_stats": stats,
        "archetype": archetype,
        "gap_prefs": gap_prefs,
        "topic_freq": topic_freq,
        "events": events_display,
        "exp_stats": exp_stats,
        "suggestions": suggestions,
    })


@app.post("/insights/accept-suggestion")
async def accept_suggestion(request: Request):
    """Accept an actionable suggestion — record it as a gap acceptance event.

   闭环: marks the source capsule as 'consumed' so it won't repeat.
    """
    body = await request.json()
    topic = body.get("topic", "")
    gap_type = body.get("gap_type", "")
    gap_title = body.get("title", "")
    description = body.get("body", "")
    s_type = body.get("type", "")
    source_cap_id = body.get("source_cap_id") or None

    try:
        from llm.insight.tracker import EvolutionTracker
        tracker = EvolutionTracker()
        tracker.record_gap_accept(
            topic=topic or "insights",
            gap_type=gap_type,
            gap_title=gap_title[:200],
            gap_description=description,
        )
        # Also encode as a capsule
        tracker.encode_capsule(
            topic=topic or "insights",
            gap_type=gap_type,
            gap_title=gap_title[:200],
            gap_description=description,
            success_score=0.7,
        )
        _mark_suggestion_consumed(gap_type, topic, gap_title, s_type)

        # Mark source capsule as 'consumed' — prevents duplicate suggestions
        if source_cap_id:
            _mark_capule_consumed(source_cap_id, tracker)

        # Trigger evolution闭环 — this is what actually IMPROVES the Gene Pool
        try:
            from llm.insight.evolution import InsightEvolution
            evo = InsightEvolution(tracker=tracker)
            evo_result = evo.evolve(topic=topic or "insights")
            improved = evo_result.get("result", {}).get("added", 0)
            return {"success": True, "evolved": improved}
        except Exception as evo_err:
            import logging
            logging.getLogger(__name__).warning(f"Evolution trigger failed: {evo_err}")
            return {"success": True, "evolved": 0}
    except Exception as e:
        import logging
        logging.getLogger(__name__).warning(f"accept_suggestion failed: {e}")
        return {"success": False, "error": str(e)}

@app.post("/insights/archive-capsule")
async def archive_capsule(request: Request):
    """Archive a capsule from both Gene Pool stores (active → archived)."""
    body = await request.json()
    capsule_id = body.get("capsule_id", "")
    if not capsule_id:
        return {"success": False, "error": "capsule_id required"}
    try:
        from llm.insight.tracker import EvolutionTracker
        tracker = EvolutionTracker()
        archived = tracker.archive_capsule(capsule_id)
        return {"success": archived}
    except Exception as e:
        import logging
        logging.getLogger(__name__).warning(f"archive_capsule failed: {e}")
        return {"success": False, "error": str(e)}


# Gap types the user has NOT explored yet but might find valuable
_UNDERREPRESENTED_GAPS = [
    ("theoretical_gap", "Theoretical foundations", "Develop formal theory or proofs for observed empirical patterns in your work"),
    ("dataset_gap", "Dataset gap", "Build or curate a benchmark dataset addressing an under-explored problem domain"),
    ("generalization_gap", "Generalization gap", "Test existing methods on out-of-distribution data to expose failure modes"),
    ("scalability_issue", "Scalability issue", "Push current methods to larger scales and characterize runtime/cost tradeoffs"),
    ("contradiction", "Contradiction", "Reproduce or challenge published findings in this area"),
    ("evaluation_gap", "Evaluation gap", "Design proper evaluation protocols and baselines for this problem"),
]


def _get_consumed_suggestions() -> set:
    """Return set of suggestion keys that have been consumed by the user."""
    try:
        path = Path.home() / ".ai_research_os" / "consumed_suggestions.json"
        if path.exists():
            import json
            return set(json.loads(path.read_text(encoding="utf-8")))
    except Exception:
        pass
    return set()


def _mark_capule_consumed(capsule_id: str, tracker) -> None:
    """Mark a capsule as consumed in both Gene Pool stores."""
    try:
        # Update gene_pool.jsonl
        capsules = tracker._load_capsules()
        updated = False
        for c in capsules:
            if c.capsule_id == capsule_id:
                c.status = "consumed"
                updated = True
                break
        if updated:
            tracker._save_capsules(capsules)

        # Update capsules.json (web UI store)
        capsules_path = Path.home() / ".ai_research_os" / "gene_pool" / "capsules.json"
        if capsules_path.exists():
            try:
                data = json.loads(capsules_path.read_text(encoding="utf-8"))
                raw = data.get("capsules", []) if isinstance(data, dict) else data
                for c in raw:
                    if c.get("capsule_id", "") == capsule_id:
                        c["status"] = "consumed"
                        break
                data["capsules"] = raw
                capsules_path.write_text(json.dumps(data, indent=2, ensure_ascii=False), encoding="utf-8")
            except Exception:
                pass
    except Exception:
        pass


def _mark_suggestion_consumed(gap_type: str, topic_hint: str, title: str, s_type: str = "") -> None:
    """Mark a suggestion as consumed so it won't appear again."""
    try:
        import json
        consumed = _get_consumed_suggestions()
        if s_type == "archetype_advice":
            key = f"archetype:{title}"
        else:
            key = f"{gap_type}:{topic_hint[:20]}:{title[:30]}"
        consumed.add(key)
        path = Path.home() / ".ai_research_os" / "consumed_suggestions.json"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(list(consumed), ensure_ascii=False), encoding="utf-8")
    except Exception:
        pass


def _generate_suggestions(capsules, gap_prefs, topic_freq, archetype, tracker) -> list:
    """Analyze Gene Pool patterns and generate actionable project suggestions."""
    suggestions = []

    if not capsules and not gap_prefs:
        return []

    consumed = _get_consumed_suggestions()
    explored_gaps = set(gap_prefs.keys())
    explored_topics = set(topic_freq.keys())

    # 1. High-performing gap types with low exploration → suggest exploring them
    high_score_gaps = {k: v for k, v in gap_prefs.items() if v > 0.3}
    suggested_gap_types = set()  # avoid duplicate suggestions
    for gap_type, score in high_score_gaps.items():
        for candidate_gap, label, description in _UNDERREPRESENTED_GAPS:
            if candidate_gap not in explored_gaps and candidate_gap not in suggested_gap_types:
                # Suggest applying this successful gap type to a new domain
                suggestions.append({
                    "type": "explore_new_gap",
                    "icon": "🔍",
                    "title": f"Explore {label} in your research",
                    "body": f"You've had success with {gap_type} (score {score:.2f}). "
                             f"Consider investigating {description.lower()}.",
                    "gap_type": candidate_gap,
                    "confidence": min(score, 0.9),
                    "topic_hint": list(explored_topics)[0] if explored_topics else "your field",
                    "consumed": False,
                })
                suggested_gap_types.add(candidate_gap)
                break

    # 2. Top topics with no evaluation_gap explored → suggest evaluation
    top_topics = list(topic_freq.items())[:3]
    evaluated = [g for g in explored_gaps if "evaluation" in g or "benchmark" in g]
    if not evaluated and top_topics:
        topic_name = top_topics[0][0][:40]
        suggestions.append({
            "type": "evaluation_gap",
            "icon": "📏",
            "title": f"Evaluate {topic_name} rigorously",
            "body": f"You've explored '{topic_name}' ({topic_freq[topic_name]}×) "
                     "but haven't investigated evaluation gaps. "
                     "Proper benchmarks and controlled experiments could unlock significant improvements.",
            "gap_type": "evaluation_gap",
            "confidence": 0.7,
            "topic_hint": topic_name,
        })

    # 3. From capsule keywords — find high-scoring capsules and suggest projects
    high_perf_capsules = [c for c in capsules if len(c) >= 5 and c[4] >= 0.7]
    if high_perf_capsules:
        best = high_perf_capsules[0]  # top capsule by score
        cap_id, topic, gap_type, gap_title, score, date, keywords, status = best
        suggestions.append({
            "type": "build_on_success",
            "icon": "🚀",
            "title": f"Build on: {gap_title[:60]}",
            "body": f"This pattern scored {score*100:.0f}% success. "
                     "Try extending it: add more keywords, test in adjacent domains, "
                     "or compose with another high-performing capsule.",
            "gap_type": gap_type,
            "confidence": score,
            "topic_hint": topic[:40],
            "keywords": keywords,
            "consumed": False,
            "source_cap_id": cap_id,
        })

    # 4. Dominant archetype-driven suggestion
    arch_label = archetype.get("archetype_label", "")
    arch_dim = archetype.get("dominant", "")
    if arch_dim == "method_focused":
        suggestions.append({
            "type": "archetype_advice",
            "icon": "⚙️",
            "title": "Your archetype: Method Hunter",
            "body": "Focus on novel architectures, training procedures, or inference optimizations. "
                    "Look for published methods with surprising results and improve or extend them.",
            "gap_type": "method_limitation",
            "confidence": archetype.get("confidence", 0.5),
            "topic_hint": list(explored_topics)[0][:40] if explored_topics else "ML",
            "consumed": False,
        })
    elif arch_dim == "high_risk":
        suggestions.append({
            "type": "archetype_advice",
            "icon": "🧗",
            "title": "Your archetype: Risk Taker",
            "body": "Pursue high-uncertainty problems with high payoff: "
                    "new domains, controversial claims, unproven scalability. "
                    "Your profile suggests you can handle the volatility.",
            "gap_type": "unexplored_application",
            "confidence": archetype.get("confidence", 0.5),
            "topic_hint": list(explored_topics)[0][:40] if explored_topics else "research",
            "consumed": False,
        })

    # 5. Cross-domain suggestion if applicable
    if archetype.get("dimensions", {}).get("cross_domain", (0, 0, "", ""))[1] >= 0.3:
        suggestions.append({
            "type": "cross_domain",
            "icon": "🌉",
            "title": "Bridge domains with your cross-domain profile",
            "body": "Your research spans multiple areas. Try combining RL concepts with "
                    "transformer architectures, or apply your NLP insights to graph problems.",
            "gap_type": "generalization_gap",
            "confidence": 0.65,
            "topic_hint": list(explored_topics)[0][:40] if explored_topics else "interdisciplinary",
            "consumed": False,
        })

    # Filter out already-consumed suggestions
    # For archetype advice, deduplicate by type so consuming "Your archetype: Risk Taker"
    # once removes it regardless of which topic it's paired with
    filtered = []
    for s in suggestions:
        if s["type"] == "archetype_advice":
            key = f"archetype:{s['title']}"
        else:
            key = f"{s['gap_type']}:{s.get('topic_hint','')[:20]}:{s.get('title','')[:30]}"
        if key not in consumed:
            filtered.append(s)

    return sorted(filtered, key=lambda s: s.get("confidence", 0), reverse=True)[:5]


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
