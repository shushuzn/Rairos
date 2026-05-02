"""Gene Pool Curator Agent — scores gaps and encodes them into the Gene Pool."""

from __future__ import annotations

import time
from typing import List
from research_loop.agents.base import BaseAgent, AgentMessage, AgentStatus


TOPIC_GAP_FOUND = "gap.found"
TOPIC_ALERT_READY = "alert.ready"


class CuratorAgent(BaseAgent):
    """Curator Agent scores gaps against the Gene Pool and generates alerts.

    It subscribes to `gap.found`, looks up matching capsules in the Gene Pool,
    computes a relevance score, and publishes `alert.ready` when a gap is
    high-scoring enough to warrant notification.
    """

    SEVERITY_RANK = {"HIGH": 0, "MEDIUM": 1, "LOW": 2}
    MIN_GAP_SEVERITY = "MEDIUM"
    MIN_GENE_POOL_SCORE = 0.3

    def __init__(self, bus=None):
        super().__init__(
            name="curator",
            topics=[TOPIC_GAP_FOUND],
            bus=bus,
        )
        self._tracker = None

    def _ensure_components(self):
        if self._tracker is not None:
            return
        try:
            from llm.insight.tracker import EvolutionTracker
            self._tracker = EvolutionTracker()
        except Exception as e:
            self._log("init_failed", error=str(e))

    def think(self, msg: AgentMessage) -> List[AgentMessage]:
        if msg.topic != TOPIC_GAP_FOUND:
            return []

        self.status = AgentStatus.WORKING
        self._ensure_components()

        gap = msg.payload
        topic = gap.get("topic", "")
        gap_type = gap.get("gap_type", "")
        gap_title = gap.get("gap_title", "")
        severity = gap.get("severity", "MEDIUM")

        self._log("scoring_gap", gap=gap_title[:40])

        if self._tracker is None:
            self.status = AgentStatus.ERROR
            return []

        # Find matching Gene Pool capsule
        capsule = None
        try:
            capsule = self._tracker.find_capsule(
                topic=topic,
                gap_type=gap_type,
                keywords=[],
            )
        except Exception as e:
            self._log("tracker_error", error=str(e))

        gene_pool_score = 0.0
        preference_boost = False
        if capsule:
            gene_pool_score = capsule.get("outcome_success_score", 0.0)
            preference_boost = gene_pool_score >= 0.5

        # Filter by minimum thresholds
        sev_rank = self.SEVERITY_RANK.get(severity, 2)
        min_sev_rank = self.SEVERITY_RANK.get(self.MIN_GAP_SEVERITY, 1)
        if sev_rank > min_sev_rank:
            self._log("filtered_severity", gap=gap_title[:40], severity=severity)
            self.status = AgentStatus.DONE
            return []

        if gene_pool_score < self.MIN_GENE_POOL_SCORE:
            self._log("filtered_score", gap=gap_title[:40], score=gene_pool_score)
            self.status = AgentStatus.DONE
            return []

        # Encode into Gene Pool
        try:
            self._tracker.record_gap_accept(
                topic=topic,
                gap_type=gap_type,
                gap_title=gap_title,
                gap_description=gap.get("description", ""),
            )
            self._log("encoded_to_gene_pool", gap=gap_title[:40])
        except Exception as e:
            self._log("encode_error", error=str(e))

        self._log("alert_ready", gap=gap_title[:40], score=gene_pool_score)
        self.status = AgentStatus.DONE

        return [
            AgentMessage(
                id="",
                topic=TOPIC_ALERT_READY,
                sender=self.name,
                payload={
                    "gap_type": gap_type,
                    "gap_title": gap_title,
                    "description": gap.get("description", ""),
                    "severity": severity,
                    "gene_pool_score": gene_pool_score,
                    "preference_boost": preference_boost,
                    "triggered_by": gap.get("triggered_by", ""),
                    "trigger_title": gap.get("trigger_title", ""),
                    "topic": topic,
                    "gaps_found": 1,
                },
                timestamp=time.time(),
            )
        ]
