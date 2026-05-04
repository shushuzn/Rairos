"""Briefing generation and history web routes."""
from __future__ import annotations

from fastapi import APIRouter, Request
from web.shared import templates, get_db

router = APIRouter()


@router.get("/briefing")
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



@router.post("/briefing")
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



@router.get("/briefing/distribute/{arxiv_id}")
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



@router.get("/b/{short_id}")
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



@router.get("/briefing/history")
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


