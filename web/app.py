"""
Rairos — FastAPI + Jinja2 Hand-Drawn UI.
Run: uvicorn web.app:app --reload --port 8501
"""

from __future__ import annotations

import sys
from pathlib import Path

# Add project root to path
PROJECT_ROOT = Path(__file__).parent.parent
sys.path.insert(0, str(PROJECT_ROOT))

from typing import Any, Dict, List
from fastapi import FastAPI, Request
from fastapi.responses import RedirectResponse
from fastapi.staticfiles import StaticFiles
from fastapi.templating import Jinja2Templates


app: FastAPI = FastAPI(title="Rairos", description="AI Research OS — Hand-drawn UI")
from web import routes_insights

app.include_router(routes_insights.router)  # type: ignore[has-type]

from web import routes_misc

app.include_router(routes_misc.router)  # type: ignore[has-type]

from web import routes_embodied

app.include_router(routes_embodied.router)  # type: ignore[has-type]

from web import routes_briefing

app.include_router(routes_briefing.router)  # type: ignore[has-type]

from web import routes_papers

app.include_router(routes_papers.router)  # type: ignore[has-type]

from web import routes_daemon

app.include_router(routes_daemon.router)  # type: ignore[has-type]

from web import routes_research

app.include_router(routes_research.router)  # type: ignore[has-type]

from web import routes_gene_pool

app.include_router(routes_gene_pool.router)  # type: ignore[has-type]

from web import routes_news

app.include_router(routes_news.router)  # type: ignore[has-type]


# Graceful error handler — catches ALL exceptions in route handlers
from starlette.exceptions import HTTPException as _HTTPExc


@app.exception_handler(Exception)
async def catch_all_handler(request: Request, exc: Exception):
    return templates.TemplateResponse(
        request,
        "generic.html",
        {
            "page": "",
            "title": "Not Available",
            "content": "<p>This feature is not available in this build.</p>",
        },
    )


@app.exception_handler(_HTTPExc)
async def http_exception_handler(request: Request, exc: _HTTPExc):
    if exc.status_code == 404:
        return templates.TemplateResponse(
            request,
            "generic.html",
            {
                "page": "",
                "title": "Not Available",
                "content": "<p>This feature is not available in this build.</p>",
            },
            status_code=404,
        )
    raise exc


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


@app.get("/report")
async def reports_page(request: Request):
    """Research reports by theme."""
    from llm.reports import report_all

    return templates.TemplateResponse(
        request,
        "reports.html",
        {"page": "reports", "title": "Reports", "reports_content": report_all()},
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


# ── Task 5: arXiv主动搜索 ──────────────────────────────────────────────────


# ── Squad Coordinator ────────────────────────────────────────────────────────────


# ── Research Log ────────────────────────────────────────────────────────────────


# ── Experiment Proposals ─────────────────────────────────────────────────────


def _get_tracker():
    from llm.insight.tracker import EvolutionTracker

    return EvolutionTracker()


# ── Research Log ────────────────────────────────────────────────────────────────

# ── Paper2Code Dashboard ─────────────────────────────────────────────────────

# PAPER2CODE_DIR moved to web/shared.py to break circular imports
from web.shared import PAPER2CODE_DIR as PAPER2CODE_DIR


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
            r.get("skipped", 0)  # noqa: F841
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


if __name__ == "__main__":
    import uvicorn

    uvicorn.run(app, host="0.0.0.0", port=8501)
