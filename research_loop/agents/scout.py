"""Paper Scout Agent — monitors arXiv and discovers new papers."""

from __future__ import annotations

import time
from typing import List
from research_loop.agents.base import BaseAgent, AgentMessage, AgentStatus


TOPIC_PAPER_DISCOVERED = "paper.discovered"
TOPIC_SUBSCRIPTION_CHECK = "subscription.check"


class ScoutAgent(BaseAgent):
    """Scout Agent watches arXiv subscriptions and publishes newly discovered papers.

    It subscribes to `subscription.check` (triggers a scan) and publishes
    `paper.discovered` for each new paper found.
    """

    def __init__(self, bus=None):
        super().__init__(
            name="scout",
            topics=[TOPIC_SUBSCRIPTION_CHECK],
            bus=bus,
        )
        self._db = None
        self._monitor = None

    def _ensure_components(self):
        if self._monitor is not None:
            return
        try:
            from llm.subscription_monitor import SubscriptionMonitor
            from llm.subscription_scorer import SubscriptionScorer
            from db.database import Database

            db = Database()
            db.init()
            self._db = db
            self._monitor = SubscriptionMonitor(db, SubscriptionScorer(db))
        except Exception as e:
            self._log("init_failed", error=str(e))

    def think(self, msg: AgentMessage) -> List[AgentMessage]:
        if msg.topic != TOPIC_SUBSCRIPTION_CHECK:
            return []

        self.status = AgentStatus.WORKING
        self._log("scanning_subscriptions")
        self._ensure_components()

        if self._monitor is None:
            self.status = AgentStatus.ERROR
            return []

        try:
            results = self._monitor.check_all()
        except Exception as e:
            self._log("scan_error", error=str(e))
            self.status = AgentStatus.ERROR
            return []

        discovered: List[AgentMessage] = []
        for sub_id, papers in results.items():
            if not papers:
                continue
            self._log("found_papers", subscription=sub_id, count=len(papers))
            for paper in papers:
                discovered.append(
                    AgentMessage(
                        id="",
                        topic=TOPIC_PAPER_DISCOVERED,
                        sender=self.name,
                        payload={
                            "subscription_id": sub_id,
                            "arxiv_id": paper.get("arxiv_id", ""),
                            "title": paper.get("title", ""),
                            "abstract": paper.get("abstract", ""),
                            "authors": paper.get("authors", []),
                            "published": paper.get("published", ""),
                            "pdf_url": paper.get("pdf_url", ""),
                            "categories": paper.get("categories", ""),
                            "discovered_at": time.time(),
                        },
                        timestamp=time.time(),
                    )
                )

        self.status = AgentStatus.DONE
        return discovered
