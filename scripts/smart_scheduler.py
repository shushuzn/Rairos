"""Smart scheduling engine for subscribe watch loop.

Implements adaptive polling interval and proactive GenePool refilling.
"""
from __future__ import annotations

import time
from dataclasses import dataclass


# ─── Thresholds ────────────────────────────────────────────────────────────────

HIGH_SATURATION = 0.80   # GenePool nearly full — slow down
LOW_SATURATION = 0.20    # GenePool depleted — speed up
EMPTY_SATURATION = 0.05   # GenePool empty — trigger cold-start research

# Interval multipliers
SLOW_FACTOR = 2.0         # Multiply interval when HIGH
FAST_FACTOR = 0.5         # Multiply interval when LOW/EMPTY
DAEMON_INTERVAL_MIN = 5   # Minimum check interval (minutes)
DAEMON_INTERVAL_MAX = 120  # Maximum check interval (minutes)


@dataclass
class SchedulerDecision:
    """What the scheduler decided to do."""
    interval_minutes: float
    reason: str
    sat_before: float
    action: str  # "check", "cold_start", "skip"


def compute_adaptive_interval(
    base_interval_minutes: int,
    saturation: float,
    n_active: int,
    has_new_papers: bool,
) -> SchedulerDecision:
    """Decide polling interval and action based on GenePool state.

    Args:
        base_interval_minutes: User-specified check interval.
        saturation: GenePool active/total ratio.
        n_active: Number of active capsules.
        has_new_papers: Whether new papers were found in last check.

    Returns:
        SchedulerDecision with interval and recommended action.
    """
    if saturation >= HIGH_SATURATION:
        interval = min(base_interval_minutes * SLOW_FACTOR, DAEMON_INTERVAL_MAX)
        reason = f"HIGH saturation ({saturation:.0%}) — backing off"
        action = "check"
    elif saturation < EMPTY_SATURATION:
        interval = DAEMON_INTERVAL_MIN
        reason = f"GenePool EMPTY ({saturation:.0%}) — maximum frequency"
        action = "cold_start"
    elif saturation < LOW_SATURATION:
        interval = max(base_interval_minutes * FAST_FACTOR, DAEMON_INTERVAL_MIN)
        reason = f"LOW saturation ({saturation:.0%}) — accelerating"
        action = "cold_start" if n_active < 10 else "check"
    else:
        interval = base_interval_minutes
        reason = f"Normal ({saturation:.0%})"
        action = "check"

    return SchedulerDecision(
        interval_minutes=interval,
        reason=reason,
        sat_before=saturation,
        action=action,
    )


def run_cold_start_research(db, topic: str | None = None) -> dict:
    """Trigger cold-start research to refill GenePool.

    Uses the most recent subscription or a default topic.
    """
    from research_loop.orchestrator import Orchestrator

    # Pick topic: use most-recently-active subscription
    if topic is None:
        subs = db.list_arxiv_subscriptions(limit=10)
        if subs:
            # Use the one with most papers or most recent
            topic = subs[0].get("topic") if hasattr(subs[0], "get") else getattr(subs[0], "topic", None)
        if not topic:
            topic = "machine learning research frontier"

    print(f"[Scheduler] Cold-start research triggered for: {topic}")

    orch = Orchestrator()
    result = orch.run_deep_research(topic, papers=[])
    gaps = result.get("gaps", [])
    print(f"[Scheduler] Cold-start complete: {len(gaps)} gaps filled")
    return result
