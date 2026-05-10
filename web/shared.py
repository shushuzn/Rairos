"""Shared state for Rairos web app — templates, DB, helpers."""

from __future__ import annotations

import json
import sys
import threading
from datetime import datetime
from pathlib import Path
from typing import Any, Dict, List, Optional

from fastapi.templating import Jinja2Templates

# Project root
PROJECT_ROOT = Path(__file__).parent.parent
sys.path.insert(0, str(PROJECT_ROOT))

WEB_DIR = Path(__file__).parent
templates = Jinja2Templates(directory=str(WEB_DIR / "templates"))


# Jinja filters
def _jinja_truncate(value: Any, length: int = 80) -> str:
    s = str(value)
    return s[:length] + "…" if len(s) > length else s


def _jinja_timestamp(value: Any) -> str:
    try:
        return datetime.fromtimestamp(float(value)).strftime("%H:%M:%S")
    except Exception:
        return str(value)[:8]


templates.env.filters["truncate"] = _jinja_truncate
templates.env.filters["timestamp"] = _jinja_timestamp


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
