"""Radar page management."""
import textwrap
from pathlib import Path
from typing import Dict, List

from core.basics import get_default_radar_dir, read_text, write_text


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


def update_radar(root: Path, tags: list, note_date: str) -> Path:
    p = ensure_radar(root)
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

    def heat(r: Dict[str, str]) -> int:
        try:
            return int(r["热度"])
        except Exception:
            return 0

    rows2.sort(key=lambda r: (-heat(r), r["主题"].lower()))
    write_text(p, render_radar(header, rows2))
    return p
