"""
Gene Pool Decay — time-weighted impact scoring and auto-archive.

闭环:
  Gene Pool capsules accumulate feedback over time
  → older capsules with low feedback decay in effective impact
  → auto-archive when impact < threshold for N consecutive cycles
  → citation_boost from ClaimGraph edges (cited-by → higher impact)

Impact Score formula:
  impact = success_score × e^(-λ × age_days) × log(feedback_count + 1) × citation_boost

  λ = 0.01  →  half-life ~69 days
  citation_boost = 1 + 0.1 × inbound_citations (from ClaimGraph)

Auto-archive condition:
  impact_score < min_impact_threshold (default 0.1)
  for consecutive_decay_cycles (default 3) → archive the capsule
"""

from __future__ import annotations

import math
from dataclasses import dataclass, field
from datetime import datetime
from pathlib import Path
from typing import Any, Dict, List, Optional

GP_DIR = Path.home() / ".ai_research_os" / "evolution"


# ─── Decay configuration ───────────────────────────────────────────────────────

DEFAULT_LAMBDA = 0.01          # half-life ~69 days
DEFAULT_MIN_IMPACT = 0.1       # archive if impact falls below this
DEFAULT_CONSECUTIVE_CYCLES = 3  # N consecutive cycles below threshold → archive
DECAY_STATE_FILE = GP_DIR / "decay_state.json"


@dataclass
class CapsuleImpact:
    """Computed impact score for a single capsule."""

    capsule_id: str
    impact_score: float
    age_days: float
    feedback_count: int
    success_score: float
    citation_boost: float
    inbound_citations: int
    archived: bool = False
    reason: str = ""


@dataclass
class DecayState:
    """Persistent state across decay cycles."""

    last_decay_at: str = ""   # ISO timestamp
    consecutive_low_impact: Dict[str, int] = field(default_factory=dict)  # capsule_id → count
    archived_this_cycle: List[str] = field(default_factory=list)  # archived this run
    total_archived: int = 0


# ─── Core impact calculation ───────────────────────────────────────────────────


def compute_impact_score(
    success_score: float,
    created_at: str,
    feedback_count: int,
    inbound_citations: int = 0,
    lambda_: float = DEFAULT_LAMBDA,
) -> tuple[float, float]:
    """Compute time-decayed impact score for a capsule.

    Returns (impact_score, age_days).
    """
    try:
        created_time = datetime.fromisoformat(created_at)
        age_days = (datetime.now() - created_time).total_seconds() / 86400.0
    except (ValueError, TypeError):
        age_days = 0.0

    # Exponential decay by age
    decay = math.exp(-lambda_ * age_days)

    # Feedback count bonus (logarithmic — diminishing returns)
    feedback_bonus = math.log(feedback_count + 1)

    # Citation boost from ClaimGraph (inbound edges = papers citing this one)
    citation_boost = 1.0 + 0.1 * inbound_citations

    # Combined impact score
    impact = success_score * decay * feedback_bonus * citation_boost

    return round(impact, 4), round(age_days, 1)


def get_inbound_citations(paper_id: str, graph=None) -> int:
    """Get count of papers that cite paper_id from ClaimGraph."""
    if graph is None:
        try:
            from research_loop.claim_graph import ClaimGraph
            graph = ClaimGraph.load()
        except Exception:
            return 0

    count = sum(
        1 for e in graph.edges
        if e.to_paper == paper_id
    )
    return count


def score_all_capsules(
    min_impact: float = DEFAULT_MIN_IMPACT,
    lambda_: float = DEFAULT_LAMBDA,
) -> tuple[List[CapsuleImpact], DecayState]:
    """Score all active capsules, apply decay, return impacts and updated state.

    Returns (impacts, updated_state).
    """
    from llm.insight.tracker import EvolutionTracker

    tracker = EvolutionTracker(data_dir=GP_DIR)
    capsules = tracker._load_capsules()
    state = _load_decay_state()

    # Build claim graph for citation lookup (lazy)
    try:
        from research_loop.claim_graph import ClaimGraph
        cg = ClaimGraph.load()
    except Exception:
        cg = None

    impacts: List[CapsuleImpact] = []
    new_consecutive: Dict[str, int] = {}

    for cap in capsules:
        if cap.status != "active":
            continue

        inbound = get_inbound_citations(cap.trigger_topic, cg) if cg else 0
        impact, age_days = compute_impact_score(
            success_score=cap.outcome_success_score,
            created_at=cap.created_at,
            feedback_count=cap.feedback_count,
            inbound_citations=inbound,
            lambda_=lambda_,
        )

        # Check consecutive low-impact cycles
        prev_streak = state.consecutive_low_impact.get(cap.capsule_id, 0)
        if impact < min_impact:
            new_streak = prev_streak + 1
            should_archive = new_streak >= DEFAULT_CONSECUTIVE_CYCLES
            reason = f"impact={impact:.3f} < {min_impact} for {new_streak} cycle(s)"
        else:
            new_streak = 0
            should_archive = False
            reason = ""

        new_consecutive[cap.capsule_id] = new_streak

        if should_archive:
            _archive_capsule(tracker, cap)
            state.archived_this_cycle.append(cap.capsule_id)
            state.total_archived += 1

        impacts.append(CapsuleImpact(
            capsule_id=cap.capsule_id,
            impact_score=impact,
            age_days=age_days,
            feedback_count=cap.feedback_count,
            success_score=cap.outcome_success_score,
            citation_boost=round(1.0 + 0.1 * inbound, 3),
            inbound_citations=inbound,
            archived=should_archive,
            reason=reason,
        ))

    # Update and save state
    state.consecutive_low_impact = new_consecutive
    state.last_decay_at = _now_iso()
    _save_decay_state(state)

    return impacts, state


def _archive_capsule(tracker: EvolutionTracker, cap: Any) -> None:
    """Archive a low-impact capsule."""
    try:
        tracker.archive_capsule(cap.capsule_id)
    except Exception:
        pass


# ─── Decay state persistence ───────────────────────────────────────────────────


def _load_decay_state() -> DecayState:
    """Load decay state from disk, or return empty state."""
    if not DECAY_STATE_FILE.exists():
        return DecayState()
    try:
        import json
        data = json.loads(DECAY_STATE_FILE.read_text(encoding="utf-8"))
        return DecayState(
            last_decay_at=data.get("last_decay_at", ""),
            consecutive_low_impact=data.get("consecutive_low_impact", {}),
            archived_this_cycle=data.get("archived_this_cycle", []),
            total_archived=data.get("total_archived", 0),
        )
    except Exception:
        return DecayState()


def _save_decay_state(state: DecayState) -> None:
    """Persist decay state to disk."""
    import json
    GP_DIR.mkdir(parents=True, exist_ok=True)
    DECAY_STATE_FILE.write_text(
        json.dumps({
            "last_decay_at": state.last_decay_at,
            "consecutive_low_impact": state.consecutive_low_impact,
            "archived_this_cycle": state.archived_this_cycle,
            "total_archived": state.total_archived,
        }, indent=2, ensure_ascii=False),
        encoding="utf-8",
    )


def _now_iso() -> str:
    return datetime.utcnow().isoformat()


# ─── Ranking ──────────────────────────────────────────────────────────────────


def get_ranked_capsules(
    limit: int = 20,
    lambda_: float = DEFAULT_LAMBDA,
) -> List[CapsuleImpact]:
    """Return capsules ranked by impact score descending."""
    impacts, _ = score_all_capsules(lambda_=lambda_)
    impacts.sort(key=lambda x: x.impact_score, reverse=True)
    return impacts[:limit]


# ─── MCP tool actions ──────────────────────────────────────────────────────────


def gene_pool_decay_action(
    action: str = "status",
    min_impact: float = DEFAULT_MIN_IMPACT,
    lambda_: float = DEFAULT_LAMBDA,
    archive: bool = False,
) -> dict:
    """MCP tool dispatcher for Gene Pool decay.

    Actions:
      status   — run decay cycle, return ranked capsule impacts
      rank      — return top N capsules by impact score
      archived  — return list of capsules archived this cycle
      reset     — reset consecutive counter for a capsule (unarchive-like)
    """
    if action == "status":
        impacts, state = score_all_capsules(
            min_impact=min_impact,
            lambda_=lambda_,
        )
        impacts.sort(key=lambda x: x.impact_score, reverse=True)

        return {
            "total_scored": len(impacts),
            "last_decay_at": state.last_decay_at,
            "archived_this_cycle": state.archived_this_cycle,
            "total_archived_ever": state.total_archived,
            "consecutive_tracking": len(state.consecutive_low_impact),
            "top_capsules": [
                {
                    "capsule_id": i.capsule_id,
                    "impact_score": i.impact_score,
                    "age_days": i.age_days,
                    "feedback_count": i.feedback_count,
                    "success_score": i.success_score,
                    "citation_boost": i.citation_boost,
                    "inbound_citations": i.inbound_citations,
                }
                for i in impacts[:10]
            ],
            "bottom_capsules": [
                {
                    "capsule_id": i.capsule_id,
                    "impact_score": i.impact_score,
                    "age_days": i.age_days,
                }
                for i in impacts[-5:]
            ],
        }

    elif action == "rank":
        impacts = get_ranked_capsules(lambda_=lambda_)
        return {
            "ranked": [
                {
                    "rank": idx + 1,
                    "capsule_id": i.capsule_id,
                    "impact_score": i.impact_score,
                    "age_days": i.age_days,
                    "feedback_count": i.feedback_count,
                    "success_score": i.success_score,
                }
                for idx, i in enumerate(impacts)
            ],
            "total": len(impacts),
        }

    elif action == "archived":
        state = _load_decay_state()
        return {
            "archived_this_cycle": state.archived_this_cycle,
            "total_archived_ever": state.total_archived,
            "last_decay_at": state.last_decay_at,
            "consecutive_tracking": {
                cid: cnt for cid, cnt in state.consecutive_low_impact.items()
                if cnt > 0
            },
        }

    elif action == "reset":
        state = _load_decay_state()
        # Reset all consecutive counters (e.g., before a new decay evaluation period)
        state.consecutive_low_impact = {}
        _save_decay_state(state)
        return {"reset": True, "message": "All consecutive counters cleared"}

    else:
        return {"error": f"Unknown action: {action}"}
