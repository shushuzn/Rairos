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
MOMENTUM_DAYS = 7  # rolling window for momentum calculation
MOMENTUM_STATE_FILE = GP_DIR / "momentum_state.json"

# Domain velocity: how fast claims in this arxiv category become stale
# Higher λ → faster decay, shorter effective half-life
# CS fields (LLM, vision) move fast; math/physics are more stable
DOMAIN_LAMBDA_FACTOR: Dict[str, float] = {
    "cs.AI": 0.02,    # fast-moving: half-life ~35 days
    "cs.LG": 0.02,
    "cs.CL": 0.02,
    "cs.CV": 0.015,   # fast: half-life ~46 days
    "cs.NE": 0.015,
    "cs.RO": 0.01,    # standard
    "cs.SE": 0.008,    # slower-moving
    "cs.CR": 0.005,   # security: more stable knowledge
    "cs.PL": 0.005,   # programming languages: very stable
    "math.ST": 0.003, # statistics theory: very stable
    "math.IT": 0.003,
    "physics.class-ph": 0.002,  # classical physics: extremely stable
    "quant-ph": 0.004,         # quantum: moderate
    "q-bio": 0.005,            # quantitative biology
    "econ.GN": 0.005,          # economic theory
}


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
    archived_by_gap_type: Dict[str, int] = field(default_factory=dict)  # gap_type → count
    total_archived: int = 0


@dataclass
class MomentumState:
    """Tracks new capsules per gap_type for momentum calculation."""

    new_by_gap_type: Dict[str, int] = field(default_factory=dict)  # gap_type → count
    archived_by_gap_type: Dict[str, int] = field(default_factory=dict)  # gap_type → count
    last_snapshot_at: str = ""  # ISO timestamp of last snapshot


# ─── Self-correction protocol ────────────────────────────────────────────────────


def _load_correction_state() -> SelfCorrectionState:
    """Load self-correction state from disk."""
    if not CORRECTION_STATE_FILE.exists():
        return SelfCorrectionState()
    try:
        import json
        data = json.loads(CORRECTION_STATE_FILE.read_text(encoding="utf-8"))
        return SelfCorrectionState(
            history=data.get("history", {}),
            corrections_triggered=data.get("corrections_triggered", {}),
            pending_gap_types=data.get("pending_gap_types", []),
            last_correction_at=data.get("last_correction_at", ""),
        )
    except Exception:
        return SelfCorrectionState()


def _save_correction_state(state: SelfCorrectionState) -> None:
    """Persist self-correction state to disk."""
    import json
    GP_DIR.mkdir(parents=True, exist_ok=True)
    CORRECTION_STATE_FILE.write_text(
        json.dumps({
            "history": state.history,
            "corrections_triggered": state.corrections_triggered,
            "pending_gap_types": state.pending_gap_types,
            "last_correction_at": state.last_correction_at,
        }, indent=2, ensure_ascii=False),
        encoding="utf-8",
    )


def check_self_correction(
    gap_type_coverage: Dict[str, float],
) -> Dict[str, Any]:
    """Check if any gap_type needs self-correction.

    A gap_type triggers self-correction when its coverage has been
    below COVERAGE_THRESHOLD for CONSECUTIVE_CYCLES_THRESHOLD consecutive cycles.

    Returns dict with triggered gap_types and pending list.
    """
    state = _load_correction_state()
    now = _now_iso()
    triggered: List[str] = []

    for gap_type, coverage in gap_type_coverage.items():
        if gap_type not in state.history:
            state.history[gap_type] = []

        # Append current coverage record
        state.history[gap_type].append({
            "coverage": coverage,
            "cycle_at": now,
        })

        # Keep only last 10 records per gap_type (rolling window)
        if len(state.history[gap_type]) > 10:
            state.history[gap_type] = state.history[gap_type][-10:]

        # Check consecutive low coverage cycles
        recent = state.history[gap_type][-CONSECUTIVE_CYCLES_THRESHOLD:]
        if len(recent) >= CONSECUTIVE_CYCLES_THRESHOLD:
            below_threshold = all(r["coverage"] < COVERAGE_THRESHOLD for r in recent)
            if below_threshold:
                already_correcting = (
                    gap_type in state.pending_gap_types or
                    state.corrections_triggered.get(gap_type, 0) > 0
                )
                if not already_correcting:
                    triggered.append(gap_type)
                    state.pending_gap_types.append(gap_type)
                    state.corrections_triggered[gap_type] = (
                        state.corrections_triggered.get(gap_type, 0) + 1
                    )

    if triggered:
        state.last_correction_at = now
        _save_correction_state(state)

    return {
        "triggered": len(triggered) > 0,
        "triggered_gap_types": triggered,
        "pending_gap_types": list(state.pending_gap_types),
        "corrections_triggered": dict(state.corrections_triggered),
    }


def dismiss_pending_correction(gap_type: str) -> bool:
    """Remove a gap_type from pending corrections (e.g., after manual intervention)."""
    state = _load_correction_state()
    if gap_type in state.pending_gap_types:
        state.pending_gap_types.remove(gap_type)
        _save_correction_state(state)
        return True
    return False


def get_self_correction_status() -> Dict[str, Any]:
    """Return current self-correction state for MCP query."""
    state = _load_correction_state()
    return {
        "pending_gap_types": list(state.pending_gap_types),
        "corrections_triggered": dict(state.corrections_triggered),
        "last_correction_at": state.last_correction_at,
        "tracked_gap_types": list(state.history.keys()),
    }

CORRECTION_STATE_FILE = GP_DIR / "correction_state.json"
COVERAGE_THRESHOLD = 0.20  # trigger correction when gap_type coverage falls below this
CONSECUTIVE_CYCLES_THRESHOLD = 3  # N consecutive cycles below threshold → trigger


@dataclass
class CoverageHistory:
    """Tracks per-cycle coverage per gap_type for self-correction."""

    gap_type: str
    coverage_ratio: float  # 0.0–1.0
    cycle_at: str  # ISO timestamp


@dataclass
class SelfCorrectionState:
    """Tracks coverage history per gap_type to trigger self-correction."""

    history: Dict[str, List[Dict[str, Any]]] = field(default_factory=dict)  # gap_type → list of {coverage, cycle_at}
    corrections_triggered: Dict[str, int] = field(default_factory=dict)  # gap_type → times corrected
    pending_gap_types: List[str] = field(default_factory=list)  # gap_types awaiting paper2code run
    last_correction_at: str = ""


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


def _get_adaptive_lambda(category: str, default_lambda: float = DEFAULT_LAMBDA) -> float:
    """Get decay lambda for a capsule based on its arxiv category.

    Fast-moving fields (LLM, vision) get higher λ → faster decay.
    Stable fields (math, physics, PL) get lower λ → longer half-life.
    Falls back to default_lambda for unknown categories.
    """
    if not category:
        return default_lambda
    # Check exact match first, then prefix match (e.g. "cs.LG" in "cs.LG", "cs.AI")
    if category in DOMAIN_LAMBDA_FACTOR:
        return DOMAIN_LAMBDA_FACTOR[category]
    # Prefix match: try "cs." prefix
    if "." in category:
        prefix = category.split(".")[0] + "."
        if prefix in DOMAIN_LAMBDA_FACTOR:
            return DOMAIN_LAMBDA_FACTOR[prefix]
    return default_lambda


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

        # Adaptive lambda: domain velocity from source_arxiv_category
        capsule_category = getattr(cap, "source_arxiv_category", "") or ""
        adaptive_lambda = _get_adaptive_lambda(capsule_category, lambda_)

        impact, age_days = compute_impact_score(
            success_score=cap.outcome_success_score,
            created_at=cap.created_at,
            feedback_count=cap.feedback_count,
            inbound_citations=inbound,
            lambda_=adaptive_lambda,
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
            gap_type = cap.action_gap_type or "unknown"
            state.archived_by_gap_type[gap_type] = state.archived_by_gap_type.get(gap_type, 0) + 1

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

    # Self-correction: check if any gap_type has chronically low coverage
    try:
        # Compute average coverage per gap_type from capsules
        coverage_by_gap_type: Dict[str, List[float]] = {}
        for cap in capsules:
            if cap.status != "active":
                continue
            cov = getattr(cap, "coverage_ratio", 0.0) or 0.0
            gt = cap.action_gap_type or "unknown"
            coverage_by_gap_type.setdefault(gt, []).append(cov)

        avg_coverage: Dict[str, float] = {}
        for gt, covs in coverage_by_gap_type.items():
            avg_coverage[gt] = round(sum(covs) / len(covs), 4) if covs else 0.0

        corr_result = check_self_correction(avg_coverage)
        if corr_result.get("triggered"):
            import logging
            logging.getLogger("decay").info(
                f"Self-correction triggered: {corr_result['triggered_gap_types']} "
                f"need paper2code coverage — pending: {corr_result['pending_gap_types']}"
            )
    except Exception:
        pass  # non-critical

    # Decay-aware subscription trigger: check if any gap_type is over-archived
    try:
        sub_result = trigger_decay_aware_subscriptions()
        if sub_result.get("triggered"):
            import logging
            logging.getLogger("decay").info(
                f"Decay-aware subs triggered: {sub_result['triggered_gap_types']} "
                f"→ added: {sub_result['subscriptions_added']}"
            )
    except Exception:
        pass  # non-critical

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
            archived_by_gap_type=data.get("archived_by_gap_type", {}),
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
            "archived_by_gap_type": state.archived_by_gap_type,
            "total_archived": state.total_archived,
        }, indent=2, ensure_ascii=False),
        encoding="utf-8",
    )


def _now_iso() -> str:
    return datetime.utcnow().isoformat()


# ─── Momentum tracking ──────────────────────────────────────────────────────────


def _load_momentum_state() -> MomentumState:
    """Load momentum state from disk, or return empty state."""
    if not MOMENTUM_STATE_FILE.exists():
        return MomentumState()
    try:
        import json
        data = json.loads(MOMENTUM_STATE_FILE.read_text(encoding="utf-8"))
        return MomentumState(
            new_by_gap_type=data.get("new_by_gap_type", {}),
            archived_by_gap_type=data.get("archived_by_gap_type", {}),
            last_snapshot_at=data.get("last_snapshot_at", ""),
        )
    except Exception:
        return MomentumState()


def _save_momentum_state(state: MomentumState) -> None:
    """Persist momentum state to disk."""
    import json
    GP_DIR.mkdir(parents=True, exist_ok=True)
    MOMENTUM_STATE_FILE.write_text(
        json.dumps({
            "new_by_gap_type": state.new_by_gap_type,
            "archived_by_gap_type": state.archived_by_gap_type,
            "last_snapshot_at": state.last_snapshot_at,
        }, indent=2, ensure_ascii=False),
        encoding="utf-8",
    )


def get_gap_type_momentum(
    days: int = MOMENTUM_DAYS,
) -> Dict[str, Dict[str, Any]]:
    """Compute momentum for each gap_type: new vs archived over rolling window.

    momentum > 1.0  → gap_type is growing (more new capsules than archived)
    momentum = 1.0  → equilibrium
    momentum < 1.0  → gap_type is shrinking
    momentum = 0.0   → no new capsules, only archives (dying)

    Returns dict: gap_type → {new, archived, momentum, trend}
    """
    from llm.insight.tracker import EvolutionTracker

    tracker = EvolutionTracker(data_dir=GP_DIR)
    capsules = tracker._load_capsules()
    state = _load_momentum_state()
    now = datetime.utcnow()
    cutoff = now.timestamp() - (days * 86400)

    # Count new capsules per gap_type since last_snapshot (or all if first run)
    if state.last_snapshot_at:
        try:
            last_ts = datetime.fromisoformat(state.last_snapshot_at).timestamp()
        except (ValueError, TypeError):
            last_ts = 0.0
    else:
        last_ts = cutoff  # first run: count all capsules in window

    new_by_gap_type: Dict[str, int] = {}
    for cap in capsules:
        if cap.status == "active":
            try:
                created_ts = datetime.fromisoformat(cap.created_at).timestamp()
            except (ValueError, TypeError):
                continue
            if created_ts >= last_ts:
                gt = cap.action_gap_type or "unknown"
                new_by_gap_type[gt] = new_by_gap_type.get(gt, 0) + 1

    # Also scan capsules created within the full window (for first-run bootstrap)
    new_by_gap_type_window: Dict[str, int] = {}
    for cap in capsules:
        if cap.status == "active":
            try:
                created_ts = datetime.fromisoformat(cap.created_at).timestamp()
            except (ValueError, TypeError):
                continue
            if created_ts >= cutoff:
                gt = cap.action_gap_type or "unknown"
                new_by_gap_type_window[gt] = new_by_gap_type_window.get(gt, 0) + 1

    # If no previous state, use window counts
    if not state.new_by_gap_type and not state.archived_by_gap_type:
        new_by_gap_type = new_by_gap_type_window
    else:
        # Merge: keep running totals of new since last snapshot
        new_by_gap_type = dict(state.new_by_gap_type)
        # Add any capsules created since last snapshot
        for cap in capsules:
            if cap.status == "active":
                try:
                    created_ts = datetime.fromisoformat(cap.created_at).timestamp()
                except (ValueError, TypeError):
                    continue
                if created_ts >= last_ts:
                    gt = cap.action_gap_type or "unknown"
                    new_by_gap_type[gt] = new_by_gap_type.get(gt, 0) + 1

    archived_by_gap_type = dict(state.archived_by_gap_type)

    # Compute momentum per gap_type
    all_gap_types = set(new_by_gap_type.keys()) | set(archived_by_gap_type.keys())
    result: Dict[str, Dict[str, Any]] = {}
    for gt in all_gap_types:
        new_count = new_by_gap_type.get(gt, 0)
        archived_count = archived_by_gap_type.get(gt, 0)
        total = new_count + archived_count
        if total == 0:
            momentum = 1.0  # no activity = equilibrium
        else:
            momentum = round(new_count / max(archived_count, 1), 3)

        if new_count > archived_count:
            trend = "rising"
        elif new_count < archived_count:
            trend = "falling"
        else:
            trend = "stable"
        if new_count == 0 and archived_count > 0:
            trend = "dying"

        result[gt] = {
            "new_7d": new_count,
            "archived_7d": archived_count,
            "momentum": momentum,
            "trend": trend,
        }

    # Prune old entries (no activity in 2x window)
    prune_cutoff = now.timestamp() - (days * 2 * 86400)
    # Save updated state with current snapshot
    _save_momentum_state(MomentumState(
        new_by_gap_type=new_by_gap_type,
        archived_by_gap_type=archived_by_gap_type,
        last_snapshot_at=_now_iso(),
    ))

    return result


# ─── Ranking ──────────────────────────────────────────────────────────────────


def get_ranked_capsules(
    limit: int = 20,
    lambda_: float = DEFAULT_LAMBDA,
) -> List[CapsuleImpact]:
    """Return capsules ranked by impact score descending."""
    impacts, _ = score_all_capsules(lambda_=lambda_)
    impacts.sort(key=lambda x: x.impact_score, reverse=True)
    return impacts[:limit]


# ─── Decay-aware subscriptions ─────────────────────────────────────────────────


GAP_TYPE_TO_FAMILY: Dict[str, str] = {
    "implementation": "other",
    "method_gap": "other",
    "dataset_gap": "vision",
    "evaluation_gap": "other",
    "efficiency_gap": "optimization",
    "scalability_gap": "other",
    "reasoning_gap": "reasoning",
}

ARCHIVE_THRESHOLD = 3  # trigger subscription refill after N consecutive archives of same gap_type


def trigger_decay_aware_subscriptions() -> Dict[str, Any]:
    """Check for over-archived gap_types and trigger GenePoolWatcher subscriptions.

    After each decay cycle, call this to check if any gap_type has been
    archived too many times relative to its active count — indicating
    that direction is under-performing and we should explore it more.

    Returns dict with triggered gap_types and subscription actions taken.
    """
    from llm.gene_pool_watcher import GenePoolWatcher, FAMILY_ARXIV_CONFIG

    state = _load_decay_state()

    if not state.archived_by_gap_type:
        return {"triggered": False, "reason": "no archived capsules to analyze"}

    # Load current Gene Pool counts by gap_type
    from llm.insight.tracker import EvolutionTracker
    tracker = EvolutionTracker(data_dir=GP_DIR)
    capsules = tracker._load_capsules()

    active_by_gap_type: Dict[str, int] = {}
    for cap in capsules:
        if cap.status == "active":
            gt = cap.action_gap_type or "unknown"
            active_by_gap_type[gt] = active_by_gap_type.get(gt, 0) + 1

    triggered_gap_types: List[str] = []
    subscriptions_added: List[str] = []

    for gap_type, archive_count in state.archived_by_gap_type.items():
        if archive_count < ARCHIVE_THRESHOLD:
            continue
        active_count = active_by_gap_type.get(gap_type, 0)
        # If archives >= active, this gap_type is dying — trigger subscription
        if archive_count >= active_count and active_count < 5:
            triggered_gap_types.append(gap_type)
            family = GAP_TYPE_TO_FAMILY.get(gap_type, "other")
            if family in FAMILY_ARXIV_CONFIG:
                watcher = GenePoolWatcher()
                from llm.gene_pool_watcher import GapSubscription
                new_sub = GapSubscription(
                    family=family,
                    keywords=FAMILY_ARXIV_CONFIG[family]["keywords"],
                    arxiv_category=FAMILY_ARXIV_CONFIG[family]["category"],
                    enabled=True,
                )
                from llm.gene_pool_watcher import _register_gap_subscription
                sub_id = _register_gap_subscription(new_sub)
                if sub_id:
                    subscriptions_added.append(family)

    # Reset counter for triggered gap_types (avoid re-triggering)
    if triggered_gap_types:
        for gt in triggered_gap_types:
            state.archived_by_gap_type[gt] = 0
        _save_decay_state(state)

    return {
        "triggered": len(triggered_gap_types) > 0,
        "triggered_gap_types": triggered_gap_types,
        "subscriptions_added": subscriptions_added,
        "archive_counts": dict(state.archived_by_gap_type),
    }


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
            "archived_by_gap_type": dict(state.archived_by_gap_type),
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

    elif action == "momentum":
        days = 7  # default rolling window
        result = get_gap_type_momentum(days=days)
        # Sort by momentum ascending (dying first)
        sorted_gap_types = sorted(
            result.items(), key=lambda x: x[1]["momentum"]
        )
        rising = [gt for gt, d in result.items() if d["trend"] == "rising"]
        falling = [gt for gt, d in result.items() if d["trend"] in ("falling", "dying")]
        return {
            "gap_types": {gt: d for gt, d in sorted_gap_types},
            "rising_gap_types": rising,
            "falling_gap_types": falling,
            "window_days": days,
            "total_gap_types": len(result),
        }

    elif action == "domain_stats":
        from llm.insight.tracker import EvolutionTracker
        tracker = EvolutionTracker(data_dir=GP_DIR)
        capsules = tracker._load_capsules()

        category_stats: Dict[str, Dict[str, Any]] = {}
        for cap in capsules:
            if cap.status != "active":
                continue
            cat = getattr(cap, "source_arxiv_category", "") or "unknown"
            lam = _get_adaptive_lambda(cat)
            half_life = round(0.693 / lam, 1) if lam > 0 else float("inf")

            if cat not in category_stats:
                category_stats[cat] = {
                    "count": 0,
                    "lambda": lam,
                    "half_life_days": half_life,
                }
            category_stats[cat]["count"] += 1

        return {
            "domain_stats": dict(category_stats),
            "default_lambda": lambda_,
            "default_half_life_days": round(0.693 / lambda_, 1),
        }

    elif action == "self_correction":
        status = get_self_correction_status()
        return status

    elif action == "dismiss_correction":
        if not capsule_id:
            return {"error": "capsule_id (gap_type) required"}
        dismissed = dismiss_pending_correction(capsule_id)
        return {"dismissed": dismissed, "gap_type": capsule_id}

    else:
        return {"error": f"Unknown action: {action}"}
