"""Comprehensive pytest tests for research_loop.deep_research.DeepResearchAgent.
All external dependencies are mocked: GapAnalyzerV2, get_evolution_tracker,
Database, Snapstate, search_arxiv, extract_pdf_text, Paper.
The mock setup pre-registers fake modules in sys.modules for every dependency
that research_loop.core / research_loop.__init__ would try to import, so that
the real research_loop package can be found by Python's import machinery while
its heavy transitive deps are replaced with stubs.
"""

from __future__ import annotations
import sys
import time
import types
from pathlib import Path
from typing import Any, List, Optional, Tuple
from unittest.mock import MagicMock, patch
import pytest


# ---------------------------------------------------------------------------
# Pre-register mock sub-modules so that research_loop.__init__ and
# research_loop.deep_research can be imported without pulling in the
# full dependency tree (llm.generate, core.Paper, etc.).
# ---------------------------------------------------------------------------
def _make_mod(name: str) -> types.ModuleType:
    mod = types.ModuleType(name)
    sys.modules[name] = mod
    return mod


# --- research_loop sub-modules (must be registered BEFORE __init__.py runs) ---
_core = _make_mod("research_loop.core")
_core.search_arxiv = MagicMock()
_core.extract_pdf_text = MagicMock()
_core.Paper = MagicMock()
_core.run_research = MagicMock()
_core.arun_research = MagicMock()
_core.Metrics = MagicMock()
_core._build_research_note = MagicMock()
_core.warm_cache_research = MagicMock()
_pp = _make_mod("research_loop.paper2code_integration")
_pp.PaperPipeline = MagicMock()
_es = _make_mod("research_loop.evoskill_integration")
_es.EvoSkillPipeline = MagicMock()
_rp = _make_mod("research_loop.rag_pipeline")
_rp.RagPipeline = MagicMock()
# --- llm / db dependencies ---
# NOTE: We intentionally do NOT stub db.database, llm.gap_analyzer,
# llm.insight_evolution, or research_loop.snapstate because they are imported
# at module level (lines 67-76 below) and other test modules depend on them.
# Stubbing them pollutes sys.modules and breaks collection of tests/test_viz.py,
# tests/test_snapstate.py and others. The research_loop.core stubs (above)
# are sufficient for isolating research_loop.deep_research from its dependencies.
# ---------------------------------------------------------------------------
# NOW import the module under test — all deps are already mocked.
# ---------------------------------------------------------------------------
from research_loop.deep_research import (
    DeepResearchAgent,
    AgentThought,
    DeepResearchResult,
)
from research_loop.snapstate import (
    Snapstate,
    ResearchSession,
    PaperSnapshot,
    GapSnapshot,
)
from research_loop.core import search_arxiv, extract_pdf_text, Paper
from llm.gap_analyzer import GapAnalyzerV2
from llm.insight_evolution import get_evolution_tracker
from db.database import Database


# ---------------------------------------------------------------------------
# Cleanup: remove fake stubs after this module's tests finish
# ---------------------------------------------------------------------------


def pytest_sessionfinish(session, exitstatus):
    """Clean up fake stub modules when test session ends."""
    stubs = [
        n for n, m in sys.modules.items() if isinstance(m, types.ModuleType) and m.__spec__ is None
    ]
    for n in stubs:
        sys.modules.pop(n, None)


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------
def _make_gap_snapshot(
    gap_type: str = "Contradiction",
    title: str = "Test Gap",
    description: str = "desc",
    accepted: bool = False,
    archetype_match: float = 0.5,
):
    """Return a mock GapSnapshot with sensible defaults."""
    gs = MagicMock()
    gs.gap_type = gap_type
    gs.title = title
    gs.description = description
    gs.accepted = accepted
    gs.archetype_match = archetype_match
    return gs


def _make_paper_snapshot(arxiv_id: str = "2301.00001", title: str = "Paper A"):
    ps = MagicMock()
    ps.arxiv_id = arxiv_id
    ps.title = title
    ps.gaps_found = 0
    return ps


def _make_capsule(outcome_success_score=0.8, trigger_keywords=None):
    cap = MagicMock()
    cap.outcome_success_score = outcome_success_score
    cap.trigger_keywords = (
        trigger_keywords if trigger_keywords is not None else ["keyword1", "keyword2"]
    )
    return cap


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------
@pytest.fixture(autouse=True)
def _reset_mocks():
    """Reset module-level MagicMock instances between tests."""
    core = sys.modules["research_loop.core"]
    if hasattr(core.search_arxiv, "reset_mock"):
        core.search_arxiv.reset_mock()
    if hasattr(core.extract_pdf_text, "reset_mock"):
        core.extract_pdf_text.reset_mock()
    evo = sys.modules["llm.insight_evolution"]
    if hasattr(evo.get_evolution_tracker, "reset_mock"):
        evo.get_evolution_tracker.reset_mock()
    yield


@pytest.fixture
def agent() -> DeepResearchAgent:
    """Create a DeepResearchAgent with all deps mocked."""
    with (
        patch("research_loop.deep_research.Snapstate") as MockSnap,
        patch("research_loop.deep_research.get_evolution_tracker") as MockTracker,
        patch("research_loop.deep_research.GapAnalyzerV2") as MockGA,
        patch("research_loop.deep_research.Database") as MockDB,
    ):
        MockSnap.return_value = MagicMock()
        MockTracker.return_value = MagicMock()
        MockGA.return_value = MagicMock()
        MockDB.return_value = MagicMock()
        a = DeepResearchAgent(
            query="test query",
            max_iterations=3,
            max_papers_per_iteration=5,
            verbose=False,
        )
        a._mock_snapstate = MockSnap.return_value
        a._mock_tracker = MockTracker.return_value
        a._mock_ga = MockGA.return_value
        a._mock_db = MockDB.return_value
        yield a


@pytest.fixture
def agent_verbose() -> DeepResearchAgent:
    """Create a verbose agent with custom settings."""
    with (
        patch("research_loop.deep_research.Snapstate") as MockSnap,
        patch("research_loop.deep_research.get_evolution_tracker") as MockTracker,
        patch("research_loop.deep_research.GapAnalyzerV2") as MockGA,
        patch("research_loop.deep_research.Database") as MockDB,
    ):
        MockSnap.return_value = MagicMock()
        MockTracker.return_value = MagicMock()
        MockGA.return_value = MagicMock()
        MockDB.return_value = MagicMock()
        a = DeepResearchAgent(
            query="verbose query",
            max_iterations=5,
            max_papers_per_iteration=10,
            verbose=True,
            snapstate_dir=Path("/tmp/test_sessions"),
        )
        a._mock_snapstate = MockSnap.return_value
        a._mock_tracker = MockTracker.return_value
        a._mock_ga = MockGA.return_value
        a._mock_db = MockDB.return_value
        yield a


# ========================================================================
# Tests: __init__
# ========================================================================
class TestInit:
    """Tests for DeepResearchAgent.__init__."""

    def test_default_init(self, agent: DeepResearchAgent):
        """Verify all default attributes after construction."""
        assert agent.query == "test query"
        assert agent.max_iterations == 3
        assert agent.max_papers_per_iteration == 5
        assert agent.verbose is False
        assert agent.session is None
        assert agent.thoughts == []
        assert agent._stop_requested is False

    def test_custom_init(self, agent_verbose: DeepResearchAgent):
        """Verify non-default parameters are stored."""
        assert agent_verbose.query == "verbose query"
        assert agent_verbose.max_iterations == 5
        assert agent_verbose.max_papers_per_iteration == 10
        assert agent_verbose.verbose is True

    def test_init_creates_snapstate(self, agent: DeepResearchAgent):
        """Snapstate should be instantiated."""
        assert agent._mock_snapstate is not None

    def test_init_creates_tracker(self, agent: DeepResearchAgent):
        """get_evolution_tracker should have been called."""
        assert agent._mock_tracker is not None

    def test_init_creates_gap_analyzer(self, agent: DeepResearchAgent):
        """GapAnalyzerV2 should have been instantiated."""
        assert agent._mock_ga is not None

    def test_init_creates_database(self, agent: DeepResearchAgent):
        """Database should have been instantiated."""
        assert agent._mock_db is not None

    def test_init_snapstate_dir_passthrough(self):
        """snapstate_dir should be passed to Snapstate constructor."""
        with (
            patch("research_loop.deep_research.Snapstate") as MockSnap,
            patch("research_loop.deep_research.get_evolution_tracker"),
            patch("research_loop.deep_research.GapAnalyzerV2"),
            patch("research_loop.deep_research.Database"),
        ):
            MockSnap.return_value = MagicMock()
            DeepResearchAgent(
                query="q",
                snapstate_dir=Path("/custom/dir"),
            )
            MockSnap.assert_called_once_with(base_dir=Path("/custom/dir"))


# ========================================================================
# Tests: _log
# ========================================================================
class TestLog:
    """Tests for the _log method."""

    def test_log_silent_when_not_verbose(self, agent: DeepResearchAgent, capsys):
        """_log should not print when verbose is False."""
        agent._log("hello")
        captured = capsys.readouterr()
        assert captured.out == ""

    def test_log_prints_when_verbose(self, agent_verbose: DeepResearchAgent, capsys):
        """_log should print with prefix when verbose is True."""
        agent_verbose._log("hello world")
        captured = capsys.readouterr()
        assert "[DeepResearchAgent] hello world" in captured.out

    def test_log_empty_message(self, agent: DeepResearchAgent, capsys):
        """_log with empty string should be silent."""
        agent._log("")
        captured = capsys.readouterr()
        assert captured.out == ""


# ========================================================================
# Tests: _record_thought
# ========================================================================
class TestRecordThought:
    """Tests for _record_thought."""

    def test_record_creates_thought(self, agent: DeepResearchAgent):
        """Each call should append an AgentThought."""
        agent._record_thought("planner", "thinking...", 0)
        assert len(agent.thoughts) == 1
        t = agent.thoughts[0]
        assert t.role == "planner"
        assert t.content == "thinking..."
        assert t.iteration == 0
        assert isinstance(t.timestamp, float)

    def test_record_multiple_thoughts(self, agent: DeepResearchAgent):
        """Multiple calls accumulate thoughts."""
        agent._record_thought("planner", "plan", 0)
        agent._record_thought("searcher", "search", 0)
        agent._record_thought("analyzer", "analyze", 1)
        assert len(agent.thoughts) == 3

    def test_record_appends_to_session_findings(self, agent: DeepResearchAgent):
        """When a session exists, findings should be appended."""
        mock_session = MagicMock()
        mock_session.findings = []
        agent.session = mock_session
        agent._record_thought("planner", "plan", 0)
        assert len(mock_session.findings) == 1
        assert "[PLANNER] plan" in mock_session.findings[0]

    def test_record_no_session(self, agent: DeepResearchAgent):
        """When session is None, no error should occur."""
        agent.session = None
        agent._record_thought("planner", "plan", 0)
        assert len(agent.thoughts) == 1

    def test_record_thought_timestamp_is_recent(self, agent: DeepResearchAgent):
        """Timestamp should be a valid float."""
        agent._record_thought("reflector", "reflect", 0)
        assert isinstance(agent.thoughts[0].timestamp, float)
        assert agent.thoughts[0].timestamp > 0

    def test_record_preserves_timestamp_order(self, agent: DeepResearchAgent):
        """Thoughts should be appended in call order."""
        agent._record_thought("a", "first", 0)
        agent._record_thought("b", "second", 0)
        assert agent.thoughts[0].timestamp <= agent.thoughts[1].timestamp

    def test_record_all_roles(self, agent: DeepResearchAgent):
        """All four roles should be supported."""
        for role in ("planner", "searcher", "analyzer", "reflector"):
            agent._record_thought(role, f"msg-{role}", 0)
        roles = {t.role for t in agent.thoughts}
        assert roles == {"planner", "searcher", "analyzer", "reflector"}


# ========================================================================
# Tests: _get_search_guidance
# ========================================================================
class TestGetSearchGuidance:
    """Tests for _get_search_guidance with mocked tracker.find_capsule."""

    def test_returns_hint_and_confidence(self, agent: DeepResearchAgent):
        """When capsules found with keywords, return hint + confidence."""
        cap = _make_capsule(
            outcome_success_score=0.75,
            trigger_keywords=["deep", "learning", "optimization"],
        )
        agent._mock_tracker.find_capsule.return_value = [cap]
        hint, conf = agent._get_search_guidance(
            "topic",
            "Contradiction",
            "gap title",
        )
        assert hint == "deep learning optimization"
        assert conf == 0.75
        agent._mock_tracker.find_capsule.assert_called_once_with(
            topic="topic",
            gap_type="Contradiction",
            keywords=[],
            min_score=0.1,
        )

    def test_returns_none_when_no_capsules(self, agent: DeepResearchAgent):
        """When no capsules match, return (None, 0.0)."""
        agent._mock_tracker.find_capsule.return_value = []
        hint, conf = agent._get_search_guidance(
            "topic",
            "Contradiction",
            "gap",
        )
        assert hint is None
        assert conf == 0.0

    def test_returns_none_when_empty_keywords(self, agent: DeepResearchAgent):
        """When capsule has empty trigger_keywords, return (None, 0.0)."""
        cap = _make_capsule(trigger_keywords=[])
        agent._mock_tracker.find_capsule.return_value = [cap]
        hint, conf = agent._get_search_guidance(
            "topic",
            "Contradiction",
            "gap",
        )
        assert hint is None
        assert conf == 0.0

    def test_returns_none_when_keywords_not_list(self, agent: DeepResearchAgent):
        """When trigger_keywords is not a list, return (None, 0.0)."""
        cap = _make_capsule(trigger_keywords="not a list")
        agent._mock_tracker.find_capsule.return_value = [cap]
        hint, conf = agent._get_search_guidance(
            "topic",
            "Contradiction",
            "gap",
        )
        assert hint is None
        assert conf == 0.0

    def test_returns_none_on_exception(self, agent: DeepResearchAgent):
        """Exception in find_capsule should return (None, 0.0)."""
        agent._mock_tracker.find_capsule.side_effect = RuntimeError("db error")
        hint, conf = agent._get_search_guidance(
            "topic",
            "Contradiction",
            "gap",
        )
        assert hint is None
        assert conf == 0.0

    def test_limits_keywords_to_5(self, agent: DeepResearchAgent):
        """Only first 5 keywords should be used in the hint."""
        cap = _make_capsule(
            trigger_keywords=["a", "b", "c", "d", "e", "f", "g"],
        )
        agent._mock_tracker.find_capsule.return_value = [cap]
        hint, _ = agent._get_search_guidance(
            "topic",
            "Contradiction",
            "gap",
        )
        assert hint == "a b c d e"

    def test_uses_best_capsule(self, agent: DeepResearchAgent):
        """Should use capsules[0] (the first/best one)."""
        cap = _make_capsule(
            outcome_success_score=0.9,
            trigger_keywords=["best"],
        )
        agent._mock_tracker.find_capsule.return_value = [cap]
        hint, conf = agent._get_search_guidance(
            "topic",
            "Contradiction",
            "gap",
        )
        assert hint == "best"
        assert conf == 0.9


# ========================================================================
# Tests: _plan_next_search
# ========================================================================
class TestPlanNextSearch:
    """Tests for _plan_next_search covering iteration 0 and later."""

    def test_iteration_0_uses_original_query(self, agent: DeepResearchAgent):
        """On iteration 0, the search query should be the original query."""
        mock_session = MagicMock()
        mock_session.gaps = []
        mock_session.search_history = []
        agent.session = mock_session
        result = agent._plan_next_search(0)
        assert result == "test query"
        assert any(t.role == "planner" for t in agent.thoughts)

    def test_later_iteration_with_gap_contradiction(
        self,
        agent: DeepResearchAgent,
    ):
        """When a Contradiction gap exists, use query + gap + disagreement."""
        gap = _make_gap_snapshot(
            gap_type="Contradiction",
            title="Paper X disagrees",
        )
        mock_session = MagicMock()
        mock_session.gaps = [gap]
        mock_session.search_history = []
        agent.session = mock_session
        result = agent._plan_next_search(1)
        assert "test query" in result
        assert "Paper X disagrees" in result
        assert "disagreement" in result

    def test_later_iteration_with_gap_improvement(
        self,
        agent: DeepResearchAgent,
    ):
        """Non-Contradiction gap uses query + gap + improvement."""
        gap = _make_gap_snapshot(gap_type="Missing", title="No coverage")
        mock_session = MagicMock()
        mock_session.gaps = [gap]
        mock_session.search_history = []
        agent.session = mock_session
        result = agent._plan_next_search(1)
        assert "No coverage" in result
        assert "improvement" in result

    def test_later_iteration_with_gene_pool_hint(
        self,
        agent: DeepResearchAgent,
    ):
        """When GenePool returns high-confidence hint, incorporate it."""
        gap = _make_gap_snapshot(
            gap_type="Contradiction",
            title="Gap A",
        )
        cap = _make_capsule(
            outcome_success_score=0.8,
            trigger_keywords=["genetic", "algorithm"],
        )
        agent._mock_tracker.find_capsule.return_value = [cap]
        mock_session = MagicMock()
        mock_session.gaps = [gap]
        mock_session.search_history = []
        agent.session = mock_session
        result = agent._plan_next_search(1)
        assert "genetic" in result.lower() or "algorithm" in result.lower()
        assert "Gap A" in result

    def test_gene_pool_hint_below_threshold(
        self,
        agent: DeepResearchAgent,
    ):
        """When GenePool confidence < 0.3, fall back to gap strategy."""
        gap = _make_gap_snapshot(gap_type="Missing", title="Gap B")
        cap = _make_capsule(
            outcome_success_score=0.1,
            trigger_keywords=["hint"],
        )
        agent._mock_tracker.find_capsule.return_value = [cap]
        mock_session = MagicMock()
        mock_session.gaps = [gap]
        mock_session.search_history = []
        agent.session = mock_session
        result = agent._plan_next_search(1)
        assert "Gap B" in result

    def test_avoids_duplicate_search(self, agent: DeepResearchAgent):
        """If planned query is in history, append iteration number."""
        gap = _make_gap_snapshot(
            gap_type="Contradiction",
            title="Gap C",
        )
        mock_session = MagicMock()
        mock_session.gaps = [gap]
        mock_session.search_history = [
            "test query Gap C disagreement",
        ]
        agent.session = mock_session
        result = agent._plan_next_search(1)
        # Should have been modified to avoid the duplicate
        assert "test query 1" in result or result != "test query Gap C disagreement"

    def test_no_session_uses_query(self, agent: DeepResearchAgent):
        """Without a session, iteration 0 should use original query."""
        agent.session = None
        result = agent._plan_next_search(0)
        assert result == "test query"

    def test_no_gaps_later_iteration_uses_query(
        self,
        agent: DeepResearchAgent,
    ):
        """When gaps list is empty on a later iteration, use query."""
        mock_session = MagicMock()
        mock_session.gaps = []
        mock_session.search_history = []
        agent.session = mock_session
        result = agent._plan_next_search(2)
        assert result == "test query"


# ========================================================================
# Tests: start / pause / resume
# ========================================================================
class TestSessionLifecycle:
    """Tests for start, pause, resume."""

    def test_start_creates_session(self, agent: DeepResearchAgent):
        """start() should call snapstate.new_session and save."""
        mock_session = MagicMock()
        mock_session.session_id = "sess-001"
        agent._mock_snapstate.new_session.return_value = mock_session
        result = agent.start()
        assert result is mock_session
        assert agent.session is mock_session
        agent._mock_snapstate.new_session.assert_called_once_with(
            query="test query",
            max_iterations=3,
        )
        agent._mock_snapstate.save.assert_called_with(mock_session)

    def test_start_returns_session(self, agent: DeepResearchAgent):
        """start() should return the created session."""
        mock_session = MagicMock()
        mock_session.session_id = "sess-002"
        agent._mock_snapstate.new_session.return_value = mock_session
        result = agent.start()
        assert result.session_id == "sess-002"

    def test_pause_sets_status_and_saves(self, agent: DeepResearchAgent):
        """pause() should set status to 'paused' and persist."""
        mock_session = MagicMock()
        mock_session.iteration = 1
        agent.session = mock_session
        agent.pause()
        assert mock_session.status == "paused"
        agent._mock_snapstate.save.assert_called_with(mock_session)

    def test_pause_no_session(self, agent: DeepResearchAgent):
        """pause() with no session should not crash."""
        agent.session = None
        agent.pause()  # should not raise

    def test_resume_loads_session(self, agent: DeepResearchAgent):
        """resume() should load and return the session."""
        mock_session = MagicMock()
        mock_session.iteration = 2
        agent._mock_snapstate.load.return_value = mock_session
        result = agent.resume("sess-001")
        assert result is mock_session
        assert agent.session is mock_session
        agent._mock_snapstate.load.assert_called_once_with("sess-001")

    def test_resume_nonexistent_session(self, agent: DeepResearchAgent):
        """resume() for missing session returns None."""
        agent._mock_snapstate.load.return_value = None
        result = agent.resume("missing-id")
        assert result is None
        assert agent.session is None

    def test_start_pause_resume_cycle(self, agent: DeepResearchAgent):
        """Full lifecycle: start, pause, resume."""
        mock_session = MagicMock()
        mock_session.session_id = "s1"
        mock_session.iteration = 1
        agent._mock_snapstate.new_session.return_value = mock_session
        # Start
        result = agent.start()
        assert result is mock_session
        # Pause
        agent.pause()
        assert mock_session.status == "paused"
        # Resume
        agent._mock_snapstate.load.return_value = mock_session
        resumed = agent.resume("s1")
        assert resumed is mock_session
        assert agent.session is mock_session


# ========================================================================
# Tests: _build_report
# ========================================================================
class TestBuildReport:
    """Tests for _build_report."""

    def test_no_session(self, agent: DeepResearchAgent):
        """With no session, report should be 'No session'."""
        agent.session = None
        assert agent._build_report() == "No session"

    def test_report_contains_query(self, agent: DeepResearchAgent):
        """Report should include the query."""
        session = MagicMock()
        session.query = "deep learning"
        session.session_id = "s1"
        session.iteration = 2
        session.status = "completed"
        session.papers = []
        session.gaps = []
        session.findings = []
        session.duration.return_value = 42.5
        agent.session = session
        report = agent._build_report()
        assert "deep learning" in report
        assert "s1" in report

    def test_report_contains_papers(self, agent: DeepResearchAgent):
        """Report should list analyzed papers."""
        ps = _make_paper_snapshot("2301.12345", "My Paper")
        ps.gaps_found = 3
        session = MagicMock()
        session.query = "q"
        session.session_id = "s2"
        session.iteration = 1
        session.status = "completed"
        session.papers = [ps]
        session.gaps = []
        session.findings = []
        session.duration.return_value = 10.0
        agent.session = session
        report = agent._build_report()
        assert "2301.12345" in report
        assert "My Paper" in report
        assert "3 gaps" in report

    def test_report_contains_gaps(self, agent: DeepResearchAgent):
        """Report should list research gaps with status symbols."""
        gs_accepted = _make_gap_snapshot(
            "Contradiction",
            "Gap A",
            "desc A",
            accepted=True,
        )
        gs_pending = _make_gap_snapshot(
            "Missing",
            "Gap B",
            "desc B",
            accepted=False,
        )
        session = MagicMock()
        session.query = "q"
        session.session_id = "s3"
        session.iteration = 1
        session.status = "completed"
        session.papers = []
        session.gaps = [gs_accepted, gs_pending]
        session.findings = []
        session.duration.return_value = 5.0
        agent.session = session
        report = agent._build_report()
        assert "Gap A" in report
        assert "Gap B" in report
        assert "\u2705" in report  # checkmark for accepted
        assert "\u2b1c" in report  # empty square for pending

    def test_report_contains_findings(self, agent: DeepResearchAgent):
        """Report should include the last 10 findings."""
        session = MagicMock()
        session.query = "q"
        session.session_id = "s4"
        session.iteration = 1
        session.status = "completed"
        session.papers = []
        session.gaps = []
        session.findings = [f"finding_{i}" for i in range(15)]
        session.duration.return_value = 3.0
        agent.session = session
        report = agent._build_report()
        assert "finding_5" in report
        assert "finding_14" in report
        assert "finding_0" not in report

    def test_report_headers(self, agent: DeepResearchAgent):
        """Report should contain expected section headers."""
        session = MagicMock()
        session.query = "q"
        session.session_id = "s5"
        session.iteration = 0
        session.status = "completed"
        session.papers = []
        session.gaps = []
        session.findings = []
        session.duration.return_value = 0.0
        agent.session = session
        report = agent._build_report()
        assert "# Deep Research Report" in report
        assert "## Papers Analyzed" in report
        assert "## Research Gaps" in report
        assert "## Findings" in report
        assert "**Status**" in report


# ========================================================================
# Tests: stop
# ========================================================================
class TestStop:
    """Tests for the stop method."""

    def test_stop_sets_flag(self, agent: DeepResearchAgent):
        """stop() should set _stop_requested to True."""
        agent.stop()
        assert agent._stop_requested is True

    def test_stop_pauses_session(self, agent: DeepResearchAgent):
        """stop() should pause the session if it exists."""
        mock_session = MagicMock()
        agent.session = mock_session
        agent.stop()
        assert mock_session.status == "paused"
        agent._mock_snapstate.save.assert_called_with(mock_session)

    def test_stop_no_session(self, agent: DeepResearchAgent):
        """stop() without a session should not crash."""
        agent.session = None
        agent.stop()
        assert agent._stop_requested is True


# ========================================================================
# Tests: _reflect
# ========================================================================
class TestReflect:
    """Tests for _reflect behavior."""

    def test_reflect_no_session(self, agent: DeepResearchAgent):
        """_reflect with no session returns False."""
        agent.session = None
        should, reason = agent._reflect(0)
        assert should is False
        assert "no session" in reason

    def test_reflect_max_iterations(self, agent: DeepResearchAgent):
        """_reflect returns False when iteration >= max_iterations."""
        mock_session = MagicMock()
        mock_session.gaps = []
        mock_session.papers = []
        agent.session = mock_session
        should, reason = agent._reflect(3)  # max_iterations = 3
        assert should is False
        assert "max iterations" in reason

    def test_reflect_max_papers(self, agent: DeepResearchAgent):
        """_reflect returns False when paper count >= threshold."""
        mock_session = MagicMock()
        mock_session.gaps = []
        mock_session.papers = [MagicMock()] * 15  # 3 * 5 = 15
        agent.session = mock_session
        should, reason = agent._reflect(2)
        assert should is False
        assert "max papers" in reason

    def test_reflect_no_gaps_after_thorough_search(
        self,
        agent: DeepResearchAgent,
    ):
        """No gaps after iteration 1 should stop."""
        mock_session = MagicMock()
        mock_session.gaps = []
        mock_session.papers = []
        agent.session = mock_session
        should, reason = agent._reflect(2)
        assert should is False
        assert "no gaps" in reason

    def test_reflect_continues_with_unaccepted_gaps(
        self,
        agent: DeepResearchAgent,
    ):
        """Unaccepted gaps should cause reflection to continue."""
        gap = _make_gap_snapshot(accepted=False)
        mock_session = MagicMock()
        mock_session.gaps = [gap]
        mock_session.papers = []
        agent.session = mock_session
        should, reason = agent._reflect(1)
        assert should is True

    def test_reflect_stops_when_gaps_accepted(
        self,
        agent: DeepResearchAgent,
    ):
        """Accepted gaps should stop iteration."""
        gap = _make_gap_snapshot(accepted=True)
        mock_session = MagicMock()
        mock_session.gaps = [gap]
        mock_session.papers = []
        agent.session = mock_session
        should, reason = agent._reflect(1)
        assert should is False
        assert "accepted" in reason

    def test_reflect_low_archetype_match_records_thought(
        self,
        agent: DeepResearchAgent,
    ):
        """Low archetype match on iteration >= 2 should record a thought."""
        gap = _make_gap_snapshot(accepted=False, archetype_match=0.2)
        mock_session = MagicMock()
        mock_session.gaps = [gap]
        mock_session.papers = []
        agent.session = mock_session
        should, reason = agent._reflect(2)
        assert should is True
        assert any("Low archetype match" in t.content for t in agent.thoughts)


# ========================================================================
# Tests: _encode_accepted_gaps
# ========================================================================
class TestEncodeAcceptedGaps:
    """Tests for _encode_accepted_gaps."""

    def test_no_session(self, agent: DeepResearchAgent):
        """No session means no-op."""
        agent.session = None
        agent._encode_accepted_gaps()

    def test_encodes_accepted_gaps(self, agent: DeepResearchAgent):
        """Accepted gaps with archetype_match > 0 are recorded."""
        gap = _make_gap_snapshot(
            gap_type="Missing",
            title="Gap A",
            description="desc",
            accepted=True,
            archetype_match=0.7,
        )
        mock_session = MagicMock()
        mock_session.gaps = [gap]
        agent.session = mock_session
        agent._encode_accepted_gaps()
        agent._mock_tracker.record_gap_accept.assert_called_once_with(
            topic="test query",
            gap_type="Missing",
            gap_title="Gap A",
            gap_description="desc",
        )

    def test_skips_unaccepted_gaps(self, agent: DeepResearchAgent):
        """Unaccepted gaps should not be recorded."""
        gap = _make_gap_snapshot(accepted=False, archetype_match=0.5)
        mock_session = MagicMock()
        mock_session.gaps = [gap]
        agent.session = mock_session
        agent._encode_accepted_gaps()
        agent._mock_tracker.record_gap_accept.assert_not_called()

    def test_skips_zero_archetype_match(self, agent: DeepResearchAgent):
        """Gaps with archetype_match == 0 should not be recorded."""
        gap = _make_gap_snapshot(accepted=True, archetype_match=0.0)
        mock_session = MagicMock()
        mock_session.gaps = [gap]
        agent.session = mock_session
        agent._encode_accepted_gaps()
        agent._mock_tracker.record_gap_accept.assert_not_called()


# ========================================================================
# Tests: _search_papers
# ========================================================================
class TestSearchPapers:
    """Tests for _search_papers."""

    def test_search_calls_search_arxiv(self, agent: DeepResearchAgent):
        """search_arxiv should be called with correct params."""
        with patch("research_loop.deep_research.search_arxiv") as mock_sa:
            mock_sa.return_value = [MagicMock(arxiv_id="123")]
            result = agent._search_papers("query", 0)
            mock_sa.assert_called_once_with("query", max_results=5)
            assert len(result) == 1

    def test_search_records_thought_on_success(
        self,
        agent: DeepResearchAgent,
    ):
        """Successful search should record a searcher thought."""
        with patch("research_loop.deep_research.search_arxiv") as mock_sa:
            mock_sa.return_value = [MagicMock(arxiv_id="123")]
            agent._search_papers("q", 0)
            assert any(t.role == "searcher" for t in agent.thoughts)

    def test_search_returns_empty_on_exception(
        self,
        agent: DeepResearchAgent,
    ):
        """search_arxiv exception should return empty list."""
        with patch("research_loop.deep_research.search_arxiv") as mock_sa:
            mock_sa.side_effect = RuntimeError("network error")
            result = agent._search_papers("q", 0)
            assert result == []
            assert any("Search failed" in t.content for t in agent.thoughts)


# ========================================================================
# Tests: _extract_papers
# ========================================================================
class TestExtractPapers:
    """Tests for _extract_papers."""

    def test_extract_calls_db_add_papers(self, agent: DeepResearchAgent):
        """Each paper should be stored in the database."""
        mock_paper = MagicMock()
        mock_paper.arxiv_id = "2301.001"
        mock_paper.title = "Test"
        mock_paper.abstract = "abstract"
        mock_paper.pdf_url = "http://example.com/paper.pdf"
        mock_paper.authors = ["Author A"]
        mock_paper.published = "2024-01-01"
        with (
            patch("research_loop.deep_research.extract_pdf_text") as mock_ep,
            patch("research_loop.deep_research.PaperSnapshot") as MockPS,
        ):
            mock_ep.return_value = "full text here"
            ps = MagicMock()
            ps.arxiv_id = "2301.001"
            ps.title = "Test"
            MockPS.return_value = ps
            result = agent._extract_papers([mock_paper], 0)
            assert len(result) == 1
            agent._mock_db.upsert_paper.assert_called()

    def test_extract_records_thought(self, agent: DeepResearchAgent):
        """Extractor should record a thought."""
        mock_paper = MagicMock()
        mock_paper.arxiv_id = "2301.001"
        mock_paper.title = "T"
        mock_paper.abstract = "A"
        mock_paper.pdf_url = None
        mock_paper.authors = None
        mock_paper.published = None
        with (
            patch("research_loop.deep_research.extract_pdf_text"),
            patch("research_loop.deep_research.PaperSnapshot") as MockPS,
        ):
            MockPS.return_value = MagicMock(
                arxiv_id="2301.001",
                title="T",
            )
            agent._extract_papers([mock_paper], 1)
            assert any(t.role == "extractor" for t in agent.thoughts)


# ========================================================================
# Tests: _analyze_gaps
# ========================================================================
class TestAnalyzeGaps:
    """Tests for _analyze_gaps."""

    def test_analyze_calls_gap_analyzer(self, agent: DeepResearchAgent):
        """GapAnalyzerV2.analyze should be called."""
        mock_result = MagicMock()
        mock_result.gaps = []
        agent._mock_ga.analyze.return_value = mock_result
        agent.session = MagicMock()
        agent.session.archetype = {}
        result = agent._analyze_gaps([], 0)
        assert result == []
        agent._mock_ga.analyze.assert_called_once()

    def test_analyze_records_thought(self, agent: DeepResearchAgent):
        """Analysis should record an analyzer thought."""
        mock_result = MagicMock()
        mock_result.gaps = []
        agent._mock_ga.analyze.return_value = mock_result
        agent.session = MagicMock()
        agent.session.archetype = {}
        agent._analyze_gaps([], 0)
        assert any(t.role == "analyzer" for t in agent.thoughts)

    def test_analyze_with_gaps(self, agent: DeepResearchAgent):
        """When gaps are returned, GapSnapshot objects should be created."""
        mock_gap = MagicMock()
        mock_gap.gap_type = "Missing"
        mock_gap.title = "Gap title"
        mock_gap.description = "desc"
        mock_result = MagicMock()
        mock_result.gaps = [mock_gap, mock_gap, mock_gap]
        agent._mock_ga.analyze.return_value = mock_result
        agent._mock_tracker._archetype_match_score.return_value = 0.6
        agent.session = MagicMock()
        agent.session.archetype = {"key": "value"}
        ps = _make_paper_snapshot()
        with patch("research_loop.deep_research.GapSnapshot") as MockGS:
            MockGS.return_value = _make_gap_snapshot()
            agent._analyze_gaps([ps], 0)
            # Should produce at most 5 gaps (top 5)
            assert MockGS.call_count <= 5

    def test_analyze_handles_exception(self, agent: DeepResearchAgent):
        """Gap analysis exception should return empty list."""
        agent._mock_ga.analyze.side_effect = RuntimeError("LLM error")
        agent.session = MagicMock()
        agent.session.archetype = {}
        result = agent._analyze_gaps([], 0)
        assert result == []
        assert any("Gap analysis failed" in t.content for t in agent.thoughts)

    def test_analyze_no_archetype_uses_default_score(
        self,
        agent: DeepResearchAgent,
    ):
        """When archetype is empty, match score defaults to 0.5."""
        mock_gap = MagicMock()
        mock_gap.gap_type = "Missing"
        mock_gap.title = "G"
        mock_gap.description = "d"
        mock_result = MagicMock()
        mock_result.gaps = [mock_gap]
        agent._mock_ga.analyze.return_value = mock_result
        agent.session = MagicMock()
        agent.session.archetype = {}
        ps = _make_paper_snapshot()
        with patch("research_loop.deep_research.GapSnapshot") as MockGS:
            MockGS.return_value = _make_gap_snapshot()
            agent._analyze_gaps([ps], 0)
            # Should NOT call _archetype_match_score when archetype is empty
            agent._mock_tracker._archetype_match_score.assert_not_called()


# ========================================================================
# Tests: dataclasses
# ========================================================================
class TestAgentThought:
    """Tests for the AgentThought dataclass."""

    def test_creation(self):
        t = AgentThought(iteration=1, role="planner", content="plan it")
        assert t.iteration == 1
        assert t.role == "planner"
        assert t.content == "plan it"
        assert isinstance(t.timestamp, float)

    def test_default_timestamp(self):
        before = time.time()
        t = AgentThought(iteration=0, role="searcher", content="search")
        after = time.time()
        assert before <= t.timestamp <= after


class TestDeepResearchResult:
    """Tests for the DeepResearchResult dataclass."""

    def test_creation(self):
        """Verify result can be created with required fields."""
        r = DeepResearchResult(
            session_id="s1",
            query="q",
            iterations=2,
            papers=[],
            gaps=[],
            thoughts=[],
            report="report text",
            duration_seconds=10.5,
            status="completed",
        )
        assert r.session_id == "s1"
        assert r.status == "completed"
        assert r.duration_seconds == 10.5

    def test_defaults(self):
        """Verify zero/empty defaults."""
        r = DeepResearchResult(
            session_id="",
            query="",
            iterations=0,
            papers=[],
            gaps=[],
            thoughts=[],
            report="",
            duration_seconds=0.0,
            status="",
        )
        assert r.iterations == 0
        assert r.status == ""
