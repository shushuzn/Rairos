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


# Section priority: higher = more important for LLM context
_SECTION_PRIORITY = {
    "abstract": 10,
    "introduction": 9,
    "method": 8,
    "methodology": 8,
    "approach": 8,
    "model": 7,
    "architecture": 7,
    "algorithm": 7,
    "experiments": 6,
    "evaluation": 6,
    "results": 6,
    "analysis": 5,
    "discussion": 4,
    "limitations": 4,
    "conclusion": 4,
    "related work": 3,
    "background": 3,
    "preliminaries": 3,
    "appendix": 1,
    "acknowledgments": 1,
    "references": 0,
    "future work": 2,
    "ablation": 5,
    "body": 1,
    "truncated": 0,
}


def _section_priority(title: str) -> int:
    t = title.lower()
    for key, prio in _SECTION_PRIORITY.items():
        if key in t:
            return prio
    return 2  # default medium-low priority for unknown sections


def format_section_snippets(
    sections: List[Tuple[str, str]],
    *,
    max_chars_total: int = 6000,
    min_chars_per_high_prio: int = 600,
) -> str:
    """
    Smart section selection: prioritize abstract/method/experiments, respect total token budget.

    Strategy:
    - Sort sections by priority descending
    - Fill budget top-down, giving high-priority sections at least min_chars_each
    - Remaining budget for lower-priority sections
    - Truncate last allowed section rather than cutting mid-sentence where possible
    """
    if not sections:
        return ""

    # Sort by priority (descending), then by position in paper (stable sort)
    indexed = [(i, title, content) for i, (title, content) in enumerate(sections)]
    indexed.sort(key=lambda x: (_section_priority(x[1]), -x[0]), reverse=True)

    out: List[Tuple[str, str, int]] = []  # (title, content, priority)
    budget = max_chars_total

    for idx, title, content in indexed:
        if budget <= 0:
            break
        raw = content.strip()
        if not raw:
            continue

        priority = _section_priority(title)

        if priority >= 8 and len(raw) >= min_chars_per_high_prio:
            # High-priority section: give it at least min_chars_each
            take = min(len(raw), max(min_chars_per_high_prio, budget))
        elif priority >= 5:
            # Medium priority: proportional share
            take = min(len(raw), max(300, budget // max(1, len([x for x in indexed if _section_priority(x[1]) >= 5]))))
        else:
            # Low priority: whatever is left
            take = min(len(raw), budget)

        if priority >= 5 and take < min(200, len(raw)) and budget >= 200:
            # Don't take a section if we can't give it meaningful content
            # EXCEPT if it's the last bite of budget
            take = min(len(raw), max(take, 200))

        snippet = raw[:take]
        # Truncate at sentence boundary if possible
        if take < len(raw):
            for punct in [". ", ".\n", "。", "！", "？"]:
                last_punct = snippet.rfind(punct)
                if last_punct > take * 0.6:
                    snippet = snippet[:last_punct + len(punct)]
                    break
            else:
                snippet = snippet.rstrip()

        out.append((title, snippet, priority))
        budget -= len(snippet)

    # Restore original order for readability
    out.sort(key=lambda x: next((i for i, t in enumerate(sections) if t[0] == x[0]), 99))

    result_parts = []
    for title, snippet, _ in out:
        lines = ["> " + ln for ln in snippet.splitlines() if ln.strip()]
        result_parts.append(f"### {title}\n\n" + "\n".join(lines))

    return ("\n\n".join(result_parts)).strip()
