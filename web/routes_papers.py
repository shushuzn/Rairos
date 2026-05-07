"""Paper browsing, detail, and gap analysis web routes."""

from __future__ import annotations

from fastapi import APIRouter, Request
from web.shared import templates, get_db

router = APIRouter()


@router.get("/papers")
async def papers(
    request: Request,
    q: str = "",
    source: str = "",
    page: int = 1,
    year_from: str = "",
    year_to: str = "",
):
    """Papers — search and list with pagination."""
    db = get_db()
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


@router.get("/paper/{paper_id}")
async def paper_detail(request: Request, paper_id: str):
    """Paper detail — full metadata."""
    db = get_db()
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


@router.get("/paper/{paper_id}/extract-gap")
async def extract_paper_gap(request: Request, paper_id: str):
    """Extract a research gap from a paper using LLM."""
    db = get_db()
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


@router.post("/paper/{paper_id}/save-gap")
async def save_paper_gap(request: Request, paper_id: str):
    """Save an extracted gap to the Gene Pool."""
    body = await request.json()
    gap_type = body.get("gap_type", "")
    gap_title = body.get("gap_title", "")
    keywords = body.get("keywords", [])
    summary = body.get("summary", "")

    from llm.paper_gap_extractor import save_gap_to_gene_pool

    db = get_db()
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


@router.get("/paper/{paper_id}/rigor")
async def paper_rigor(request: Request, paper_id: str):
    """Compute and return research rigor score for a paper as JSON."""
    from llm.rigor_scorer import RigorScorer

    db = get_db()
    paper = db.get_paper(paper_id)
    if not paper:
        return {"error": f"Paper '{paper_id}' not found."}
    scorer = RigorScorer()
    score = scorer.score_paper(paper_id, abstract=paper.abstract or "", title=paper.title or "")
    return score.to_dict()


@router.get("/paper/{paper_id}/replication")
async def paper_replication(request: Request, paper_id: str):
    """Run replication checker on a paper — returns JSON report."""
    from llm.replication_checker import ReplicationChecker

    db = get_db()
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


@router.get("/papers/gap-analysis")
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

    db = get_db()
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


@router.get("/papers/gap-analysis/questions")
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

    db = get_db()
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
