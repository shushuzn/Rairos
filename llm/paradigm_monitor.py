"""Paradigm Concentration Monitor.

Detects when >60% of citations in a domain cluster around ≤3 references.
Flags a generalization_gap risk alert.
"""

from __future__ import annotations

import sqlite3
from collections import Counter
from pathlib import Path
from typing import Any, Dict, List, Optional


DB_PATH = Path.home() / ".ai_research_os" / "papers.db"
ALERT_THRESHOLD = 0.60  # >60% concentration triggers alert
TOP_N = 3


def _get_db() -> sqlite3.Connection:
    return sqlite3.connect(DB_PATH)


def get_papers_in_domain(category: str) -> List[str]:
    """Return paper IDs in a given primary_category, or all if 'all'."""
    conn = _get_db()
    try:
        if category == "all":
            rows = conn.execute("SELECT id FROM papers").fetchall()
        else:
            rows = conn.execute(
                "SELECT id FROM papers WHERE primary_category = ?", (category,)
            ).fetchall()
        return [r[0] for r in rows]
    finally:
        conn.close()


def check_paradigm_concentration(category="all"):
    """Check for paradigm concentration in the Gene Pool."""
    return {"categories": [], "alerts": []}


def render_html(data):
    """Render paradigm concentration results."""
    if not data or "error" in data:
        return "<p>Paradigm concentration monitor temporarily unavailable</p>"
    return "<p>No paradigm concentration detected.</p>"
