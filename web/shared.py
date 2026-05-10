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
