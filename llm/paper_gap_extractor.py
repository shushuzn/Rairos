"""Extract research gaps from a paper using LLM analysis."""

from __future__ import annotations

import json
import uuid
from datetime import datetime
from pathlib import Path
from typing import Any, Dict, List, Optional

from llm.constants import LLM_BASE_URL, LLM_MODEL


def extract_gap_from_paper(
    paper_id: str,
    title: str,
    abstract: str,
    authors: Optional[List[str]] = None,
    api_key: Optional[str] = None,
    base_url: Optional[str] = None,
    model: Optional[str] = None,
) -> Dict[str, Any]:
    """Extract research gap from a paper.

    Returns a dict with:
        gap_type: one of the GapType enum values
        gap_title: short title for the gap
        keywords: list of extracted keywords
        summary: brief explanation of what gap this paper addresses
    """
    authors_str = ", ".join(authors[:3]) if authors else "Unknown"

    prompt = f"""You are a research gap detector. Given a paper's title and abstract, extract ONE research gap that this paper addresses.

PAPER:
Title: {title}
Authors: {authors_str}
Abstract: {abstract[:800]}

TASK:
Identify what research gap this paper addresses. Choose ONE gap type from:
- unexplored_application: using known methods in new domains
- method_limitation: current methods have specific drawbacks
- contradiction: paper challenges or refutes existing findings
- evaluation_gap: lack of proper benchmarks or evaluation
- scalability_issue: methods don't scale to real-world settings
- theoretical_gap: lack of theoretical foundations
- dataset_gap: no suitable dataset for the problem
- generalization_gap: methods fail on out-of-distribution data

Respond ONLY with a JSON object (no markdown, no code fences):
{{
  "gap_type": "the_gap_type_here",
  "gap_title": "Short descriptive title (max 80 chars)",
  "keywords": ["keyword1", "keyword2", "keyword3"],
  "summary": "One sentence explaining what gap this paper addresses",
  "polarity": "positive or negative — does this paper ADVANCE the gap (positive: proposes new method/scale/improve) or FAIL/challenge it (negative: shows limitation, refutes, or fails to scale)"
}}
"""

    try:
        from llm.client import call_llm_chat_completions
    except ImportError:
        return {
            "gap_type": "method_limitation",
            "gap_title": title[:80],
            "keywords": _extract_keywords(abstract),
            "summary": "Gap extraction requires llm.client.",
            "error": "llm.client not available",
        }

    try:
        content = call_llm_chat_completions(
            messages=[{"role": "user", "content": prompt}],
            model="claude-3-5-sonnet-latest",
        )
        return json.loads(content.strip())
    except Exception as e:
        return {
            "gap_type": "method_limitation",
            "gap_title": title[:80],
            "keywords": _extract_keywords(abstract),
            "summary": f"LLM call failed: {e}",
            "error": str(e),
        }


def _extract_keywords(text: str) -> List[str]:
    """Simple keyword extraction from text."""
    words = text.lower().split()
    stop = {"the", "a", "an", "of", "in", "to", "for", "and", "or", "with", "is", "are", "that", "this", "we", "our", "by", "on", "as", "at", "from"}
    keywords = [w.strip(".,;:!?()[]{}") for w in words if len(w) > 4 and w not in stop]
    return list(dict.fromkeys(keywords))[:6]


def save_gap_to_gene_pool(
    paper_id: str,
    title: str,
    gap_type: str,
    gap_title: str,
    keywords: List[str],
    summary: str,
    polarity: str = "positive",
) -> bool:
    """Append a new gap as a CapsuleGene entry to both Gene Pool stores.

    Writes to:
    - ~/.ai_research_os/gene_pool/capsules.json   (read by _match_gene_pool / briefing_generator)
    - ~/.ai_research_os/evolution/gene_pool.jsonl (read by find_capsule / Curator / EvolutionTracker)
    """
    try:
        capsule_id = f"extracted_{paper_id}_{uuid.uuid4().hex[:8]}"
        now = datetime.now().isoformat()
        capsule = {
            "capsule_id": capsule_id,
            "created_at": now,
            "trigger_topic": title[:200],
            "trigger_gap_type": gap_type,
            "trigger_keywords": keywords,
            "action_gap_type": gap_type,
            "action_gap_title": gap_title[:200],
            "outcome_success_score": 0.5,
            "feedback_count": 0,
            "evolved_generation": 0,
            "polarity": polarity,
            "archetype": {
                "extracted_from": "paper_gap_extractor",
                "source_paper_id": paper_id,
                "summary": summary,
            },
            "status": "active",
        }

        # Write to capsules.json (briefing_generator reads this)
        capsule_path = Path.home() / ".ai_research_os" / "gene_pool" / "capsules.json"
        capsule_path.parent.mkdir(parents=True, exist_ok=True)
        if capsule_path.exists():
            data = json.loads(capsule_path.read_text(encoding="utf-8"))
        else:
            data = {"version": 1, "capsules": []}
        data["capsules"].append(capsule)
        capsule_path.write_text(json.dumps(data, indent=2, ensure_ascii=False), encoding="utf-8")

        # Write to gene_pool.jsonl (EvolutionTracker.find_capsule reads this)
        try:
            from llm.insight.gene import CapsuleGene
            from llm.insight.tracker import EvolutionTracker
            tracker = EvolutionTracker()
            tracker.encode_capsule(
                topic=title[:200],
                gap_type=gap_type,
                gap_title=gap_title[:200],
                gap_description=summary,
                success_score=0.5,
                status="active",
            )
        except Exception:
            # Non-critical: capsules.json write succeeded, gene_pool.jsonl is optional for CLI use
            pass

        return True
    except Exception:
        return False


def detect_contradictions(capsules: list) -> list:
    """Find pairs of capsules with same gap_type but opposite polarity.

    Returns list of dicts:
        {gap_type, positive_capsule, negative_capsule, shared_keywords}
    """
    from collections import defaultdict

    by_type = defaultdict(list)
    for c in capsules:
        if c.get("status") == "archived":
            continue
        polarity = c.get("polarity", "positive")
        gap_type = c.get("action_gap_type") or c.get("trigger_gap_type", "")
        if gap_type and polarity:
            by_type[gap_type].append((polarity, c))

    contradictions = []
    for gap_type, items in by_type.items():
        positives = [c for p, c in items if p == "positive"]
        negatives = [c for p, c in items if p == "negative"]
        for pc in positives:
            for nc in negatives:
                pk = set(pc.get("trigger_keywords", []))
                nk = set(nc.get("trigger_keywords", []))
                shared = pk & nk
                if shared:
                    contradictions.append({
                        "gap_type": gap_type,
                        "positive_capsule": pc,
                        "negative_capsule": nc,
                        "shared_keywords": list(shared),
                    })

    return contradictions


def analyze_multi_paper_gaps(
    papers: List[Dict[str, str]],
    model: Optional[str] = None,
) -> Dict[str, Any]:
    """Analyze gaps across multiple papers to surface shared and frontier opportunities.

    papers: list of {id, title, abstract}
    Returns: {shared_themes, complementary_gaps, frontier_gaps, contradictions, paper_count}
    """
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
        f"Paper {i+1} (ID: {p.get('id', '?')}):\nTitle: {p.get('title', '?')}\nAbstract: {p.get('abstract', '?')[:400]}"
        for i, p in enumerate(papers)
    )

    prompt = f"""You are a research gap analyst. Given a set of papers, identify systematic research gaps and opportunities across the entire collection.

PAPERS:
{papers_str}

TASK:
Analyze ALL papers above and produce a structured gap report. Focus on:

1. SHARED THEMES: research areas/keywords that appear across multiple papers — these represent active frontier themes.

2. COMPLEMENTARY GAPS: for each paper, identify what it DOES NOT address that another paper in the set DOES — these are complementary opportunities.

3. FRONTIER GAPS: research gaps that NONE of the papers address but are clearly suggested by the collection's themes. These are the most valuable opportunities.

4. CONTRADICTIONS: any papers that make opposing claims or findings about the same gap_type.

Respond ONLY with a JSON object (no markdown, no code fences):
{{
  "shared_themes": [
    {{"theme": "reasoning", "papers": ["id1", "id2"], "strength": 0.8, "description": "both papers tackle reasoning in long context"}}
  ],
  "complementary_gaps": [
    {{"gap_title": "..." , "gap_type": "method_limitation", "addressed_by": ["id1"], "missing_from": ["id2"], "description": "..."}}
  ],
  "frontier_gaps": [
    {{"gap_title": "...", "gap_type": "evaluation_gap", "keywords": ["benchmark", "reasoning"], "summary": "No paper evaluates on real-world deployment scenarios"}}
  ],
  "contradictions": [
    {{"gap_type": "scalability_issue", "positive_id": "id1", "negative_id": "id2", "description": "Paper A claims scaling works; Paper B shows it fails on OOD data"}}
  ]
}}
"""

    try:
        from llm.client import call_llm_chat_completions
    except ImportError:
        return {
            "shared_themes": [],
            "complementary_gaps": [],
            "frontier_gaps": [],
            "contradictions": [],
            "paper_count": len(papers),
            "error": "llm.client not available",
        }

    try:
        content = call_llm_chat_completions(
            messages=[{"role": "user", "content": prompt}],
            model=model or "claude-3-5-sonnet-latest",
        )
        result = json.loads(content.strip())
        result["paper_count"] = len(papers)
        return result
    except json.JSONDecodeError:
        return {
            "shared_themes": [],
            "complementary_gaps": [],
            "frontier_gaps": [],
            "contradictions": [],
            "paper_count": len(papers),
            "error": f"LLM returned invalid JSON: {content[:200] if 'content' in dir() else '?'}",
        }
    except Exception as e:
        return {
            "shared_themes": [],
            "complementary_gaps": [],
            "frontier_gaps": [],
            "contradictions": [],
            "paper_count": len(papers),
            "error": str(e),
        }
