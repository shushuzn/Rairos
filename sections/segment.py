"""PDF section segmentation."""
import re
from typing import List, Tuple


def looks_like_heading(line: str) -> bool:
    s = line.strip()
    if len(s) < 3 or len(s) > 120:
        return False

    if re.match(r"^(\d+(\.\d+)*)\.?\s+[A-Za-z].{2,}$", s):
        return True
    if re.match(r"^(I|II|III|IV|V|VI|VII|VIII|IX|X)\.?\s+[A-Za-z].{2,}$", s):
        return True

    keywords = [
        "abstract", "introduction", "background", "related work", "method",
        "approach", "preliminaries", "experiments", "evaluation", "results",
        "discussion", "limitations", "conclusion", "future work", "references",
        "appendix", "acknowledgments", "ablation"
    ]
    low = s.lower()
    if any(low == k for k in keywords):
        return True
    if any(low.startswith(k + " ") for k in keywords):
        return True

    if s.isupper() and 4 <= len(s) <= 40:
        return True

    return False


def segment_into_sections(text: str, max_sections: int = 18) -> List[Tuple[str, str]]:
    lines = text.splitlines()
    sections: List[Tuple[str, List[str]]] = []
    cur_title = "BODY"
    cur_buf: List[str] = []

    for line in lines:
        stripped = line.strip()
        md_heading_match = re.match(r"^(#{1,6})\s+(.+)$", stripped)
        if md_heading_match:
            if cur_buf:
                sections.append((cur_title, cur_buf))
            cur_title = md_heading_match.group(2).strip()
            cur_buf = []
        elif looks_like_heading(line):
            if cur_buf:
                sections.append((cur_title, cur_buf))
            cur_title = line.strip()
            cur_buf = []
        else:
            cur_buf.append(line)

    if cur_buf:
        sections.append((cur_title, cur_buf))

    merged: List[Tuple[str, str]] = []
    for title, buf in sections:
        content = "\n".join(buf).strip()
        if not content:
            continue
        if False and merged and len(content) < 400 and title != "BODY":  # disabled
            pt, pc = merged[-1]
            merged[-1] = (pt, (pc + "\n\n" + title + "\n" + content).strip())
        else:
            merged.append((title, content))

    if len(merged) > max_sections:
        merged = merged[:max_sections] + [("TRUNCATED", "...(text truncated)...")]

    return merged


def format_section_snippets(sections: List[Tuple[str, str]], max_chars_each: int = 1800) -> str:
    out = []
    for title, content in sections:
        snippet = content.strip()
        if len(snippet) > max_chars_each:
            snippet = snippet[:max_chars_each].rstrip() + "\n…"
        out.append(f"### {title}\n\n" + "\n".join(["> " + ln for ln in snippet.splitlines() if ln.strip()]))
        out.append("")
    return "\n".join(out).strip()
