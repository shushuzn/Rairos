"""arXiv Watch Alert Channels — multiple feed configurations with different matching criteria.

Channels:
  - general: broad ML/AI coverage
  - climate: climate + AI intersection
  - ai_safety: alignment, robustness, interpretability
  - regulation: AI policy, governance, law
"""

from __future__ import annotations

import json
import re
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Dict, List, Optional

CHANNELS_FILE = Path.home() / ".ai_research_os" / "arxiv_channels.json"

DEFAULT_CHANNELS = {
    "general": {
        "name": "General AI/ML",
        "categories": ["cs.AI", "cs.LG", "cs.CL", "cs.CV", "cs.NE"],
        "keywords": [],
        "priority": 1,
        "enabled": True,
    },
    "climate": {
        "name": "Climate AI",
        "categories": ["cs.AI", "cs.LG", "cs.ET", "envir.ArXiv"],
        "keywords": [
            "climate",
            "carbon",
            "emissions",
            "renewable",
            "energy",
            "sustainability",
            "green AI",
        ],
        "priority": 3,
        "enabled": True,
    },
    "ai_safety": {
        "name": "AI Safety",
        "categories": ["cs.AI", "cs.LG"],
        "keywords": [
            "safety",
            "alignment",
            "robustness",
            "interpretability",
            "fairness",
            "trustworthy",
            "hazard",
            "risk",
        ],
        "priority": 3,
        "enabled": True,
    },
    "regulation": {
        "name": "AI Regulation",
        "categories": ["cs.AI", "cs.CY", "cs.SI"],
        "keywords": [
            "regulation",
            "policy",
            "governance",
            "law",
            "GDPR",
            "compliance",
            "legal",
            "legislation",
        ],
        "priority": 2,
        "enabled": True,
    },
}


def _load_channels() -> Dict[str, Any]:
    if not CHANNELS_FILE.exists():
        CHANNELS_FILE.parent.mkdir(parents=True, exist_ok=True)
        CHANNELS_FILE.write_text(
            json.dumps(DEFAULT_CHANNELS, indent=2, ensure_ascii=False), encoding="utf-8"
        )
        return DEFAULT_CHANNELS
    return json.loads(CHANNELS_FILE.read_text(encoding="utf-8"))  # type: ignore[no-any-return]


def _save_channels(channels: Dict[str, Any]) -> None:
    CHANNELS_FILE.parent.mkdir(parents=True, exist_ok=True)
    CHANNELS_FILE.write_text(json.dumps(channels, indent=2, ensure_ascii=False), encoding="utf-8")


def match_paper_to_channels(paper: Dict[str, Any]) -> List[str]:
    """Return list of channel IDs a paper matches."""
    channels = _load_channels()
    cats = set(paper.get("categories", []) or [])
    abstract = (paper.get("abstract", "") + " " + paper.get("title", "")).lower()
    _title = paper.get("title", "").lower()
    matched: List[str] = []

    for cid, cfg in channels.items():
        if not cfg.get("enabled", True):
            continue
        # Category match
        if any(c in cats for c in cfg.get("categories", [])):
            matched.append(cid)
            continue
        # Keyword match
        keywords = cfg.get("keywords", [])
        if keywords and any(kw.lower() in abstract for kw in keywords):
            matched.append(cid)

    return matched


@dataclass
class ChannelConfig:
    id: str
    name: str
    categories: List[str]
    keywords: List[str]
    priority: int
    enabled: bool


def get_channels() -> List[ChannelConfig]:
    channels = _load_channels()
    return [
        ChannelConfig(
            id=cid,
            name=cfg["name"],
            categories=cfg.get("categories", []),
            keywords=cfg.get("keywords", []),
            priority=cfg.get("priority", 1),
            enabled=cfg.get("enabled", True),
        )
        for cid, cfg in channels.items()
    ]


def update_channel(cid: str, updates: Dict[str, Any]) -> bool:
    channels = _load_channels()
    if cid not in channels:
        return False
    channels[cid].update(updates)
    _save_channels(channels)
    return True


def render_channels_html(check_results: Optional[Dict[str, List[Dict[str, Any]]]] = None) -> str:
    channels = get_channels()
    check_results = check_results or {}

    lines = ['<div class="channels-panel">']
    lines.append("<h3>📡 arXiv Watch Alert Channels</h3>")
    lines.append(
        "<p style='font-size:13px;color:#A89E8C;margin-bottom:16px'>"
        "Configure multiple feed channels with different matching criteria. "
        "Higher priority = shown first in alerts.</p>"
    )

    # Run Check button
    lines.append("""
    <div style="margin-bottom: 20px;">
      <button id="run-check-btn" onclick="runCheck()" style="
        background: #1a73e8; color: #fff; border: none; border-radius: 6px;
        padding: 10px 20px; font-size: 14px; cursor: pointer; font-family: Georgia, serif;">
        🔍 Run Check Now
      </button>
      <span id="check-status" style="font-size:13px;color:#888;margin-left:12px;display:none;"></span>
    </div>
    <div id="check-results"></div>
    """)

    for ch in channels:
        color = {3: "#C4706A", 2: "#D4A055", 1: "#6B8FB5"}.get(ch.priority, "#A89E8C")
        status = "✅ Enabled" if ch.enabled else "❌ Disabled"
        kw_str = ", ".join(f"<code>{k}</code>" for k in ch.keywords[:6])
        cat_str = ", ".join(ch.categories[:4])
        # Results from last check for this channel
        channel_results = check_results.get(ch.id, [])
        result_rows = ""
        for rp in channel_results[:5]:
            result_rows += f"""
            <div style="display:flex;gap:8px;align-items:flex-start;padding:6px 0;border-bottom:1px solid #f0ebe5;">
              <span style="color:#4CAF50;font-size:12px;">●</span>
              <div style="flex:1;">
                <div style="font-size:12px;color:#2a2a2a;font-weight:600;">{rp.get("title", "")[:80]}</div>
                <div style="font-size:11px;color:#888;">{rp.get("published", "")} · score={rp.get("score", 0):.2f}</div>
              </div>
            </div>"""
        if not result_rows:
            result_rows = "<div style='font-size:12px;color:#bbb;padding:4px 0;'>No new papers in last check</div>"

        lines.append(f"""
<div style='border: 1px solid #e0dbd4; border-radius: 6px; padding: 14px; margin-bottom: 12px; border-left: 4px solid {color};'>
  <div style='display: flex; justify-content: space-between; align-items: center; margin-bottom: 6px;'>
    <div style='font-weight: 700; font-size: 14px; color: #2a2a2a'>{ch.name}</div>
    <div style='font-size: 11px; color: #A89E8C'>priority {ch.priority} · {status}</div>
  </div>
  <div style='font-size: 12px; color: #7a7570; margin-bottom: 4px'>Categories: {cat_str}</div>
  <div style='font-size: 12px; color: #A89E8C; margin-bottom: 8px'>Keywords: {kw_str or "(none)"}</div>
  <div style='margin-bottom: 10px; padding: 8px; background: #faf9f7; border-radius: 4px;'>
    <div style='font-size:11px;color:#888;margin-bottom:6px;'>Recent papers from this channel:</div>
    {result_rows}
  </div>
  <div style='display: flex; gap: 8px;'>
    <button onclick="toggleChannel('{ch.id}')" style="font-size: 11px; padding: 3px 10px; cursor: pointer; border-radius: 3px; border: 1px solid #ccc; background: transparent">
      Toggle
    </button>
  </div>
</div>""")

    lines.append("""
<script>
function toggleChannel(cid) {
    fetch('/arxiv-channels/toggle/' + cid, {method: 'POST'})
      .then(function(r) { return r.json(); })
      .then(function(d) { if (d.success) location.reload(); });
}
function runCheck() {
    var btn = document.getElementById('run-check-btn');
    var status = document.getElementById('check-status');
    btn.disabled = true;
    btn.textContent = '⏳ Checking...';
    status.style.display = 'inline';
    status.textContent = 'Querying arXiv...';
    fetch('/arxiv-channels/check', {method: 'POST'})
      .then(function(r) { return r.json(); })
      .then(function(d) {
          btn.disabled = false;
          btn.textContent = '🔍 Run Check Now';
          status.textContent = '';
          location.reload();
      })
      .catch(function(e) {
          btn.disabled = false;
          btn.textContent = '🔍 Run Check Now';
          status.textContent = 'Error: ' + e.message;
      });
}
</script>""")

    lines.append("<style>.channels-panel { font-family: Georgia, serif; }</style>")
    lines.append("</div>")
    return "\n".join(lines)
