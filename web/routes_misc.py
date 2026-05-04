"""Miscellaneous web routes: auth, chat, notifications, etc."""
from __future__ import annotations

from fastapi import APIRouter, Request
from fastapi.responses import RedirectResponse
from web.shared import templates

router = APIRouter()


@router.get("/notifications")
async def get_notifications(request: Request):
    """Get current notifications (contradictions, trends, alerts)."""
    from fastapi.responses import JSONResponse

    return JSONResponse({"notifications": _notification_store})



@router.post("/notifications/dismiss")
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



@router.get("/research-log")
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



@router.post("/research-log/note")
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



@router.get("/research-log/notes")
async def get_notes(request: Request, paper_id: str = ""):
    """Get notes JSON, optionally filtered by paper_id."""
    from fastapi.responses import JSONResponse
    from llm.research_log import get_notes

    notes = get_notes(paper_id or None)
    return JSONResponse({"notes": notes})



@router.get("/paper2code")
async def paper2code_dashboard(request: Request):
    """Paper2Code pipeline dashboard — run and view results."""
    results = get_paper2code_results()
    html = _render_paper2code_html(results)
    return templates.TemplateResponse(
        request,
        "generic.html",
        {"page": "paper2code", "title": "⚡ Paper2Code Pipeline", "content": html},
    )



@router.get("/paper2code/stream/{job_id}")
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



@router.post("/paper2code/run")
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

# --- Fallback routes for sidebar items (graceful "not available") ---

# removed old fallback(request: Request):
    return templates.TemplateResponse(request, "generic.html",
        {"page": "chat", "title": "Chat", "content": "<p>Chat module loading...</p>"})

@router.get("/citation-chain")
async def _citation_fallback(request: Request):
    return templates.TemplateResponse(request, "generic.html",
        {"page": "citation_chain", "title": "Citation Chain", "content": "<p>Citation chain module loading...</p>"})

@router.get("/arxiv-channels")
async def _arxiv_channels_fallback(request: Request):
    return templates.TemplateResponse(request, "generic.html",
        {"page": "arxiv-channels", "title": "arXiv Channels", "content": "<p>arXiv channels module loading...</p>"})

@router.get("/climate-monitor")
async def _climate_fallback(request: Request):
    return templates.TemplateResponse(request, "generic.html",
        {"page": "climate-monitor", "title": "Climate AI", "content": "<p>Climate monitor module loading...</p>"})

@router.get("/voice-capsule")
async def _voice_fallback(request: Request):
    return templates.TemplateResponse(request, "generic.html",
        {"page": "voice-capsule", "title": "Voice Capsule", "content": "<p>Voice capsule module loading...</p>"})

@router.get("/policy-impact")
async def _policy_fallback(request: Request):
    return templates.TemplateResponse(request, "generic.html",
        {"page": "policy-impact", "title": "Policy Impact", "content": "<p>Policy impact module loading...</p>"})

@router.get("/labor-displacement")
async def _labor_fallback(request: Request):
    return templates.TemplateResponse(request, "generic.html",
        {"page": "labor-displacement", "title": "Labor Track", "content": "<p>Labor displacement module loading...</p>"})

@router.get("/researchers")
async def _researchers_fallback(request: Request):
    return templates.TemplateResponse(request, "generic.html",
        {"page": "multi-researcher", "title": "Researchers", "content": "<p>Researchers module loading...</p>"})

@router.get("/insights/queue")
async def _queue_fallback(request: Request):
    return templates.TemplateResponse(request, "generic.html",
        {"page": "review-queue", "title": "Review Queue", "content": "<p>Review queue module loading...</p>"})

def get_paper2code_results():
    return []


# --- Graceful fallbacks for missing sidebar routes ---
MISSING_ROUTES = [
    "/chat", "/citation-chain", "/arxiv-channels", "/climate-monitor",
    "/voice-capsule", "/policy-impact", "/labor-displacement", 
    "/researchers", "/insights/queue",
]

@router.get("/chat")
async def chat_page(request: Request):
    """Gene Pool browser — explore capsules in a chat-like interface."""
    from llm.insight.tracker import EvolutionTracker
    tracker = EvolutionTracker()
    capsules = tracker._load_capsules()

    cards = []
    for c in sorted(capsules, key=lambda x: x.outcome_success_score, reverse=True)[:20]:
        badge = c.credibility_badge.upper()
        cards.append(
            f'<div style="border:1px solid var(--border);border-radius:8px;padding:12px;margin-bottom:8px;">'
            f'<div style="font-size:11px;color:var(--ink-faint);">'
            f'[{badge}] score={c.outcome_success_score:.2f} cred={c.credibility_score:.2f} {c.action_gap_type}</div>'
            f'<div style="font-size:14px;margin-top:4px;">{c.action_gap_title}</div>'
            f'</div>'
        )

    html = f"""
    <style>
    .chat-container {{ max-width: 800px; margin: 0 auto; }}
    .chat-header {{ font-size: 18px; font-weight: 700; margin-bottom: 16px; color: var(--ink); }}
    .chat-count {{ font-size: 13px; color: var(--ink-faint); margin-bottom: 16px; }}
    </style>
    <div class="chat-container">
        <div class="chat-header">Gene Pool Browser</div>
        <div class="chat-count">{len(capsules)} capsules · sorted by score</div>
        {''.join(cards)}
    </div>"""
    return templates.TemplateResponse(request, "generic.html", {"page": "chat", "title": "Gene Pool", "content": html})

@router.get("/citation-chain")
async def citation_fb(request: Request):
    return templates.TemplateResponse(request, "generic.html", {"page": "citation_chain", "title": "Citation Chain", "content": "<p>Citation chain not available</p>"})

@router.get("/arxiv-channels")
async def arxiv_fb(request: Request):
    return templates.TemplateResponse(request, "generic.html", {"page": "arxiv-channels", "title": "arXiv Channels", "content": "<p>arXiv channels not available</p>"})

@router.get("/climate-monitor")
async def climate_fb(request: Request):
    return templates.TemplateResponse(request, "generic.html", {"page": "climate-monitor", "title": "Climate AI", "content": "<p>Climate monitor not available</p>"})

@router.get("/voice-capsule")
async def voice_fb(request: Request):
    return templates.TemplateResponse(request, "generic.html", {"page": "voice-capsule", "title": "Voice Capsule", "content": "<p>Voice capsule not available</p>"})

@router.get("/policy-impact")
async def policy_fb(request: Request):
    return templates.TemplateResponse(request, "generic.html", {"page": "policy-impact", "title": "Policy Impact", "content": "<p>Policy impact not available</p>"})

@router.get("/labor-displacement")
async def labor_fb(request: Request):
    return templates.TemplateResponse(request, "generic.html", {"page": "labor-displacement", "title": "Labor Track", "content": "<p>Labor track not available</p>"})

@router.get("/researchers")
async def researchers_fb(request: Request):
    return templates.TemplateResponse(request, "generic.html", {"page": "multi-researcher", "title": "Researchers", "content": "<p>Multi-researcher not available</p>"})

@router.get("/insights/queue")
async def queue_fb(request: Request):
    return templates.TemplateResponse(request, "generic.html", {"page": "review-queue", "title": "Review Queue", "content": "<p>Review queue not available</p>"})
