"""Miscellaneous web routes: auth, chat, notifications, etc."""

from __future__ import annotations

from fastapi import APIRouter, Request
from datetime import datetime as _datetime
from typing import Any, Dict, List
from web.shared import templates, get_db, p2c_progress, _notification_store, _save_paper2code_result

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


def get_paper2code_results():
    return []


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
        "created_at": _datetime.now().isoformat(),
    }
    _save_paper2code_result(record)

    import threading

    def _run():
        try:
            p2c_progress.update(
                job_id,
                status="running",
                stage="parse",
                message="Downloading paper...",
                progress_pct=10,
            )
            from research_loop.paper2code_integration import PaperPipeline

            pipeline = PaperPipeline()

            p2c_progress.update(
                job_id, stage="generate", message="Generating code skeleton...", progress_pct=30
            )
            p2c_progress.update(
                job_id, stage="test", message="Extracting tests...", progress_pct=50
            )
            p2c_progress.update(
                job_id, stage="benchmark", message="Running benchmarks...", progress_pct=70
            )
            result = pipeline.run(arxiv_id, framework=framework)

            p2c_progress.update(
                job_id, stage="encode", message="Encoding to Gene Pool...", progress_pct=90
            )
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
    return {
        "success": True,
        "job_id": job_id,
        "message": f"Paper2Code pipeline started for {arxiv_id}",
    }

    # --- Fallback routes for sidebar items (graceful "not available") ---

    # removed old fallback(request: Request):
    return templates.TemplateResponse(
        request,
        "generic.html",
        {"page": "chat", "title": "Chat", "content": "<p>Chat module loading...</p>"},
    )


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
            f"[{badge}] score={c.outcome_success_score:.2f} cred={c.credibility_score:.2f} {c.action_gap_type}</div>"
            f'<div style="font-size:14px;margin-top:4px;">{c.action_gap_title}</div>'
            f"</div>"
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
        {"".join(cards)}
    </div>"""
    return templates.TemplateResponse(
        request, "generic.html", {"page": "chat", "title": "Gene Pool", "content": html}
    )


@router.get("/citation-chain")
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


@router.get("/citation-chain/graph")
async def citation_chain_graph(request: Request, paper_id: str = "", title: str = ""):
    """Interactive SVG citation graph: paper → cited refs → Gene Pool capsules."""
    from llm.citation_pathfinder_web import render_citation_chain_html

    cited_paper_ids = ["p1", "p2", "p3"]  # placeholders; real impl reads from DB
    cited_capsule_ids: List[str] = []
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


@router.get("/arxiv-channels")
async def arxiv_channels(request: Request):
    """arXiv Watch Alert Channels — configure multiple feed configs."""
    from llm.arxiv_alert_channels import render_channels_html

    db = get_db()
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


@router.post("/arxiv-channels/toggle/{channel_id}")
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


@router.post("/arxiv-channels/check")
async def arxiv_check(request: Request):
    """Run arXiv subscription check across all enabled subscriptions."""
    from fastapi.responses import JSONResponse

    try:
        db = get_db()
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


@router.get("/climate-monitor")
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


@router.post("/climate-monitor/toggle-watch")
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


@router.get("/voice-capsule")
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


@router.post("/voice-capsule/transcribe")
async def voice_transcribe(request: Request):
    """Receive audio file, transcribe with Whisper, extract gap with LLM."""
    from llm.voice_to_capsule import extract_gap_from_text, transcribe_audio
    from fastapi.responses import JSONResponse

    try:
        form = await request.form()
        audio_file = form.get("audio")
        if not audio_file:
            return JSONResponse({"error": "No audio file"}, status_code=400)
        audio_bytes = await audio_file.read()  # type: ignore[union-attr]
        text = transcribe_audio(audio_bytes)
        if text.startswith("[Transcription error"):
            return JSONResponse({"error": text})
        gap = extract_gap_from_text(text)
        return JSONResponse(gap)
    except Exception as e:
        return JSONResponse({"error": str(e)}, status_code=500)


@router.post("/voice-capsule/save")
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


@router.get("/policy-impact")
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


@router.get("/labor-displacement")
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


@router.get("/researchers")
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


@router.post("/researchers/add")
async def add_researcher_route(request: Request):
    from llm.multi_researcher import add_researcher
    from fastapi.responses import JSONResponse

    body = await request.json()
    uid = body.get("user_id", "")
    name = body.get("name", "")
    ok = add_researcher(uid, name)
    return JSONResponse({"success": ok, "error": None if ok else "already exists"})


@router.get("/researchers/capsules/{user_id}")
async def researcher_capsules(user_id: str, request: Request):
    from llm.multi_researcher import get_capsules_for_user
    from fastapi.responses import JSONResponse

    capsules = get_capsules_for_user(user_id)
    return JSONResponse({"count": len(capsules), "capsules": capsules[:10]})


@router.get("/insights/queue")
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


@router.post("/insights/queue/verdict")
async def submit_verdict(request: Request):
    """Record a user's verdict on a queued capsule."""
    from llm.insight.tracker import EvolutionTracker
    from llm.review_queue import _load_capsules

    body = await request.json()
    capsule_id = body.get("capsule_id", "")

    capsules = _load_capsules()
    for cap in capsules:
        if cap.get("capsule_id", "") == capsule_id:
            tracker = EvolutionTracker()
            tracker.record_gap_accept(topic=capsule_id, gap_type="queued_capsule", gap_title=capsule_id)
            break

    return {"success": True}


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
            _skipped = r.get("skipped", 0)
            gp = r.get("gene_pool_encoded", False)
            ts = r.get("created_at", "")[:19]
            status = r.get("status", "done")
            _status_dot = {"done": "✅", "failed": "❌", "running": "⏳", "pending": "⏳"}.get(
                status, "❓"
            )
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
