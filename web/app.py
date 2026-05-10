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

from typing import Any, Dict, List, Optional
from fastapi import FastAPI, Request, Form
from fastapi.responses import RedirectResponse
from fastapi.staticfiles import StaticFiles
from fastapi.templating import Jinja2Templates


from web.shared import templates as _templates, get_db, get_tracker, p2c_progress
from web.renderer import render_gene_pool_graph_html

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
from starlette.responses import HTMLResponse


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


# In-memory notification store (per-process, reset on restart — lightweight)
_notification_store: List[Dict[str, Any]] = []

# ── Task 5: arXiv主动搜索 ──────────────────────────────────────────────────


# ── Squad Coordinator ────────────────────────────────────────────────────────────
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


def _get_tracker():
    from llm.insight.tracker import EvolutionTracker

    return EvolutionTracker()


# ── Research Log ────────────────────────────────────────────────────────────────

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
