"""Deep Research Agent — iterative research with gap detection and archetype-aware refinement.







Architecture inspired by:



- gpt-researcher: multi-agent research with planning



- deer-flow: sandbox + memory + tool use



- snapstate: session persistence for pause/resume



"""



from __future__ import annotations





import asyncio
import signal



import time



from dataclasses import dataclass, field



from pathlib import Path



from typing import Any, Callable, Dict, List, Optional, Tuple





from llm.reasoning import StreamingReasoner

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
from research_loop.workspace_snapshot import WorkspaceSnapshot







@dataclass

class AgentThought:

    """A single reasoning step in the agent loop."""



    iteration: int



    role: str  # "planner" | "searcher" | "analyzer" | "reflector"



    content: str



    timestamp: float = field(default_factory=lambda: time.time())





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

        mode: str = "agent",

        snapstate_dir: Optional[Path] = None,

        on_thought: Optional[callable] = None,

        mcp_tools: Optional[List[Dict[str, Any]]] = None,
        auto_checkpoint: bool = True,
        checkpoint_every_n_steps: int = 1,
        checkpoint_interval_seconds: int = 60,
        use_streaming_reasoning: bool = False,
    ):



        self.query = query



        self.max_iterations = max_iterations



        self.mode = mode



        self.on_thought = on_thought



        self.max_papers_per_iteration = max_papers_per_iteration



        self.verbose = verbose



        self.snapstate = Snapstate(base_dir=snapstate_dir)



        self.workspace_snapshot = WorkspaceSnapshot()



        self.tracker = get_evolution_tracker()



        self.gap_analyzer = GapAnalyzerV2()



        self.db = Database()



        self.session: Optional[ResearchSession] = None



        self.thoughts: List[AgentThought] = []



        self._stop_requested = False

        # Auto-checkpoint configuration
        self.auto_checkpoint = auto_checkpoint
        self.checkpoint_every_n_steps = checkpoint_every_n_steps
        self.checkpoint_interval_seconds = checkpoint_interval_seconds
        self._checkpoint_counter = 0
        self._last_checkpoint_time = time.time()

        # StreamingReasoner for extended thinking (DeepSeek V3 / MiniMax)
        self.use_streaming_reasoning = use_streaming_reasoning
        self._streaming_reasoner = StreamingReasoner() if use_streaming_reasoning else None

        # MCP tool registry — dynamically discovered tools agent can call
        self.mcp_tools = mcp_tools if mcp_tools is not None else self._discover_mcp_tools()
        self._mcp_tool_map: Dict[str, Dict[str, Any]] = {
            t["name"]: t for t in self.mcp_tools
        }

        # Skill discovery — dynamically discover Claude Code skill packs
        from research_loop.skill_discovery import discover_skills, match_skills
        self._skills = discover_skills()
        self._skill_map = {s.name: s for s in self._skills}



    @staticmethod
    def _discover_mcp_tools() -> List[Dict[str, Any]]:
        """Auto-discover Rairos MCP tools by importing the tool definitions."""
        try:
            from mcp.tools_defs import get_tools
            return get_tools()
        except Exception:
            return []

    def _find_skills(self, query: str) -> List[Any]:
        """Find skills matching a query string (keyword match in name + description).

        Returns skills sorted by relevance: name match > description match.
        """
        try:
            from research_loop.skill_discovery import match_skills
            return match_skills(query, self._skills)
        except Exception:
            return []

    def _get_skill(self, name: str) -> Optional[Any]:
        """Get a skill by exact name. Returns Skill object or None."""
        return self._skill_map.get(name)

    def _log(self, msg: str):



        if self.verbose:

            print(f"[DeepResearchAgent] {msg}")



    def _auto_checkpoint(self) -> None:
        """Save a named checkpoint for the current iteration state.

        Saves after each Plan+Analyze phase with session_id, step, timestamp, query.
        """
        if not self.session or not self.auto_checkpoint:
            return
        ck_name = f"iter{self.session.iteration:03d}"
        ck_id = self.snapstate.create_checkpoint(self.session)
        self._log(f"[checkpoint] {ck_name} ({ck_id}) saved")



    def _setup_signal_handlers(self):
        """Register signal handlers for graceful shutdown with checkpointing."""
        def handler(sig, frame):
            self._log("Received shutdown signal, checkpointing...")
            if self.session:
                self.snapstate.save(self.session)
                self._auto_checkpoint()
            self._stop_requested = True
        signal.signal(signal.SIGINT, handler)
        signal.signal(signal.SIGTERM, handler)



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



        # Streaming callback for DeepSeek-TUI-style output
        if self.on_thought:
            self.on_thought(role, content, iteration)



    def _call_mcp_tool(self, name: str, arguments: Dict[str, Any]) -> Any:
        """Dispatch an MCP tool call by name.

        Tools are registered at init via mcp_tools list. This enables the agent
        to call any Rairos MCP tool (KG queries, paper search, gap detection,
        paper2code pipeline, etc.) during research loops.
        """
        tool_def = self._mcp_tool_map.get(name)
        if not tool_def:
            return {"error": f"Unknown tool: {name}", "available": list(self._mcp_tool_map.keys())}

        try:
            # Dynamically resolve tool: try rairos_mcp handler dispatch
            # We use the same tool name → handler mapping as the MCP server
            handler_map = {
                "kg_query": "tool_kg_query",
                "kg_paper_subgraph": "tool_kg_paper_subgraph",
                "kg_tag_graph": "tool_kg_tag_graph",
                "kg_full_graph": "tool_kg_full_graph",
                "gap_detect": "tool_gap_detect",
                "gap_evolve": "tool_gap_evolve",
                "paper_search": "tool_paper_search",
                "paper_analyze": "tool_paper_analyze",
                "paper2code_run": "tool_paper2code_run",
                "citation_graph": "tool_citation_graph",
                "trends_detect_trending": "tool_trends_detect_trending",
                "trends_predict_next": "tool_trends_predict_next",
                "litreview_generate": "tool_litreview_generate",
                "review_simulate": "tool_review_simulate",
                "routeplan_create": "tool_routeplan_create",
                "routeplan_list": "tool_routeplan_list",
                "routeplan_update_step": "tool_routeplan_update_step",
                "impact_rank": "tool_impact_rank",
                "impact_score_paper": "tool_impact_score_paper",
                "impact_leaderboard": "tool_impact_leaderboard",
                "replication_check": "tool_replication_check",
                "replication_compare": "tool_replication_compare",
                "hypothesis_generate": "tool_hypothesis_generate",
                "hypothesis_list": "tool_hypothesis_list",
                "experiment_record": "tool_experiment_record",
                "slides_generate": "tool_slides_generate",
                "briefing_generate": "tool_briefing_generate",
                "research_agent_start": "tool_research_agent_start",
                "research_agent_stop": "tool_research_agent_stop",
                "research_agent_status": "tool_research_agent_status",
                "research_agent_trigger": "tool_research_agent_trigger",
            }

            handler_name = handler_map.get(name)
            if not handler_name:
                return {"error": f"No handler mapped for tool: {name}"}

            # Import and call handler dynamically
            from mcp import rairos_mcp as _mcp_module

            handler = getattr(_mcp_module, handler_name, None)
            if not handler:
                return {"error": f"Handler {handler_name} not found in rairos_mcp"}

            # Call with appropriate kwargs
            result = handler(**arguments) if arguments else handler()
            return result

        except Exception as e:
            self._log(f"MCP tool error {name}: {e}")
            return {"error": str(e)}


    def _approve_step(self, iteration: int, step: str, detail: str) -> bool:
        """Ask the caller to approve the next step (plan mode confirmation).

        In plan mode, calls on_thought with step info and waits.
        Returns True to proceed, False to abort.
        """
        if self.on_thought:
            return self.on_thought(step, f"[PLAN] {detail}", iteration) is not False
        return True



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



    def _stream_plan_search(self, iteration: int) -> str:
        """PLANNER with streaming extended thinking (DeepSeek V3 / MiniMax)."""
        messages = [{"role": "user", "content": self.query}]
        reasoning_lines: List[str] = []

        def on_reasoning(block):
            if block.content:
                reasoning_lines.append(f"[{block.phase or 'reasoning'}] {block.content}")

        content_chunks: List[str] = []
        def on_chunk(text):
            content_chunks.append(text)

        try:
            for _ in self._streaming_reasoner.stream_messages(
                messages,
                on_chunk=on_chunk,
                on_reasoning=on_reasoning,
            ):
                pass
        except Exception as e:
            self._log(f"StreamingReasoner failed, falling back: {e}")
            return self.query

        if reasoning_lines:
            self._record_thought(
                "planner",
                f"[streaming] reasoning: {''.join(reasoning_lines[-3:])}",
                iteration,
            )

        return "".join(content_chunks) if content_chunks else self.query


    def _search_papers(self, search_query: str, iteration: int) -> List[Paper]:

        """SEARCHER: fetch papers via MCP tool or local arXiv fallback."""



        self._log(f"[iter {iteration}] Searching: {search_query}")



        papers: List[Paper] = []



        # Try MCP paper_search tool first (GenePool-guided)

        if "paper_search" in self._mcp_tool_map:

            try:

                result = self._call_mcp_tool("paper_search", {
                    "query": search_query,
                    "max_results": self.max_papers_per_iteration,
                })

                if result and not result.get("error") and result.get("papers"):

                    from research_loop.core import Paper

                    for p in result["papers"][:self.max_papers_per_iteration]:

                        papers.append(Paper(
                            uid=p.get("arxiv_id", ""),
                            title=p.get("title", ""),
                            abstract=p.get("abstract", ""),
                            authors=[a.get("name", "") for a in p.get("authors", [])],
                            source="mcp",
                            pdf_url=p.get("pdf_url", ""),
                            published=p.get("published", ""),
                        ))

                    self._record_thought(
                        "searcher",
                        f"MCP paper_search found {len(papers)} papers: {[p.uid for p in papers]}",
                        iteration,
                    )

            except Exception as e:

                self._log(f"MCP paper_search failed, falling back: {e}")



        # Fallback to local arXiv search

        if not papers:

            try:

                papers = search_arxiv(search_query, max_results=self.max_papers_per_iteration)

            except Exception as e:

                self._record_thought("searcher", f"Search failed: {e}", iteration)

                return []




        self._record_thought(
            "searcher", f"Found {len(papers)} papers: {[p.uid for p in papers]}", iteration
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

                arxiv_id=paper.uid or paper.title[:20],

                title=paper.title,

                abstract=paper.abstract or "",

                url=paper.pdf_url or "",

                extracted_text=extracted[:5000],

            )



            snapshots.append(snapshot)



            # Store in DB



            # Store in DB
            self.db.upsert_paper(
                paper_id=paper.uid,
                source=paper.source,
                title=paper.title,
                authors=str(paper.authors) if paper.authors else "",
                abstract=paper.abstract,
                published=paper.published,
            )





        self._record_thought("extractor", f"Extracted {len(snapshots)} papers", iteration)



        return snapshots



    def _analyze_gaps(self, snapshots: List[PaperSnapshot], iteration: int) -> List[GapSnapshot]:

        """ANALYZER: detect research gaps via MCP tool or local GapAnalyzerV2."""



        self._log(f"[iter {iteration}] Analyzing gaps for {len(snapshots)} papers")



        gap_snapshots: List[GapSnapshot] = []



        # Try MCP gap_detect tool first

        if "gap_detect" in self._mcp_tool_map:

            try:

                paper_contexts = [
                    {"arxiv_id": s.arxiv_id, "title": s.title, "abstract": s.abstract}
                    for s in snapshots
                ]
                result = self._call_mcp_tool("gap_detect", {
                    "topic": self.query,
                    "papers": paper_contexts,
                })
                if result and not result.get("error") and result.get("gaps"):
                    archetype = self.session.archetype if self.session else {}
                    for g in result["gaps"][:5]:
                        # Build a Gap-like object for archetype matching
                        gap_obj = type("Gap", (), {
                            "gap_type": g.get("gap_type", "improvement"),
                            "title": g.get("title", ""),
                            "description": g.get("description", ""),
                        })()
                        match_score = self.tracker._archetype_match_score(
                            gap_obj, archetype
                        ) if archetype else 0.5
                        gs = GapSnapshot(
                            gap_type=g.get("gap_type", "improvement"),
                            title=g.get("title", ""),
                            description=g.get("description", ""),
                            matched_papers=[s.arxiv_id for s in snapshots],
                            archetype_match=match_score,
                        )
                        gap_snapshots.append(gs)
                    self._record_thought(
                        "analyzer",
                        f"MCP gap_detect found {len(gap_snapshots)} gaps: {[g.gap_type for g in gap_snapshots]}",
                        iteration,
                    )

            except Exception as e:

                self._log(f"MCP gap_detect failed, falling back: {e}")



        # Fallback to local GapAnalyzerV2

        if not gap_snapshots:

            try:

                result = self.gap_analyzer.analyze(
                    topic=self.query,
                    use_insights=True,
                    min_papers=3,
                    use_llm=False,
                )

            except Exception as e:

                self._record_thought("analyzer", f"Gap analysis failed: {e}", iteration)

                return []




            archetype = {}

            if self.session:

                archetype = self.session.archetype



            for gap in result.gaps[:5]:

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


        self._setup_signal_handlers()


        start_time = time.time()



        iteration = self.session.iteration



        self._log(f"Starting run from iteration {iteration}")



        while iteration < self.max_iterations and not self._stop_requested:

            self.session.iteration = iteration



            self.snapstate.save(self.session)



            # Step 0: Skill check - load relevant skills before planning

            matched_skills = self._find_skills(self.query)

            if matched_skills:

                skill_names = [s.name for s in matched_skills[:3]]

                self._record_thought(
                    "planner",
                    f"Matched skills: {skill_names}",
                    iteration,
                )



            # Step 1: Plan (streaming variant for extended-thinking models)
            if self.use_streaming_reasoning and self._streaming_reasoner:
                search_query = self._stream_plan_search(iteration)
            else:
                search_query = self._plan_next_search(iteration)



            # Plan-mode approval gate
            if self.mode == "plan":
                confirmed = self._approve_step(iteration, "planner", f"Search query: {search_query}")
                if not confirmed:
                    self.session.status = "paused"
                    self.snapstate.save(self.session)
                    break

            # Checkpoint after Plan step (every N steps)
            self._checkpoint_counter += 1
            if self._checkpoint_counter % self.checkpoint_every_n_steps == 0:
                self._auto_checkpoint()
            # Periodic time-based checkpoint
            if time.time() - self._last_checkpoint_time >= self.checkpoint_interval_seconds:
                self._auto_checkpoint()
                self._last_checkpoint_time = time.time()

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



            # Workspace snapshot: capture generated code after extract phase

            if self.workspace_snapshot and self.session:

                code_paths = []

                output_base = Path.cwd() / "output"

                if output_base.exists():

                    for ext in [".py", ".md", ".json", ".txt"]:

                        code_paths.extend(output_base.glob(f"*{ext}"))

                if code_paths:

                    self.workspace_snapshot.capture(
                        session_id=self.session.session_id,
                        step=iteration,
                        paths=code_paths,
                        metadata={"phase": "extract", "papers_found": len(papers)},
                    )



            # Step 4: Analyze gaps



            gap_snapshots = self._analyze_gaps(snapshots, iteration)



            if self.session:

                self.session.gaps.extend(gap_snapshots)

            # Checkpoint after Analyze gaps step (every N steps)
            self._checkpoint_counter += 1
            if self._checkpoint_counter % self.checkpoint_every_n_steps == 0:
                self._auto_checkpoint()
            # Periodic time-based checkpoint
            if time.time() - self._last_checkpoint_time >= self.checkpoint_interval_seconds:
                self._auto_checkpoint()
                self._last_checkpoint_time = time.time()

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

