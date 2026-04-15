"""C-Note creation and link management."""
import re
from pathlib import Path

from core.basics import read_text, write_text
from notes.pnotes import wikilink_for_pnote
from renderers.cnote import render_cnote


def ensure_cnote(concept_dir: Path, concept: str) -> Path:
    path = concept_dir / f"C - {concept}.md"
    if not path.exists():
        write_text(path, render_cnote(concept))
    return path


def upsert_link_under_heading(md: str, heading: str, link_line: str) -> str:
    # Strip leading ##/ heading prefix so we match against just the text
    clean_heading = re.sub(r"^#+\s+", "", heading).strip()
    # Look for the heading (## prefix already in pattern)
    pattern = rf"(^##\s+{re.escape(clean_heading)}(?:\s*|\s+.*)$)"
    m = re.search(pattern, md, flags=re.M)
    if not m:
        return md.rstrip() + f"\n\n## {clean_heading}\n\n{link_line}\n"

    match_line = m.group(0).split('\n')[0]  # just the heading line without trailing content
    start = m.start() + len(match_line)  # end of heading line in the full string
    after = md[start:]
    m2 = re.match(r"(\s*\n)*", after)  # skip blank lines
    insert_pos = start + (m2.end() if m2 else 0)

    # Find section end: next ## heading or end of file
    rest = after[m2.end() if m2 else 0:]
    m3 = re.search(r"\n##\s+", rest)
    section_end = insert_pos + m3.start() if m3 else len(md)

    # Extract current section content (skip the leading \n from after the heading)
    section_content = md[insert_pos:section_end].lstrip("\n")

    # Remove existing wikilink lines under this heading.
    # Only removes lines that are wikilinks: "- [[...]]" or "- [[...]] text"
    # Preserves any manual bullet lines the user may have added.
    cleaned = re.sub(r"^-\s*\[\[[^\]]+\]\](?:[^\n]*)?\n?", "", section_content, flags=re.M).strip("\n")
    section_content = cleaned.strip("\n")
    new_section = link_line.rstrip() + "\n" + section_content if section_content.strip() else link_line.rstrip()

    return md[:insert_pos] + new_section + md[section_end:]


def update_cnote_links(cnote_path: Path, pnote_path: Path) -> None:
    md = read_text(cnote_path)
    md2 = upsert_link_under_heading(md, "关联笔记", wikilink_for_pnote(pnote_path))
    write_text(cnote_path, md2)
