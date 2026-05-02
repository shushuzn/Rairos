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
