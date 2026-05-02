"""P-Note collection and tag-based queries."""
import datetime as dt
import re
from pathlib import Path
from typing import Dict, List, Tuple

_RE_TITLE = re.compile(r"^#\s+(.+)$", re.MULTILINE)
_RE_SOURCE = re.compile(r"\*\*Source:\*\*\s+(\w+):\s+(\S+)")

from core.basics import read_text
from notes.frontmatter import parse_date_from_frontmatter, parse_frontmatter, parse_tags_from_frontmatter


def collect_pnotes(root: Path) -> List[Path]:
    # Scan all research tree subdirectories (00-Radar through 11-Future-Directions)
    # and legacy Papers/02-Papers for backwards compat
    all_dirs = set()
    for d in root.iterdir():
        if d.is_dir() and d.name[0].isdigit():
            all_dirs.add(d.name)
    all_dirs.update(["02-Papers", "Papers", "papers"])
    return sorted([p for p in root.rglob("*.md") if p.is_file() and p.parent.name in all_dirs])


def pnotes_by_tag(root: Path) -> Dict[str, List[Tuple[str, Path]]]:
    mapping: Dict[str, List[Tuple[str, Path]]] = {}
    for p in collect_pnotes(root):
        md = read_text(p)
        fm = parse_frontmatter(md)
        tags = parse_tags_from_frontmatter(fm)
        if not tags:
            continue
        d = parse_date_from_frontmatter(fm)
        if not d:
            d = dt.date.fromtimestamp(p.stat().st_mtime).isoformat()

        for t in tags:
            mapping.setdefault(t, []).append((d, p))

    for t in mapping:
        mapping[t].sort(key=lambda x: x[0], reverse=True)
    return mapping


def wikilink_for_pnote(pnote_path: Path) -> str:
    stem = Path(pnote_path).stem
    return f"[[{stem}]]"


def read_pnote_metadata(pnote_path: Path) -> dict:
    """Read a P-Note and return a dict suitable for C-Note AI draft generation."""
    md = read_text(pnote_path)
    fm = parse_frontmatter(md)
    tags = parse_tags_from_frontmatter(fm)
    date = parse_date_from_frontmatter(fm) or ""
    # Extract year from date (YYYY-MM-DD)
    year = date[:4] if len(date) >= 4 else ""

    # Extract title from markdown heading (# Title)
    title_match = _RE_TITLE.search(md)
    title = title_match.group(1).strip() if title_match else pnote_path.stem

    # Source/uid from content: "**Source:** ARXIV: XXXXX"
    src = fm.get("source", "arxiv").lower()
    uid = ""
    source_match = _RE_SOURCE.search(md)
    if source_match:
        src = source_match.group(1).lower()
        uid = source_match.group(2)

    return {
        "title": title,
        "authors": [],  # not stored in frontmatter; would need PDF metadata
        "year": year,
        "date": date,
        "source": src,
        "uid": uid,
        "abstract": "",  # P-note body may have abstract
        "tags": tags,
        "path": str(pnote_path),
    }
