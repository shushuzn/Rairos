"""Shared state for Rairos web app — templates, DB, helpers."""

from __future__ import annotations

import json
import sys
from datetime import datetime
from pathlib import Path
from typing import Any, Dict, List

from fastapi.templating import Jinja2Templates

# Project root
PROJECT_ROOT = Path(__file__).parent.parent
sys.path.insert(0, str(PROJECT_ROOT))

WEB_DIR = Path(__file__).parent
templates = Jinja2Templates(directory=str(WEB_DIR / "templates"))

# Jinja filters
def _jinja_truncate(value, length=80):
    s = str(value)
    return s[:length] + "…" if len(s) > length else s

def _jinja_timestamp(value):
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
