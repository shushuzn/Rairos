"""Research log — per-paper notes stored in ~/.ai_research_os/gene_pool/research_log.jsonl."""

from __future__ import annotations

import json
from datetime import datetime
from pathlib import Path
from typing import Any, Dict, List, Optional


LOG_PATH = Path.home() / ".ai_research_os" / "gene_pool" / "research_log.jsonl"


def add_note(paper_id: str, note: str, tags: Optional[List[str]] = None) -> bool:
    try:
        LOG_PATH.parent.mkdir(parents=True, exist_ok=True)
        entry = {
            "timestamp": datetime.now().isoformat(),
            "paper_id": paper_id,
            "note": note,
            "tags": tags or [],
        }
        with open(LOG_PATH, "a", encoding="utf-8") as f:
            f.write(json.dumps(entry, ensure_ascii=False) + "\n")
        return True
    except Exception:
        return False


def get_notes(paper_id: Optional[str] = None, limit: int = 20) -> List[Dict[str, Any]]:
    try:
        if not LOG_PATH.exists():
            return []
        notes = []
        with open(LOG_PATH, encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                try:
                    entry = json.loads(line)
                except Exception:
                    continue
                if paper_id and entry.get("paper_id") != paper_id:
                    continue
                notes.append(entry)
        notes.sort(key=lambda x: x.get("timestamp", ""), reverse=True)
        return notes[:limit]
    except Exception:
        return []


def render_log(paper_id: Optional[str] = None) -> str:
    """Return HTML timeline of research notes."""
    notes = get_notes(paper_id=paper_id, limit=50)
    if not notes:
        empty_msg = "No research notes yet."
        if paper_id:
            empty_msg = f"No notes for paper {paper_id} yet."
        return f"""
        <div style="text-align:center;padding:60px 20px;color:#888;font-family:var(--font-display);">
          <div style="font-size:48px;margin-bottom:12px;">📝</div>
          <div style="font-size:15px;font-weight:600;margin-bottom:6px;">{empty_msg}</div>
          <div style="font-size:13px;">Add notes from a paper detail page.</div>
        </div>"""

    paper_titles: Dict[str, str] = {}
    if paper_id is None:
        seen_ids = list(dict.fromkeys(n.get("paper_id", "") for n in notes if n.get("paper_id")))
        if seen_ids:
            try:
                from db.database import Database
                db = Database()
                db.init()
                for pid in seen_ids:
                    p = db.get_paper(pid)
                    if p:
                        paper_titles[pid] = p.title
            except Exception:
                pass

    cards = ""
    for n in notes:
        ts = n.get("timestamp", "")
        date_str = ts[:16].replace("T", " ") if ts else "—"
        pid = n.get("paper_id", "")
        title = paper_titles.get(pid, pid[:20] if pid else "—")
        note_text = n.get("note", "")
        tags = n.get("tags", [])

        tags_html = ""
        if tags:
            tags_html = "".join(
                f"<span style='display:inline-block;background:#e8f0fe;color:#1a73e8;padding:2px 8px;border-radius:12px;font-size:11px;margin:2px;'>{t}</span>"
                for t in tags
            )

        cards += f"""
        <div style="border:1px solid #e0e8f0;border-radius:8px;padding:16px;margin-bottom:12px;background:#fff;box-shadow:0 2px 4px rgba(0,0,0,0.05);">
          <div style="display:flex;justify-content:space-between;margin-bottom:6px;">
            <span style="font-size:13px;font-weight:600;color:#1a1a2e;">{title}</span>
            <span style="font-size:11px;color:#888;flex-shrink:0;margin-left:12px;">{date_str}</span>
          </div>
          <div style="font-size:13px;color:#444;line-height:1.5;margin-bottom:8px;white-space:pre-wrap;">{note_text}</div>
          {tags_html}
        </div>"""

    return f"""<div style="max-width:700px;margin:0 auto;">{cards}</div>"""
