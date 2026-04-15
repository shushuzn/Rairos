"""P-Note collection and tag-based queries."""
import datetime as dt
import re
from pathlib import Path
from typing import Dict, List, Tuple

from core.basics import read_text
from notes.frontmatter import parse_date_from_frontmatter, parse_frontmatter, parse_tags_from_frontmatter


def collect_pnotes(root: Path) -> List[Path]:
    return sorted([p for p in root.rglob("*.md") if p.is_file() and p.parent.name in ("02-Papers", "Papers", "papers")])


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
