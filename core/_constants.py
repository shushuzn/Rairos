"""Canonical constants — single source of truth.

All hardcoded strings that span multiple modules are centralized here
to avoid duplication and ensure consistency.
"""
from pathlib import Path

# ─── API endpoints ───────────────────────────────────────────────────────────
ARXIV_API = "https://export.arxiv.org/api/query?id_list={arxiv_id}"
CROSSREF_WORKS = "https://api.crossref.org/works/{doi}"
DOI_RESOLVER = "https://doi.org/"

# ─── Output filenames ──────────────────────────────────────────────────────
RADAR_FILE = "Radar.md"
TIMELINE_FILE = "Timeline.md"

# ─── Research tree directory names (canonical order) ───────────────────────
# NOTE: must stay in sync with completions/fish and completions/zsh
DEFAULT_RESEARCH_DIRS = [
    "00-Radar",
    "01-Foundations",
    "02-Models",
    "03-Training",
    "04-Scaling",
    "05-Alignment",
    "06-Agents",
    "07-Infrastructure",
    "08-Optimization",
    "09-Evaluation",
    "10-Applications",
    "11-Future-Directions",
]
