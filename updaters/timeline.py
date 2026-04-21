"""Timeline page management."""
import textwrap
from pathlib import Path

from core.basics import get_default_radar_dir, read_text, write_text
from notes.pnotes import wikilink_for_pnote


def ensure_timeline(root: Path) -> Path:
    p = root / get_default_radar_dir() / "Timeline.md"
    if p.exists():
        return p
    md = """\
# Timeline（技术演进）

按年份记录关键论文与技术拐点。

"""
    write_text(p, textwrap.dedent(md).strip() + "\n")
    return p


def update_timeline(root: Path, year: str, pnote_path: Path, title: str) -> Path:
    p = ensure_timeline(root)
    md = read_text(p)

    section = f"## {year}"
    bullet = f"- {wikilink_for_pnote(pnote_path)} — {title}"

    if section not in md:
        md = md.rstrip() + f"\n\n{section}\n\n{bullet}\n"
        write_text(p, md.strip() + "\n")
        return p

    if bullet in md:
        return p

    import re
    pattern = rf"^##\s+{re.escape(year)}\s*$"
    m = re.search(pattern, md, flags=re.M)
    if not m:
        return p

    start = m.end()
    rest = md[start:]
    m2 = re.search(r"^\s*##\s+", rest, flags=re.M)
    end = start + (m2.start() if m2 else len(rest))

    block = md[start:end].rstrip() + "\n" + bullet + "\n"
    md2 = md[:start] + block + md[end:]
    write_text(p, md2.strip() + "\n")
    return p
