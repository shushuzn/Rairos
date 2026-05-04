"""CapsuleGene dataclass — encoded research action patterns."""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Dict, List


@dataclass
class CapsuleGene:
    """A successful (trigger → action → outcome) pattern encoded from user behavior.


    This is the core unit of the self-evolving engine. When a gap is ACCEPTED,

    it represents a successful user choice in a specific context. That choice is

    encoded as a Capsule and stored in the Gene Pool for future retrieval.


    Trigger:   what context caused the user to act (topic, gap_type, keywords)

    Action:    what the user did (gap_type they accepted, keywords they matched)

    Outcome:   how it turned out (success_score, feedback_count)

    """

    capsule_id: str

    created_at: str

    trigger_topic: str

    trigger_gap_type: str

    trigger_keywords: List[str]

    action_gap_type: str

    action_gap_title: str

    outcome_success_score: float  # 0.0–1.0, from accepts vs rejects ratio

    feedback_count: int  # number of times this pattern was referenced

    evolved_generation: int = 0  # incremented when the capsule is refined
    archetype: Dict[str, Any] = field(default_factory=dict)  # research archetype at creation time

    # ─── Lifecycle status ──────────────────────────────────────────────────────
    # active:   in Gene Pool, shown in UI, eligible for matching/suggestions
    # consumed: suggestion was "taken on" — user acted on it, still visible as evidence
    # archived: low-quality / superseded / manually archived — hidden from active view
    status: str = "active"  # "active" | "consumed" | "archived"

    # ─── Credibility (computed by CredibilityScorer) ────────────────────
    credibility_score: float = 0.5  # 0.0–1.0, composite credibility
    trendslop: bool = False  # True if keyword overlap > 70% with existing capsules
    trendslop_reason: str = ""  # explanation of trendslop flag
    credibility_badge: str = "medium"  # "high" | "medium" | "low"

    # ─── Source Tracking ────────────────────────────────────────────────
    source_arxiv_category: str = ""  # e.g. "cs.LG", "cs.CL"

    # Auto-archive tracking: consecutive evolution cycles with score < 0.3
    low_score_streak: int = 0  # incremented each cycle if score < 0.3, reset if >= 0.3

    def trigger_match(self, topic: str, gap_type: str, keywords: List[str]) -> float:
        """Score how well this capsule matches a new context [0.0–1.0].


        Uses keyword overlap + gap_type alignment.

        """

        score = 0.0

        # Topic substring match

        if topic and self.trigger_topic:
            # Handle trigger_topic as str or list (legacy data)
            tt = (
                self.trigger_topic
                if isinstance(self.trigger_topic, str)
                else " ".join(self.trigger_topic)
                if self.trigger_topic
                else ""
            )
            if topic.lower() in tt.lower():
                score += 0.4

            elif tt.lower() in topic.lower():
                score += 0.3

        # Gap type exact match

        if gap_type and gap_type == self.trigger_gap_type:
            score += 0.4

        # Keyword overlap

        if keywords and self.trigger_keywords:
            overlap = set(k.lower() for k in keywords) & set(
                k.lower() for k in self.trigger_keywords
            )

            if overlap:
                score += 0.2 * (len(overlap) / max(len(keywords), len(self.trigger_keywords)))

        return min(score, 1.0)

    def to_dict(self) -> Dict[str, Any]:

        return {
            "capsule_id": self.capsule_id,
            "created_at": self.created_at,
            "trigger_topic": self.trigger_topic,
            "trigger_gap_type": self.trigger_gap_type,
            "trigger_keywords": self.trigger_keywords,
            "action_gap_type": self.action_gap_type,
            "action_gap_title": self.action_gap_title,
            "outcome_success_score": self.outcome_success_score,
            "feedback_count": self.feedback_count,
            "evolved_generation": self.evolved_generation,
            "archetype": self.archetype,
            "status": self.status,
            "low_score_streak": self.low_score_streak,
            "credibility_score": self.credibility_score,
            "trendslop": self.trendslop,
            "trendslop_reason": self.trendslop_reason,
            "credibility_badge": self.credibility_badge,
            "source_arxiv_category": self.source_arxiv_category,
        }

    @classmethod
    def from_dict(cls, d: Dict[str, Any]) -> CapsuleGene:

        return cls(
            capsule_id=d["capsule_id"],
            created_at=d["created_at"],
            trigger_topic=d.get("trigger_topic", ""),
            trigger_gap_type=d.get("trigger_gap_type", ""),
            trigger_keywords=d.get("trigger_keywords", []),
            action_gap_type=d.get("action_gap_type", ""),
            action_gap_title=d.get("action_gap_title", ""),
            outcome_success_score=d.get("outcome_success_score", 0.5),
            feedback_count=d.get("feedback_count", 0),
            evolved_generation=d.get("evolved_generation", 0),
            archetype=d.get("archetype", {}),
            status=d.get("status", "active"),
            low_score_streak=d.get("low_score_streak", 0),
            credibility_score=d.get("credibility_score", 0.5),
            trendslop=d.get("trendslop", False),
            trendslop_reason=d.get("trendslop_reason", ""),
            credibility_badge=d.get("credibility_badge", "medium"),
            source_arxiv_category=d.get("source_arxiv_category", ""),
        )
