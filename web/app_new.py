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
         "queue_list": queue_list, "parsing": parsing},
    )


@app.get("/papers")
async def papers(request: Request, q: str = "", source: str = "", page: int = 1):
    """Papers — search and list with pagination."""
    db = _get_db()
    limit = 20
    offset = (max(1, page) - 1) * limit

    if q:
        rows, total = db.search_papers(q, limit=limit, offset=offset)
    else:
        rows, total = db.list_papers(limit=limit, offset=offset, source=source if source else None)

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
            "error": f"Paper '{paper_id}' not found.",
        })

    authors = ", ".join((paper.authors or [])[:10])
    year = (paper.published or "")[:4] if paper.published else "?"
    all_categories = (paper.categories or "").split(",")[:8]

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
        "error": None,
    })


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
