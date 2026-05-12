"""Research Gap Impact Tracker — quantifies how identified gaps are resolved over time.

Tracks the lifecycle: gap identified → papers address it → resolved/ceded.

Tables
──────
gap_impact_events:
  gap_hash       TEXT    NOT NULL   — key into gap_history
  paper_id       TEXT    NOT NULL   — arxiv_id or generated ID
  paper_title    TEXT              — for human readability
  event_type     TEXT    NOT NULL   — 'addresses' | 'partially_addresses' | 'contradicts'
  confidence     REAL              — 0.0–1.0 how well this paper addresses the gap
  addressed_at   TEXT    NOT NULL   — ISO timestamp

gap_impact_summary (materialized per gap):
  gap_hash         TEXT  PK
  topic            TEXT
  gap_type         TEXT
  first_identified TEXT
  first_addressed  TEXT              — when first paper addressed it
  resolved_at       TEXT              — when resolution_confidence reached threshold
  resolution_type  TEXT              — 'fully_resolved' | 'partially_resolved' | 'ceded' | NULL
  num_addressing   INTEGER           — count of addressing events
  resolution_confidence REAL 0.0–1.0
  impact_score     REAL 0.0–1.0

Resolution logic
────────────────
  - A gap is "resolved" when resolution_confidence >= RESOLVE_THRESHOLD (0.75)
  - resolution_confidence = 1 - prod(1 - confidence_i) for all addressing events i
    (i.e., Bayesian updating with uniform prior)
  - A gap is "ceded" when another gap with higher novelty supersedes it
  - A gap is "contradicted" when an addressing paper refutes the gap's premise
"""

from __future__ import annotations

import math
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path
from typing import Any, Dict, List, Optional

GP_DIR = Path.home() / ".ai_research_os" / "evolution"
IMPACT_EVENTS_FILE = GP_DIR / "gap_impact_events.jsonl"
IMPACT_SUMMARY_FILE = GP_DIR / "gap_impact_summary.json"

RESOLVE_THRESHOLD = 0.75
MIN_ADDRESSING_CONFIDENCE = 0.30


# ─── Dataclasses ─────────────────────────────────────────────────────────────


@dataclass
class GapImpactEvent:
    """A single event where a paper addresses (or contradicts) a gap."""

    gap_hash: str
    paper_id: str
    paper_title: str
    event_type: str  # 'addresses' | 'partially_addresses' | 'contradicts'
    confidence: float  # 0.0–1.0
    addressed_at: str  # ISO timestamp

    def to_dict(self) -> Dict[str, Any]:
        return {
            "gap_hash": self.gap_hash,
            "paper_id": self.paper_id,
            "paper_title": self.paper_title,
            "event_type": self.event_type,
            "confidence": self.confidence,
            "addressed_at": self.addressed_at,
        }


@dataclass
class GapImpactSummary:
    """Aggregated impact metrics for a single gap."""

    gap_hash: str
    topic: str
    gap_type: str
    first_identified: str  # ISO timestamp when gap was first recorded
    first_addressed: str = ""  # ISO timestamp
    resolved_at: str = ""
    resolution_type: str = (
        ""  # 'fully_resolved' | 'partially_resolved' | 'ceded' | 'contradicted' | ''
    )
    num_addressing: int = 0
    resolution_confidence: float = 0.0
    impact_score: float = 0.0
    last_updated: str = ""


# ─── Helpers ─────────────────────────────────────────────────────────────────


def _now_iso() -> str:
    return datetime.now().isoformat()


def _load_impact_events() -> List[GapImpactEvent]:
    if not IMPACT_EVENTS_FILE.exists():
        return []
    events = []
    try:
        import json

        with open(IMPACT_EVENTS_FILE, "r", encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                d = json.loads(line)
                events.append(GapImpactEvent(**d))
    except Exception:
        pass
    return events


def _append_event(event: GapImpactEvent) -> None:
    GP_DIR.mkdir(parents=True, exist_ok=True)
    import json

    with open(IMPACT_EVENTS_FILE, "a", encoding="utf-8") as f:
        f.write(json.dumps(event.to_dict(), ensure_ascii=False) + "\n")


def _load_summaries() -> Dict[str, GapImpactSummary]:
    if not IMPACT_SUMMARY_FILE.exists():
        return {}
    try:
        import json

        data = json.loads(IMPACT_SUMMARY_FILE.read_text(encoding="utf-8"))
        return {k: GapImpactSummary(**v) for k, v in data.items()}
    except Exception:
        return {}


def _save_summaries(summaries: Dict[str, GapImpactSummary]) -> None:
    GP_DIR.mkdir(parents=True, exist_ok=True)
    import json

    IMPACT_SUMMARY_FILE.write_text(
        json.dumps({k: v.__dict__ for k, v in summaries.items()}, ensure_ascii=False),
        encoding="utf-8",
    )


def _compute_resolution_confidence(events: List[GapImpactEvent]) -> float:
    """Bayesian resolution confidence: 1 - prod(1 - c_i) for all addressing events."""
    if not events:
        return 0.0
    p = 1.0
    for e in events:
        if e.event_type == "contradicts":
            # Contradiction resets confidence to 0
            return 0.0
        p *= 1.0 - e.confidence
    return 1.0 - p


def _compute_impact_score(
    novelty: float,
    num_addressing: int,
    resolution_confidence: float,
    days_to_first_addressing: float,
) -> float:
    """Composite impact score for a gap.

    Weights:
      - novelty: how original the gap was (30%)
      - resolution_depth: how many papers addressed it (20%)
      - resolution_strength: confidence that it's resolved (25%)
      - speed: how quickly it was addressed (25%)
    """
    novelty_weight = 0.30
    depth_weight = 0.20
    strength_weight = 0.25
    speed_weight = 0.25

    depth_score = min(1.0, num_addressing / 5.0)  # 5+ papers = full depth
    speed_score = math.exp(-0.01 * max(0, days_to_first_addressing))  # ~69-day half-life

    return round(
        novelty * novelty_weight
        + depth_score * depth_weight
        + resolution_confidence * strength_weight
        + speed_score * speed_weight,
        4,
    )


# ─── Main API ────────────────────────────────────────────────────────────────


def record_addressing_event(
    gap_hash: str,
    topic: str,
    gap_type: str,
    paper_id: str,
    paper_title: str,
    confidence: float,
    event_type: str = "addresses",
    first_identified: Optional[str] = None,
) -> Dict[str, Any]:
    """Record that a paper addressed (or contradicted) a known gap.

    This is called from paper ingestion or gap extraction pipelines.
    Returns the updated impact summary for this gap.
    """
    if confidence < MIN_ADDRESSING_CONFIDENCE and event_type in (
        "addresses",
        "partially_addresses",
    ):
        return {"skipped": True, "reason": f"confidence {confidence} below minimum"}

    event = GapImpactEvent(
        gap_hash=gap_hash,
        paper_id=paper_id,
        paper_title=paper_title,
        event_type=event_type,
        confidence=confidence,
        addressed_at=_now_iso(),
    )
    _append_event(event)

    # Reload all events for this gap and recompute summary
    all_events = _load_impact_events()
    gap_events = [e for e in all_events if e.gap_hash == gap_hash]

    summaries = _load_summaries()
    existing = summaries.get(gap_hash)

    first_identified_iso = first_identified or (
        existing.first_identified if existing else _now_iso()
    )

    # Compute resolution confidence
    resolution_conf = _compute_resolution_confidence(gap_events)

    # Determine resolution type
    resolution_type = ""
    resolved_at = ""
    if resolution_conf >= RESOLVE_THRESHOLD:
        resolution_type = "fully_resolved"
        resolved_at = _now_iso()
    elif any(e.event_type == "contradicts" for e in gap_events):
        resolution_type = "contradicted"
        resolved_at = _now_iso()
    elif existing and existing.resolution_type in ("ceded", "contradicted"):
        resolution_type = existing.resolution_type

    num_addressing = sum(1 for e in gap_events if e.event_type != "contradicts")

    # Days from first identified to first addressing
    try:
        first_id_time = datetime.fromisoformat(first_identified_iso)
        first_addr_time = datetime.fromisoformat(min(e.addressed_at for e in gap_events))
        days_to_first = (first_addr_time - first_id_time).total_seconds() / 86400.0
    except Exception:
        days_to_first = 0.0

    novelty = float(existing.impact_score) if existing else 0.5
    impact_score = _compute_impact_score(
        novelty=novelty,
        num_addressing=num_addressing,
        resolution_confidence=resolution_conf,
        days_to_first_addressing=days_to_first,
    )

    summary = GapImpactSummary(
        gap_hash=gap_hash,
        topic=topic,
        gap_type=gap_type,
        first_identified=first_identified_iso,
        first_addressed=min(e.addressed_at for e in gap_events) if gap_events else "",
        resolved_at=resolved_at,
        resolution_type=resolution_type,
        num_addressing=num_addressing,
        resolution_confidence=round(resolution_conf, 4),
        impact_score=impact_score,
        last_updated=_now_iso(),
    )
    summaries[gap_hash] = summary
    _save_summaries(summaries)

    return {
        "gap_hash": gap_hash,
        "num_addressing": num_addressing,
        "resolution_confidence": resolution_conf,
        "resolution_type": resolution_type,
        "impact_score": impact_score,
    }


def mark_gap_ceded(gap_hash: str, superseded_by_hash: str) -> bool:
    """Mark a gap as ceded — superseded by another higher-novelty gap."""
    summaries = _load_summaries()
    if gap_hash not in summaries:
        return False
    summaries[gap_hash].resolution_type = "ceded"
    summaries[gap_hash].resolved_at = _now_iso()
    summaries[gap_hash].last_updated = _now_iso()
    _save_summaries(summaries)
    return True


def get_gap_impact(gap_hash: str) -> Optional[GapImpactSummary]:
    """Return impact summary for a specific gap."""
    summaries = _load_summaries()
    return summaries.get(gap_hash)


def get_all_impacts(
    topic: Optional[str] = None,
    resolution_type: Optional[str] = None,
    min_impact_score: float = 0.0,
    limit: int = 50,
) -> List[GapImpactSummary]:
    """Return all gap impacts, optionally filtered."""
    summaries = _load_summaries()
    results = []
    for s in summaries.values():
        if topic and s.topic != topic:
            continue
        if resolution_type and s.resolution_type != resolution_type:
            continue
        if s.impact_score < min_impact_score:
            continue
        results.append(s)

    results.sort(key=lambda x: x.impact_score, reverse=True)
    return results[:limit]


def get_top_gaps_by_impact(limit: int = 20) -> List[Dict[str, Any]]:
    """Return top gaps ranked by impact_score — for MCP query."""
    all_impacts = get_all_impacts(min_impact_score=0.0, limit=limit)
    return [
        {
            "gap_hash": s.gap_hash,
            "topic": s.topic,
            "gap_type": s.gap_type,
            "impact_score": s.impact_score,
            "resolution_type": s.resolution_type or "open",
            "num_addressing": s.num_addressing,
            "resolution_confidence": s.resolution_confidence,
            "days_open": _days_open(s),
        }
        for s in all_impacts
    ]


def _days_open(summary: GapImpactSummary) -> float:
    end = summary.resolved_at or _now_iso()
    try:
        start = datetime.fromisoformat(summary.first_identified)
        end_dt = datetime.fromisoformat(end)
        return round((end_dt - start).total_seconds() / 86400.0, 1)
    except Exception:
        return 0.0
