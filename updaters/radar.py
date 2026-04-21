"""Radar page management."""
import datetime as dt
import textwrap
from pathlib import Path
from typing import Dict, List, Optional

from core.basics import get_default_radar_dir, read_text, write_text

# In-memory accumulation for batch updates — keyed by (root, tag)
_pending: Dict[tuple, Dict[str, str]] = {}


def ensure_radar(root: Path) -> Path:
    p = root / get_default_radar_dir() / "Radar.md"
    if p.exists():
        return p

    md = """\
# Radar（长期跟踪页）

| 主题 | 热度 | 证据质量 | 成本变化 | 我的信心 | 最近更新 |
| -- | -- | ---- | ---- | ---- | ---- |
"""
    write_text(p, textwrap.dedent(md).strip() + "\n")
    return p


def parse_radar_table(md: str) -> tuple:
    lines = md.splitlines()
    start = None
    for i, ln in enumerate(lines):
        if ln.strip().startswith("| 主题 |"):
            start = i
            break
    if start is None:
        return md.rstrip() + "\n", []

    header = "\n".join(lines[:start]).rstrip() + "\n"
    rows: List[Dict[str, str]] = []
    for ln in lines[start + 2:]:
        if not ln.strip().startswith("|"):
            continue
        cols = [c.strip() for c in ln.strip().strip("|").split("|")]
        if len(cols) < 6:
            continue
        rows.append({
            "主题": cols[0],
            "热度": cols[1],
            "证据质量": cols[2],
            "成本变化": cols[3],
            "我的信心": cols[4],
            "最近更新": cols[5],
        })
    return header, rows


def render_radar(header: str, rows: List[Dict[str, str]]) -> str:
    out = [
        header.rstrip(),
        "",
        "| 主题 | 热度 | 证据质量 | 成本变化 | 我的信心 | 最近更新 |",
        "| -- | -- | ---- | ---- | ---- | ---- |",
    ]
    for r in rows:
        out.append(f"| {r['主题']} | {r['热度']} | {r['证据质量']} | {r['成本变化']} | {r['我的信心']} | {r['最近更新']} |")
    return "\n".join(out).strip() + "\n"


def update_radar(
    root: Path,
    tags: list,
    note_date: str,
    flush: bool = True,
) -> Path:
    """Update radar with incremented heat for tags.

    Args:
        root: Research OS root
        tags: List of tag names to bump
        note_date: Date string for '最近更新' field
        flush: If True (default), write immediately. If False, accumulate
                in memory and caller must call flush_radar() afterwards.
    """
    p = ensure_radar(root)
    key = (root.resolve(), None)  # None sentinel for single-update path

    if flush:
        _pending.clear()  # clear any stale batch state
        md = read_text(p)
        header, rows = parse_radar_table(md)
        row_map = {r["主题"]: r for r in rows}
        for t in tags:
            if t not in row_map:
                row_map[t] = {"主题": t, "热度": "1", "证据质量": "", "成本变化": "", "我的信心": "", "最近更新": note_date}
            else:
                try:
                    row_map[t]["热度"] = str(int(row_map[t]["热度"] or "0") + 1)
                except Exception:
                    row_map[t]["热度"] = "1"
                row_map[t]["最近更新"] = note_date
        rows2 = list(row_map.values())
        rows2.sort(key=lambda r: (-_heat(r), r["主题"].lower()))
        write_text(p, render_radar(header, rows2))
        return p

    # Deferred mode: accumulate in memory
    if key not in _pending:
        md = read_text(p)
        header, rows = parse_radar_table(md)
        _pending[key] = {"_header": header, "_rows": {r["主题"]: r for r in rows}}
    state = _pending[key]
    for t in tags:
        if t not in state["_rows"]:
            state["_rows"][t] = {"主题": t, "热度": "1", "证据质量": "", "成本变化": "", "我的信心": "", "最近更新": note_date}
        else:
            try:
                state["_rows"][t]["热度"] = str(int(state["_rows"][t]["热度"] or "0") + 1)
            except Exception:
                state["_rows"][t]["热度"] = "1"
            state["_rows"][t]["最近更新"] = note_date
    return p


def _heat(r: Dict[str, str]) -> int:
    try:
        return int(r["热度"])
    except Exception:
        return 0


def flush_radar(root: Path) -> None:
    """Write all accumulated radar updates for a root to disk."""
    key = (root.resolve(), None)
    if key not in _pending:
        return
    state = _pending.pop(key)
    p = ensure_radar(root)
    rows2 = list(state["_rows"].values())
    rows2.sort(key=lambda r: (-_heat(r), r["主题"].lower()))
    write_text(p, render_radar(state["_header"], rows2))
