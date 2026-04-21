"""C-Note creation and link management."""
import re
from pathlib import Path

import ai_research_os as airo
from core.basics import get_default_concept_dir, read_text, write_text
from llm.generate import ai_generate_cnote_draft
from notes.pnotes import pnotes_by_tag, read_pnote_metadata, wikilink_for_pnote
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


def _section_is_empty(md: str, section: str) -> bool:
    """Check if a section has meaningful content (not just placeholder text)."""
    # Match the section heading and capture content until next ## heading or end
    pattern = rf"(?:^|\n)(##\s+{re.escape(section)}\s*\n)(.*?)(?=\n##\s+|\Z)"
    m = re.search(pattern, md, flags=re.DOTALL)
    if not m:
        return True
    content = m.group(2).strip()
    # Empty, or only placeholder dashes/comments
    if not content or re.match(r"^[-–—\s]+$", content):
        return True
    # Only the template placeholder text (short, no sentence-ending punctuation)
    if len(content) < 20 and not re.search(r"[.。?！]", content):
        return True
    return False


def _fill_cnote_section(md: str, section: str, new_content: str) -> str:
    """Replace a section's placeholder content with new_content, preserving the heading."""
    pattern = rf"((?:^|\n)(##\s+{re.escape(section)}\s*\n))(.*?)(?=\n##\s+|\Z)"
    m = re.search(pattern, md, flags=re.DOTALL)
    if not m:
        # Section doesn't exist; append it
        return md.rstrip() + f"\n\n## {section}\n\n{new_content.strip()}\n"
    heading = m.group(1)  # includes trailing newlines
    return md[:m.start()] + heading + new_content.strip() + md[m.end():]


def _parse_cnote_sections(draft: str) -> dict:
    """Parse a C-note draft into section_name -> content dict."""
    sections = {}
    current_section = None
    current_content = []

    for line in draft.split("\n"):
        m = re.match(r"^(##\s+.+)$", line)
        if m:
            if current_section:
                sections[current_section] = "\n".join(current_content).strip()
            current_section = m.group(1).replace("##", "").strip()
            current_content = []
        else:
            if current_section is not None:
                current_content.append(line)

    if current_section:
        sections[current_section] = "\n".join(current_content).strip()
    return sections


def auto_fill_cnotes_with_ai(
    root: Path,
    api_key: str,
    base_url: str,
    model: str,
    min_papers: int = 1,
    call_llm=None,
) -> list:
    """
    Scan P-notes by tag, generate AI C-note drafts, and fill empty sections.

    Args:
        root: Research OS root (contains 01-Foundations/, 02-Papers/, etc.)
        api_key: LLM API key
        base_url: OpenAI-compatible base URL
        model: Model name
        min_papers: Minimum P-notes needed to trigger AI draft (default 1)
        call_llm: Callable to use for LLM generation (default: call_llm_chat_completions).
                  Allows dependency injection for testing.

    Returns:
        List of (concept, status) tuples: status is 'filled' | 'skipped' | 'no-papers'
    """
    if call_llm is None:
        call_llm = airo.call_llm_chat_completions

    results = []
    tag_map = pnotes_by_tag(root)
    concept_dir = root / get_default_concept_dir()

    for concept, pnote_entries in tag_map.items():
        # pnote_entries: List[Tuple[str, Path]] sorted by date desc
        pnote_paths = [p for _, p in pnote_entries]

        if len(pnote_paths) < min_papers:
            results.append((concept, "skipped"))
            continue

        cnote_path = concept_dir / f"C - {concept}.md"

        # Read existing C-note content
        if cnote_path.exists():
            md = read_text(cnote_path)
        else:
            md = render_cnote(concept)
            write_text(cnote_path, md)

        # Check if any section is empty (meaning we should generate)
        core_empty = _section_is_empty(md, "核心定义")
        bg_empty = _section_is_empty(md, "产生背景")
        tech_empty = _section_is_empty(md, "技术本质")

        if not (core_empty or bg_empty or tech_empty):
            # All major sections have content; skip to avoid overwriting manual work
            results.append((concept, "skipped"))
            continue

        # Collect P-note metadata
        pnotes_meta = [read_pnote_metadata(p) for p in pnote_paths]

        try:
            draft = ai_generate_cnote_draft(
                concept=concept,
                pnotes=pnotes_meta,
                api_key=api_key,
                base_url=base_url,
                model=model,
                call_llm=call_llm,
            )
        except Exception as e:
            print(f"  [WARN] C-note AI draft failed for {concept}: {e}")
            results.append((concept, "failed"))
            continue

        # Parse generated sections and fill only empty ones
        parsed = _parse_cnote_sections(draft)
        for section, content in parsed.items():
            if content.strip():
                md = _fill_cnote_section(md, section, content)

        write_text(cnote_path, md)
        results.append((concept, "filled"))

    return results
