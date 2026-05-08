"""Climate AI Monitor — arXiv watch tuned to climate+AI intersection; high priority in gap watch.

Tracks papers at the intersection of climate science and AI:
  - Energy efficiency of AI models (carbon footprint, FLOPs/watthours)
  - AI for climate modeling, weather, carbon capture
  - Climate risks of AI (data center water consumption, e-waste)
  - Green/sustainable ML techniques
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any, Dict, List, Optional

PAPERS_DIR = Path.home() / ".ai_research_os"
PAPERS_DB = PAPERS_DIR / "papers.json"
CLIMATE_WATCH_FILE = PAPERS_DIR / "climate_watch.json"

CLIMATE_KEYWORDS = [
    "climate change",
    "global warming",
    "carbon",
    "emissions",
    "greenhouse gas",
    "renewable energy",
    "solar",
    "wind power",
    "energy efficiency",
    "sustainable",
    "sustainability",
    "fossil fuel",
    "net-zero",
    "carbon neutral",
    "climate model",
    "weather prediction",
    "earth system",
    "carbon capture",
    "data center",
    "water consumption",
    "e-waste",
    "environmental impact",
    "green AI",
    "energy-aware",
    "low-carbon",
    "carbon footprint",
    "FLOPs per watt",
    "compute efficiency",
    "model efficiency",
]

CLIMATE_CATS = ["cs.AI", "cs.LG", "cs.ET", "physics.ao-ph", "atm.ph"]


def _load_papers() -> List[Dict[str, Any]]:
    if not PAPERS_DB.exists():
        return []
    data = json.loads(PAPERS_DB.read_text(encoding="utf-8"))  # type: ignore[no-any-return]
    return data.get("papers", [])  # type: ignore[no-any-return]


def _load_watch_list() -> Dict[str, Any]:
    if not CLIMATE_WATCH_FILE.exists():
        return {}
    return json.loads(CLIMATE_WATCH_FILE.read_text(encoding="utf-8"))  # type: ignore[no-any-return]


def _save_watch_list(data: Dict[str, Any]) -> None:
    CLIMATE_WATCH_FILE.parent.mkdir(parents=True, exist_ok=True)
    CLIMATE_WATCH_FILE.write_text(json.dumps(data, indent=2, ensure_ascii=False), encoding="utf-8")


def is_climate_related(paper: Dict[str, Any]) -> bool:
    text = (paper.get("title", "") + " " + paper.get("abstract", "")).lower()
    cats = set(paper.get("categories", []) or [])
    if any(c in CLIMATE_CATS for c in cats):
        return True
    return any(kw.lower() in text for kw in CLIMATE_KEYWORDS)


def get_climate_papers() -> List[Dict[str, Any]]:
    """Return all papers in library related to climate+AI."""
    papers = _load_papers()
    return [p for p in papers if is_climate_related(p)]


def get_watch_stats() -> Dict[str, Any]:
    """Return statistics about climate AI monitoring."""
    climate_papers = get_climate_papers()
    watch_list = _load_watch_list()
    watched_ids = set(watch_list.get("watched_ids", []))

    recent = [p for p in climate_papers if p.get("published", "") >= "2025-01-01"]

    return {
        "total_climate_papers": len(climate_papers),
        "watched_count": len([p for p in climate_papers if p.get("id", "") in watched_ids]),
        "recent_count": len(recent),
        "last_scan": watch_list.get("last_scan", "never"),
    }


def render_climate_monitor_html(stats: Optional[Dict[str, Any]] = None) -> str:
    if stats is None:
        stats = get_watch_stats()

    climate_papers = get_climate_papers()
    watch_list = _load_watch_list()
    watched_ids = set(watch_list.get("watched_ids", []))

    lines = ['<div class="climate-monitor">']
    lines.append("<h3>🌍 Climate AI Monitor</h3>")
    lines.append(
        "<p style='font-size:13px;color:#A89E8C;margin-bottom:14px'>"
        "Papers at the intersection of climate science and AI. "
        "High priority in gap watch matching.</p>"
    )

    # Stats row
    lines.append(
        "<div style='display:grid;grid-template-columns:1fr 1fr 1fr;gap:12px;margin-bottom:20px'>"
    )
    stats_cells = [
        ("Total Climate Papers", stats.get("total_climate_papers", 0), "#6B8FB5"),
        ("In Your Watch List", stats.get("watched_count", 0), "#6BBF8A"),
        ("Published 2025+", stats.get("recent_count", 0), "#D4A055"),
    ]
    for label, val, color in stats_cells:
        lines.append(
            f"<div style='background:#f8f4ef;border-radius:6px;padding:12px;text-align:center'>"
            f"<div style='font-size:22px;font-weight:700;color:{color}'>{val}</div>"
            f"<div style='font-size:11px;color:#A89E8C;margin-top:2px'>{label}</div></div>"
        )
    lines.append("</div>")

    # Paper list
    if not climate_papers:
        lines.append(
            "<p style='color:#A89E8C;font-size:13px'>No climate-related papers in your library yet.</p>"
        )
    else:
        for p in climate_papers[:15]:
            pid = p.get("id", "")
            is_watched = pid in watched_ids
            cats = ", ".join((p.get("categories", []) or [])[:2])
            title = p.get("title", "")[:70]
            published = p.get("published", "")[:4]

            kw_matches = [
                kw
                for kw in CLIMATE_KEYWORDS
                if kw.lower() in (p.get("title", "") + " " + p.get("abstract", "")).lower()
            ]
            kw_display = ", ".join(f"<code>{k}</code>" for k in kw_matches[:3])

            lines.append(f"""
<div style='border: 1px solid #e0dbd4; border-radius: 6px; padding: 12px; margin-bottom: 10px'>
  <div style='display: flex; justify-content: space-between; align-items: flex-start'>
    <div style='flex:1'>
      <div style='font-size: 12px; color: #6B8FB5; font-weight: 600'>{title}</div>
      <div style='font-size: 11px; color: #A89E8C; margin-top: 2px'>{cats} · {published}</div>
      <div style='font-size: 11px; color: #7a7570; margin-top: 4px'>{kw_display}</div>
    </div>
    <button onclick="toggleWatch('{pid}', this)"
      style='font-size: 10px; padding: 3px 8px; cursor: pointer; border-radius: 3px;
             border: 1px solid #ccc; background: transparent; color: {"#6BBF8A" if is_watched else "#A89E8C"}'>
      {"✓ Watched" if is_watched else "+ Watch"}
    </button>
  </div>
</div>""")

    lines.append("""
<script>
function toggleWatch(paperId, btn) {
    fetch('/climate-monitor/toggle-watch', {
        method: 'POST',
        headers: {'Content-Type': 'application/json'},
        body: JSON.stringify({paper_id: paperId})
    }).then(function(r) { return r.json(); })
      .then(function(d) {
          if (d.success) {
              var isWatched = btn.textContent.trim() === 'Watched';
              btn.textContent = isWatched ? '+ Watch' : '✓ Watched';
              btn.style.color = isWatched ? '#A89E8C' : '#6BBF8A';
          }
      });
}
</script>""")

    lines.append("<style>.climate-monitor { font-family: Georgia, serif; }</style>")
    lines.append("</div>")
    return "\n".join(lines)
