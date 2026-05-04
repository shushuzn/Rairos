"""Citation Hunter Agent — traces citation chains for discovered papers."""

from __future__ import annotations

import time
from typing import List
from research_loop.agents.base import BaseAgent, AgentMessage, AgentStatus


TOPIC_PAPER_DISCOVERED = "paper.discovered"
TOPIC_CITATION_FOUND = "citation.found"


class CitationHunterAgent(BaseAgent):
    """Citation Hunter Agent traces the citation relationships of discovered papers.

    It subscribes to `paper.discovered` and publishes `citation.found` messages
    for each citation link detected.
    """

    def __init__(self, bus=None):
        super().__init__(
            name="citation_hunter",
            topics=[TOPIC_PAPER_DISCOVERED],
            bus=bus,
        )
        self._chain_builder = None

    def _ensure_components(self):
        if self._chain_builder is not None:
            return
        try:
            from llm.citation_chain import CitationChainBuilder

            self._chain_builder = CitationChainBuilder()
        except Exception as e:
            self._log("init_failed", error=str(e))

    def think(self, msg: AgentMessage) -> List[AgentMessage]:
        if msg.topic != TOPIC_PAPER_DISCOVERED:
            return []

        self.status = AgentStatus.WORKING
        self._ensure_components()

        paper = msg.payload
        arxiv_id = paper.get("arxiv_id", "")

        if not arxiv_id:
            self.status = AgentStatus.DONE
            return []

        self._log("tracing_citations", paper=arxiv_id)
        citations: List[AgentMessage] = []

        if self._chain_builder is None:
            self.status = AgentStatus.ERROR
            return []

        try:
            chain = self._chain_builder.build_chain(
                seed_arxiv_id=arxiv_id,
                max_depth=1,
            )
        except Exception as e:
            self._log("trace_error", paper=arxiv_id, error=str(e))
            self.status = AgentStatus.ERROR
            return []

        for edge in (chain.edges or [])[:10]:
            citations.append(
                AgentMessage(
                    id="",
                    topic=TOPIC_CITATION_FOUND,
                    sender=self.name,
                    payload={
                        "source_arxiv_id": edge.get("source", ""),
                        "target_arxiv_id": edge.get("target", ""),
                        "depth": 1,
                    },
                    timestamp=time.time(),
                )
            )

        self._log("citations_found", paper=arxiv_id, count=len(citations))
        self.status = AgentStatus.DONE
        return citations
