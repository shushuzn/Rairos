"""Deep Research Agent — iterative research with gap detection and archetype-aware refinement.



Architecture inspired by:

- gpt-researcher: multi-agent research with planning

- deer-flow: sandbox + memory + tool use

- snapstate: session persistence for pause/resume

"""

from __future__ import annotations


import asyncio

import time

from dataclasses import dataclass, field

from pathlib import Path

from typing import Any, Dict, List, Optional, Tuple


from llm.gap_analyzer import GapAnalyzerV2

from llm.insight_evolution import get_evolution_tracker

from db.database import Database

from research_loop.snapstate import (
    Snapstate,
    ResearchSession,
    PaperSnapshot,
    GapSnapshot,
)

from research_loop.core import search_arxiv, extract_pdf_text, Paper


@dataclass
class AgentThought:
    """A single reasoning step in the agent loop."""

    iteration: int

    role: str  # "planner" | "searcher" | "analyzer" | "reflector"

    content: str

    timestamp: float = field(default_factory=time.time)


@dataclass
class DeepResearchResult:
    """Final result of a deep research agent run."""

    session_id: str

    query: str

    iterations: int

    papers: List[PaperSnapshot]

    gaps: List[GapSnapshot]

    thoughts: List[AgentThought]

    report: str

    duration_seconds: float

    status: str  # completed | paused | failed


class DeepResearchAgent:
    """Deep Research Agent with iterative gap-aware refinement loop.



    Loop:

        1. PLANNER: decide search strategy based on gaps found

        2. SEARCHER: fetch papers from arXiv

        3. EXTRACTOR: pull abstracts/full text

        4. ANALYZER: detect gaps via GapAnalyzerV2

        5. REFLECTOR: assess progress, decide to iterate or stop

        6. GENETIC: encode accepted gaps into Gene Pool

    """

    def __init__(
        self,
        query: str,
        max_iterations: int = 3,
        max_papers_per_iteration: int = 5,
        verbose: bool = False,
        snapstate_dir: Optional[Path] = None,
    ):

        self.query = query

        self.max_iterations = max_iterations

        self.max_papers_per_iteration = max_papers_per_iteration

        self.verbose = verbose

        self.snapstate = Snapstate(base_dir=snapstate_dir)

        self.tracker = get_evolution_tracker()

        self.gap_analyzer = GapAnalyzerV2()

        self.db = Database()

        self.session: Optional[ResearchSession] = None

        self.thoughts: List[AgentThought] = []

        self._stop_requested = False

    def _log(self, msg: str):

        if self.verbose:
            print(f"[DeepResearchAgent] {msg}")

    # -------------------------------------------------------------------------

    # Session lifecycle

    # -------------------------------------------------------------------------

    def start(self) -> ResearchSession:
        """Start a new research session."""

        self.session = self.snapstate.new_session(
            query=self.query,
            max_iterations=self.max_iterations,
        )

        self.snapstate.save(self.session)

        self._log(f"Session started: {self.session.session_id}")

        return self.session

    def resume(self, session_id: str) -> Optional[ResearchSession]:
        """Resume an existing session."""

        self.session = self.snapstate.load(session_id)

        if self.session:
            self._log(f"Session resumed: {session_id}, iteration {self.session.iteration}")

        return self.session

    def pause(self):
        """Pause and persist current session state."""

        if self.session:
            self.session.status = "paused"

            self.snapstate.save(self.session)

            self._log(f"Session paused at iteration {self.session.iteration}")

    def _record_thought(self, role: str, content: str, iteration: int):

        thought = AgentThought(
            iteration=iteration,
            role=role,
            content=content,
        )

        self.thoughts.append(thought)

        if self.session:
            # keep thoughts in session for resume

            self.session.findings.append(f"[{role.upper()}] {content}")

    # -------------------------------------------------------------------------

    # Core iteration steps

    # -------------------------------------------------------------------------

    def _get_search_guidance(
        self, topic: str, gap_type: str, gap_title: str
    ) -> Tuple[Optional[str], float]:
        """Query GenePool for successful search strategies matching this context.

        Returns: (search_hint, confidence) — hint is a suggested search query string,
                 confidence is 0.0–1.0 based on historical outcome_success_score.
        """
        try:
            capsules = self.tracker.find_capsule(
                topic=topic,
                gap_type=gap_type,
                keywords=[],
                min_score=0.1,
            )
            if not capsules:
                return None, 0.0

            # Pick the best-scoring capsule
            best = capsules[0]
            confidence = best.outcome_success_score

            # Use the best capsule's trigger_keywords as search hint
            # (these encode what search terms previously succeeded)
            hint_keywords = best.trigger_keywords
            if hint_keywords and isinstance(hint_keywords, list) and len(hint_keywords) > 0:
                # Construct search hint from historical keywords + gap context
                hint = " ".join(str(k) for k in hint_keywords[:5])
                return hint, confidence
            return None, 0.0
        except Exception:
            return None, 0.0

    def _plan_next_search(self, iteration: int) -> str:
        """PLANNER: decide next search query based on session state + GenePool history."""
        gaps = self.session.gaps if self.session else []
        search_history = self.session.search_history if self.session else []

        if iteration == 0:
            planned = self.query
        elif gaps:
            latest_gap = gaps[-1] if gaps else None
            if latest_gap:
                # Ask GenePool for successful search strategies on this gap type/topic
                hint, confidence = self._get_search_guidance(
                    topic=self.query,
                    gap_type=latest_gap.gap_type,
                    gap_title=latest_gap.title,
                )

                if hint and confidence >= 0.3:
                    # GenePool has a successful pattern — incorporate it
                    planned = f"{hint} {latest_gap.title}"
                    self._record_thought(
                        "planner",
                        f"GenePool-guided search (confidence={confidence:.2f}): {planned}",
                        iteration,
                    )
                elif latest_gap.gap_type == "Contradiction":
                    planned = f"{self.query} {latest_gap.title} disagreement"
                else:
                    planned = f"{self.query} {latest_gap.title} improvement"
            else:
                planned = self.query
        else:
            planned = self.query

        # Avoid duplicate searches
        if planned in search_history:
            planned = f"{self.query} {iteration}"

        self._record_thought("planner", f"Planned search: {planned}", iteration)
        return planned

    def _search_papers(self, search_query: str, iteration: int) -> List[Paper]:
        """SEARCHER: fetch papers from arXiv."""

        self._log(f"[iter {iteration}] Searching: {search_query}")

        try:
            papers = search_arxiv(search_query, max_results=self.max_papers_per_iteration)

        except Exception as e:
            self._record_thought("searcher", f"Search failed: {e}", iteration)

            return []

        self._record_thought(
            "searcher", f"Found {len(papers)} papers: {[p.arxiv_id for p in papers]}", iteration
        )

        return papers

    def _extract_papers(self, papers: List[Paper], iteration: int) -> List[PaperSnapshot]:
        """EXTRACTOR: extract text from papers and build snapshots."""

        snapshots = []

        for paper in papers:
            try:
                extracted = extract_pdf_text(str(paper.pdf_url or "")) if paper.pdf_url else ""

            except Exception:
                extracted = ""

            snapshot = PaperSnapshot(
                arxiv_id=paper.arxiv_id or paper.title[:20],
                title=paper.title,
                abstract=paper.abstract or "",
                url=paper.pdf_url or "",
                extracted_text=extracted[:5000],
            )

            snapshots.append(snapshot)

            # Store in DB

            self.db.add_papers(
                [
                    {
                        "arxiv_id": paper.arxiv_id,
                        "title": paper.title,
                        "abstract": paper.abstract,
                        "pdf_url": paper.pdf_url,
                        "authors": str(paper.authors) if paper.authors else "",
                        "published": str(paper.published) if paper.published else "",
                        "source": "arxiv",
                    }
                ]
            )

        self._record_thought("extractor", f"Extracted {len(snapshots)} papers", iteration)

        return snapshots

    def _analyze_gaps(self, snapshots: List[PaperSnapshot], iteration: int) -> List[GapSnapshot]:
        """ANALYZER: detect research gaps using GapAnalyzerV2."""

        self._log(f"[iter {iteration}] Analyzing gaps for {len(snapshots)} papers")

        topic = self.query

        try:
            result = self.gap_analyzer.analyze(
                topic=topic,
                use_insights=True,
                min_papers=3,
                use_llm=False,  # Use rules-based for speed in agent loop
            )

        except Exception as e:
            self._record_thought("analyzer", f"Gap analysis failed: {e}", iteration)

            return []

        archetype = {}

        if self.session:
            archetype = self.session.archetype

        gap_snapshots = []

        for gap in result.gaps[:5]:  # Top 5 gaps
            match_score = self.tracker._archetype_match_score(gap, archetype) if archetype else 0.5

            gs = GapSnapshot(
                gap_type=gap.gap_type or "improvement",
                title=gap.title,
                description=gap.description or "",
                matched_papers=[s.arxiv_id for s in snapshots],
                archetype_match=match_score,
            )

            gap_snapshots.append(gs)

        self._record_thought(
            "analyzer",
            f"Found {len(gap_snapshots)} gaps: {[g.gap_type for g in gap_snapshots]}",
            iteration,
        )

        return gap_snapshots

    def _reflect(self, iteration: int) -> Tuple[bool, str]:
        """REFLECTOR: decide whether to continue iterating or stop.



        Returns: (should_continue, reason)

        """

        if self.session is None:
            return False, "no session"

        gaps = self.session.gaps

        papers = self.session.papers

        # Stop conditions

        if iteration >= self.max_iterations:
            return False, f"max iterations ({self.max_iterations}) reached"

        if len(papers) >= self.max_iterations * self.max_papers_per_iteration:
            return False, "max papers reached"

        if not gaps and iteration > 1:
            return False, "no gaps found after thorough search"

        # Continue conditions

        recent_gaps = [g for g in gaps if g.accepted]

        if recent_gaps:
            return False, f"{len(recent_gaps)} gaps accepted, stopping"

        # Check archetype alignment

        if gaps:
            avg_match = sum(g.archetype_match for g in gaps) / len(gaps)

            if avg_match < 0.3 and iteration >= 2:
                self._record_thought(
                    "reflector",
                    f"Low archetype match ({avg_match:.2f}), broadening search",
                    iteration,
                )

        return True, "continue iterating"

    def _encode_accepted_gaps(self):
        """GENETIC: encode all accepted gaps into the Gene Pool."""

        if self.session is None:
            return

        for gap in self.session.gaps:
            if gap.accepted and gap.archetype_match > 0:
                self.tracker.record_gap_accept(
                    topic=self.query,
                    gap_type=gap.gap_type,
                    gap_title=gap.title,
                    gap_description=gap.description,
                )

                self._log(f"Encoded gap into Gene Pool: {gap.title}")

    # -------------------------------------------------------------------------

    # Main run loop

    # -------------------------------------------------------------------------

    def run(self) -> DeepResearchResult:
        """Run the deep research agent synchronously."""

        if self.session is None:
            self.start()

        assert self.session is not None

        start_time = time.time()

        iteration = self.session.iteration

        self._log(f"Starting run from iteration {iteration}")

        while iteration < self.max_iterations and not self._stop_requested:
            self.session.iteration = iteration

            self.snapstate.save(self.session)

            # Step 1: Plan

            search_query = self._plan_next_search(iteration)

            # Step 2: Search

            papers = self._search_papers(search_query, iteration)

            if not papers:
                self._log("No papers found, trying alternative query")

                papers = self._search_papers(self.query, iteration)

            if self.session:
                self.session.search_history.append(search_query)

            if not papers:
                iteration += 1

                continue

            # Step 3: Extract

            snapshots = self._extract_papers(papers, iteration)

            if self.session:
                self.session.papers.extend(snapshots)

            # Step 4: Analyze gaps

            gap_snapshots = self._analyze_gaps(snapshots, iteration)

            if self.session:
                self.session.gaps.extend(gap_snapshots)

            # Step 5: Reflect

            should_continue, reason = self._reflect(iteration)

            self._record_thought("reflector", reason, iteration)

            self._log(f"[iter {iteration}] Reflect: {reason}")

            if not should_continue:
                break

            iteration += 1

            self.snapstate.save(self.session)

        # Encode accepted gaps into Gene Pool

        self._encode_accepted_gaps()

        # Finalize session

        if self.session:
            self.session.status = "completed" if not self._stop_requested else "paused"

            self.session.iteration = iteration

            self.snapstate.save(self.session)

        duration = time.time() - start_time

        result = DeepResearchResult(
            session_id=self.session.session_id if self.session else "",
            query=self.query,
            iterations=iteration + 1,
            papers=self.session.papers if self.session else [],
            gaps=self.session.gaps if self.session else [],
            thoughts=self.thoughts,
            report=self._build_report(),
            duration_seconds=duration,
            status=self.session.status if self.session else "failed",
        )

        self._log(f"Run complete: {result.status}, {result.iterations} iterations, {duration:.1f}s")

        return result

    def _build_report(self) -> str:
        """Build a markdown report from the research session."""

        if self.session is None:
            return "No session"

        s = self.session

        lines = [
            f"# Deep Research Report: {s.query}",
            "",
            f"**Session**: {s.session_id} | **Iterations**: {s.iteration} | **Duration**: {s.duration():.1f}s",
            f"**Status**: {s.status}",
            "",
            "## Papers Analyzed",
        ]

        for p in s.papers:
            lines.append(f"- [{p.arxiv_id}] {p.title} — {p.gaps_found} gaps")

        lines.append("")

        lines.append("## Research Gaps")

        for g in s.gaps:
            status = "✅" if g.accepted else "⬜"

            lines.append(f"- {status} [{g.gap_type}] {g.title}")

            lines.append(f"  {g.description[:100]}")

        lines.append("")

        lines.append("## Findings")

        for f in s.findings[-10:]:
            lines.append(f"- {f}")

        return "\n".join(lines)

    def stop(self):
        """Request the agent to stop at next reflection point."""

        self._stop_requested = True

        if self.session:
            self.session.status = "paused"

            self.snapstate.save(self.session)
