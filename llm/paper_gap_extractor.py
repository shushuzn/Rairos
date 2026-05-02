"""Extract research gaps from a paper using LLM analysis."""

from __future__ import annotations

import json
import uuid
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
  "summary": "One sentence explaining what gap this paper addresses"
}}
"""

    try:
        import httpx
    except ImportError:
        return {
            "gap_type": "method_limitation",
            "gap_title": title[:80],
            "keywords": _extract_keywords(abstract),
            "summary": "Gap extraction requires httpx/llm library.",
            "error": "httpx not available",
        }

    key = api_key or LLM_BASE_URL
    url = (base_url or LLM_BASE_URL or "https://api.openai.com/v1") + "/chat/completions"
    headers = {"Authorization": f"Bearer {key}"} if key and "openai" not in key else {}
    payload = {
        "model": model or LLM_MODEL or "gpt-4o-mini",
        "messages": [{"role": "user", "content": prompt}],
        "temperature": 0.3,
        "max_tokens": 300,
    }

    try:
        with httpx.Client(timeout=30.0) as client:
            resp = client.post(url, json=payload, headers=headers)
            resp.raise_for_status()
            data = resp.json()
            content = data["choices"][0]["message"]["content"].strip()
            return json.loads(content)
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
) -> bool:
    """Append a new gap as a CapsuleGene entry to capsules.json."""
    try:
        capsule_path = Path.home() / ".ai_research_os" / "gene_pool" / "capsules.json"
        capsule_path.parent.mkdir(parents=True, exist_ok=True)

        if capsule_path.exists():
            data = json.loads(capsule_path.read_text(encoding="utf-8"))
        else:
            data = {"version": 1, "capsules": []}

        capsule = {
            "capsule_id": f"extracted_{paper_id}_{uuid.uuid4().hex[:8]}",
            "created_at": "",
            "trigger_topic": title[:200],
            "trigger_gap_type": gap_type,
            "trigger_keywords": keywords,
            "action_gap_type": gap_type,
            "action_gap_title": gap_title[:200],
            "outcome_success_score": 0.5,
            "feedback_count": 0,
            "evolved_generation": 0,
            "archetype": {
                "extracted_from": "paper_gap_extractor",
                "source_paper_id": paper_id,
                "summary": summary,
            },
        }
        data["capsules"].append(capsule)
        capsule_path.write_text(json.dumps(data, indent=2, ensure_ascii=False), encoding="utf-8")
        return True
    except Exception:
        return False
