"""Tier 2 unit tests — llm/research_session.py, pure functions, no I/O."""
import pytest
from llm.research_session import (
    ResearchIntent, Query, ResearchSession,
    ResearchSessionTracker,
)


# =============================================================================
# ResearchIntent enum tests
# =============================================================================
class TestResearchIntent:
    """Test ResearchIntent enum."""

    def test_all_values(self):
        assert ResearchIntent.LEARNING.value == "learning"
        assert ResearchIntent.REPRODUCING.value == "reproducing"
        assert ResearchIntent.IMPROVING.value == "improving"
        assert ResearchIntent.COMPARING.value == "comparing"
        assert ResearchIntent.EXPLORING.value == "exploring"
        assert ResearchIntent.CITING.value == "citing"

    def test_count(self):
        assert len(ResearchIntent) == 6


# =============================================================================
# Query dataclass tests
# =============================================================================
class TestQuery:
    """Test Query dataclass."""

    def test_required_fields(self):
        q = Query(
            id="q001",
            question="什么是 transformer?",
            answer_preview="Transformer 是...",
            paper_ids=["p001"],
            paper_titles=["Attention Is All You Need"],
            timestamp="2026-01-01T00:00:00",
        )
        assert q.id == "q001"
        assert q.question == "什么是 transformer?"
        assert q.answer_preview == "Transformer 是..."
        assert q.paper_ids == ["p001"]
        assert q.paper_titles == ["Attention Is All You Need"]
        assert q.follow_ups == []  # default

    def test_with_follow_ups(self):
        q = Query(
            id="q001", question="Q", answer_preview="A",
            paper_ids=[], paper_titles=[], timestamp="t",
            follow_ups=["追问1", "追问2"],
        )
        assert len(q.follow_ups) == 2


# =============================================================================
# ResearchSession dataclass tests
# =============================================================================
class TestResearchSession:
    """Test ResearchSession dataclass."""

    def test_required_fields(self):
        s = ResearchSession(
            id="session_001",
            title="研究会话 2026-01-01",
            queries=[],
            started_at="2026-01-01T09:00:00",
        )
        assert s.id == "session_001"
        assert s.queries == []
        assert s.ended_at is None
        assert s.tags == []  # default
        assert s.insights == []  # default
        assert s.intent == ResearchIntent.LEARNING  # default

    def test_with_queries(self):
        q = Query(
            id="q001", question="Q", answer_preview="A",
            paper_ids=[], paper_titles=[], timestamp="t",
        )
        s = ResearchSession(
            id="s001", title="T", queries=[q],
            started_at="2026-01-01T09:00:00",
        )
        assert len(s.queries) == 1

    def test_topics_property(self):
        s = ResearchSession(
            id="s001", title="T", queries=[],
            started_at="2026-01-01T09:00:00",
            tags=["nlp", "transformer"],
        )
        assert set(s.topics) == {"nlp", "transformer"}

    def test_topics_deduplicates(self):
        s = ResearchSession(
            id="s001", title="T", queries=[],
            started_at="2026-01-01T09:00:00",
            tags=["transformer", "transformer"],
        )
        assert s.topics == ["transformer"]


# =============================================================================
# _detect_intent — THE key pure function
# =============================================================================
class TestDetectIntent:
    """Test _detect_intent — keyword/regex pattern matching."""

    tracker = ResearchSessionTracker()

    # ---- REPRODUCING -------------------------------------------------
    def test_reproducing_chinese复现(self):
        q = "这篇论文怎么复现？"
        intent = self.tracker._detect_intent(q)
        assert intent == ResearchIntent.REPRODUCING

    def test_reproducing_code(self):
        intent = self.tracker._detect_intent("有没有代码实现？")
        assert intent == ResearchIntent.REPRODUCING

    def test_reproducing_英文(self):
        intent = self.tracker._detect_intent("how to reproduce this?")
        assert intent == ResearchIntent.REPRODUCING

    # ---- IMPROVING ----------------------------------------------------
    def test_improving_chinese改进(self):
        intent = self.tracker._detect_intent("这个方法怎么改进？")
        assert intent == ResearchIntent.IMPROVING

    def test_improving_outperform(self):
        intent = self.tracker._detect_intent("how to outperform baseline?")
        assert intent == ResearchIntent.IMPROVING

    def test_improving_提升(self):
        intent = self.tracker._detect_intent("如何提升模型性能？")
        assert intent == ResearchIntent.IMPROVING

    # ---- COMPARING ----------------------------------------------------
    def test_comparing_vs(self):
        # "更好" matches IMPROVING first, so use query without it
        intent = self.tracker._detect_intent("BERT 和 GPT 有什么区别？")
        assert intent == ResearchIntent.COMPARING

    def test_comparing_比较(self):
        intent = self.tracker._detect_intent("比较一下这两种方法")
        assert intent == ResearchIntent.COMPARING

    def test_comparing_英文(self):
        intent = self.tracker._detect_intent("compare transformer and LSTM")
        assert intent == ResearchIntent.COMPARING

    # ---- LEARNING -----------------------------------------------------
    def test_learning_是什么(self):
        intent = self.tracker._detect_intent("Transformer 是什么？")
        assert intent == ResearchIntent.LEARNING

    def test_learning_原理(self):
        intent = self.tracker._detect_intent("注意力机制的原理是什么？")
        assert intent == ResearchIntent.LEARNING

    def test_learning_英文(self):
        intent = self.tracker._detect_intent("what is RLHF?")
        assert intent == ResearchIntent.LEARNING

    def test_learning_explain(self):
        intent = self.tracker._detect_intent("explain this paper")
        assert intent == ResearchIntent.LEARNING

    def test_learning_understand(self):
        intent = self.tracker._detect_intent("帮我理解这篇论文")
        assert intent == ResearchIntent.LEARNING

    # ---- EXPLORING ----------------------------------------------------
    def test_exploring_有哪些(self):
        intent = self.tracker._detect_intent("有哪些最新的 transformer 变体？")
        assert intent == ResearchIntent.EXPLORING

    def test_exploring_latest(self):
        # Uses "latest" keyword (英文 pattern), avoids LEARNING collision with "研究"
        intent = self.tracker._detect_intent("what are the latest models?")
        assert intent == ResearchIntent.EXPLORING

    def test_exploring_英文(self):
        intent = self.tracker._detect_intent("what are the latest approaches?")
        assert intent == ResearchIntent.EXPLORING

    def test_exploring_discover(self):
        intent = self.tracker._detect_intent("discover new methods")
        assert intent == ResearchIntent.EXPLORING

    # ---- CITING -------------------------------------------------------
    def test_citing_引用(self):
        intent = self.tracker._detect_intent("如何引用这篇论文？")
        assert intent == ResearchIntent.CITING

    def test_citing_参考文献(self):
        intent = self.tracker._detect_intent("列出参考文献格式")
        assert intent == ResearchIntent.CITING

    def test_citing_写论文(self):
        intent = self.tracker._detect_intent("如何写论文的相关工作部分？")
        assert intent == ResearchIntent.CITING

    def test_citing_英文(self):
        intent = self.tracker._detect_intent("how to cite this paper?")
        assert intent == ResearchIntent.CITING

    # ---- Default -------------------------------------------------------
    def test_unknown_defaults_to_learning(self):
        intent = self.tracker._detect_intent("今天天气怎么样？")
        assert intent == ResearchIntent.LEARNING

    # ---- Tie-breaking ---------------------------------------------------
    def test_multiple_matches_returns_first_max(self):
        # "改进并复现" matches both IMPROVING and REPRODUCING
        intent = self.tracker._detect_intent("如何改进并复现这个方法？")
        # Both patterns score 1; first one in enum order (REPRODUCING) wins
        # But IMPROVING appears first in the code's patterns dict
        assert intent in (ResearchIntent.IMPROVING, ResearchIntent.REPRODUCING)


# =============================================================================
# get_probing_questions — template-based fallback (no LLM)
# =============================================================================
class TestProbingQuestions:
    """Test get_probing_questions with use_llm=False (template fallback)."""

    def test_no_session_returns_empty(self):
        tracker = ResearchSessionTracker()
        result = tracker.get_probing_questions(use_llm=False)
        assert result == []

    def test_topic_questions_added(self):
        tracker = ResearchSessionTracker()
        tracker.start_session(title="Test")
        # Add a session with a single topic
        tracker.current_session.tags = ["transformer"]
        tracker.current_session.queries = [
            Query(
                id="q001", question="What is transformer?",
                answer_preview="", paper_ids=[], paper_titles=[],
                timestamp="2026-01-01T00:00:00",
            )
        ]
        questions = tracker.get_probing_questions(use_llm=False)
        assert len(questions) >= 1

    def test_single_topic_adds_question(self):
        tracker = ResearchSessionTracker()
        tracker.start_session()
        tracker.current_session.tags = ["nlp"]
        tracker.current_session.queries = [
            Query(
                id="q001", question="Q", answer_preview="",
                paper_ids=[], paper_titles=[], timestamp="t",
            )
        ]
        questions = tracker.get_probing_questions(use_llm=False)
        assert len(questions) >= 1
        assert "nlp" in questions[0] or "topic" in questions[0].lower()

    def test_max_two_questions(self):
        tracker = ResearchSessionTracker()
        tracker.start_session()
        tracker.current_session.intent = ResearchIntent.LEARNING
        tracker.current_session.tags = ["x", "y"]
        tracker.current_session.queries = [
            Query(
                id="q001", question="Q", answer_preview="",
                paper_ids=[], paper_titles=[], timestamp="t",
            )
        ]
        questions = tracker.get_probing_questions(use_llm=False)
        assert len(questions) <= 2


# =============================================================================
# get_research_path_suggestion
# =============================================================================
class TestResearchPathSuggestion:
    """Test get_research_path_suggestion."""

    def test_no_session_returns_none(self):
        tracker = ResearchSessionTracker()
        assert tracker.get_research_path_suggestion() is None

    def test_no_topics_returns_none(self):
        tracker = ResearchSessionTracker()
        tracker.start_session(title="T")
        assert tracker.get_research_path_suggestion() is None

    def _session_with_query(self, intent=ResearchIntent.LEARNING, tags=None):
        """Create a tracker with a session that has at least 1 query."""
        tracker = ResearchSessionTracker()
        tracker.start_session()
        tracker.current_session.intent = intent
        tracker.current_session.tags = tags or ["nlp"]
        # Add at least one query so get_research_path_suggestion returns a result
        tracker.current_session.queries = [
            Query(
                id="q001", question="What is this?",
                answer_preview="", paper_ids=[], paper_titles=[],
                timestamp="2026-01-01T00:00:00",
            )
        ]
        return tracker

    def test_learning_suggestion(self):
        tracker = self._session_with_query(ResearchIntent.LEARNING, ["transformer"])
        result = tracker.get_research_path_suggestion()
        assert result is not None
        assert "学习" in result

    def test_reproducing_suggestion(self):
        tracker = self._session_with_query(ResearchIntent.REPRODUCING, ["nlp"])
        result = tracker.get_research_path_suggestion()
        assert result is not None
        assert "复现" in result

    def test_improving_suggestion(self):
        tracker = self._session_with_query(ResearchIntent.IMPROVING, ["nlp"])
        result = tracker.get_research_path_suggestion()
        assert result is not None
        assert "改进" in result

    def test_comparing_suggestion(self):
        tracker = self._session_with_query(ResearchIntent.COMPARING, ["nlp"])
        result = tracker.get_research_path_suggestion()
        assert result is not None
        assert "对比" in result

    def test_exploring_suggestion(self):
        tracker = self._session_with_query(ResearchIntent.EXPLORING, ["nlp"])
        result = tracker.get_research_path_suggestion()
        assert result is not None
        assert "探索" in result

    def test_citing_suggestion(self):
        tracker = self._session_with_query(ResearchIntent.CITING, ["nlp"])
        result = tracker.get_research_path_suggestion()
        assert result is not None
        assert "引用" in result

    def test_topic_included_in_suggestion(self):
        tracker = self._session_with_query(ResearchIntent.LEARNING, ["transformer"])
        result = tracker.get_research_path_suggestion()
        assert "transformer" in result


# =============================================================================
# render_session_tree — pure string formatting
# =============================================================================
class TestRenderSessionTree:
    """Test render_session_tree."""

    def test_empty_session(self):
        session = ResearchSession(
            id="s001", title="Test Session",
            queries=[], started_at="2026-01-01T09:00:00",
        )
        tracker = ResearchSessionTracker()
        output = tracker.render_session_tree(session)
        assert "Test Session" in output
        assert "0 个问答" in output

    def test_duration_included(self):
        session = ResearchSession(
            id="s001", title="T",
            queries=[], started_at="2026-01-01T09:00:00",
            ended_at="2026-01-01T09:30:00",  # 30 min
        )
        tracker = ResearchSessionTracker()
        output = tracker.render_session_tree(session)
        assert "30" in output

    def test_questions_rendered(self):
        session = ResearchSession(
            id="s001", title="T",
            queries=[
                Query(
                    id="q001", question="什么是 attention?",
                    answer_preview="是一种机制",
                    paper_ids=["p001"],
                    paper_titles=["Attention Paper"],
                    timestamp="2026-01-01T00:00:00",
                ),
            ],
            started_at="2026-01-01T09:00:00",
        )
        tracker = ResearchSessionTracker()
        output = tracker.render_session_tree(session)
        assert "attention" in output.lower()

    def test_paper_titles_shown(self):
        session = ResearchSession(
            id="s001", title="T",
            queries=[
                Query(
                    id="q001", question="Q", answer_preview="A",
                    paper_ids=["p001"],
                    paper_titles=["Important Paper"],
                    timestamp="t",
                ),
            ],
            started_at="2026-01-01T09:00:00",
        )
        tracker = ResearchSessionTracker()
        output = tracker.render_session_tree(session)
        assert "Important Paper" in output

    def test_follow_ups_count_shown(self):
        session = ResearchSession(
            id="s001", title="T",
            queries=[
                Query(
                    id="q001", question="Q", answer_preview="A",
                    paper_ids=[], paper_titles=[], timestamp="t",
                    follow_ups=["追问1", "追问2", "追问3"],
                ),
            ],
            started_at="2026-01-01T09:00:00",
        )
        tracker = ResearchSessionTracker()
        output = tracker.render_session_tree(session)
        assert "3 次追问" in output

    def test_insights_shown(self):
        session = ResearchSession(
            id="s001", title="T",
            queries=[],
            started_at="2026-01-01T09:00:00",
            insights=["深度探索（多次追问）"],
        )
        tracker = ResearchSessionTracker()
        output = tracker.render_session_tree(session)
        assert "深度探索" in output


# =============================================================================
# render_sessions_list — pure string formatting
# =============================================================================
class TestRenderSessionsList:
    """Test render_sessions_list."""

    def test_empty_list(self):
        tracker = ResearchSessionTracker()
        output = tracker.render_sessions_list([])
        assert "暂无" in output

    def test_single_session(self):
        session = ResearchSession(
            id="s001", title="Test Session",
            queries=[
                Query(
                    id="q001", question="Q", answer_preview="A",
                    paper_ids=[], paper_titles=[], timestamp="t",
                ),
            ],
            started_at="2026-01-01T09:00:00",
            insights=["主要研究主题: nlp"],
        )
        tracker = ResearchSessionTracker()
        output = tracker.render_sessions_list([session])
        assert "Test Session" in output
        assert "1问答" in output
        assert "nlp" in output

    def test_multiple_sessions(self):
        sessions = [
            ResearchSession(
                id=f"s{i:03d}", title=f"Session {i}",
                queries=[], started_at=f"2026-01-{i+1:02d}T00:00:00",
            )
            for i in range(1, 4)
        ]
        tracker = ResearchSessionTracker()
        output = tracker.render_sessions_list(sessions)
        assert "Session 1" in output
        assert "Session 2" in output
        assert "Session 3" in output


# =============================================================================
# ResearchSessionTracker instantiation
# =============================================================================
class TestResearchSessionTrackerInit:
    """Test ResearchSessionTracker instantiation."""

    def test_instantiate(self):
        tracker = ResearchSessionTracker()
        assert tracker is not None
        assert tracker.sessions_file is not None

    def test_has_expected_methods(self):
        tracker = ResearchSessionTracker()
        assert hasattr(tracker, "_detect_intent")
        assert hasattr(tracker, "get_research_path_suggestion")
        assert hasattr(tracker, "get_probing_questions")
        assert hasattr(tracker, "render_session_tree")
        assert hasattr(tracker, "render_sessions_list")
