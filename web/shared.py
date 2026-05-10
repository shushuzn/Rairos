"""Shared state for Rairos web app — templates, DB, helpers."""

from __future__ import annotations

import json
import sys
import threading
from datetime import datetime
from pathlib import Path
from typing import Any, Dict, List, Optional

# Project root
PROJECT_ROOT = Path(__file__).parent.parent
sys.path.insert(0, str(PROJECT_ROOT))

WEB_DIR = Path(__file__).parent

# Lazy-init: Jinja2Templates import deferred so ProgressStore can be tested
# without requiring fastapi. Routes use `from web.shared import templates` which
# triggers the real import; tests only need ProgressStore.
_templates_instance = None
_filters_registered = False

def _get_templates():
    global _templates_instance, _filters_registered
    if _templates_instance is None:
        from fastapi.templating import Jinja2Templates

        _templates_instance = Jinja2Templates(directory=str(WEB_DIR / "templates"))
        # Register filters on first access (deferred past module-load time)
        if not _filters_registered:
            _templates_instance.env.filters["truncate"] = _jinja_truncate
            _templates_instance.env.filters["timestamp"] = _jinja_timestamp
            _filters_registered = True
    return _templates_instance

class _TemplatesProxy:
    """Proxy that lazy-loads Jinja2Templates on first attribute access."""
    def __getattr__(self, name):
        return getattr(_get_templates(), name)

templates = _TemplatesProxy()


# ── Jinja filters (used by templates) ───────────────────────────────────────


def _jinja_truncate(value: Any, length: int = 80) -> str:
    s = str(value)
    return s[:length] + "…" if len(s) > length else s


def _jinja_timestamp(value: Any) -> str:
    try:
        return datetime.fromtimestamp(float(value)).strftime("%H:%M:%S")
    except Exception:
        return str(value)[:8]


def get_db():
    from db.database import Database

    db = Database()
    db.init()
    return db


def get_tracker():
    from llm.insight.tracker import EvolutionTracker

    return EvolutionTracker()


# ── Paper2Code Progress Store ──────────────────────────────────────────────


class ProgressStore:
    """Thread-safe in-memory store for pipeline progress."""

    def __init__(self):
        self._lock = threading.Lock()
        self._jobs: Dict[str, Dict[str, Any]] = {}

    def create(self, job_id: str) -> None:
        with self._lock:
            self._jobs[job_id] = {
                "status": "pending",
                "stage": "",
                "message": "Queued",
                "progress_pct": 0,
            }

    def update(
        self,
        job_id: str,
        status: str = "",
        stage: str = "",
        message: str = "",
        progress_pct: int = -1,
    ) -> None:
        with self._lock:
            job = self._jobs.get(job_id)
            if not job:
                return
            if status:
                job["status"] = status
            if stage:
                job["stage"] = stage
            if message:
                job["message"] = message
            if progress_pct >= 0:
                job["progress_pct"] = progress_pct

    def get(self, job_id: str) -> Optional[Dict[str, Any]]:
        with self._lock:
            return self._jobs.get(job_id)

    def cleanup(self, max_age_seconds: int = 600) -> None:
        with self._lock:
            self._jobs.clear()


p2c_progress = ProgressStore()

# ── In-memory notification store (per-process) ────────────────────────────────
# Moved from web/app.py to break circular import:
# routes_misc.py and routes_embodied.py import this at module level, but
# _notification_store was defined after those imports in app.py.

_notification_store: List[Dict[str, Any]] = []

# ── Paper2Code result persistence ────────────────────────────────────────────

PAPER2CODE_DIR = Path.home() / ".ai_research_os" / "paper2code"
PAPER2CODE_DIR.mkdir(parents=True, exist_ok=True)


def _save_paper2code_result(result: Dict[str, Any]) -> None:
    slug = result.get("arxiv_id", "unknown").replace("/", "_").replace(":", "_")
    path = PAPER2CODE_DIR / f"result_{slug}.json"
    path.write_text(json.dumps(result, indent=2, ensure_ascii=False), encoding="utf-8")


# ── Gap Analysis HTML renderer ─────────────────────────────────────────────────


def _render_gap_analysis_html(result: Dict[str, Any], papers: List[Dict[str, Any]]) -> str:
    paper_titles = {p["id"]: p["title"] for p in papers}

    def paper_link(pid: str) -> str:
        title = paper_titles.get(pid, pid)
        return f"<a href='/paper/{pid}'>{title[:60]}</a>"

    sections = []

    if "error" in result:
        sections.append(f"<div class='ga-error'>Error: {result['error']}</div>")

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


# ── Research Questions HTML renderer ──────────────────────────────────────────


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


# ── Paper2Code results helper ───────────────────────────────────────────────────


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
