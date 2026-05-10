"""
Research Survey Generator — generates markdown research survey from gap findings.

Called after DeepResearch completes and gaps are scored. Produces a structured
markdown report summarizing: gap distribution, top gaps by severity/novelty,
papers analyzed, and recommended next steps.

Usage:
    from research_loop.survey_generator import generate_research_survey
    path = generate_research_survey(topic, scored_gaps, papers_analyzed, session_id)
"""

from __future__ import annotations

import datetime
import os
from pathlib import Path
from typing import Any

# ─── Survey Templates ──────────────────────────────────────────────────────────

_SURVEY_SYSTEM_PROMPT = """You are an expert research strategist. Given a set of research gaps
discovered from literature, write a concise strategic research survey.

Structure:
1. **Executive Summary** — 3-5 bullet points of the most critical gaps
2. **Gap Landscape** — breakdown of gaps by type (capability, improvement, contradiction, etc.)
3. **Top Priority Gaps** — ranked list of 3-5 highest-value gaps with justification
4. **Analysis Statistics** — papers analyzed, iterations run, gap types found
5. **Recommended Next Steps** — concrete actions (which gaps to pursue first, what to explore)

Be direct and actionable. Use academic but concise language.
Reference gaps by their short titles."""


_SURVEY_USER_TEMPLATE = """Topic: {topic}

Gap Findings ({gap_count} total, {new_count} new since last session):

GAPS (severity, novelty, type):
{gap_list}

Papers Analyzed: {papers_analyzed}
Research Iterations: {iterations}
Session: {session_id}

Generate a concise strategic research survey."""


# ─── Survey Generation ─────────────────────────────────────────────────────────

def _gap_sort_key(gap: dict) -> tuple:
    """Sort gaps: HIGH severity first, then by novelty_score desc."""
    sev_rank = {"HIGH": 0, "MEDIUM": 1, "LOW": 2}
    sev = sev_rank.get(gap.get("severity", "LOW"), 2)
    novelty = gap.get("novelty_score", 0.0) or 0.0
    return (sev, -novelty)


def _build_gap_list(scored_gaps: list) -> str:
    """Format gaps for LLM prompt."""
    lines = []
    for g in scored_gaps[:15]:  # cap at 15 for prompt length
        sev = g.get("severity", "LOW")
        novelty = f"{g.get('novelty_score', 0.0):.2f}"
        gap_type = g.get("gap_type", "unknown")
        title = g.get("title", "Untitled")[:80]
        gene_score = f"{g.get('gene_pool_score', 0.0):.2f}"
        lines.append(
            f"- [{sev}] ({gap_type}) novelty={novelty} gp_score={gene_score} | {title}"
        )
    return "\n".join(lines) if lines else "No gaps found."


def generate_research_survey(
    topic: str,
    scored_gaps: list[dict[str, Any]],
    papers_analyzed: int = 0,
    session_id: str = "",
    iterations: int = 0,
    output_dir: str | None = None,
    api_key: str | None = None,
    base_url: str | None = None,
    model: str | None = None,
    gap_history_stats: dict | None = None,
) -> str:
    """Generate a markdown research survey from scored gaps.

    Returns the path to the generated markdown file.
    """

    # Sort gaps by severity then novelty
    sorted_gaps = sorted(scored_gaps, key=_gap_sort_key)

    gap_list_text = _build_gap_list(sorted_gaps)

    # Stats
    gap_count = len(scored_gaps)
    new_count = gap_history_stats.get("new", "?") if gap_history_stats else "?"
    sev_counts: dict[str, int] = {}
    type_counts: dict[str, int] = {}
    for g in scored_gaps:
        sev = g.get("severity", "LOW")
        gap_type = g.get("gap_type", "unknown")
        sev_counts[sev] = sev_counts.get(sev, 0) + 1
        type_counts[gap_type] = type_counts.get(gap_type, 0) + 1

    # Build user prompt
    user_prompt = _SURVEY_USER_TEMPLATE.format(
        topic=topic,
        gap_count=gap_count,
        new_count=new_count,
        gap_list=gap_list_text,
        papers_analyzed=papers_analyzed,
        iterations=iterations,
        session_id=session_id,
    )

    # Try LLM generation
    llm_markdown: str | None = None
    try:
        import os as _os
        try:
            from llm.chat import call_llm_chat_completions
        except ImportError:
            from llm.client import call_llm_chat_completions

        llm_markdown = call_llm_chat_completions(
            base_url=base_url or _os.getenv("OPENAI_BASE_URL", "") or "https://api.minimaxi.chat/v1",
            api_key=api_key or _os.getenv("MINIMAX_API_KEY", "") or _os.getenv("OPENAI_API_KEY", ""),
            model=model or _os.getenv("LLM_MODEL", "") or "MiniMax-M2.7",
            system_prompt=_SURVEY_SYSTEM_PROMPT,
            user_prompt=user_prompt,
        )
    except Exception:
        llm_markdown = None  # Will use template fallback

    # Build full markdown
    now = datetime.datetime.now().strftime("%Y-%m-%d %H:%M")
    session_short = session_id[:8] if session_id else "unknown"

    # Severity badge helper
    def sev_badge(s):
        return {"HIGH": "🔴 HIGH", "MEDIUM": "🟡 MEDIUM", "LOW": "🟢 LOW"}.get(s, s)

    # Gap table rows
    gap_rows = []
    for g in sorted_gaps[:20]:  # top 20 in table
        sev = g.get("severity", "LOW")
        novelty = f"{g.get('novelty_score', 0.0):.2f}"
        gp = f"{g.get('gene_pool_score', 0.0):.2f}"
        pref = "✓" if g.get("preference_boost") else ""
        gap_type = g.get("gap_type", "unknown")
        title = g.get("title", "Untitled")[:70]
        gap_rows.append(
            f"| {sev_badge(sev)} | {gap_type} | {novelty} | {gp} | {pref} | {title} |"
        )
    gap_table = "\n".join(gap_rows)

    # Gap type distribution
    type_lines = [f"- **{t}**: {c}" for t, c in sorted(type_counts.items(), key=lambda x: -x[1])]
    type_dist = "\n".join(type_lines) if type_lines else "—"

    # Severity distribution
    sev_lines = [f"- **{sev_badge(s)}**: {c}" for s, c in
                 [("HIGH", sev_counts.get("HIGH", 0)),
                  ("MEDIUM", sev_counts.get("MEDIUM", 0)),
                  ("LOW", sev_counts.get("LOW", 0))] if c > 0]
    sev_dist = "\n".join(sev_lines) if sev_lines else "—"

    # If LLM succeeded, merge its content
    if llm_markdown:
        llm_section = f"\n\n## LLM Strategic Analysis\n\n{llm_markdown}\n"
    else:
        llm_section = ""

    # Build static report (always generated)
    markdown = f"""# Research Survey: {topic}

**Generated:** {now}
**Session:** `{session_short}`
**Papers Analyzed:** {papers_analyzed} | **Iterations:** {iterations} | **Gaps Found:** {gap_count} ({new_count} new)

---

## Gap Distribution

### By Severity
{sev_dist}

### By Type
{type_dist}

---

## Gap Details (Top {min(20, len(sorted_gaps))} by Priority)

| Severity | Type | Novelty | GenePool | Pref | Title |
|----------|------|---------|----------|------|-------|
{gap_table}

{llm_section}
---

## Next Steps

1. Review 🔴 HIGH severity gaps — these are the most impactful research opportunities
2. Check GenePool for existing code implementations matching top gaps
3. Run `airos paper trace <topic>` to see paper→code lineage
4. Consider running a focused DeepResearch iteration on the top gap

> _This survey was auto-generated by Rairos AI Research OS_
"""

    # Save to file
    if output_dir is None:
        output_dir = os.getenv("AIROS_OUTPUT_DIR", "output/surveys")
    out_path = Path(output_dir)
    out_path.mkdir(parents=True, exist_ok=True)

    # Sanitize topic for filename
    safe_topic = topic.replace(" ", "_").replace("/", "-")[:50]
    ts = datetime.datetime.now().strftime("%Y%m%d_%H%M")
    filename = f"survey_{safe_topic}_{ts}_{session_short}.md"
    file_path = out_path / filename

    with open(file_path, "w", encoding="utf-8") as f:
        f.write(markdown)

    return str(file_path)
