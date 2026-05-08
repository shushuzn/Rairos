"""Extract research gaps from a paper using LLM analysis.

This module is reorganized into focused submodules:
- gap_configs: GAP_ANALYZER_CONFIGS templates (Q1-Q10)
- gap_extract: extract_gap_from_paper + _extract_keywords
- gene_pool_writer: save_gap_to_gene_pool
- contradiction_detector: detect_field_contradiction + detect_contradictions
- embodied_planning: embodied-specific analysis + dashboard rendering
- embodied_pipeline: run_embodied_analysis orchestration
- multi_paper_analyzer: analyze_multi_paper_gaps + gaps_to_research_questions
- research_notes: add_research_note + get_research_notes
- hypothesis_generator: generate_hypothesis_from_contradiction + append_hypothesis_to_roadmap
"""

from __future__ import annotations

from typing import Any, Dict, List, Optional

# Re-export configs and core functions
from llm.research.gap_configs import GAP_ANALYZER_CONFIGS
from llm.research.gap_extract import extract_gap_from_paper, _extract_keywords
from llm.research.gene_pool_writer import save_gap_to_gene_pool
from llm.research.contradiction_detector import (
    detect_field_contradiction,
    detect_polarity_contradiction,
    detect_evidence_contradiction,
    detect_contradictions,
)


# Lazy LLM import
def _call_llm(messages: list, model: Optional[str] = None, api_key: Optional[str] = None):
    from llm.client import call_llm_chat_completions

    return call_llm_chat_completions(
        messages=messages, model=model or "claude-3-5-sonnet-latest", api_key=api_key
    )


def analyze_gap(
    paper_id: str,
    title: str,
    abstract: str,
    gap_type: str,
    authors: Optional[List[str]] = None,
    api_key: Optional[str] = None,
    model: Optional[str] = None,
    extra_fields: Optional[Dict[str, Any]] = None,
) -> Dict[str, Any]:
    """Generalized gap analyzer using GAP_ANALYZER_CONFIGS templates."""
    if gap_type not in GAP_ANALYZER_CONFIGS:
        return {
            "error": f"Unknown gap_type: {gap_type}. Available: {list(GAP_ANALYZER_CONFIGS.keys())}",
        }

    config = GAP_ANALYZER_CONFIGS[gap_type]
    result_fields = config["result_fields"]
    keywords = config.get("keywords", [])
    authors_str = ", ".join(authors[:3]) if authors else "Unknown"

    prompt = config["prompt_template"].format(
        title=title,
        authors=authors_str,
        abstract=abstract[:800],
    )

    try:
        content = _call_llm([{"role": "user", "content": prompt}], model=model, api_key=api_key)

        lines = content.strip().split("\n")
        result: Dict[str, Any] = {f: "" for f in result_fields}
        result["confidence"] = 0.5
        for line in lines:
            stripped = line.strip().strip("*").strip()
            for field in result_fields:
                prefix = field + ":"
                if stripped.startswith(prefix):
                    val = stripped.split(":", 1)[1].strip().strip("*`-").strip()
                    if field == "confidence":
                        try:
                            result[field] = float(val)
                        except ValueError:
                            pass
                    else:
                        result[field] = val

        gap_extra = extra_fields.copy() if extra_fields else {}
        for field in result_fields:
            if result.get(field) and result[field] != "unknown":
                gap_extra[field] = result[field]

        capsule_id = save_gap_to_gene_pool(
            paper_id=paper_id,
            title=title,
            gap_type=gap_type,
            gap_title=result.get("gap_title") or title[:80],
            keywords=keywords + [str(result.get(result_fields[0], ""))],
            summary=result.get("summary")
            or f"{gap_type} analysis. {result_fields[0]}: {result.get(result_fields[0], 'unknown')}",
            polarity="open",
            extra_fields=gap_extra,
        )
        result["saved_to_pool"] = capsule_id

        result["contradiction_with"] = None
        result["contradiction_type"] = None
        if capsule_id:
            primary_field = result_fields[0]
            current_val = result.get(primary_field, "unknown")
            conflict = detect_field_contradiction(gap_type, primary_field, current_val)
            if conflict:
                result["contradiction_with"] = conflict["source_paper_id"]
                result["contradiction_type"] = conflict["conflicting_value"]

        if gap_type == "embodied_planning":
            from llm.research.embodied_planning import track_embodied_evolution

            track_embodied_evolution(
                paper_id=paper_id,
                title=title,
                representation_type=result.get("representation_type", "unknown"),
                confidence=float(result.get("confidence", 0.5)),
                gap_title=result.get("gap_title")
                or f"Latent Reasoning: {result.get('representation_type', 'unknown')}",
            )

        return result

    except Exception as e:
        result = {f: "unknown" for f in result_fields}
        result["gap_title"] = title[:80]
        result["summary"] = f"Analysis failed: {e}"
        result["error"] = str(e)
        return result


def analyze_embodied_planning(
    paper_id: str,
    title: str,
    abstract: str,
    authors: Optional[List[str]] = None,
    api_key: Optional[str] = None,
    model: Optional[str] = None,
) -> Dict[str, Any]:
    """Analyze embodied planning paper: discrete vs continuous latent representation."""
    return analyze_gap(
        paper_id=paper_id,
        title=title,
        abstract=abstract,
        gap_type="embodied_planning",
        authors=authors,
        api_key=api_key,
        model=model,
    )


def batch_analyze_embodied_planning(
    paper_ids: List[str],
    db=None,
) -> Dict[str, Any]:
    """Batch analyze multiple papers for embodied planning representation types."""
    if db is None:
        from db.database import Database

        db = Database()

    results = []
    for pid in paper_ids:
        paper = db.get_paper(pid)
        if not paper:
            continue
        r = analyze_embodied_planning(
            paper_id=pid,
            title=paper.title,
            abstract=paper.abstract or "",
            authors=paper.authors,
        )
        results.append(r)

    type_counts = {"discrete": 0, "continuous": 0, "hybrid": 0, "unknown": 0}
    for r in results:
        rt = r.get("representation_type", "unknown")
        type_counts[rt] = type_counts.get(rt, 0) + 1

    return {
        "results": results,
        "type_counts": type_counts,
        "total": len(results),
        "trend": max(type_counts, key=type_counts.get) if results else "unknown",  # type: ignore[arg-type]
    }


def semantic_search_papers(
    query: str,
    top_k: int = 5,
    db=None,
) -> List[Dict[str, Any]]:
    """Semantic search across papers in the database."""
    if db is None:
        from db.database import Database

        db = Database()

    rows, _ = db.search_papers(query, limit=top_k * 2)
    query_tokens = set(_extract_keywords(query))
    scored: List[Dict[str, Any]] = []
    for row in rows:
        content = f"{getattr(row, 'title', '')} {getattr(row, 'abstract', '')}".lower()
        matched = sum(1 for t in query_tokens if t in content)
        if matched > 0:
            scored.append(
                {"score": matched, "matched_terms": list(query_tokens & set(content.split()))}
            )

    scored.sort(key=lambda x: x["score"], reverse=True)
    return scored[:top_k]


def analyze_multi_paper_gaps(
    papers: List[Dict[str, str]],
    model: Optional[str] = None,
) -> Dict[str, Any]:
    """Analyze gaps across multiple papers."""
    if len(papers) < 2:
        return {
            "shared_themes": [],
            "complementary_gaps": [],
            "frontier_gaps": [],
            "contradictions": [],
            "paper_count": len(papers),
            "error": "Need at least 2 papers for multi-paper gap analysis",
        }

    papers_str = "\n\n".join(
        f"Paper {i + 1} (ID: {p.get('id', '?')}):\nTitle: {p.get('title', '?')}\nAbstract: {p.get('abstract', '?')[:400]}"
        for i, p in enumerate(papers)
    )

    prompt = f"""You are a research gap analyst. Given a set of papers, identify systematic research gaps and opportunities across the entire collection.

PAPERS:
{papers_str}

TASK:
Analyze ALL papers above and produce a structured gap report focusing on:
1. SHARED THEMES: research areas appearing across multiple papers
2. COMPLEMENTARY GAPS: what each paper DOES NOT address that another DOES
3. FRONTIER GAPS: gaps that NONE of the papers address
4. CONTRADICTIONS: opposing claims about the same gap_type

Respond ONLY with JSON (no markdown):
{{
  "shared_themes": [
    {{"theme": "reasoning", "papers": ["id1", "id2"], "strength": 0.8, "description": "..."}}
  ],
  "complementary_gaps": [
    {{"gap_title": "...", "gap_type": "method_limitation", "addressed_by": ["id1"], "missing_from": ["id2"], "description": "..."}}
  ],
  "frontier_gaps": [
    {{"gap_title": "...", "gap_type": "evaluation_gap", "keywords": ["benchmark"], "summary": "..."}}
  ],
  "contradictions": []
}}"""

    try:
        content = _call_llm([{"role": "user", "content": prompt}], model=model)
        import json as _json

        return _json.loads(content.strip())  # type: ignore[no-any-return]
    except Exception as e:
        return {
            "error": str(e),
            "shared_themes": [],
            "complementary_gaps": [],
            "frontier_gaps": [],
            "contradictions": [],
        }


def gaps_to_research_questions(
    frontier_gaps: List[Dict[str, Any]],
    paper_titles: Optional[Dict[str, str]] = None,
    model: Optional[str] = None,
) -> Dict[str, Any]:
    """Convert frontier gaps into actionable research questions."""
    if not frontier_gaps:
        return {"questions": [], "gap_count": 0}

    gaps_str = "\n".join(
        f"- Gap {i + 1}: {g.get('gap_title', '?')} [{g.get('gap_type', '?')}] "
        f"keywords={', '.join(g.get('keywords', [])[:5])} "
        f"summary={g.get('summary', '?')}"
        for i, g in enumerate(frontier_gaps)
    )

    prompt = f"""Given the following frontier research gaps, generate concrete, actionable research questions.

Each question must be:
- Specific: mentions method names, evaluation criteria, or constraints
- Answerable: can be addressed by running experiments or analyzing existing papers
- Novelty-justified: motivated by the gap it addresses

Output JSON (no markdown):
{{
  "questions": [
    {{
      "question": "Can chain-of-thought prompting improve factual recall in 70B+ models on medical VQA when evaluated on the MedQA dataset?",
      "gap_title": "Limited medical VQA benchmark coverage",
      "gap_type": "evaluation_gap",
      "keywords": ["medical-vqa", "chain-of-thought"],
      "difficulty": "medium",
      "hypothesis": "Chain-of-thought reasoning will improve factual recall by enabling multi-step diagnostic inference"
    }}
  ]
}}

GAP DATA:
{gaps_str}

Respond ONLY with the JSON object."""

    try:
        content = _call_llm([{"role": "user", "content": prompt}], model=model)
        import json as _json

        result = _json.loads(content.strip())  # type: ignore[no-any-return]
        result["gap_count"] = len(frontier_gaps)
        return result  # type: ignore[no-any-return]
    except Exception as e:
        return {"questions": [], "gap_count": len(frontier_gaps), "error": str(e)}


def add_research_note(paper_id, note, tags=None):
    from llm.research_log import add_note

    return add_note(paper_id, note, tags)


def get_research_notes(paper_id=None, limit=20):
    from llm.research_log import get_notes

    return get_notes(paper_id=paper_id, limit=limit)


def generate_hypothesis_from_contradiction(contradiction_pair: dict) -> str:
    """Generate a new hypothesis suggestion from a contradiction pair."""
    rep_a = contradiction_pair.get("representation_a", "unknown").lower().strip()
    rep_b = contradiction_pair.get("representation_b", "unknown").lower().strip()
    effect_a = contradiction_pair.get("effectiveness_a", "").lower().strip()
    effect_b = contradiction_pair.get("effectiveness_b", "").lower().strip()

    if ("discrete" in rep_a and "continuous" in rep_b) or (
        "continuous" in rep_a and "discrete" in rep_b
    ):
        return (
            "Hybrid architecture combining discrete reasoning with continuous execution may capture "
            "benefits of both: use discrete latent tokens for high-level planning and continuous "
            "distributions for low-level control, potentially achieving both interpretability and precision."
        )
    if effect_a == "effective" and effect_b == "ineffective":
        return "Combining ineffective and effective approaches in a multi-stage pipeline may yield improvements."
    if effect_a == "ineffective" and effect_b == "effective":
        return "Sequential combination of the effective method followed by the ineffective one may provide orthogonal benefits."
    return (
        f"A+B hybrid method combining {rep_a} and {rep_b} may achieve better trade-offs "
        f"between the conflicting findings in papers {contradiction_pair.get('paper_a_id', '?')} "
        f"and {contradiction_pair.get('paper_b_id', '?')}."
    )


def append_hypothesis_to_roadmap(
    hypothesis: str,
    paper_id_a: str,
    paper_id_b: str,
) -> bool:
    """Append a hypothesis to ROADMAP.md under ### Pending Hypotheses."""
    try:
        from pathlib import Path

        roadmap_path = Path("D:/OpenClaw/workspace/80-PROJECTS/ai_research_os/ROADMAP.md")
        if not roadmap_path.exists():
            return False

        content = roadmap_path.read_text(encoding="utf-8")
        marker = f"- [ ] HYPOTHESIS: {hypothesis} (from: {paper_id_a} vs {paper_id_b})"

        if marker in content:
            return True

        pending_marker = "### Pending Hypotheses"
        if pending_marker in content:
            lines = content.split("\n")
            new_lines = []
            for i, line in enumerate(lines):
                new_lines.append(line)
                if line.strip() == pending_marker:
                    j = i + 1
                    while j < len(lines) and not lines[j].startswith("### "):
                        j += 1
                    insert_pos = j if j < len(lines) else len(lines)
                    new_lines.insert(insert_pos, marker)
                    content = "\n".join(new_lines)
                    break
        else:
            content += f"\n\n{pending_marker}\n- [ ] HYPOTHESIS: {hypothesis} (from: {paper_id_a} vs {paper_id_b})\n"

        roadmap_path.write_text(content, encoding="utf-8")
        return True
    except Exception:
        return False


def run_embodied_analysis(
    paper_ids: List[str],
    db=None,
    save_to_pool: bool = True,
    gap_type: str = "embodied_planning",
) -> Dict[str, Any]:
    """Run embodied planning analysis on a list of paper IDs."""
    if db is None:
        from db.database import Database

        db = Database()
        db.init()

    analyzed: List[Dict[str, Any]] = []
    contradictions: List[Dict[str, Any]] = []
    type_counts: Dict[str, int] = {"discrete": 0, "continuous": 0, "hybrid": 0, "unknown": 0}

    for pid in paper_ids:
        paper = db.get_paper(pid)
        if not paper:
            continue
        r = analyze_embodied_planning(
            paper_id=pid,
            title=paper.title,
            abstract=paper.abstract or "",
            authors=paper.authors,
        )
        rep_type = r.get("representation_type", "unknown")
        type_counts[rep_type] = type_counts.get(rep_type, 0) + 1
        entry = {
            "paper_id": pid,
            "title": paper.title[:60],
            "representation_type": rep_type,
            "confidence": r.get("confidence", 0),
            "saved": "error" not in r,
        }
        if r.get("contradiction_with"):
            entry["contradiction_with"] = r["contradiction_with"]
            entry["contradiction_type"] = r.get("contradiction_type")
            contradictions.append(entry)
        analyzed.append(entry)

    total = len(analyzed)
    trend = max(type_counts, key=type_counts.get) if total > 0 else "unknown"  # type: ignore[arg-type]
    trend_pct = type_counts[trend] / total if total > 0 else 0

    return {
        "analyzed": analyzed,
        "contradictions": contradictions,
        "type_counts": type_counts,
        "total_analyzed": total,
        "trend": trend,
        "trend_pct": trend_pct,
    }
