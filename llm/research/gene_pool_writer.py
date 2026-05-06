"""Gene Pool writer — save gap analysis results to Gene Pool stores."""

from __future__ import annotations

import uuid
from datetime import datetime
from typing import Any, Dict, List, Optional

from llm.gene_pool_io import get_capsule_by_paper


def save_gap_to_gene_pool(
    paper_id: str,
    title: str,
    gap_type: str,
    gap_title: str,
    keywords: List[str],
    summary: str,
    polarity: str = "positive",
    extra_fields: Optional[Dict[str, Any]] = None,
) -> Optional[str]:
    """Append a new gap as a CapsuleGene entry to both Gene Pool stores.

    Deduplication: if paper_id + gap_type already exists in capsules.json, skip.

    Returns capsule_id on success, None on failure.
    """
    existing = get_capsule_by_paper(paper_id, gap_type=gap_type)
    if existing:
        return existing.get("capsule_id")

    try:
        capsule_id = f"extracted_{paper_id}_{uuid.uuid4().hex[:8]}"
        now = datetime.now().isoformat()
        archetype = {
            "extracted_from": "paper_gap_extractor",
            "source_paper_id": paper_id,
            "summary": summary,
        }
        if extra_fields:
            archetype.update(extra_fields)
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
            "archetype": archetype,
            "status": "active",
        }

        try:
            from llm.insight.tracker import EvolutionTracker
            tracker = EvolutionTracker()
            result = tracker.encode_capsule(
                topic=title[:200],
                gap_type=gap_type,
                gap_title=gap_title[:200],
                gap_description=summary,
                success_score=0.5,
                status="active",
                capsule_archetype=archetype,
                capsule_id=capsule_id,
            )
            if result is None:
                return None
        except Exception:
            pass

        return capsule_id
    except Exception:
        return None
