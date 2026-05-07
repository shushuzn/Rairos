"""Code-Paper Traceability — parse provenance comments and build bidirectional traces."""

from __future__ import annotations

import re
from dataclasses import dataclass
from typing import Any, Dict, List, Tuple

# Matches: "# source: @eq[0] — Attention from §3.2"
SOURCE_COMMENT_RE = re.compile(
    r"#\s*source:\s*((?:@(?:\w+)\[\d+\]\s*(?:,\s*)?)+)",
    re.MULTILINE,
)
TAG_RE = re.compile(r"@(\w+)\[(\d+)\]")


@dataclass
class ParsedSourceComment:
    """A parsed `# source:` comment from generated code."""

    line_number: int
    tags: List[Tuple[str, int]]  # e.g. [("eq", 0), ("algo", 1)]
    description: str


def parse_source_comments(code: str) -> List[ParsedSourceComment]:
    """Extract all `# source:` comments from generated code."""
    results = []
    for lineno, line in enumerate(code.splitlines(), start=1):
        m = SOURCE_COMMENT_RE.search(line)
        if not m:
            continue
        tags = [(tag, int(idx)) for tag, idx in TAG_RE.findall(m.group(1))]
        desc = ""
        dash_pos = line.find("—")
        if dash_pos != -1:
            desc = line[dash_pos + 1 :].strip()
        results.append(
            ParsedSourceComment(line_number=lineno, tags=tags, description=desc)
        )
    return results


def build_paper_section_refs(
    paper_content: Any,
    parsed_comments: List[ParsedSourceComment],
) -> List[Dict[str, Any]]:
    """Build paper_section_refs list for CapsuleGene archetype.

    Resolves tag indices to actual paper text using paper_content sources.
    Returns a list of dicts suitable for archetype["paper_section_refs"].
    """
    # Resolve tag to paper text — use *_sources if available, else fall back to flat lists
    equation_sources = getattr(paper_content, "equation_sources", []) or []
    claim_sources = getattr(paper_content, "claim_sources", []) or []
    algorithm_sources = getattr(paper_content, "algorithm_sources", []) or []

    def resolve(tag_type: str, idx: int) -> str:
        if tag_type == "eq":
            for s in equation_sources:
                if s.index == idx:
                    return s.equation[:80]
        elif tag_type == "claim":
            for s in claim_sources:
                if s.index == idx:
                    return s.claim[:80]
        elif tag_type == "algo":
            for s in algorithm_sources:
                if s.index == idx:
                    return s.description[:80]
        return f"[unknown {tag_type}[{idx}]]"

    refs = []
    for comment in parsed_comments:
        for tag_type, idx in comment.tags:
            refs.append(
                {
                    "type": tag_type,
                    "source_ref": f"@{tag_type}[{idx}]",
                    "paper_text": resolve(tag_type, idx),
                    "code_range": (comment.line_number, comment.line_number),
                    "confidence": 1.0,  # explicit LLM annotation = high confidence
                }
            )
    return refs


def code_to_paper_trace(code_str: str, paper_content: Any) -> Dict[str, Any]:
    """Bidirectional trace between generated code and paper sources.

    Returns:
        dict with keys:
          - "forward":  [{source_ref, code_ranges, paper_text, location}] per source
          - "untagged_ranges": [(start, end), ...] code lines with no provenance
          - "unreferenced_sources": [(type, idx, text), ...] paper items not in code
          - "total_tagged_lines": int
          - "total_code_lines": int
    """
    comments = parse_source_comments(code_str)
    lines = code_str.splitlines()

    # Map (tag_type, idx) -> list of line numbers
    tag_to_lines: Dict[Tuple[str, int], List[int]] = {}
    for c in comments:
        for tag_type, idx in c.tags:
            tag_to_lines.setdefault((tag_type, idx), []).append(c.line_number)

    # Coalesce consecutive line numbers into ranges
    def coalesce(sorted_lines: List[int]) -> List[Tuple[int, int]]:
        if not sorted_lines:
            return []
        ranges = []
        start = prev = sorted_lines[0]
        for cur in sorted_lines[1:]:
            if cur == prev + 1:
                prev = cur
            else:
                ranges.append((start, prev))
                start = prev = cur
        ranges.append((start, prev))
        return ranges

    equation_sources = getattr(paper_content, "equation_sources", []) or []
    claim_sources = getattr(paper_content, "claim_sources", []) or []
    algorithm_sources = getattr(paper_content, "algorithm_sources", []) or []

    # Build forward map (paper source -> code range)
    forward = []
    for (tag_type, idx), line_nums in tag_to_lines.items():
        ranges = coalesce(sorted(set(line_nums)))
        paper_text = ""
        location_info = ""
        source_ref = f"@{tag_type}[{idx}]"

        sources = []
        if tag_type == "eq":
            sources = equation_sources
        elif tag_type == "claim":
            sources = claim_sources
        elif tag_type == "algo":
            sources = algorithm_sources

        for s in sources:
            if s.index == idx:
                paper_text = (
                    (s.equation if tag_type == "eq" else s.claim if tag_type == "claim" else s.description)[:80]
                )
                location_info = f"§{s.location.section} p{s.location.page}"
                break

        forward.append(
            {
                "source_ref": source_ref,
                "code_ranges": ranges,
                "paper_text": paper_text,
                "location": location_info,
            }
        )

    # Find untagged line ranges
    all_tagged = set()
    for ln in tag_to_lines.values():
        all_tagged.update(ln)

    untagged = []
    if all_tagged:
        sorted_tagged = sorted(all_tagged)
        if sorted_tagged[0] > 1:
            untagged.append((1, sorted_tagged[0] - 1))
        for i in range(len(sorted_tagged) - 1):
            gap_s, gap_e = sorted_tagged[i] + 1, sorted_tagged[i + 1] - 1
            if gap_s <= gap_e:
                untagged.append((gap_s, gap_e))
        if sorted_tagged[-1] < len(lines):
            untagged.append((sorted_tagged[-1] + 1, len(lines)))
    elif len(lines) > 0:
        untagged.append((1, len(lines)))

    # Find unreferenced paper sources
    all_sources: List[Tuple[str, int, str]] = []
    for s in equation_sources:
        all_sources.append(("eq", s.index, s.equation[:60]))
    for s in claim_sources:
        all_sources.append(("claim", s.index, s.claim[:60]))
    for s in algorithm_sources:
        all_sources.append(("algo", s.index, s.description[:60]))

    referenced = {(t, i) for t, i, _ in all_sources if (t, i) in tag_to_lines}
    unreferenced = [(t, i, txt) for t, i, txt in all_sources if (t, i) not in referenced]

    return {
        "forward": forward,
        "untagged_ranges": untagged,
        "unreferenced_sources": unreferenced,
        "total_tagged_lines": len(all_tagged),
        "total_code_lines": len(lines),
    }
