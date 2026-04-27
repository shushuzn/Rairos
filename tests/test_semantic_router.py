"""Tests for llm/semantic_router — pure logic, no I/O."""
import pytest

from llm.semantic_router import (
    QueryType,
    Route,
    _route_by_keyword,
    _QUERY_TYPE_TO_COMMAND,
)


# =============================================================================
# QueryType and mapping
# =============================================================================
class TestQueryType:
    def test_all_values(self):
        assert QueryType.GAP_ANALYSIS.value == "gap_analysis"
        assert QueryType.HYPOTHESIS_GENERATION.value == "hypothesis_generation"
        assert QueryType.EXPERIMENT.value == "experiment"
        assert QueryType.INSIGHT.value == "insight"
        assert QueryType.NARRATIVE.value == "narrative"
        assert QueryType.PAPER_SEARCH.value == "paper_search"
        assert QueryType.QUESTION_ANSWER.value == "question_answer"
        assert QueryType.GENERAL.value == "general"


class TestQueryTypeToCommand:
    def test_gap(self):
        assert _QUERY_TYPE_TO_COMMAND[QueryType.GAP_ANALYSIS] == "gap"

    def test_hypothesis(self):
        assert _QUERY_TYPE_TO_COMMAND[QueryType.HYPOTHESIS_GENERATION] == "hypothesize"

    def test_experiment(self):
        assert _QUERY_TYPE_TO_COMMAND[QueryType.EXPERIMENT] == "experiment"

    def test_insight(self):
        assert _QUERY_TYPE_TO_COMMAND[QueryType.INSIGHT] == "insight"

    def test_narrative(self):
        assert _QUERY_TYPE_TO_COMMAND[QueryType.NARRATIVE] == "narrative"

    def test_paper_search(self):
        assert _QUERY_TYPE_TO_COMMAND[QueryType.PAPER_SEARCH] == "search"

    def test_question_answer(self):
        assert _QUERY_TYPE_TO_COMMAND[QueryType.QUESTION_ANSWER] == "ask"

    def test_general(self):
        assert _QUERY_TYPE_TO_COMMAND[QueryType.GENERAL] == "chat"


# =============================================================================
# Route dataclass
# =============================================================================
class TestRoute:
    def test_defaults(self):
        r = Route(query_type=QueryType.GAP_ANALYSIS, confidence=0.9, primary_command="gap")
        assert r.query_type == QueryType.GAP_ANALYSIS
        assert r.confidence == 0.9
        assert r.primary_command == "gap"
        assert r.multi_intent is False
        assert r.sub_commands == []

    def test_multi_intent(self):
        r = Route(
            query_type=QueryType.GAP_ANALYSIS,
            confidence=0.9,
            primary_command="gap",
            multi_intent=True,
            secondary_query_type=QueryType.HYPOTHESIS_GENERATION,
            sub_commands=["gap", "hypothesize"],
        )
        assert r.multi_intent is True
        assert r.secondary_query_type == QueryType.HYPOTHESIS_GENERATION
        assert r.sub_commands == ["gap", "hypothesize"]

    def test_to_dict(self):
        r = Route(
            query_type=QueryType.EXPERIMENT,
            confidence=0.7,
            primary_command="experiment",
            reasoning="query mentions experiments",
            multi_intent=False,
        )
        d = r.to_dict()
        assert d["query_type"] == "experiment"
        assert d["confidence"] == 0.7
        assert d["primary_command"] == "experiment"
        assert d["reasoning"] == "query mentions experiments"
        assert d["multi_intent"] is False
        assert d["secondary_query_type"] is None
        assert d["sub_commands"] == []

    def test_to_dict_with_secondary(self):
        r = Route(
            query_type=QueryType.GAP_ANALYSIS,
            confidence=0.8,
            primary_command="gap",
            multi_intent=True,
            secondary_query_type=QueryType.HYPOTHESIS_GENERATION,
            sub_commands=["gap", "hypothesize"],
        )
        d = r.to_dict()
        assert d["multi_intent"] is True
        assert d["secondary_query_type"] == "hypothesis_generation"
        assert d["sub_commands"] == ["gap", "hypothesize"]


# =============================================================================
# Keyword routing — pure, no network
# =============================================================================
class TestKeywordRouting:
    """_route_by_keyword is deterministic and network-free."""

    def test_gap_chinese(self):
        r = _route_by_keyword("transformer 的研究空白有哪些？")
        assert r.query_type == QueryType.GAP_ANALYSIS
        assert r.primary_command == "gap"
        assert r.confidence > 0

    def test_gap_english(self):
        r = _route_by_keyword("what are the research gaps in RAG?")
        assert r.query_type == QueryType.GAP_ANALYSIS

    def test_hypothesis_chinese(self):
        r = _route_by_keyword("基于空白提出假设和预测")
        assert r.query_type == QueryType.HYPOTHESIS_GENERATION

    def test_hypothesis_english(self):
        r = _route_by_keyword("generate a hypothesis and make predictions")
        assert r.query_type == QueryType.HYPOTHESIS_GENERATION

    def test_experiment(self):
        r = _route_by_keyword("run an experiment to validate")
        assert r.query_type == QueryType.EXPERIMENT
        assert r.primary_command == "experiment"

    def test_paper_search(self):
        r = _route_by_keyword("find papers about attention mechanism")
        assert r.query_type == QueryType.PAPER_SEARCH
        assert r.primary_command == "search"

    def test_insight(self):
        r = _route_by_keyword("key insights from these papers")
        assert r.query_type == QueryType.INSIGHT
        assert r.primary_command == "insight"

    def test_narrative(self):
        r = _route_by_keyword("track my research progress on transformers")
        assert r.query_type == QueryType.NARRATIVE
        assert r.primary_command == "narrative"

    def test_question_answer(self):
        r = _route_by_keyword("how does scaled dot-product attention work?")
        assert r.query_type == QueryType.QUESTION_ANSWER
        assert r.primary_command == "ask"

    def test_general(self):
        r = _route_by_keyword("let's chat about transformers")
        assert r.query_type == QueryType.GENERAL
        assert r.primary_command == "chat"

    def test_confidence_bounded(self):
        r = _route_by_keyword("gap gap gap gap")
        assert 0.0 <= r.confidence <= 1.0

    def test_reasoning_contains_keyword_marker(self):
        r = _route_by_keyword("what are the gaps")
        assert "[keyword fallback" in r.reasoning


# =============================================================================
# SemanticRouter — routing and execution (mocked)
# =============================================================================
class TestSemanticRouter:
    def test_route_uses_keyword_when_llm_fails(self):
        """When LLM is unavailable, route() falls back to keyword routing."""
        from llm.semantic_router import SemanticRouter
        router = SemanticRouter()
        r = router.route("transformer 的研究空白")
        # LLM will fail (no API key in test), so falls back to keyword
        assert r.query_type == QueryType.GAP_ANALYSIS
        assert r.primary_command == "gap"
        assert 0.0 <= r.confidence <= 1.0

    def test_route_returns_valid_route_object(self):
        from llm.semantic_router import SemanticRouter
        router = SemanticRouter()
        r = router.route("design an experiment for my hypothesis")
        assert isinstance(r, Route)
        assert isinstance(r.query_type, QueryType)
        assert isinstance(r.primary_command, str)
        assert r.primary_command in _QUERY_TYPE_TO_COMMAND.values()


class TestRouterExecute:
    def test_execute_returns_dict(self):
        """execute() returns a dict keyed by command name."""
        from llm.semantic_router import SemanticRouter, Route
        router = SemanticRouter()
        r = Route(
            query_type=QueryType.PAPER_SEARCH,
            confidence=0.9,
            primary_command="search",
        )
        outputs = router.execute(r, "transformer", exec_all=False)
        assert isinstance(outputs, dict)

    def test_execute_error_isolation(self):
        """One failing command does not stop the chain."""
        from llm.semantic_router import SemanticRouter, Route
        router = SemanticRouter()
        r = Route(
            query_type=QueryType.GAP_ANALYSIS,
            confidence=0.9,
            primary_command="gap",
            multi_intent=True,
            secondary_query_type=QueryType.HYPOTHESIS_GENERATION,
            sub_commands=["gap", "nonexistent_command_xyz"],
        )
        outputs = router.execute(r, "test", exec_all=True)
        assert isinstance(outputs, dict)
        assert len(outputs) >= 1
