"""Gap Analyzer Agent — detects research gaps from paper batches."""

from __future__ import annotations

import time
from typing import List
from research_loop.agents.base import BaseAgent, AgentMessage, AgentStatus


TOPIC_PAPER_DISCOVERED = "paper.discovered"
TOPIC_GAP_FOUND = "gap.found"


class AnalyzerAgent(BaseAgent):
    """Gap Analyzer Agent takes discovered papers and extracts research gaps.

    It subscribes to `paper.discovered` and publishes `gap.found` for each gap
    detected using GapAnalyzerV2.
    """

    def __init__(self, bus=None):
        super().__init__(
            name="analyzer",
            topics=[TOPIC_PAPER_DISCOVERED],
            bus=bus,
        )
        self._analyzer = None

    def _ensure_components(self):
        if self._analyzer is not None:
            return
        try:
            from llm.research.gap_analyzer import GapAnalyzerV2

            self._analyzer = GapAnalyzerV2()
        except Exception as e:
            self._log("init_failed", error=str(e))

    def think(self, msg: AgentMessage) -> List[AgentMessage]:
        if msg.topic != TOPIC_PAPER_DISCOVERED:
            return []

        self.status = AgentStatus.WORKING
        self._ensure_components()

        paper = msg.payload
        title = paper.get("title", "")
        _abstract = paper.get("abstract", "") or ""
        arxiv_id = paper.get("arxiv_id", "")
        topic = paper.get("subscription_id", title[:50])

        self._log("analyzing", paper=arxiv_id)
        gaps_found: List[AgentMessage] = []

        if self._analyzer is None:
            self.status = AgentStatus.ERROR
            return []

        try:
            result = self._analyzer.analyze(
                topic=topic,
                use_insights=True,
                min_papers=1,
                use_llm=False,
            )
        except Exception as e:
            self._log("analyze_error", paper=arxiv_id, error=str(e))
            self.status = AgentStatus.ERROR
            return []

        for gap in (result.gaps or [])[:5]:
            gap_type_name = (
                gap.gap_type.value
                if hasattr(gap.gap_type, "value")
                else str(gap.gap_type or "improvement")
            )
            gaps_found.append(
                AgentMessage(
                    id="",
                    topic=TOPIC_GAP_FOUND,
                    sender=self.name,
                    payload={
                        "arxiv_id": arxiv_id,
                        "paper_title": title,
                        "gap_type": gap_type_name,
                        "gap_title": gap.title or "",
                        "description": gap.description or "",
                        "severity": gap.severity.value
                        if hasattr(gap.severity, "value")
                        else "MEDIUM",
                        "matched_papers": [arxiv_id],
                        "triggered_by": arxiv_id,
                        "trigger_title": title,
                        "topic": topic,
                    },
                    timestamp=time.time(),
                )
            )

        self._log("gaps_detected", paper=arxiv_id, count=len(gaps_found))
        self.status = AgentStatus.DONE
        return gaps_found
