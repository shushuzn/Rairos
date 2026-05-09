"""Parallel Deep Research Coordinator — multiple agents investigating different gap directions concurrently.

Architecture
────────────
Given a list of gap clusters from GapClusterer, this coordinator:

1. Groups agents by gap type / cluster
2. Launches N concurrent agent threads (max_concurrency agents in parallel)
3. Each agent runs DeepResearchAgent independently on its gap sub-direction
4. Results are merged: gaps deduplicated, papers merged, insights combined

This turns a sequential N-gap research run into a parallel O(1) wall-clock research pass.

Usage
─────
    coordinator = ParallelResearchCoordinator(
        max_concurrency=3,
        max_iterations_per_agent=2,
    )
    result = coordinator.run(
        topic="transformer efficiency",
        gap_clusters=[
            {"cluster_id": "c0", "gaps": [...], "gap_type": "method_limitation"},
            {"cluster_id": "c1", "gaps": [...], "gap_type": "scalability_issue"},
        ],
        existing_papers=[...],
    )
"""

from __future__ import annotations

import concurrent.futures
import threading
import time
import uuid
from collections import defaultdict
from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional

logger = __import__("logging").getLogger(__name__)


# ─── Dataclasses ─────────────────────────────────────────────────────────────


@dataclass
class AgentResult:
    """Result from a single parallel agent run."""

    agent_id: str
    cluster_id: str
    gaps: List[Any] = field(default_factory=list)
    papers_analyzed: int = 0
    iterations: int = 0
    insights: List[Any] = field(default_factory=list)
    error: Optional[str] = None
    duration_seconds: float = 0.0


@dataclass
class ParallelResearchResult:
    """Combined result from all parallel agents."""

    total_gaps: int
    unique_gaps: int
    total_papers_analyzed: int
    total_iterations: int
    agent_results: List[AgentResult]
    merged_insights: List[Any] = field(default_factory=list)
    duration_seconds: float = 0.0


# ─── Merge Logic ─────────────────────────────────────────────────────────────


def _gap_hash(gap: Any) -> str:
    """Compute a hash key for a gap to deduplicate across agents."""
    try:
        title = getattr(gap, "title", "") or getattr(gap, "gap_title", "") or ""
        gap_type = (
            getattr(gap, "gap_type", "") or ""
        )
        if hasattr(gap_type, "value"):
            gap_type = gap_type.value
        import hashlib
        return hashlib.sha256(f"{gap_type}:{title}".encode()).hexdigest()[:16]
    except Exception:
        return str(uuid.uuid4())


def _merge_gaps(all_results: List[AgentResult]) -> List[Any]:
    """Deduplicate gaps across all agent results.

    Uses title+gap_type hash to identify duplicates.
    Keeps the gap with highest novelty_score.
    """
    seen: Dict[str, Any] = {}
    seen_novelty: Dict[str, float] = {}

    for result in all_results:
        for gap in result.gaps:
            key = _gap_hash(gap)
            novelty = getattr(gap, "novelty_score", 0.0) or 0.0
            if key not in seen or novelty > seen_novelty[key]:
                seen[key] = gap
                seen_novelty[key] = novelty

    return list(seen.values())


def _merge_insights(all_results: List[AgentResult]) -> List[Any]:
    """Collect all insights from all agents, deduplicated by title."""
    seen_titles: set = set()
    merged: List[Any] = []

    for result in all_results:
        for insight in result.insights:
            title = getattr(insight, "title", "") or ""
            if title and title not in seen_titles:
                seen_titles.add(title)
                merged.append(insight)
            elif not title:
                merged.append(insight)

    return merged


# ─── Single Agent Runner ──────────────────────────────────────────────────────


def _run_single_agent(
    cluster_id: str,
    gap_sub_topic: str,
    initial_papers: List[Dict],
    max_iterations: int,
    agent_id: str,
    barrier: Optional[threading.Barrier],
) -> AgentResult:
    """Run one DeepResearchAgent on a single gap sub-direction.

    Called in a thread pool executor.
    barrier: optional threading.Barrier to synchronize agent starts.
    """
    start = time.time()
    try:
        if barrier:
            barrier.wait()  # Synchronize start

        from research_loop.orchestrator import Orchestrator

        orchestrator = Orchestrator()
        agent_result = orchestrator.run_deep_research(
            topic=gap_sub_topic,
            new_papers=initial_papers,
        )

        gaps = agent_result.get("gaps", [])
        papers_analyzed = agent_result.get("papers_analyzed", 0)
        iterations = agent_result.get("iterations", 0)

        # Collect insights from the agent's session
        insights: List[Any] = []
        try:
            from research_loop.core import Insight
            if gaps:
                insights = [Insight(title=str(g), summary="", sources=[], confidence=0.5) for g in gaps]
        except Exception:
            pass

        return AgentResult(
            agent_id=agent_id,
            cluster_id=cluster_id,
            gaps=gaps,
            papers_analyzed=papers_analyzed,
            iterations=iterations,
            insights=insights,
            error=None,
            duration_seconds=time.time() - start,
        )

    except Exception as e:
        logger.warning(f"Parallel agent {agent_id} failed: {e}")
        return AgentResult(
            agent_id=agent_id,
            cluster_id=cluster_id,
            gaps=[],
            papers_analyzed=0,
            iterations=0,
            insights=[],
            error=str(e),
            duration_seconds=time.time() - start,
        )


# ─── Main Coordinator ────────────────────────────────────────────────────────


class ParallelResearchCoordinator:
    """Coordinate multiple DeepResearchAgent instances for concurrent gap investigation.

    Usage:
        coordinator = ParallelResearchCoordinator(max_concurrency=3)
        result = coordinator.run(topic, gap_clusters, existing_papers)
    """

    def __init__(
        self,
        max_concurrency: int = 3,
        max_iterations_per_agent: int = 2,
        agent_timeout_seconds: int = 300,
    ):
        """
        Args:
            max_concurrency: max parallel agents (default 3 — avoid rate limiting)
            max_iterations_per_agent: iterations per agent (default 2)
            agent_timeout_seconds: kill agent if it exceeds this timeout
        """
        self.max_concurrency = max_concurrency
        self.max_iterations_per_agent = max_iterations_per_agent
        self.agent_timeout = agent_timeout_seconds

    def run(
        self,
        topic: str,
        gap_clusters: List[Dict[str, Any]],
        existing_papers: Optional[List[Dict[str, Any]]] = None,
    ) -> ParallelResearchResult:
        """Run parallel deep research across multiple gap clusters.

        Args:
            topic: overall research topic
            gap_clusters: list of dicts from GapClusterer, each with:
                - cluster_id: str
                - gaps: List[ResearchGapV2]
                - gap_type: str
                - keywords: List[str] (optional)
            existing_papers: papers already collected to pass to agents

        Returns:
            ParallelResearchResult with merged gaps, papers, insights
        """
        if not gap_clusters:
            return ParallelResearchResult(
                total_gaps=0,
                unique_gaps=0,
                total_papers_analyzed=0,
                total_iterations=0,
                agent_results=[],
            )

        existing_papers = existing_papers or []
        start_time = time.time()

        # ── Build sub-topics for each cluster ──────────────────────────────────
        agent_tasks: List[Dict[str, Any]] = []
        for cluster in gap_clusters:
            cluster_id = cluster.get("cluster_id", str(uuid.uuid4())[:8])
            gaps = cluster.get("gaps", [])
            gap_type = cluster.get("gap_type", "unknown")
            keywords = cluster.get("keywords", [])

            if not gaps:
                continue

            # Build a focused sub-topic from the gap cluster
            gap_titles = [getattr(g, "title", "") or getattr(g, "gap_title", "") or "" for g in gaps]
            sub_topic = f"{topic} — {gap_type}"
            if keywords:
                sub_topic += f": {', '.join(keywords[:3])}"
            else:
                sub_topic += f": {gap_titles[0][:80]}"

            agent_tasks.append({
                "cluster_id": cluster_id,
                "sub_topic": sub_topic,
                "gaps": gaps,
                "gap_type": gap_type,
                "initial_papers": existing_papers[:5],  # limit papers per agent
            })

        if not agent_tasks:
            return ParallelResearchResult(
                total_gaps=0,
                unique_gaps=0,
                total_papers_analyzed=0,
                total_iterations=0,
                agent_results=[],
            )

        # ── Run agents in parallel ─────────────────────────────────────────────
        results: List[AgentResult] = []
        thread_local = threading.local()

        def run_with_semaphore(task: Dict[str, Any], agent_id: str, barrier: threading.Barrier) -> AgentResult:
            return _run_single_agent(
                cluster_id=task["cluster_id"],
                gap_sub_topic=task["sub_topic"],
                initial_papers=task["initial_papers"],
                max_iterations=self.max_iterations_per_agent,
                agent_id=agent_id,
                barrier=barrier,
            )

        barrier = threading.Barrier(len(agent_tasks))
        semaphore = threading.Semaphore(self.max_concurrency)

        def throttled_run(task: Dict[str, Any], agent_id: str, barrier: threading.Barrier) -> AgentResult:
            with semaphore:
                return run_with_semaphore(task, agent_id, barrier)

        with concurrent.futures.ThreadPoolExecutor(max_workers=self.max_concurrency) as executor:
            futures = [
                executor.submit(throttled_run, task, f"agent_{i}", barrier)
                for i, task in enumerate(agent_tasks)
            ]

            for future in concurrent.futures.as_completed(futures, timeout=self.agent_timeout):
                try:
                    result = future.result(timeout=60)
                    results.append(result)
                except Exception as e:
                    logger.warning(f"Agent future failed: {e}")

        # ── Merge results ──────────────────────────────────────────────────────
        total_gaps = sum(len(r.gaps) for r in results)
        unique_gaps = _merge_gaps(results)
        merged_insights = _merge_insights(results)

        return ParallelResearchResult(
            total_gaps=total_gaps,
            unique_gaps=len(unique_gaps),
            total_papers_analyzed=sum(r.papers_analyzed for r in results),
            total_iterations=sum(r.iterations for r in results),
            agent_results=results,
            merged_insights=merged_insights,
            duration_seconds=time.time() - start_time,
        )

    def run_on_gap_clusters(
        self,
        topic: str,
        clusters: List[Any],
        existing_papers: Optional[List[Dict[str, Any]]] = None,
    ) -> ParallelResearchResult:
        """Convenience: run parallel research on GapClusterer clusters directly.

        Args:
            clusters: list of GapCluster namedtuples (cluster_id, gaps, gap_type, keywords, size)
        """
        gap_clusters = [
            {
                "cluster_id": c.cluster_id,
                "gaps": c.gaps,
                "gap_type": c.gap_type,
                "keywords": c.keywords if hasattr(c, "keywords") else [],
            }
            for c in clusters
        ]
        return self.run(topic, gap_clusters, existing_papers)
