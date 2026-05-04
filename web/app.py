"""
Rairos — FastAPI + Jinja2 Hand-Drawn UI.
Run: uvicorn web.app:app --reload --port 8501
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

# Add project root to path
PROJECT_ROOT = Path(__file__).parent.parent
sys.path.insert(0, str(PROJECT_ROOT))

from typing import Optional
from fastapi import FastAPI, Request, Form
from fastapi.responses import RedirectResponse
from fastapi.staticfiles import StaticFiles
from fastapi.templating import Jinja2Templates

from web.shared import templates, get_db, get_tracker, p2c_progress
from web.renderers import render_gene_pool_graph_html
from web.suggestions import (
    generate_suggestions,
    mark_capsule_consumed,
    _get_consumed_suggestions,
    _mark_suggestion_consumed,
    get_experiment_queue,
    save_experiment,
    render_experiments_html,
)

app = FastAPI(title="Rairos", description="AI Research OS — Hand-drawn UI")


# Auth middleware — skip if auth not enabled
@app.middleware("http")
async def auth_middleware(request: Request, call_next):
    from llm.auth import is_auth_enabled, validate_session

    if not is_auth_enabled():
        return await call_next(request)
    # Skip auth routes
    if request.url.path.startswith("/auth"):
        return await call_next(request)
    if request.url.path.startswith("/static"):
        return await call_next(request)
    token = request.cookies.get("session_token") or request.headers.get("X-Session-Token", "")
    username = validate_session(token) if token else None
    if not username:
        return RedirectResponse(url="/auth/login", status_code=303)
    request.state.username = username
    return await call_next(request)


# Static files + templates
WEB_DIR = Path(__file__).parent
app.mount("/static", StaticFiles(directory=str(WEB_DIR / "static")), name="static")
app.mount(
    "/data/briefings",
    StaticFiles(directory=str(PROJECT_ROOT / "data" / "briefings")),
    name="briefings",
)
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
        pid = r.paper_id if hasattr(r, "paper_id") and r.paper_id else (r.id or "")
        recent.append((pid, r.title, authors, year, r.source, f"/paper/{pid}"))

    # Queue jobs + papers being parsed
    queue_jobs = db.get_queue_jobs(limit=10)
    queue_list = []
    for row in queue_jobs:
        pid = row.paper_id or ""
        title = db.get_paper_title(pid) if pid else ""
        queue_list.append(
            (
                pid,
                title[:70] if title else "(unknown)",
                row.status or "queued",
                row.job_type or "parse",
            )
        )

    # Papers currently parsing
    parsing_rows, _ = db.list_papers(limit=10, parse_status="running")
    parsing = [(r.id, r.title[:60], r.source) for r in parsing_rows]

    # Category distribution
    cur = db.conn.cursor()
    cur.execute(
        "SELECT primary_category, COUNT(*) FROM papers WHERE primary_category != '' GROUP BY primary_category ORDER BY COUNT(*) DESC LIMIT 8"
    )
    by_category = tuple((r[0] or "uncategorized", r[1]) for r in cur.fetchall())

    # Activity: papers added in last 7 days
    cur.execute(
        "SELECT id, title, added_at FROM papers WHERE added_at != '' ORDER BY added_at DESC LIMIT 7"
    )
    activity = []
    for r in cur.fetchall():
        pid = getattr(r, "paper_id", None) or r.id if hasattr(r, "id") else r[0]
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
        {
            "page": "dashboard",
            "stats": stats_flat,
            "recent": recent,
            "queue_list": queue_list,
            "parsing": parsing,
            "by_category": by_category,
            "activity": activity,
        },
    )


@app.get("/papers")
async def papers(
    request: Request,
    q: str = "",
    source: str = "",
    page: int = 1,
    year_from: str = "",
    year_to: str = "",
):
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
            limit=limit,
            offset=offset,
            source=source if source else None,
            date_from=date_from,
            date_to=date_to,
        )

    papers_list = []
    for r in rows:
        authors = ", ".join((r.authors or [])[:3])
        year = (r.published or "")[:4] if r.published else "?"
        snippet = getattr(r, "snippet", "") or (r.abstract or "")[:150]
        pid = getattr(r, "paper_id", None) or getattr(r, "id", "") or ""
        papers_list.append(
            (
                pid,
                r.title,
                authors,
                year,
                r.source,
                getattr(r, "primary_category", "") or "",
                snippet,
                f"/paper/{pid}",
            )
        )

    total_pages = max(1, (total + limit - 1) // limit)

    return templates.TemplateResponse(
        request,
        "papers.html",
        {
            "page": page,
            "papers": papers_list,
            "query": q,
            "total": total,
            "total_pages": total_pages,
            "year_from": year_from,
            "year_to": year_to,
            "contradiction_map": {},
        },
    )


@app.get("/paper/{paper_id}")
async def paper_detail(request: Request, paper_id: str):
    """Paper detail — full metadata."""
    db = _get_db()
    paper = db.get_paper(paper_id)

    if not paper:
        return templates.TemplateResponse(
            request,
            "paper_detail.html",
            {
                "page": "paper",
                "paper": None,
                "paper_id": paper_id,
                "error": f"Paper '{paper_id}' not found.",
            },
        )

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
        paper.citation_count if hasattr(paper, "citation_count") and paper.citation_count else 0,
        paper.page_count or 0,
        paper.word_count or 0,
        paper.parse_status or "unknown",
        paper.reading_status or "unread",
        paper.added_at or "",
        all_categories,
    )

    return templates.TemplateResponse(
        request,
        "paper_detail.html",
        {
            "page": "paper",
            "paper": paper_tuple,
            "paper_id": paper_id,
            "error": None,
            "gene_matches": gene_matches,
            "rigor_score": None,  # lazy — use /paper/{id}/rigor to compute
        },
    )


@app.delete("/paper/{paper_id}")
async def delete_paper(paper_id: str):
    """Delete a paper by ID."""
    db = _get_db()
    deleted = db.delete_paper(paper_id)
    return {"deleted": deleted, "paper_id": paper_id}


@app.delete("/papers")
async def delete_papers_bulk(request: Request):
    """Bulk delete papers. Accepts JSON body with list of paper_ids."""
    from starlette.datastructures import URL

    try:
        body = await request.json()
        paper_ids = body.get("paper_ids", [])
    except Exception:
        return {"error": "Invalid JSON"}, 400

    if not paper_ids:
        return {"deleted": 0, "paper_ids": []}

    db = _get_db()
    deleted = 0
    for pid in paper_ids:
        if db.delete_paper(pid):
            deleted += 1

    return {"deleted": deleted, "paper_ids": paper_ids}


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


@app.get("/embodied-planning/batch")
async def embodied_planning_batch(request: Request, ids: str = ""):
    """Batch analyze multiple papers for embodied planning representation types.

    Query param: ids=pid1,pid2,pid3
    Returns comparative report: discrete vs continuous vs hybrid grouping,
    contradiction pairs, and summary statistics.
    """
    if not ids:
        return {"error": "Provide paper IDs via ?ids=pid1,pid2,pid3"}
    paper_ids = [p.strip() for p in ids.split(",") if p.strip()]
    if not paper_ids:
        return {"error": "No valid paper IDs provided"}

    from llm.paper_gap_extractor import batch_analyze_embodied_planning

    result = batch_analyze_embodied_planning(paper_ids=paper_ids)
    return result


@app.get("/embodied-planning/dashboard")
async def embodied_planning_dashboard(request: Request):
    """Render the embodied planning domain-wide dashboard.

    Shows all analyzed papers grouped by representation type (discrete/
    continuous/hybrid), with confidence scores and contradiction pairs.
    """
    from llm.paper_gap_extractor import render_embodied_planning_dashboard

    html = render_embodied_planning_dashboard()
    return templates.TemplateResponse(
        request,
        "generic.html",
        {
            "page": "embodied-planning-dashboard",
            "title": "🦾 Embodied Planning — Representation Atlas",
            "content": html,
        },
    )


@app.get("/embodied-planning/evolution")
async def embodied_evolution_timeline(request: Request):
    """Render Mermaid Gantt chart showing belief evolution over time."""
    from llm.paper_gap_extractor import render_evolution_timeline

    graph = render_evolution_timeline()
    if not graph:
        graph = "<div style='text-align:center;padding:40px;color:#888;'>No timeline data yet — analyze some papers first.</div>"
    return templates.TemplateResponse(
        request,
        "generic.html",
        {
            "page": "embodied-evolution",
            "title": "🦾 Belief Evolution Timeline",
            "content": f"<div style='overflow-x:auto;'>{graph}</div>",
        },
    )


@app.get("/embodied-planning/compare")
async def embodied_planning_compare(request: Request, ids: str = ""):
    """Compare representation types across 2 papers side-by-side."""
    from llm.paper_gap_extractor import render_compare_view

    paper_ids = [p.strip() for p in ids.split(",") if p.strip()][:2]
    html = render_compare_view(paper_ids)
    return templates.TemplateResponse(
        request,
        "generic.html",
        {
            "page": "embodied-compare",
            "title": "🦾 Embodied Planning — Compare",
            "content": html,
        },
    )


@app.get("/embodied-planning/semantic-search")
async def semantic_search(request: Request, q: str = "", top_k: int = 5):
    """Semantic search across analyzed papers."""
    from fastapi.responses import JSONResponse
    from llm.paper_gap_extractor import semantic_search_papers

    results = semantic_search_papers(q, top_k=top_k)
    return JSONResponse({"query": q, "results": results})


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


@app.get("/paper/{paper_id}/rigor")
async def paper_rigor(request: Request, paper_id: str):
    """Compute and return research rigor score for a paper as JSON."""
    from llm.rigor_scorer import RigorScorer

    db = _get_db()
    paper = db.get_paper(paper_id)
    if not paper:
        return {"error": f"Paper '{paper_id}' not found."}
    scorer = RigorScorer()
    score = scorer.score_paper(paper_id, abstract=paper.abstract or "", title=paper.title or "")
    return score.to_dict()


@app.get("/paper/{paper_id}/replication")
async def paper_replication(request: Request, paper_id: str):
    """Run replication checker on a paper — returns JSON report."""
    from llm.replication_checker import ReplicationChecker

    db = _get_db()
    paper = db.get_paper(paper_id)
    if not paper:
        return {"error": f"Paper '{paper_id}' not found."}
    checker = ReplicationChecker()
    report = checker.check_paper(
        paper_id=paper_id,
        title=paper.title or "",
        abstract=paper.abstract or "",
        full_text=paper.plain_text or "",
    )
    return report.to_dict()


@app.get("/briefing")
async def briefing(request: Request, arxiv_id: str = ""):
    """Research Briefing — generate or show."""
    return templates.TemplateResponse(
        request,
        "briefing.html",
        {
            "page": "briefing",
            "arxiv_id": arxiv_id,
            "result": None,
            "error": None,
        },
    )


@app.post("/briefing")
async def briefing_generate(
    request: Request,
    arxiv_id: str = Form(""),
    use_llm: bool = Form(True),
):
    """Generate a research briefing."""
    if not arxiv_id.strip():
        return templates.TemplateResponse(
            request,
            "briefing.html",
            {
                "page": "briefing",
                "arxiv_id": arxiv_id,
                "result": None,
                "error": "Please enter an arXiv ID.",
            },
        )

    try:
        from llm.briefing_generator import BriefingGenerator

        gen = BriefingGenerator()
        result = gen.generate(
            arxiv_id=arxiv_id.strip(),
            use_llm=use_llm,
            output_dir=PROJECT_ROOT / "data" / "briefings",
        )

        if result.success:
            slug = "".join(c if c.isalnum() else "_" for c in arxiv_id.strip().lower())
            md_path = f"/data/briefings/briefing_{slug}.md"
            return templates.TemplateResponse(
                request,
                "briefing.html",
                {
                    "page": "briefing",
                    "arxiv_id": arxiv_id,
                    "result": result,
                    "markdown_path": md_path,
                    "error": None,
                },
            )
        else:
            return templates.TemplateResponse(
                request,
                "briefing.html",
                {
                    "page": "briefing",
                    "arxiv_id": arxiv_id,
                    "result": None,
                    "error": result.error,
                },
            )
    except Exception as e:
        return templates.TemplateResponse(
            request,
            "briefing.html",
            {
                "page": "briefing",
                "arxiv_id": arxiv_id,
                "result": None,
                "error": str(e),
            },
        )


@app.get("/briefing/distribute/{arxiv_id}")
async def distribute_briefing(request: Request, arxiv_id: str, audience: str = "researcher"):
    """Render a briefing in a specific audience format."""
    from llm.briefing_distributor import (
        get_latest_briefing_markdown,
        render_distributed_briefing,
    )

    markdown = get_latest_briefing_markdown(arxiv_id)
    if not markdown:
        return {"error": "No briefing found for " + arxiv_id}
    html = render_distributed_briefing(arxiv_id, arxiv_id, markdown, audience)
    return templates.TemplateResponse(
        request,
        "generic.html",
        {
            "page": "briefing",
            "title": f"Briefing — {audience}",
            "content": html,
        },
    )


@app.get("/b/{short_id}")
async def shared_briefing(request: Request, short_id: str):
    """Resolve a short share link to the appropriate briefing."""
    from llm.briefing_distributor import (
        _load_links,
        get_latest_briefing_markdown,
        render_distributed_briefing,
    )

    links = _load_links()
    info = links.get(short_id)
    if not info:
        return templates.TemplateResponse(
            request,
            "error.html",
            {
                "page": "error",
                "error": "Link not found or expired.",
            },
        )
    arxiv_id = info.get("arxiv_id", "")
    title = info.get("title", arxiv_id)
    audience = info.get("audience", "researcher")
    markdown = get_latest_briefing_markdown(arxiv_id)
    if not markdown:
        markdown = f"# {title}\n\nNo briefing available."
    html = render_distributed_briefing(arxiv_id, title, markdown, audience)
    return templates.TemplateResponse(
        request,
        "generic.html",
        {
            "page": "briefing",
            "title": f"Shared Briefing — {title[:40]}",
            "content": html,
        },
    )


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
                title = (
                    lines[0][len("# Research Briefing: ") :]
                    if len(lines) > 0 and lines[0].startswith("# Research Briefing: ")
                    else f.stem
                )
                # Line 1: metadata — "**arXiv:** [id](url) | **Authors:** ... | **Generated:** ..."
                arxiv_id = ""
                generated = ""
                if len(lines) > 1:
                    import re

                    aid_match = re.search(r"\*\*arXiv:\*\* \[([^\]]+)\]", lines[1])
                    if aid_match:
                        arxiv_id = aid_match.group(1)
                    gen_match = re.search(r"\*\*Generated:\*\* (\S+)", lines[1])
                    if gen_match:
                        generated = gen_match.group(1)
                verdict = ""
                verdict_emoji = ""
                if len(lines) > 2:
                    v_match = re.search(r"\*\*Verdict:\*\* (.) \*\*([A-Z]+)\*\*", lines[2])
                    if v_match:
                        verdict_emoji = v_match.group(1)
                        verdict = v_match.group(2).lower()
                briefings.append(
                    (
                        arxiv_id,
                        title[:90],
                        generated,
                        verdict,
                        verdict_emoji,
                    )
                )
            except Exception:
                pass

    return templates.TemplateResponse(
        request,
        "briefing_history.html",
        {
            "page": "briefing-history",
            "briefings": briefings,
        },
    )


@app.get("/trust-scores")
async def trust_scores(request: Request):
    """Source Trust Scores — per-arXiv-category credibility ratings."""
    from llm.insight.trust_tracker import SourceTrustTracker

    tracker = SourceTrustTracker()
    html = tracker.render_html()
    return templates.TemplateResponse(
        request,
        "generic.html",
        {
            "page": "trust-scores",
            "title": "Source Trust Scores",
            "content": html,
        },
    )


@app.get("/gene-pool/credibility")
async def gene_pool_credibility(request: Request):
    """Gap Credibility — flags trendslop capsules with high keyword overlap."""
    from llm.insight.credibility import CredibilityScorer

    scorer = CredibilityScorer()
    html = scorer.render_html()
    return templates.TemplateResponse(
        request,
        "generic.html",
        {
            "page": "gene-pool-credibility",
            "title": "Gap Credibility",
            "content": html,
        },
    )


@app.get("/gene-pool/graph")
async def gene_pool_graph(request: Request):
    """D3.js force-directed graph of all Gene Pool capsules."""
    from fastapi.responses import HTMLResponse

    return HTMLResponse(content=render_gene_pool_graph_html())


@app.get("/gene-pool/evolution-log")
async def gene_pool_evolution_log(request: Request):
    """Evolution Log — shows what the Gene Pool learned over time."""
    from llm.insight.tracker import EvolutionTracker

    tracker = EvolutionTracker()
    events = tracker.get_evolution_log(limit=100)
    html = _render_evolution_log_html(events)
    return templates.TemplateResponse(
        request,
        "generic.html",
        {
            "page": "gene-pool-evolution-log",
            "title": "Evolution Log",
            "content": html,
        },
    )


def _render_evolution_log_html(events: list) -> str:
    """Render lifecycle events as a timeline HTML fragment."""
    ACTION_ICON = {
        "created": "🆕",
        "merged": "🔀",
        "evolved": "🧬",
        "archived": "📦",
        "consumed": "⚡",
    }
    ACTION_COLOR = {
        "created": "#4CAF50",
        "merged": "#9C27B0",
        "evolved": "#2196F3",
        "archived": "#757575",
        "consumed": "#FF9800",
    }

    rows = []
    for ev in events:
        icon = ACTION_ICON.get(ev["action"], "📌")
        color = ACTION_COLOR.get(ev["action"], "#666")
        ts = ev.get("timestamp", "")
        # Human-friendly: just show date+time
        date_str = ts.replace("T", " ").split(".")[0] if ts else "—"
        details = ev.get("details", "")
        gap_title = ev.get("gap_title", "") or "—"
        gap_type = ev.get("gap_type", "") or "—"
        cap_id = ev.get("capsule_id", "") or ""

        details_html = f"<div class='ev-details'>{details}</div>" if details else ""

        rows.append(f"""
        <div class='ev-row'>
            <span class='ev-icon'>{icon}</span>
            <div class='ev-body'>
                <div class='ev-header'>
                    <span class='ev-action' style='color:{color}'>{ev["action"].upper()}</span>
                    <span class='ev-time'>{date_str}</span>
                </div>
                <div class='ev-title'>{gap_title}</div>
                <div class='ev-meta'>
                    <span class='ev-type'>{gap_type}</span>
                    <span class='ev-id'>{cap_id}</span>
                </div>
                {details_html}
            </div>
        </div>""")

    if not rows:
        return """
        <div class="evo-empty">
            <div class="evo-empty-icon">🧬</div>
            <div class="evo-empty-msg">No evolution events yet.</div>
            <div class="evo-empty-sub">Accept suggestions or submit verdicts to grow your Gene Pool.</div>
        </div>"""

    rows_html = "\n".join(rows)
    return f"""
    <style>
    .evo-log {{ font-family: 'Courier New', monospace; }}
    .ev-row {{ display: flex; gap: 14px; padding: 10px 0; border-bottom: 1px solid #eee; align-items: flex-start; }}
    .ev-icon {{ font-size: 18px; flex-shrink: 0; width: 28px; text-align: center; padding-top: 2px; }}
    .ev-body {{ flex: 1; min-width: 0; }}
    .ev-header {{ display: flex; align-items: center; gap: 10px; margin-bottom: 3px; }}
    .ev-action {{ font-size: 11px; font-weight: bold; letter-spacing: 0.05em; }}
    .ev-time {{ font-size: 11px; color: #888; }}
    .ev-title {{ font-size: 14px; color: #222; margin-bottom: 3px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }}
    .ev-meta {{ display: flex; gap: 10px; align-items: center; }}
    .ev-type {{ font-size: 11px; background: #e8f0fe; color: #1a73e8; padding: 1px 6px; border-radius: 4px; }}
    .ev-id {{ font-size: 10px; color: #aaa; }}
    .ev-details {{ font-size: 12px; color: #666; margin-top: 3px; font-style: italic; }}
    .evo-empty {{ text-align: center; padding: 60px 20px; }}
    .evo-empty-icon {{ font-size: 48px; margin-bottom: 16px; opacity: 0.4; }}
    .evo-empty-msg {{ font-size: 18px; color: #444; margin-bottom: 8px; }}
    .evo-empty-sub {{ font-size: 13px; color: #999; }}
    </style>
    <div class="evo-log">{rows_html}</div>
    """


@app.get("/heatmap")
async def contradiction_heatmap(request: Request):
    """Contradiction Heatmap — papers colored by contradiction count."""
    from llm.contradiction_heatmap import compute_paper_contradictions, render_heatmap_html

    db = _get_db()
    rows, _ = db.list_papers(limit=200, offset=0)
    papers = [
        {
            "id": r.id,
            "title": r.title,
            "primary_category": getattr(r, "primary_category", "") or "",
            "published": r.published,
        }
        for r in rows
    ]
    contrad_map = compute_paper_contradictions()
    html = render_heatmap_html(papers, contrad_map)
    return templates.TemplateResponse(
        request,
        "generic.html",
        {
            "page": "heatmap",
            "title": "Contradiction Heatmap",
            "content": html,
        },
    )


@app.get("/game-mode")
async def game_mode(request: Request):
    """Research Game Mode — badges and progression."""
    from llm.game_mode import compute_badges, render_game_mode_html

    badges = compute_badges()
    html = render_game_mode_html(badges)
    return templates.TemplateResponse(
        request,
        "generic.html",
        {
            "page": "game-mode",
            "title": "Research Game Mode",
            "content": html,
        },
    )


@app.get("/alerts/paradigm")
async def paradigm_alert(request: Request):
    """Paradigm Concentration Alert — flags when >60% citations cluster around ≤3 papers."""
    from llm.paradigm_monitor import check_paradigm_concentration, render_html

    result = check_paradigm_concentration("all")
    html = render_html(result)
    return templates.TemplateResponse(
        request,
        "generic.html",
        {
            "page": "paradigm-alert",
            "title": "Paradigm Alert",
            "content": html,
        },
    )


@app.get("/alerts/eval-gap")
async def eval_gap_alert(request: Request):
    """Evaluation Gap Monitor — flags domains where deployment outpaces benchmark research."""
    from llm.eval_gap_monitor import check_eval_gaps, render_eval_gap_html

    data = check_eval_gaps()
    html = render_eval_gap_html(data)
    return templates.TemplateResponse(
        request,
        "generic.html",
        {
            "page": "eval-gap-alert",
            "title": "Evaluation Gap",
            "content": html,
        },
    )


@app.get("/gene-pool/bold")
async def gene_pool_bold(request: Request):
    """Bold Hypothesis Vault — high-risk/high-reward Gene Pool capsules."""
    from llm.bold_vault import get_bold_capsules, render_html

    capsules = get_bold_capsules()
    html = render_html(capsules)
    return templates.TemplateResponse(
        request,
        "generic.html",
        {
            "page": "gene-pool-bold",
            "title": "Bold Hypothesis Vault",
            "content": html,
        },
    )


@app.get("/gene-pool/backup")
async def gene_pool_backup(request: Request):
    """Gene Pool Backup — view snapshots and restore."""
    from llm.gene_pool_backup import get_backup_info, render_backup_html

    info = get_backup_info()
    html = render_backup_html(info)
    return templates.TemplateResponse(
        request,
        "generic.html",
        {
            "page": "gene-pool-backup",
            "title": "Gene Pool Backup",
            "content": html,
        },
    )


@app.post("/gene-pool/backup/create")
async def create_backup(request: Request):
    """Trigger an immediate backup."""
    from llm.gene_pool_backup import create_backup
    from fastapi.responses import JSONResponse

    try:
        stamp = create_backup()
        return JSONResponse({"success": True, "stamp": stamp})
    except Exception as e:
        return JSONResponse({"success": False, "error": str(e)}, status_code=500)


@app.post("/gene-pool/backup/restore/{stamp}")
async def restore_backup(stamp: str, request: Request):
    """Restore Gene Pool from a specific backup stamp."""
    from llm.gene_pool_backup import restore_backup
    from fastapi.responses import JSONResponse

    ok = restore_backup(stamp)
    return JSONResponse({"success": ok, "message": "Restored" if ok else "Failed"})


@app.get("/gene-pool/at-risk")
async def gene_pool_at_risk(request: Request):
    """Show at-risk capsules (low_score_streak >= 2) with keep/pin actions."""
    from llm.at_risk_scanner import get_at_risk_capsules, render_html

    capsules = get_at_risk_capsules()
    html = render_html(capsules)
    return templates.TemplateResponse(
        request,
        "generic.html",
        {
            "page": "gene-pool-at-risk",
            "title": "At-Risk Capsules",
            "content": html,
        },
    )


@app.post("/gene-pool/at-risk/keep-active")
async def at_risk_keep_active(request: Request):
    """Reset low_score_streak for a capsule (keep active)."""
    from llm.at_risk_scanner import keep_active

    body = await request.json()
    capsule_id = body.get("capsule_id", "")
    success = keep_active(capsule_id)
    return {"success": success}


@app.post("/gene-pool/at-risk/pin")
async def at_risk_pin(request: Request):
    """Pin a capsule to TTL cycles."""
    from llm.at_risk_scanner import pin_to_ttl

    body = await request.json()
    capsule_id = body.get("capsule_id", "")
    ttl = int(body.get("ttl", 3))
    success = pin_to_ttl(capsule_id, ttl)
    return {"success": success}


@app.get("/auth/login")
async def login_page(request: Request):
    """Login page — redirects to / if already authenticated."""
    from llm.auth import is_auth_enabled, validate_session

    if not is_auth_enabled():
        return RedirectResponse(url="/", status_code=303)
    token = request.cookies.get("session_token", "")
    if validate_session(token):
        return RedirectResponse(url="/", status_code=303)
    return templates.TemplateResponse(
        "login.html", {"request": request, "error": None, "not_setup": False}
    )


@app.post("/auth/login")
async def login_submit(request: Request, username: str = Form(""), password: str = Form("")):
    from llm.auth import create_session, verify_login

    if verify_login(username, password):
        token = create_session(username)
        response = RedirectResponse(url="/", status_code=303)
        response.set_cookie(
            key="session_token", value=token, httponly=True, samesite="lax", max_age=86400 * 7
        )
        return response
    return templates.TemplateResponse(
        "login.html", {"request": request, "error": "Invalid credentials", "not_setup": False}
    )


@app.get("/auth/setup")
async def setup_page(request: Request):
    """First-time setup — create admin account."""
    from llm.auth import is_auth_enabled

    if is_auth_enabled():
        return RedirectResponse(url="/auth/login", status_code=303)
    return templates.TemplateResponse("setup.html", {"request": request, "error": None})


@app.post("/auth/setup")
async def setup_submit(
    request: Request, username: str = Form(""), password: str = Form(""), password2: str = Form("")
):
    from llm.auth import is_auth_enabled, setup_admin

    if is_auth_enabled():
        return RedirectResponse(url="/auth/login", status_code=303)
    if not username or len(username) < 3:
        return templates.TemplateResponse(
            "setup.html", {"request": request, "error": "Username must be at least 3 characters"}
        )
    if not password or len(password) < 8:
        return templates.TemplateResponse(
            "setup.html", {"request": request, "error": "Password must be at least 8 characters"}
        )
    if password != password2:
        return templates.TemplateResponse(
            "setup.html", {"request": request, "error": "Passwords do not match"}
        )
    ok = setup_admin(username, password)
    if not ok:
        return templates.TemplateResponse(
            "setup.html", {"request": request, "error": "Setup failed"}
        )
    token = create_session(username)
    response = RedirectResponse(url="/", status_code=303)
    response.set_cookie(
        key="session_token", value=token, httponly=True, samesite="lax", max_age=86400 * 7
    )
    return response


@app.post("/auth/logout")
async def logout(request: Request):
    from llm.auth import revoke_session
    from fastapi.responses import JSONResponse

    token = request.cookies.get("session_token", "")
    if token:
        revoke_session(token)
    response = RedirectResponse(url="/auth/login", status_code=303)
    response.delete_cookie("session_token")
    return response


@app.get("/chat")
async def chat(request: Request):
    """Web Chat — streaming RAG chat over the paper library."""
    from llm.web_chat import render_chat_html

    html = render_chat_html()
    return templates.TemplateResponse(
        request,
        "generic.html",
        {
            "page": "chat",
            "title": "Research Chat",
            "content": html,
        },
    )


@app.post("/chat/stream")
async def chat_stream(request: Request):
    """Streaming chat endpoint — SSE."""
    from llm.web_chat import chat_stream as ws_chat_stream

    return await ws_chat_stream(request)


@app.get("/insights/queue")
async def review_queue(request: Request):
    """Capsule Review Queue — new capsules pending first feedback."""
    from llm.review_queue import get_review_queue, render_review_queue_html

    queue = get_review_queue()
    html = render_review_queue_html(queue)
    return templates.TemplateResponse(
        request,
        "generic.html",
        {
            "page": "review-queue",
            "title": "Capsule Review Queue",
            "content": html,
        },
    )


@app.post("/insights/queue/verdict")
async def submit_verdict(request: Request):
    """Record a user's verdict on a queued capsule."""
    from llm.insight.tracker import record_gap_accept
    from llm.review_queue import _load_capsules

    body = await request.json()
    capsule_id = body.get("capsule_id", "")
    verdict = body.get("verdict", "")

    score_map = {"match": 1.0, "partial": 0.5, "not_relevant": 0.0}
    score = score_map.get(verdict, 0.5)

    capsules = _load_capsules()
    for cap in capsules:
        if cap.get("capsule_id", "") == capsule_id:
            record_gap_accept(capsule_id, score=score)
            break

    return {"success": True}


@app.get("/gene-pool/io")
async def gene_pool_io(request: Request):
    """Gene Pool Import/Export — backup and restore as JSON."""
    from llm.gene_pool_io import render_io_html

    html = render_io_html()
    return templates.TemplateResponse(
        request,
        "generic.html",
        {
            "page": "gene-pool-io",
            "title": "Gene Pool Import/Export",
            "content": html,
        },
    )


@app.get("/gene-pool/io/export")
async def export_pool(request: Request):
    """Export full Gene Pool as JSON."""
    from llm.gene_pool_io import export_pool
    from fastapi.responses import JSONResponse

    return JSONResponse(export_pool())


@app.post("/gene-pool/io/import")
async def import_pool(request: Request):
    """Import Gene Pool from JSON."""
    from llm.gene_pool_io import import_pool
    from fastapi.responses import JSONResponse

    body = await request.json()
    stats = import_pool(body, merge=True)
    return JSONResponse({"success": True, **stats})


@app.get("/arxiv-channels")
async def arxiv_channels(request: Request):
    """arXiv Watch Alert Channels — configure multiple feed configs."""
    from llm.arxiv_alert_channels import render_channels_html

    db = _get_db()
    try:
        recent = db.get_recent_subscription_papers_grouped(limit_per=5)
    except Exception:
        recent = {}
    html = render_channels_html(check_results=recent)
    return templates.TemplateResponse(
        request,
        "generic.html",
        {
            "page": "arxiv-channels",
            "title": "arXiv Watch Channels",
            "content": html,
        },
    )


@app.post("/arxiv-channels/toggle/{channel_id}")
async def toggle_channel(channel_id: str, request: Request):
    """Toggle an alert channel on/off."""
    from llm.arxiv_alert_channels import update_channel
    from fastapi.responses import JSONResponse
    from llm.arxiv_alert_channels import _load_channels

    channels = _load_channels()
    if channel_id not in channels:
        return JSONResponse({"success": False}, status_code=404)
    current = channels[channel_id].get("enabled", True)
    update_channel(channel_id, {"enabled": not current})
    return JSONResponse({"success": True})


@app.post("/arxiv-channels/check")
async def arxiv_check(request: Request):
    """Run arXiv subscription check across all enabled subscriptions."""
    from fastapi.responses import JSONResponse

    try:
        db = _get_db()
        from llm.subscription_monitor import SubscriptionMonitor

        monitor = SubscriptionMonitor(db)
        results = monitor.check_all()
        total = sum(len(v) for v in results.values())
        return JSONResponse(
            {
                "success": True,
                "new_papers": total,
                "details": {k: len(v) for k, v in results.items()},
            }
        )
    except Exception as e:
        return JSONResponse({"success": False, "error": str(e)}, status_code=500)


def _get_global_rep_type_counts() -> dict:
    """Count representation types across all embodied_planning capsules in Gene Pool."""
    from llm.gene_pool_io import load_capsules

    capsules = load_capsules(gap_type="embodied_planning")
    counts: Dict[str, int] = {"discrete": 0, "continuous": 0, "hybrid": 0, "unknown": 0}
    for c in capsules:
        rt = c.get("archetype", {}).get("representation_type", "unknown")
        if rt in counts:
            counts[rt] += 1
    return counts


@app.post("/embodied-planning/auto-scan")
async def embodied_planning_auto_scan(request: Request):
    """Auto-scan new VLA/robotics papers from subscriptions for embodied planning analysis.

    Runs subscription check, then for each new paper auto-analyzes
    embodied planning representation type and saves to Gene Pool.
    """
    from fastapi.responses import JSONResponse

    try:
        db = _get_db()
        from llm.subscription_monitor import SubscriptionMonitor
        from llm.paper_gap_extractor import run_embodied_analysis

        monitor = SubscriptionMonitor(db)
        results = monitor.check_all()  # {topic: [paper_ids]}
        all_new_ids = []
        for papers in results.values():
            all_new_ids.extend(papers)

        # Filter to VLA/robotics papers only, then run shared analysis pipeline
        vla_ids = [
            pid
            for pid in all_new_ids
            if any(
                c in (db.get_paper(pid).categories or "").lower()
                for c in ["cs.ro", "cs.cv", "cs.ai", "cs.lg"]
            )
        ]
        result = run_embodied_analysis(vla_ids, db=db, save_to_pool=True)
        analyzed = result["analyzed"]
        contradictions = result["contradictions"]
        type_counts = result["type_counts"]
        total_analyzed = result["total_analyzed"]
        trend = result["trend"]
        trend_pct = result["trend_pct"]

        # Recommend next paper: under-represented type
        all_counts = _get_global_rep_type_counts()
        for rt in ["discrete", "continuous", "hybrid"]:
            all_counts[rt] = all_counts.get(rt, 0) + type_counts.get(rt, 0)
        underrep = min(all_counts, key=all_counts.get) if all_counts else "hybrid"
        recommend_msg = ""
        if total_analyzed > 0:
            recommend_msg = (
                f"Only {type_counts.get(underrep, 0)}/{total_analyzed} papers "
                f"in this batch used {underrep} — consider searching for more."
            )

        notification = None
        if contradictions:
            notification = {
                "type": "contradiction",
                "uid": f"contra_{all_new_ids[0] if all_new_ids else 'none'}",
                "message": f"⚠️ {len(contradictions)} contradiction(s) detected — papers disagree on representation type",
                "details": contradictions[:3],
            }
            _notification_store.append(notification)

            # ── Hypothesis generation from contradictions ──────────────────────
            try:
                from llm.paper_gap_extractor import (
                    generate_hypothesis_from_contradiction,
                    append_hypothesis_to_roadmap,
                )

                for contra in contradictions:
                    pid_a = contra.get("paper_id", "")
                    pid_b = contra.get("contradiction_with", "")
                    contra_pair = {
                        "paper_a_id": pid_a,
                        "paper_b_id": pid_b,
                        "paper_a_title": contra.get("title", ""),
                        "paper_b_title": "",
                        "representation_a": contra.get("representation_type", "unknown"),
                        "representation_b": contra.get("contradiction_type", "unknown"),
                        "effectiveness_a": "effective",
                        "effectiveness_b": "ineffective",
                    }
                    hyp = generate_hypothesis_from_contradiction(contra_pair)
                    append_hypothesis_to_roadmap(hyp, pid_a, pid_b)
            except Exception:
                pass  # Non-critical: don't fail scan if hypothesis generation fails
        elif total_analyzed > 2 and trend_pct > 0.7:
            notification = {
                "type": "trend",
                "uid": f"trend_{all_new_ids[0] if all_new_ids else 'none'}",
                "message": f"📊 Strong trend: {trend} representation dominates ({int(trend_pct * 100)}% of this batch)",
            }
            _notification_store.append(notification)

        # ── Task 2: Append recommended papers to ROADMAP.md ──────────────────
        if total_analyzed > 0 and analyzed:
            try:
                recommended_type = underrep
                # Pick the top paper from the batch that matches the under-represented type
                rec_paper = next(
                    (r for r in analyzed if r.get("representation_type") == recommended_type),
                    analyzed[0] if analyzed else None,
                )
                if rec_paper:
                    from pathlib import Path as _Path

                    _rm_path = _Path("D:/OpenClaw/workspace/80-PROJECTS/ai_research_os/ROADMAP.md")
                    if _rm_path.exists():
                        existing_content = _rm_path.read_text(encoding="utf-8")
                        # Only write if not already listed (avoid duplicates)
                        rec_id = rec_paper.get("paper_id", "unknown")
                        rec_title = rec_paper.get("title", "Unknown Title")
                        marker = f"[ ] {recommended_type} paper: {rec_title} ({rec_id})"
                        if marker not in existing_content:
                            # Append under v2.2 section
                            with open(_rm_path, "a", encoding="utf-8") as _f:
                                _f.write(f"\n### Pending Readings\n- {marker}\n")
            except Exception as _e:
                pass  # Non-critical: don't fail the scan if roadmap write fails

        # ── Fallback: Gene Pool keyword-driven scan if no subscriptions configured ──
        if not all_new_ids:
            try:
                from llm.gene_pool_io import load_capsules, get_capsule_by_paper
                from llm.paper_gap_extractor import analyze_gap
                from llm.subscription_monitor import search_arxiv

                GAP = "embodied_planning"
                capsules = load_capsules(status="active", gap_type=GAP)
                kw_set: set = set()
                for cap in capsules:
                    for kw in cap.get("trigger_keywords", []):
                        if len(kw) > 3:
                            kw_set.add(kw.lower())
                kw_list = list(kw_set)[:8]
                query = " AND ".join(f'"{k}"' for k in kw_list)
                papers = search_arxiv(query, max_results=6)

                db = _get_db()
                new_analyzed = []
                for p in papers:
                    arxiv_id = p.get("arxiv_id", "")
                    if get_capsule_by_paper(arxiv_id, gap_type=GAP):
                        continue  # already in Gene Pool
                    r = analyze_gap(
                        paper_id=arxiv_id,
                        title=p.get("title", ""),
                        abstract=p.get("abstract", ""),
                        gap_type=GAP,
                    )
                    capsule_id = r.get("saved_to_pool")
                    new_analyzed.append(
                        {
                            "paper_id": arxiv_id,
                            "title": p.get("title", "")[:80],
                            "representation_type": r.get("representation_type", "unknown"),
                            "confidence": r.get("confidence", 0),
                            "capsule_id": capsule_id,
                        }
                    )
                if new_analyzed:
                    all_new_ids = [p["paper_id"] for p in new_analyzed]
                    analyzed = new_analyzed
                    notification = None
                    recommend_msg = (
                        f"Scanned {len(new_analyzed)} new papers via Gene Pool keywords "
                        f"(no subscriptions configured — using keyword fallback)."
                    )
            except Exception:
                pass  # Non-critical fallback

        return JSONResponse(
            {
                "success": True,
                "total_new": len(all_new_ids),
                "analyzed": len(analyzed),
                "results": analyzed,
                "contradictions": contradictions,
                "trend": {"dominant": trend, "pct": int(trend_pct * 100), "counts": type_counts},
                "recommended_next_type": underrep,
                "recommend_msg": recommend_msg,
                "notification": notification,
            }
        )
    except Exception as e:
        return JSONResponse({"success": False, "error": str(e)}, status_code=500)


# In-memory notification store (per-process, reset on restart — lightweight)
_notification_store: List[Dict[str, Any]] = []


@app.get("/notifications")
async def get_notifications(request: Request):
    """Get current notifications (contradictions, trends, alerts)."""
    from fastapi.responses import JSONResponse

    return JSONResponse({"notifications": _notification_store})


@app.post("/notifications/dismiss")
async def dismiss_notification(request: Request):
    """Dismiss all or specific notifications."""
    from fastapi.responses import JSONResponse

    try:
        body = await request.json()
        uid = body.get("uid")
        if uid:
            _notification_store[:] = [n for n in _notification_store if n.get("uid") != uid]
        else:
            _notification_store.clear()
        return JSONResponse({"success": True, "remaining": len(_notification_store)})
    except Exception as e:
        return JSONResponse({"success": False, "error": str(e)}, status_code=500)


# ── Task 5: arXiv主动搜索 ──────────────────────────────────────────────────


@app.post("/embodied-planning/search")
async def embodied_planning_search(request: Request):
    """主动搜索arXiv论文并分析embodied planning representation type.

    Query: "latent reasoning" OR "physical reasoning" OR "embodied planning" site:arxiv.org
    """
    from fastapi.responses import JSONResponse
    from pathlib import Path as _Path

    try:
        body = await request.json()
        query = body.get("query", "latent reasoning OR physical reasoning OR embodied planning")
        max_results = body.get("max_results", 10)

        # Use SubscriptionMonitor.search_arxiv — unified, no duplicate XML parsing
        from llm.subscription_monitor import search_arxiv

        papers = search_arxiv(query, max_results)

        if not papers:
            return JSONResponse({"success": True, "query": query, "results": [], "analyzed": []})

        # Analyze each paper with analyze_embodied_planning
        from llm.paper_gap_extractor import analyze_embodied_planning

        analyzed = []
        for p in papers:
            result = analyze_embodied_planning(
                paper_id=p["arxiv_id"],
                title=p["title"],
                abstract=p["abstract"],
            )
            result["arxiv_id"] = p["arxiv_id"]
            result["published"] = p["published"]
            analyzed.append(result)

        return JSONResponse(
            {
                "success": True,
                "query": query,
                "total": len(papers),
                "analyzed": len(analyzed),
                "results": analyzed,
            }
        )
    except Exception as e:
        return JSONResponse({"success": False, "error": str(e)}, status_code=500)


@app.get("/gene-pool/cross-domain")
async def cross_domain_bridge(request: Request):
    """Cross-Domain Gap Bridge — find Gene Pool connections across research fields."""
    from llm.cross_domain_bridge import find_cross_domain_bridges, render_cross_domain_html

    bridges = find_cross_domain_bridges()
    html = render_cross_domain_html(bridges)
    return templates.TemplateResponse(
        request,
        "generic.html",
        {
            "page": "cross-domain",
            "title": "Cross-Domain Gap Bridge",
            "content": html,
        },
    )


@app.get("/climate-monitor")
async def climate_monitor(request: Request):
    """Climate AI Monitor — papers at climate+AI intersection."""
    from llm.climate_ai_monitor import get_watch_stats, render_climate_monitor_html

    stats = get_watch_stats()
    html = render_climate_monitor_html(stats)
    return templates.TemplateResponse(
        request,
        "generic.html",
        {
            "page": "climate-monitor",
            "title": "Climate AI Monitor",
            "content": html,
        },
    )


@app.post("/climate-monitor/toggle-watch")
async def climate_toggle_watch(request: Request):
    """Toggle watch status for a climate paper."""
    from llm.climate_ai_monitor import _load_watch_list, _save_watch_list
    from fastapi.responses import JSONResponse

    body = await request.json()
    paper_id = body.get("paper_id", "")
    watch = _load_watch_list()
    watched = set(watch.get("watched_ids", []))
    if paper_id in watched:
        watched.discard(paper_id)
    else:
        watched.add(paper_id)
    watch["watched_ids"] = list(watched)
    _save_watch_list(watch)
    return JSONResponse({"success": True})


@app.get("/citation-chain")
async def citation_chain(request: Request, arxiv_id: str = ""):
    """Citation Chain — build and visualize."""
    return templates.TemplateResponse(
        request,
        "citation_chain.html",
        {
            "page": "citation_chain",
            "arxiv_id": arxiv_id,
            "chain_data": None,
            "error": None,
        },
    )


@app.get("/citation-chain/graph")
async def citation_chain_graph(request: Request, paper_id: str = "", title: str = ""):
    """Interactive SVG citation graph: paper → cited refs → Gene Pool capsules."""
    from llm.citation_pathfinder_web import render_citation_chain_html

    cited_paper_ids = ["p1", "p2", "p3"]  # placeholders; real impl reads from DB
    cited_capsule_ids = []
    html = render_citation_chain_html(paper_id, title, cited_paper_ids, cited_capsule_ids)
    return templates.TemplateResponse(
        request,
        "generic.html",
        {
            "page": "citation_chain",
            "title": "Citation Pathfinder",
            "content": html,
        },
    )


@app.get("/voice-capsule")
async def voice_capsule(request: Request):
    """Voice-to-Capsule — upload audio, transcribe, extract gap, save to Gene Pool."""
    from llm.voice_to_capsule import render_voice_upload_html

    html = render_voice_upload_html()
    return templates.TemplateResponse(
        request,
        "generic.html",
        {
            "page": "voice-capsule",
            "title": "Voice-to-Capsule",
            "content": html,
        },
    )


@app.post("/voice-capsule/transcribe")
async def voice_transcribe(request: Request):
    """Receive audio file, transcribe with Whisper, extract gap with LLM."""
    from llm.voice_to_capsule import extract_gap_from_text, transcribe_audio
    from fastapi.responses import JSONResponse

    try:
        form = await request.form()
        audio_file = form.get("audio")
        if not audio_file:
            return JSONResponse({"error": "No audio file"}, status_code=400)
        audio_bytes = await audio_file.read()
        text = transcribe_audio(audio_bytes)
        if text.startswith("[Transcription error"):
            return JSONResponse({"error": text})
        gap = extract_gap_from_text(text)
        return JSONResponse(gap)
    except Exception as e:
        return JSONResponse({"error": str(e)}, status_code=500)


@app.post("/voice-capsule/save")
async def voice_save(request: Request):
    """Save extracted voice gap to Gene Pool."""
    from llm.voice_to_capsule import save_voice_capsule
    from fastapi.responses import JSONResponse

    try:
        body = await request.json()
        cid = save_voice_capsule(body)
        return JSONResponse({"success": True, "capsule_id": cid})
    except Exception as e:
        return JSONResponse({"success": False, "error": str(e)}, status_code=500)


@app.get("/policy-impact")
async def policy_impact(request: Request):
    """Policy Impact Tracer — map regulations to Gene Pool priority weights."""
    from llm.policy_impact_tracer import render_policy_tracer_html

    html = render_policy_tracer_html()
    return templates.TemplateResponse(
        request,
        "generic.html",
        {
            "page": "policy-impact",
            "title": "Policy Impact Tracer",
            "content": html,
        },
    )


@app.get("/labor-displacement")
async def labor_displacement(request: Request):
    """Labor Displacement Tracker — AI vs. human labor gaps."""
    from llm.labor_displacement_tracker import render_labor_tracker_html

    html = render_labor_tracker_html()
    return templates.TemplateResponse(
        request,
        "generic.html",
        {
            "page": "labor-displacement",
            "title": "Labor Displacement Tracker",
            "content": html,
        },
    )


@app.get("/researchers")
async def multi_researcher(request: Request):
    """Multi-Researcher Support — shared Gene Pool with source_user tags."""
    from llm.multi_researcher import render_multi_researcher_html

    html = render_multi_researcher_html()
    return templates.TemplateResponse(
        request,
        "generic.html",
        {
            "page": "multi-researcher",
            "title": "Multi-Researcher",
            "content": html,
        },
    )


@app.post("/researchers/add")
async def add_researcher_route(request: Request):
    from llm.multi_researcher import add_researcher
    from fastapi.responses import JSONResponse

    body = await request.json()
    uid = body.get("user_id", "")
    name = body.get("name", "")
    ok = add_researcher(uid, name)
    return JSONResponse({"success": ok, "error": None if ok else "already exists"})


@app.get("/researchers/capsules/{user_id}")
async def researcher_capsules(user_id: str, request: Request):
    from llm.multi_researcher import get_capsules_for_user
    from fastapi.responses import JSONResponse

    capsules = get_capsules_for_user(user_id)
    return JSONResponse({"count": len(capsules), "capsules": capsules[:10]})


@app.post("/citation-chain")
async def citation_chain_build(
    request: Request,
    arxiv_id: str = Form(""),
    max_depth: int = Form(2),
    fmt: str = Form("mermaid"),
):
    """Build a citation chain."""
    if not arxiv_id.strip():
        return templates.TemplateResponse(
            request,
            "citation_chain.html",
            {
                "page": "citation_chain",
                "arxiv_id": arxiv_id,
                "chain_data": None,
                "error": "Please enter an arXiv ID.",
            },
        )

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

        return templates.TemplateResponse(
            request,
            "citation_chain.html",
            {
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
            },
        )
    except Exception as e:
        return templates.TemplateResponse(
            request,
            "citation_chain.html",
            {
                "page": "citation_chain",
                "arxiv_id": arxiv_id,
                "chain_data": None,
                "error": str(e),
            },
        )


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
        status = {
            "running": False,
            "interval_minutes": 30,
            "last_check": "",
            "alerts_count": 0,
            "error": "Orchestrator unavailable",
        }
        alerts_raw = []

    alerts = []
    for a in alerts_raw:
        created = a.created_at if hasattr(a, "created_at") else a.get("created_at", "")
        alerts.append(
            (
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
            )
        )

    try:
        db = _get_db()
        subs_raw = db.list_arxiv_subscriptions()
    except Exception:
        subs_raw = []

    subscriptions = []
    for s in subs_raw:
        subscriptions.append(
            {
                "id": s.get("id", ""),
                "topic": s.get("topic", ""),
            }
        )

    return templates.TemplateResponse(
        request,
        "research_loop.html",
        {
            "page": "research-loop",
            "status": status,
            "alerts": alerts,
            "subscriptions": subscriptions,
        },
    )


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
        squad_status = {
            "running": False,
            "agents": {},
            "error": str(e),
            "interval_minutes": 30,
            "last_cycle": "",
        }
        activity = []
        alerts = []

    return templates.TemplateResponse(
        request,
        "squad_dashboard.html",
        {
            "page": "squad-dashboard",
            "squad_status": squad_status,
            "activity": activity,
            "alerts": alerts,
        },
    )


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
    """JSON endpoint for squad activity stream + gap watch stats."""
    try:
        from research_loop.agents.squad import SquadCoordinator

        coord = SquadCoordinator()
        activity = coord.get_activity(limit=50)

        # Extract arXiv-related events for gap watch stats
        arxiv_events = [e for e in activity if "arxiv" in (e.get("payload") or "").lower()]
        watch_stats = {
            "arxiv_events_today": len(arxiv_events),
            "last_arxiv_event": arxiv_events[0]["ts"] if arxiv_events else None,
            "squad_running": coord.get_status().get("running", False),
        }
        return {
            "activity": activity,
            "status": coord.get_status(),
            "watch_stats": watch_stats,
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
                capsules.append(
                    (
                        c.get("capsule_id", "")[:12],
                        c.get("trigger_topic", "")[:60],
                        c.get("action_gap_type", ""),
                        c.get("action_gap_title", "")[:80],
                        c.get("outcome_success_score", 0.0),
                        c.get("created_at", "")[:10],
                        c.get("trigger_keywords", [])[:5],
                        status,
                    )
                )

        # Gene Pool stats from tracker
        stats = tracker.get_gene_pool_stats()

        # User archetype
        archetype = tracker.get_archetype()

        # Top gap type preferences
        profile = tracker.get_profile()
        gap_prefs = dict(
            sorted((profile.gap_type_preferences or {}).items(), key=lambda x: x[1], reverse=True)
        )

        # Top topics
        topic_freq = dict(
            sorted((profile.topic_frequency or {}).items(), key=lambda x: x[1], reverse=True)[:8]
        )

        # Recent events (last 15)
        recent_events = tracker.get_recent_events(limit=15)
        events_display = []
        for e in reversed(recent_events):
            ts = e.timestamp[11:16] if e.timestamp else ""
            date = e.timestamp[:10] if e.timestamp else ""
            events_display.append(
                (
                    ts,
                    date,
                    e.action.value if hasattr(e.action, "value") else str(e.action),
                    e.topic[:40] if e.topic else "—",
                    e.gap_type or "—",
                    e.gap_title[:50] if e.gap_title else "—",
                )
            )

        # Exploration stats
        exp_stats = tracker.get_exploration_stats()

        # ── Actionable Project Suggestions ────────────────────────────────────────
        # Analyze Gene Pool patterns to generate concrete next-step suggestions
        suggestions = generate_suggestions(capsules, gap_prefs, topic_freq, archetype, tracker)

        # ── Gene Pool Prefetch ─────────────────────────────────────────────────────
        # Find capsules matching the top research topic for prefetch indicator
        prefetched_ids: set = set()
        if topic_freq:
            top_topic = max(topic_freq.items(), key=lambda x: x[1])[0] if topic_freq else ""
            if top_topic:
                from llm.briefing_generator import _match_gene_pool

                matches = _match_gene_pool(top_topic, "", "")
                prefetched_ids = {m.get("capsule_id", "")[:12] for m in matches}

    except Exception as e:
        capsules, stats, archetype, gap_prefs, topic_freq, events_display, exp_stats = (
            [],
            {},
            {},
            {},
            {},
            [],
            {},
        )
        import logging

        logging.getLogger(__name__).warning(f"Insights unavailable: {e}")

    return templates.TemplateResponse(
        request,
        "insights.html",
        {
            "page": "insights",
            "capsules": capsules,
            "gene_pool_stats": stats,
            "archetype": archetype,
            "gap_prefs": gap_prefs,
            "topic_freq": topic_freq,
            "events": events_display,
            "exp_stats": exp_stats,
            "suggestions": suggestions,
            "prefetched_ids": prefetched_ids,
        },
    )


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
            mark_capsule_consumed(source_cap_id, tracker)

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



@app.get("/impact")
async def impact(request: Request):
    """Impact Ranking — leaderboard."""
    db = _get_db()
    rows, _ = db.list_papers(
        limit=100
    )  # no citation_count column — sort in Python after fetching real counts

    papers = []
    for r in rows:
        pid = getattr(r, "paper_id", "") or getattr(r, "id", "")
        if not pid:
            continue
        citation_data = db.get_citation_count(pid)
        year_raw = getattr(r, "published", "") or ""
        try:
            year = int(str(year_raw)[:4]) if year_raw else 2020
        except (ValueError, TypeError):
            year = 2020
        papers.append(
            {
                "paper_id": pid,
                "title": r.title,
                "year": year,
                "citation_count": citation_data.get("forward", 0) or 0,
            }
        )

    # Sort by citation_count desc, then score in Python
    papers.sort(key=lambda p: p["citation_count"], reverse=True)
    papers = papers[:50]

    try:
        from llm.impact_scorer import ImpactScorer

        scorer = ImpactScorer(db=db)
        ranking = scorer.rank_papers(papers, top_k=min(20, len(papers)))
    except Exception:
        ranking = []

    return templates.TemplateResponse(
        request,
        "impact.html",
        {
            "page": "impact",
            "ranking": ranking,
        },
    )


@app.get("/papers/gap-analysis")
async def papers_gap_analysis(request: Request, ids: str = ""):
    """Multi-paper gap analysis — surface shared and frontier gaps across N papers."""
    if not ids:
        return templates.TemplateResponse(
            request,
            "generic.html",
            {
                "page": "papers-gap-analysis",
                "title": "Gap Analysis",
                "content": """
            <div class="gap-analysis-empty">
                <div class="gap-analysis-empty-icon">🔬</div>
                <div class="gap-analysis-empty-msg">No papers selected.</div>
                <div class="gap-analysis-empty-sub">Select 2+ papers from the Papers page, then click "Analyze Gaps".</div>
            </div>""",
            },
        )

    paper_ids = [i.strip() for i in ids.split(",") if i.strip()]
    if len(paper_ids) < 2:
        return templates.TemplateResponse(
            request,
            "generic.html",
            {
                "page": "papers-gap-analysis",
                "title": "Gap Analysis",
                "content": """
            <div class="gap-analysis-empty">
                <div class="gap-analysis-empty-icon">🔬</div>
                <div class="gap-analysis-empty-msg">Need at least 2 papers.</div>
                <div class="gap-analysis-empty-sub">Select more papers from the Papers page.</div>
            </div>""",
            },
        )

    db = _get_db()
    paper_map = db.get_papers_bulk(paper_ids)
    papers = [
        {"id": pid, "title": getattr(p, "title", ""), "abstract": getattr(p, "abstract", "") or ""}
        for pid, p in paper_map.items()
    ]

    from llm.paper_gap_extractor import analyze_multi_paper_gaps

    result = analyze_multi_paper_gaps(papers)
    html = _render_gap_analysis_html(result, papers)
    return templates.TemplateResponse(
        request,
        "generic.html",
        {
            "page": "papers-gap-analysis",
            "title": f"Gap Analysis ({len(papers)} papers)",
            "content": html,
        },
    )


def _render_gap_analysis_html(result: Dict[str, Any], papers: List[Dict[str, Any]]) -> str:
    paper_titles = {p["id"]: p["title"] for p in papers}

    def paper_link(pid: str) -> str:
        title = paper_titles.get(pid, pid)
        return f"<a href='/paper/{pid}'>{title[:60]}</a>"

    sections = []

    # Error
    if "error" in result:
        sections.append(f"<div class='ga-error'>Error: {result['error']}</div>")

    # Shared themes
    themes = result.get("shared_themes", [])
    if themes:
        theme_rows = ""
        for t in themes:
            pids = t.get("papers", [])
            theme_rows += f"<tr><td>{t.get('theme', '')}</td><td>{', '.join(pids)}</td><td>{t.get('strength', '')}</td><td>{t.get('description', '')}</td></tr>"
        sections.append(f"""
        <div class='ga-section'>
          <div class='ga-section-title'>🧠 Shared Themes ({len(themes)})</div>
          <table class='ga-table'>
            <thead><tr><th>Theme</th><th>Papers</th><th>Strength</th><th>Description</th></tr></thead>
            <tbody>{theme_rows}</tbody>
          </table>
        </div>""")

    # Frontier gaps
    frontier = result.get("frontier_gaps", [])
    if frontier:
        gap_rows = ""
        for g in frontier:
            gap_rows += f"<tr><td>{g.get('gap_title', '')}</td><td><span class='ga-tag'>{g.get('gap_type', '')}</span></td><td>{', '.join(g.get('keywords', []))}</td><td>{g.get('summary', '')}</td></tr>"
        sections.append(f"""
        <div class='ga-section'>
          <div class='ga-section-title'>🚀 Frontier Gaps ({len(frontier)})</div>
          <table class='ga-table'>
            <thead><tr><th>Gap Title</th><th>Type</th><th>Keywords</th><th>Summary</th></tr></thead>
            <tbody>{gap_rows}</tbody>
          </table>
        </div>""")

    # Complementary gaps
    comp = result.get("complementary_gaps", [])
    if comp:
        comp_rows = ""
        for g in comp:
            comp_rows += f"<tr><td>{g.get('gap_title', '')}</td><td><span class='ga-tag'>{g.get('gap_type', '')}</span></td><td>{g.get('description', '')}</td></tr>"
        sections.append(f"""
        <div class='ga-section'>
          <div class='ga-section-title'>🔗 Complementary Gaps ({len(comp)})</div>
          <table class='ga-table'>
            <thead><tr><th>Gap Title</th><th>Type</th><th>Description</th></tr></thead>
            <tbody>{comp_rows}</tbody>
          </table>
        </div>""")

    # Contradictions
    contrad = result.get("contradictions", [])
    if contrad:
        contrad_rows = ""
        for c in contrad:
            contrad_rows += f"<tr><td><span class='ga-tag'>{c.get('gap_type', '')}</span></td><td>{c.get('description', '')}</td></tr>"
        sections.append(f"""
        <div class='ga-section'>
          <div class='ga-section-title'>⚡ Contradictions ({len(contrad)})</div>
          <table class='ga-table'>
            <thead><tr><th>Gap Type</th><th>Description</th></tr></thead>
            <tbody>{contrad_rows}</tbody>
          </table>
        </div>""")

    if not sections:
        sections.append(
            "<div class='ga-empty'>No gaps identified. Try papers with more diverse abstracts.</div>"
        )

    return f"""
    <style>
    .ga-section {{ margin-bottom: 32px; }}
    .ga-section-title {{ font-size: 16px; font-weight: bold; color: #1a1a2e; margin-bottom: 12px; padding-bottom: 6px; border-bottom: 2px solid #e8f0fe; }}
    .ga-table {{ width: 100%; border-collapse: collapse; font-size: 13px; }}
    .ga-table th {{ background: #f8f9fa; text-align: left; padding: 8px 12px; border-bottom: 2px solid #ddd; color: #555; font-size: 11px; text-transform: uppercase; }}
    .ga-table td {{ padding: 8px 12px; border-bottom: 1px solid #eee; vertical-align: top; }}
    .ga-table tr:hover td {{ background: #fafbff; }}
    .ga-tag {{ background: #e8f0fe; color: #1a73e8; padding: 2px 8px; border-radius: 4px; font-size: 11px; }}
    .ga-error {{ background: #fef0f0; border: 1px solid #f5c6cb; color: #721c24; padding: 12px; border-radius: 6px; margin-bottom: 16px; }}
    .ga-empty {{ text-align: center; color: #888; padding: 40px; }}
    .gap-analysis-empty {{ text-align: center; padding: 60px 20px; }}
    .gap-analysis-empty-icon {{ font-size: 48px; opacity: 0.4; margin-bottom: 16px; }}
    .gap-analysis-empty-msg {{ font-size: 18px; color: #444; margin-bottom: 8px; }}
    .gap-analysis-empty-sub {{ font-size: 13px; color: #999; }}
    </style>
    {"".join(sections)}"""


@app.get("/papers/gap-analysis/questions")
async def gap_analysis_questions(request: Request, ids: str = ""):
    """Generate research questions from frontier gaps for selected papers."""
    if not ids:
        return templates.TemplateResponse(
            request,
            "generic.html",
            {
                "page": "gap-questions",
                "title": "Research Questions",
                "content": "<div class='gap-analysis-empty'><div class='gap-analysis-empty-icon'>🔬</div><div class='gap-analysis-empty-msg'>No papers selected.</div></div>",
            },
        )

    paper_ids = [i.strip() for i in ids.split(",") if i.strip()]
    if len(paper_ids) < 2:
        return templates.TemplateResponse(
            request,
            "generic.html",
            {
                "page": "gap-questions",
                "title": "Research Questions",
                "content": "<div class='gap-analysis-empty'><div class='gap-analysis-empty-icon'>🔬</div><div class='gap-analysis-empty-msg'>Need at least 2 papers.</div></div>",
            },
        )

    db = _get_db()
    paper_map = db.get_papers_bulk(paper_ids)
    papers = [
        {"id": pid, "title": getattr(p, "title", ""), "abstract": getattr(p, "abstract", "") or ""}
        for pid, p in paper_map.items()
    ]

    from llm.paper_gap_extractor import analyze_multi_paper_gaps, gaps_to_research_questions

    gap_result = analyze_multi_paper_gaps(papers)
    frontier = gap_result.get("frontier_gaps", [])
    paper_titles = {p["id"]: p["title"] for p in papers}
    questions_result = gaps_to_research_questions(frontier, paper_titles)
    html = _render_rq_html(questions_result, frontier, paper_titles)
    return templates.TemplateResponse(
        request,
        "generic.html",
        {
            "page": "gap-questions",
            "title": f"Research Questions ({len(papers)} papers)",
            "content": html,
        },
    )


def _render_rq_html(
    result: Dict[str, Any], frontier_gaps: List[Dict[str, Any]], paper_titles: Dict[str, str]
) -> str:
    DIFFICULTY_COLOR = {"easy": "#4CAF50", "medium": "#FF9800", "hard": "#F44336"}

    questions = result.get("questions", [])
    if not questions:
        error = result.get("error", "")
        return f"<div class='ga-empty'>No questions generated. {error}</div>"

    q_rows = ""
    for i, q in enumerate(questions, 1):
        diff = q.get("difficulty", "medium").lower()
        diff_color = DIFFICULTY_COLOR.get(diff, "#757575")
        gap_title = q.get("gap_title", "")
        gap_type = q.get("gap_type", "")
        keywords = ", ".join(q.get("keywords", [])[:6])
        hypothesis = q.get("hypothesis", "")

        q_rows += f"""
        <div class='rq-item'>
          <div class='rq-header'>
            <span class='rq-num'>{i}</span>
            <div class='rq-question'>{q.get("question", "?")}</div>
            <span class='rq-diff' style='color:{diff_color};'>{diff.upper()}</span>
          </div>
          <div class='rq-meta'>
            <span class='ga-tag'>{gap_type}</span>
            <span class='rq-kw'>{keywords}</span>
          </div>
          <div class='rq-gap-title'>From gap: {gap_title}</div>
          {"<div class='rq-hypothesis'>💡 Hypothesis: " + hypothesis + "</div>" if hypothesis else ""}
        </div>"""

    return f"""
    <style>
    .rq-item {{ background: #fff; border: 1px solid #e0e8f0; border-radius: 10px; padding: 16px 20px; margin-bottom: 16px; box-shadow: 0 2px 6px rgba(0,0,0,0.06); }}
    .rq-header {{ display: flex; align-items: flex-start; gap: 12px; margin-bottom: 10px; }}
    .rq-num {{ background: #1a73e8; color: #fff; width: 26px; height: 26px; border-radius: 50%; display: flex; align-items: center; justify-content: center; font-size: 12px; font-weight: bold; flex-shrink: 0; padding-top: 1px; }}
    .rq-question {{ flex: 1; font-size: 15px; color: #1a1a2e; line-height: 1.5; }}
    .rq-diff {{ font-size: 11px; font-weight: bold; flex-shrink: 0; padding-top: 4px; }}
    .rq-meta {{ display: flex; gap: 8px; align-items: center; flex-wrap: wrap; margin-bottom: 6px; }}
    .rq-kw {{ font-size: 12px; color: #666; }}
    .rq-gap-title {{ font-size: 12px; color: #888; margin-bottom: 6px; }}
    .rq-hypothesis {{ font-size: 13px; color: #555; background: #f8f4e8; border-left: 3px solid #f0c040; padding: 8px 12px; border-radius: 4px; margin-top: 6px; line-height: 1.5; }}
    .ga-tag {{ background: #e8f0fe; color: #1a73e8; padding: 2px 8px; border-radius: 4px; font-size: 11px; }}
    .ga-empty {{ text-align: center; color: #888; padding: 40px; }}
    </style>
    <div class='rq-list'>{q_rows}</div>"""


# ── Experiment Proposals ─────────────────────────────────────────────────────


@app.get("/insights/experiments")
async def insights_experiments(request: Request):
    """List queued experiment proposals."""
    queue = get_experiment_queue()
    return templates.TemplateResponse(
        request,
        "generic.html",
        {
            "page": "experiments",
            "title": "🔬 Experiment Proposals",
            "content": render_experiments_html(queue),
        },
    )



@app.post("/insights/generate-experiment")
async def generate_experiment(request: Request):
    """Generate a concrete experiment proposal from a gap/suggestion.

    If source_cap_id is provided, resolves it to a paper_id so the
    experiment can run Paper2Code when executed.
    """
    body = await request.json()
    gap_type = body.get("gap_type", "")
    topic = body.get("topic", "")
    gap_title = body.get("title", "")
    description = body.get("body", "")
    keywords = body.get("keywords", [])
    source_cap_id = body.get("source_cap_id", "")

    # Resolve source_cap_id → paper_id
    paper_id = ""
    if source_cap_id:
        try:
            from llm.gene_pool_io import load_capsules
            capsules = load_capsules()
            for c in capsules:
                if c.get("capsule_id", "") == source_cap_id:
                    paper_id = c.get("archetype", {}).get("source_paper_id", "") or ""
                    break
        except Exception:
            pass

    try:
        from llm.paper_gap_extractor import gaps_to_research_questions

        frontier_gaps = [
            {
                "gap_title": gap_title or topic or "Research gap",
                "gap_type": gap_type,
                "keywords": keywords,
                "summary": description,
            }
        ]
        questions_result = gaps_to_research_questions(frontier_gaps)
        questions = questions_result.get("questions", [])
        if questions:
            q = questions[0]  # take top question as experiment
            exp_title = q.get("question", gap_title or topic)[:100]
        else:
            q = {}
            exp_title = gap_title or topic or "Experiment"
        exp_id = f"exp_{gap_type[:10]}_{topic[:15].replace(' ', '_')}"
        exp = {
            "id": exp_id,
            "title": exp_title,
            "gap_type": gap_type,
            "topic": topic,
            "paper_id": paper_id,
            "description": q.get("question", description),
            "hypothesis": q.get("hypothesis", ""),
            "difficulty": q.get("difficulty", "medium"),
            "method": q.get("question", ""),
            "keywords": keywords,
            "status": "pending",
            "created_at": datetime.now().isoformat(),
        }
        save_experiment(exp)
        return {"success": True, "experiment": exp}
    except Exception as e:
        import logging

        logging.getLogger(__name__).warning(f"Experiment generation failed: {e}")
        return {"success": False, "error": str(e)}


@app.post("/insights/run-experiment")
async def run_experiment(request: Request):
    """Trigger paper2code pipeline for an experiment proposal (background)."""
    body = await request.json()
    exp_id = body.get("id", "")
    queue = get_experiment_queue()
    exp = next((e for e in queue if e.get("id") == exp_id), None)
    if not exp:
        return {"success": False, "error": "Experiment not found"}

    # Update status
    exp["status"] = "running"
    exp["started_at"] = datetime.now().isoformat()
    _save_experiment(exp)

    # Run in background thread
    import threading

    def _run():
        try:
            paper_id = exp.get("paper_id", "")
            if paper_id:
                from research_loop.paper2code_integration import PaperPipeline

                pipeline = PaperPipeline()
                result = pipeline.run(paper_id)
                exp["status"] = "done"
                exp["result"] = result
            else:
                # No specific paper — just update Gene Pool with verdict-based score
                tracker = _get_tracker()
                tracker.record_gap_accept(
                    topic=exp.get("topic", "experiment"),
                    gap_type=exp.get("gap_type", ""),
                    gap_title=exp.get("title", "")[:200],
                    gap_description=exp.get("description", ""),
                )
                exp["status"] = "done"
                exp["result"] = {"verdict_encoded": True}
        except Exception as e:
            exp["status"] = "failed"
            exp["error"] = str(e)
        finally:
            save_experiment(exp)

    t = threading.Thread(target=_run, daemon=True)
    t.start()
    return {"success": True, "message": f"Experiment '{exp_id}' started in background"}


def _get_tracker():
    from llm.insight.tracker import EvolutionTracker

    return EvolutionTracker()


# ── Research Log ────────────────────────────────────────────────────────────────


@app.get("/research-log")
async def research_log(request: Request, paper_id: str = ""):
    """Research Log page — view and add research notes."""
    from llm.research_log import render_log

    html = render_log(paper_id or None)
    return templates.TemplateResponse(
        request,
        "generic.html",
        {
            "page": "research-log",
            "title": "Research Log",
            "content": html,
        },
    )


@app.post("/research-log/note")
async def add_note(request: Request):
    """Append a research note."""
    from fastapi.responses import JSONResponse

    body = await request.json()
    paper_id = body.get("paper_id", "")
    note = body.get("note", "")
    tags = body.get("tags", [])
    from llm.research_log import add_note

    ok = add_note(paper_id, note, tags)
    return JSONResponse({"success": ok})


@app.get("/research-log/notes")
async def get_notes(request: Request, paper_id: str = ""):
    """Get notes JSON, optionally filtered by paper_id."""
    from fastapi.responses import JSONResponse
    from llm.research_log import get_notes

    notes = get_notes(paper_id or None)
    return JSONResponse({"notes": notes})


# ── Paper2Code Dashboard ─────────────────────────────────────────────────────

PAPER2CODE_DIR = Path.home() / ".ai_research_os" / "paper2code"
PAPER2CODE_DIR.mkdir(parents=True, exist_ok=True)


def _get_paper2code_results() -> List[Dict[str, Any]]:
    try:
        if not PAPER2CODE_DIR.exists():
            return []
        files = sorted(
            PAPER2CODE_DIR.glob("result_*.json"),
            key=lambda p: p.stat().st_mtime,
            reverse=True,
        )
        return [json.loads(f.read_text(encoding="utf-8")) for f in files[:50]]
    except Exception:
        return []


def _save_paper2code_result(result: Dict[str, Any]) -> None:
    slug = result.get("arxiv_id", "unknown").replace("/", "_").replace(":", "_")
    path = PAPER2CODE_DIR / f"result_{slug}.json"
    path.write_text(json.dumps(result, indent=2, ensure_ascii=False), encoding="utf-8")


def _render_paper2code_html(results: List[Dict[str, Any]]) -> str:
    lines = ['<div class="paper2code-dash">']

    # Run form
    lines.append("""
    <div class="card" style="margin-bottom:24px;">
      <div class="card-title">⚡ Run Paper2Code Pipeline</div>
      <p style="font-size:13px;color:var(--ink-faint);margin-bottom:12px;">
        Download an arXiv paper, generate code skeleton, extract tests, run benchmarks, and encode results to the Gene Pool.
      </p>
      <form id="p2c-form" style="display:flex;gap:12px;align-items:flex-end;flex-wrap:wrap;">
        <div>
          <label style="font-size:11px;color:var(--ink-faint);display:block;margin-bottom:4px;">arXiv ID</label>
          <input type="text" id="arxiv-id" placeholder="e.g. 1706.03762" required
                 style="padding:8px 12px;border:1px solid var(--border);border-radius:4px;font-size:13px;width:200px;">
        </div>
        <div>
          <label style="font-size:11px;color:var(--ink-faint);display:block;margin-bottom:4px;">Framework</label>
          <select id="framework" style="padding:8px 12px;border:1px solid var(--border);border-radius:4px;font-size:13px;">
            <option value="pytorch">PyTorch</option>
            <option value="jax">JAX</option>
            <option value="numpy">NumPy</option>
          </select>
        </div>
        <button type="submit" class="btn btn-primary" style="font-size:14px;">▶ Run</button>
      </form>
      <div id="p2c-progress" style="margin-top:12px;display:none;">
        <div style="display:flex;align-items:center;gap:12px;margin-bottom:6px;">
          <span id="p2c-stage" style="font-size:12px;font-weight:600;color:var(--pen-blue);text-transform:uppercase;letter-spacing:0.5px;">—</span>
          <span id="p2c-message" style="font-size:13px;color:var(--ink);">—</span>
        </div>
        <div style="height:6px;background:var(--paper-alt);border-radius:3px;border:1px solid var(--border-light);overflow:hidden;">
          <div id="p2c-bar" style="height:100%;width:0%;background:var(--pen-green);border-radius:3px;transition:width 0.4s;"></div>
        </div>
      </div>
    </div>
    <script>
    document.getElementById('p2c-form').addEventListener('submit', function(e) {
      e.preventDefault();
      var btn = this.querySelector('button[type=submit]');
      var progressEl = document.getElementById('p2c-progress');
      var stageEl = document.getElementById('p2c-stage');
      var msgEl = document.getElementById('p2c-message');
      var barEl = document.getElementById('p2c-bar');
      btn.disabled = true; btn.textContent = 'Running...';
      progressEl.style.display = 'block';
      stageEl.textContent = 'Starting...';
      msgEl.textContent = 'Queued';
      barEl.style.width = '0%';
      fetch('/paper2code/run', {
        method: 'POST',
        headers: {'Content-Type': 'application/json'},
        body: JSON.stringify({
          arxiv_id: document.getElementById('arxiv-id').value.trim(),
          framework: document.getElementById('framework').value,
        }),
      }).then(function(r) { return r.json(); }).then(function(d) {
        if (d.success && d.job_id) {
          var es = new EventSource('/paper2code/stream/' + d.job_id);
          es.onmessage = function(ev) {
            var data = JSON.parse(ev.data);
            if (data.status === 'done') {
              stageEl.textContent = 'Done';
              msgEl.textContent = '';
              barEl.style.width = '100%';
              es.close();
              setTimeout(function() { location.reload(); }, 1500);
            } else if (data.status === 'failed') {
              stageEl.textContent = 'Failed';
              msgEl.textContent = data.message || 'Error';
              barEl.style.width = '0%';
              barEl.style.background = '#e05050';
              es.close();
            } else {
              stageEl.textContent = data.stage || '—';
              msgEl.textContent = data.message || '—';
              barEl.style.width = (data.progress_pct || 0) + '%';
            }
          };
          es.onerror = function() { es.close(); setTimeout(function() { location.reload(); }, 5000); };
        } else {
          stageEl.textContent = 'Error';
          msgEl.textContent = d.error || 'Failed';
        }
      }).catch(function(err) {
        stageEl.textContent = 'Error';
        msgEl.textContent = err.message;
      });
    });
    </script>
    """)

    # History
    if not results:
        lines.append("""
        <div class="card">
          <div class="card-title">📋 Run History</div>
          <div class="empty-state">
            <div class="empty-state-icon">⚡</div>
            <div class="empty-state-text">No paper2code runs yet. Submit an arXiv ID above to get started.</div>
          </div>
        </div>""")
    else:
        lines.append('<div class="card"><div class="card-title">📋 Run History</div>')
        lines.append("""
        <table style="width:100%;border-collapse:collapse;font-size:12px;">
        <thead>
          <tr style="border-bottom:2px solid var(--border);">
            <th style="padding:8px 10px;text-align:left;color:var(--ink-faint);">arXiv</th>
            <th style="padding:8px 10px;text-align:left;color:var(--ink-faint);">Framework</th>
            <th style="padding:8px 10px;text-align:left;color:var(--ink-faint);">Pass</th>
            <th style="padding:8px 10px;text-align:left;color:var(--ink-faint);">Fail</th>
            <th style="padding:8px 10px;text-align:left;color:var(--ink-faint);">Gene Pool</th>
            <th style="padding:8px 10px;text-align:left;color:var(--ink-faint);">When</th>
          </tr>
        </thead>
        <tbody>""")
        for r in results:
            arxiv = r.get("arxiv_id", "?")
            fw = r.get("framework", "pytorch")
            passed = r.get("passed", 0)
            failed = r.get("failed", 0)
            skipped = r.get("skipped", 0)
            gp = r.get("gene_pool_encoded", False)
            ts = r.get("created_at", "")[:19]
            status = r.get("status", "done")
            status_dot = {"done": "✅", "failed": "❌", "running": "⏳", "pending": "⏳"}.get(status, "❓")
            lines.append(f"""
            <tr style="border-bottom:1px solid var(--border-light);">
              <td style="padding:8px 10px;"><a href="/paper/{arxiv}" style="color:var(--pen-blue);">{arxiv}</a></td>
              <td style="padding:8px 10px;">{fw}</td>
              <td style="padding:8px 10px;color:var(--pen-green);">{passed}</td>
              <td style="padding:8px 10px;color:#e05050;">{failed}</td>
              <td style="padding:8px 10px;">{"✅" if gp else "—"}</td>
              <td style="padding:8px 10px;color:var(--ink-faint);">{ts}</td>
            </tr>""")
        lines.append("</tbody></table></div>")

    lines.append("</div>")
    return "\n".join(lines)


@app.get("/paper2code")
async def paper2code_dashboard(request: Request):
    """Paper2Code pipeline dashboard — run and view results."""
    results = _get_paper2code_results()
    html = _render_paper2code_html(results)
    return templates.TemplateResponse(
        request,
        "generic.html",
        {"page": "paper2code", "title": "⚡ Paper2Code Pipeline", "content": html},
    )


@app.get("/paper2code/stream/{job_id}")
async def paper2code_stream(job_id: str):
    """SSE endpoint — live Paper2Code pipeline progress."""
    from fastapi.responses import StreamingResponse
    import asyncio

    async def event_generator():
        import json as _json
        _last = None
        while True:
            state = p2c_progress.get(job_id)
            if state and state != _last:
                _last = dict(state)
                yield f"data: {_json.dumps(state)}\n\n"
                if state["status"] in ("done", "failed"):
                    yield f"data: {_json.dumps({'status': 'done'})}\n\n"
                    return
            await asyncio.sleep(1)

    return StreamingResponse(
        event_generator(),
        media_type="text/event-stream",
        headers={
            "Cache-Control": "no-cache",
            "Connection": "keep-alive",
            "X-Accel-Buffering": "no",
        },
    )


@app.post("/paper2code/run")
async def paper2code_run(request: Request):
    """Run the Paper2Code pipeline for a given arXiv ID."""
    body = await request.json()
    arxiv_id = body.get("arxiv_id", "").strip()
    framework = body.get("framework", "pytorch")

    if not arxiv_id:
        return {"success": False, "error": "arxiv_id is required"}

    job_id = arxiv_id.replace(".", "_")
    p2c_progress.create(job_id)

    # Save pending record
    record = {
        "arxiv_id": arxiv_id,
        "framework": framework,
        "status": "running",
        "passed": 0,
        "failed": 0,
        "skipped": 0,
        "gene_pool_encoded": False,
        "created_at": datetime.now().isoformat(),
    }
    _save_paper2code_result(record)

    import threading

    def _run():
        try:
            p2c_progress.update(job_id, status="running", stage="parse", message="Downloading paper...", progress_pct=10)
            from research_loop.paper2code_integration import PaperPipeline

            pipeline = PaperPipeline()

            p2c_progress.update(job_id, stage="generate", message="Generating code skeleton...", progress_pct=30)
            p2c_progress.update(job_id, stage="test", message="Extracting tests...", progress_pct=50)
            p2c_progress.update(job_id, stage="benchmark", message="Running benchmarks...", progress_pct=70)
            result = pipeline.run(arxiv_id, framework=framework)

            p2c_progress.update(job_id, stage="encode", message="Encoding to Gene Pool...", progress_pct=90)
            if result and isinstance(result, dict):
                record["passed"] = result.get("passed", 0)
                record["failed"] = result.get("failed", 0)
                record["skipped"] = result.get("skipped", 0)
                record["gene_pool_encoded"] = (
                    result.get("gene_pool_encoded", False) or result.get("capsule_id") is not None
                )
                record["status"] = "done" if record["failed"] == 0 else "failed"
            else:
                record["status"] = "done"
            p2c_progress.update(job_id, status=record["status"], message="Done", progress_pct=100)
        except Exception as e:
            record["status"] = "failed"
            p2c_progress.update(job_id, status="failed", message=str(e)[:100], progress_pct=0)
            import logging
            logging.getLogger(__name__).warning(f"paper2code run failed for {arxiv_id}: {e}")
        finally:
            _save_paper2code_result(record)

    t = threading.Thread(target=_run, daemon=True)
    t.start()
    return {"success": True, "job_id": job_id, "message": f"Paper2Code pipeline started for {arxiv_id}"}


if __name__ == "__main__":
    import uvicorn

    uvicorn.run(app, host="0.0.0.0", port=8501)
