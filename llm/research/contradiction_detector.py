"""Contradiction detection across gap analyses."""

from __future__ import annotations

from typing import Any, Dict, List, Optional

from llm.gene_pool_io import load_capsules


def detect_field_contradiction(
    gap_type: str,
    primary_field: str,
    current_val: str,
    capsules: Optional[List[Dict[str, Any]]] = None,
) -> Optional[Dict[str, Any]]:
    """Find a capsule with same gap_type but different primary field value.

    Returns {source_paper_id, conflicting_value} or None.
    """
    if current_val == "unknown" or current_val is None:
        return None
    if capsules is None:
        capsules = load_capsules(gap_type=gap_type, status="active")
    for c in capsules:
        if c.get("archetype", {}).get("source_paper_id") is None:
            continue
        ex_val = c.get("archetype", {}).get(primary_field, "unknown")
        if ex_val != current_val and ex_val != "unknown":
            return {
                "source_paper_id": c["archetype"]["source_paper_id"],
                "conflicting_value": ex_val,
            }
    return None


def detect_polarity_contradiction(
    gap_type: str,
    capsules: List[Dict[str, Any]],
) -> List[Dict[str, Any]]:
    """Find capsules in same gap_type with opposing polarity."""
    contradictions = []
    for i, c in enumerate(capsules):
        p_i = c.get("polarity", "open")
        for _j, other in enumerate(capsules[i + 1 :], i + 1):
            p_j = other.get("polarity", "open")
            if p_i != p_j and p_i != "open" and p_j != "open":
                contradictions.append(
                    {
                        "type": "polarity",
                        "gap_type": gap_type,
                        "capsule_a": c.get("capsule_id"),
                        "capsule_b": other.get("capsule_id"),
                        "paper_a": c.get("archetype", {}).get("source_paper_id"),
                        "paper_b": other.get("archetype", {}).get("source_paper_id"),
                        "polarity_a": p_i,
                        "polarity_b": p_j,
                    }
                )
    return contradictions


def detect_evidence_contradiction(
    gap_type: str,
    capsules: List[Dict[str, Any]],
) -> List[Dict[str, Any]]:
    """Find capsules with same gap_type but contradictory evidence."""
    contradictions = []
    for i, c in enumerate(capsules):
        ev_i = c.get("archetype", {}).get("evidence", "")
        for _j, other in enumerate(capsules[i + 1 :], i + 1):
            ev_j = other.get("archetype", {}).get("evidence", "")
            if ev_i and ev_j and ev_i != ev_j:
                contradictions.append(
                    {
                        "type": "evidence",
                        "gap_type": gap_type,
                        "capsule_a": c.get("capsule_id"),
                        "capsule_b": other.get("capsule_id"),
                        "evidence_a": ev_i,
                        "evidence_b": ev_j,
                    }
                )
    return contradictions


def detect_contradictions(capsules: list) -> list:
    """Detect all contradictions across a list of capsules."""
    by_type: Dict[str, list] = {}
    for c in capsules:
        gt = c.get("trigger_gap_type")
        if gt:
            by_type.setdefault(gt, []).append(c)
    all_results = []
    for gt, caps in by_type.items():
        all_results.extend(detect_polarity_contradiction(gt, caps))
    return all_results
