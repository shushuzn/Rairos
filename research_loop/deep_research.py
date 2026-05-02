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
from research_loop.paper2code_integration import PaperPipeline
from llm.insight.gene import CapsuleGene
from llm.text_utils import extract_keywords


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
        4b. CODER: run PaperPipeline on high-signal papers (gap archetype_match > 0.5)
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
    # GenePoolGuide — query historically successful capsule patterns
    # -------------------------------------------------------------------------

    def _query_gene_pool(self, topic: str, gap_type: Optional[str] = None, keywords: Optional[List[str]] = None, top_k: int = 3) -> List[CapsuleGene]:
        """Query Gene Pool for historically successful capsule patterns matching topic/gap_type/keywords."""
        try:
            capsules = self.tracker.get_all_capsules()
        except Exception:
            return []

        if not capsules:
            return []

        scored = []
        for capsule in capsules:
            score = 0.0
            # keyword match
            if keywords:
                kw_lower = [k.lower() for k in keywords]
                capsule_kw = [k.lower() for k in capsule.trigger_keywords]
                overlap = sum(1 for kw in kw_lower if any(kw in ck or ck in kw for ck in capsule_kw))
                if overlap > 0:
                    score += overlap / len(kw_lower) * 0.5
            # archetype match
            archetype_kw = [k.lower() for k in capsule.trigger_keywords]
            topic_lower = topic.lower()
            if any(k in topic_lower for k in archetype_kw):
                score += 0.3
            # gap_type match
            if gap_type and capsule.archetype:
                if gap_type.lower() in capsule.archetype.lower():
                    score += 0.2
            scored.append((score, capsule))

        scored.sort(key=lambda x: x[0], reverse=True)
        return [c for _, c in scored[:top_k] if _ > 0]

    def _gene_pool_enhanced_query(self, base_query: str, capsules: List[CapsuleGene], iteration: int) -> str:
        """Build enhanced search query from historically successful capsule patterns."""
        if not capsules:
            return base_query

        parts = [base_query]
        for capsule in capsules[:2]:
            if capsule.title and capsule.trigger_keywords:
                kw_list = " ".join(capsule.trigger_keywords[:3])
                parts.append(f"| {kw_list}")

        enhanced = " ".join(parts)
        self._log(f"[iter {iteration}] GenePool enhanced query: {enhanced[:100]}")
        return enhanced

    def _score_gap_with_capsules(self, gap_type: str, gap_title: str, capsules: List[CapsuleGene]) -> float:
        """Score a gap based on historical capsule quality and keyword/type match."""
        if not capsules:
            return 0.0

        base_score = 0.5
        topic_lower = gap_title.lower()

        for capsule in capsules:
            quality = getattr(capsule, "quality_score", 1.0) if hasattr(capsule, "quality_score") else 1.0
            kw_match = sum(1 for kw in capsule.trigger_keywords if kw.lower() in topic_lower)
            type_match = 0.1 if gap_type and capsule.archetype and gap_type.lower() in capsule.archetype.lower() else 0.0
            boost = (kw_match * 0.1) + type_match + (quality * 0.05)
            base_score = min(1.0, base_score + boost)

        return base_score

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

    def _plan_next_search(self, iteration: int) -> str:
        """PLANNER: decide next search query based on session state, enhanced by GenePool patterns."""
        gaps = self.session.gaps if self.session else []
        search_history = self.session.search_history if self.session else []

        if iteration == 0:
            planned = self.query
            # GenePool-guided initial query
            capsules = self._query_gene_pool(self.query, keywords=self.query.split()[:5])
            if capsules:
                planned = self._gene_pool_enhanced_query(planned, capsules, iteration)
        elif gaps:
            latest_gap = gaps[-1] if gaps else None
            if latest_gap and latest_gap.gap_type == "Contradiction":
                planned = f"{self.query} {latest_gap.title} disagreement"
            elif latest_gap:
                planned = f"{self.query} {latest_gap.title} improvement"
            else:
                planned = self.query
            # GenePool-guided refinement
            capsules = self._query_gene_pool(
                self.query,
                gap_type=latest_gap.gap_type if latest_gap else None,
                keywords=latest_gap.title.split()[:5] if latest_gap else None
            )
            if capsules:
                planned = self._gene_pool_enhanced_query(planned, capsules, iteration)
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

        self._record_thought("searcher", f"Found {len(papers)} papers: {[p.arxiv_id for p in papers]}", iteration)
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
            self.db.add_papers([{
                "arxiv_id": paper.arxiv_id,
                "title": paper.title,
                "abstract": paper.abstract,
                "pdf_url": paper.pdf_url,
                "authors": str(paper.authors) if paper.authors else "",
                "published": str(paper.published) if paper.published else "",
                "source": "arxiv",
            }])

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

    def _generate_implementations(
        self,
        snapshots: List[PaperSnapshot],
        iteration: int,
    ) -> List[dict]:
        """CODER: run PaperPipeline for papers that have interesting gaps.

        For each paper with a gap that has archetype_match > 0.5,
        generate code + tests + benchmark. Results feed back to Gene Pool.
        """
        if not snapshots:
            return []

        # Only process papers where at least one gap has high archetype match
        high_signal = [
            s for s in snapshots
            if any(
                g.archetype_match > 0.5
                for g in (self.session.gaps if self.session else [])
                if s.arxiv_id in g.matched_papers
            )
        ]

        if not high_signal:
            self._record_thought("coder", "No high-signal papers for implementation", iteration)
            return []

        self._log(f"[iter {iteration}] Running PaperPipeline on {len(high_signal)} paper(s)")

        results = []
        for snap in high_signal[:3]:  # Cap at 3 per iteration
            arxiv_id = snap.arxiv_id
            self._record_thought("coder", f"Generating implementation for {arxiv_id}", iteration)

            pipeline = PaperPipeline(work_dir=f".paper2code_work/{arxiv_id}")

            try:
                result = pipeline.run(
                    arxiv_id=arxiv_id,
                    mode="minimal",
                    skip_gene_pool=False,
                )
                results.append({
                    "arxiv_id": arxiv_id,
                    "title": snap.title,
                    "paper_dir": result["paper_dir"],
                    "benchmark": result.get("benchmark"),
                    "status": "success",
                })

                bench = result.get("benchmark")
                if bench:
                    passed = bench.get("passed", 0)
                    failed = bench.get("failed", 0)
                    self._record_thought(
                        "coder",
                        f"[{arxiv_id}] benchmark: {passed} passed, {failed} failed",
                        iteration,
                    )
                else:
                    self._record_thought("coder", f"[{arxiv_id}] code generated (benchmark skipped)", iteration)

            except Exception as e:
                self._record_thought("coder", f"[{arxiv_id}] PaperPipeline failed: {e}", iteration)
                results.append({
                    "arxiv_id": arxiv_id,
                    "title": snap.title,
                    "status": "failed",
                    "error": str(e),
                })

        self._record_thought("coder", f"PaperPipeline done: {len(results)} result(s)", iteration)
        return results

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

        all_implementation_results: List[dict] = []

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

            # Step 4b: Generate implementations for high-signal papers
            impl_results = self._generate_implementations(snapshots, iteration)
            all_implementation_results.extend(impl_results)

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
