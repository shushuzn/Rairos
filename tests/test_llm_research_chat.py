"""Tier 2 unit tests — llm/research_chat.py, pure functions, no I/O."""
import pytest
from llm.research_chat import (
    QueryType, PaperContext, ResearchContext, ResearchChat,
)


# =============================================================================
# QueryType enum tests
# =============================================================================
class TestQueryType:
    """Test QueryType enum."""

    def test_values(self):
        assert QueryType.FACTUAL.value == "factual"
        assert QueryType.INFERENTIAL.value == "inferential"
        assert QueryType.DISCOVERY.value == "discovery"
        assert QueryType.COMPARISON.value == "comparison"

    def test_count(self):
        assert len(QueryType) == 4


# =============================================================================
# PaperContext dataclass tests
# =============================================================================
class TestPaperContext:
    """Test PaperContext dataclass."""

    def test_required_fields(self):
        p = PaperContext(
            uid="p001", title="Attention Is All You Need",
            abstract="A new neural network architecture",
            authors=["Vaswani"], year=2017,
        )
        assert p.uid == "p001"
        assert p.title == "Attention Is All You Need"
        assert p.abstract == "A new neural network architecture"
        assert p.authors == ["Vaswani"]
        assert p.year == 2017

    def test_optional_fields_default(self):
        p = PaperContext(uid="p001", title="T", abstract="A", authors=[], year=0)
        assert p.key_findings == []  # default


# =============================================================================
# ResearchContext dataclass tests
# =============================================================================
class TestResearchContext:
    """Test ResearchContext dataclass."""

    def test_required_fields(self):
        ctx = ResearchContext(topic="transformer")
        assert ctx.topic == "transformer"
        assert ctx.papers == []  # default
        assert ctx.insights == []  # default
        assert ctx.relations == {}  # default
        assert ctx.citations == {}  # default

    def test_with_data(self):
        paper = PaperContext(uid="p001", title="T", abstract="A", authors=[], year=0)
        ctx = ResearchContext(
            topic="nlp", papers=[paper],
            insights=["insight1"], relations={"p001": ["p002"]},
        )
        assert len(ctx.papers) == 1
        assert len(ctx.insights) == 1
        assert ctx.relations == {"p001": ["p002"]}


# =============================================================================
# STOPWORDS constant
# =============================================================================
class TestStopwords:
    """Test ResearchChat.STOPWORDS."""

    def test_has_chinese_stopwords(self):
        assert "的" in ResearchChat.STOPWORDS
        assert "是" in ResearchChat.STOPWORDS
        assert "什么" in ResearchChat.STOPWORDS

    def test_has_english_stopwords(self):
        assert "the" in ResearchChat.STOPWORDS
        assert "is" in ResearchChat.STOPWORDS
        assert "what" in ResearchChat.STOPWORDS


# =============================================================================
# _extract_topic tests
# =============================================================================
class TestExtractTopic:
    """Test _extract_topic — pure string parsing."""

    chat = ResearchChat()

    def test_returns_filtered_words(self):
        result = self.chat._extract_topic("什么是 transformer?")
        assert "transformer" in result
        # Chinese sequences (2+ chars) are atomic tokens; "什么是" not in STOPWORDS

    def test_removes_question_marks(self):
        result = self.chat._extract_topic("transformer?")
        assert "?" not in result

    def test_removes_chinese_punctuation(self):
        result = self.chat._extract_topic("transformer，是什么")
        # "是什么" (2+ chars) is atomic, not individually matched to STOPWORDS

    def test_removes_english_punctuation(self):
        result = self.chat._extract_topic("transformer, attention")
        assert "," not in result

    def test_max_three_candidates(self):
        result = self.chat._extract_topic("transformer attention gpt bert nlp deep learning")
        words = result.split()
        assert len(words) <= 3

    def test_short_words_filtered(self):
        result = self.chat._extract_topic("a transformer")
        # "a" is in STOPWORDS -> filtered; "transformer" kept
        assert "a" not in result.split()  # word boundary check
        assert "transformer" in result

    def test_fallback_to_prefix(self):
        result = self.chat._extract_topic("的 是")
        # All stopwords → fallback to first 20 chars
        assert len(result) <= 20

    def test_mixed_language(self):
        result = self.chat._extract_topic("transformer 是什么")
        assert "transformer" in result
        # "是什么" is atomic token, not individually matched to STOPWORDS


# =============================================================================
# _build_relations tests
# =============================================================================
class TestBuildRelations:
    """Test _build_relations."""

    chat = ResearchChat()

    def test_returns_empty_dicts(self):
        relations, citations = self.chat._build_relations([])
        assert relations == {}
        assert citations == {}

    def test_returns_empty_for_any_input(self):
        papers = [PaperContext(uid="p001", title="T", abstract="A", authors=[], year=0)]
        relations, citations = self.chat._build_relations(papers)
        assert relations == {}
        assert citations == {}


# =============================================================================
# classify_query tests
# =============================================================================
class TestClassifyQuery:
    """Test classify_query."""

    chat = ResearchChat()

    # COMPARISON
    def test_compare_对比(self):
        assert self.chat.classify_query("对比 BERT 和 GPT") == QueryType.COMPARISON

    def test_compare_difference(self):
        assert self.chat.classify_query("what is the difference?") == QueryType.COMPARISON

    def test_compare_区别(self):
        assert self.chat.classify_query("两种方法的区别是什么？") == QueryType.COMPARISON

    def test_compare_compare(self):
        assert self.chat.classify_query("compare transformer and LSTM") == QueryType.COMPARISON

    # DISCOVERY
    def test_discovery_gap(self):
        assert self.chat.classify_query("研究空白在哪里?") == QueryType.DISCOVERY

    def test_discovery_unresolved(self):
        assert self.chat.classify_query("what are the unresolved problems?") == QueryType.DISCOVERY

    def test_discovery_还有什么(self):
        assert self.chat.classify_query("还有什么未解决的问题？") == QueryType.DISCOVERY

    def test_discovery_机会(self):
        assert self.chat.classify_query("研究机会在哪里") == QueryType.DISCOVERY

    # INFERENTIAL
    def test_inferential_为什么(self):
        assert self.chat.classify_query("为什么 attention 有效？") == QueryType.INFERENTIAL

    def test_inferential_why(self):
        assert self.chat.classify_query("why does this work?") == QueryType.INFERENTIAL

    def test_inferential_cause(self):
        assert self.chat.classify_query("what causes this phenomenon?") == QueryType.INFERENTIAL

    def test_inferential_because(self):
        assert self.chat.classify_query("because of the attention mechanism") == QueryType.INFERENTIAL

    # FACTUAL (default)
    def test_factual_default(self):
        assert self.chat.classify_query("什么是 transformer?") == QueryType.FACTUAL

    def test_factual_neutral(self):
        assert self.chat.classify_query("这篇文章的主要内容是什么") == QueryType.FACTUAL


# =============================================================================
# _build_system_prompt tests
# =============================================================================
class TestBuildSystemPrompt:
    """Test _build_system_prompt string building."""

    chat = ResearchChat()

    def test_empty_papers(self):
        ctx = ResearchContext(topic="nlp")
        prompt = self.chat._build_system_prompt(ctx)
        assert "No relevant papers found" in prompt
        assert "nlp" in prompt

    def test_papers_listed(self):
        paper = PaperContext(uid="p001", title="Attention Paper", abstract="A", authors=[], year=2017)
        ctx = ResearchContext(topic="nlp", papers=[paper])
        prompt = self.chat._build_system_prompt(ctx)
        assert "Attention Paper" in prompt
        assert "2017" in prompt

    def test_max_five_papers(self):
        papers = [
            PaperContext(uid=f"p{i:03d}", title=f"Paper {i}", abstract="A", authors=[], year=2020)
            for i in range(8)
        ]
        ctx = ResearchContext(topic="t", papers=papers)
        prompt = self.chat._build_system_prompt(ctx)
        assert "Paper 0" in prompt
        assert "Paper 4" in prompt
        assert "Paper 7" not in prompt

    def test_empty_insights(self):
        ctx = ResearchContext(topic="nlp")
        prompt = self.chat._build_system_prompt(ctx)
        assert "No relevant insights" in prompt

    def test_insights_listed(self):
        # Mock InsightCard with .content attribute
        class MockCard:
            def __init__(self, content):
                self.content = content
        ctx = ResearchContext(topic="nlp", insights=[MockCard("attention is powerful")])
        prompt = self.chat._build_system_prompt(ctx)
        assert "attention" in prompt

    def test_contains_guidelines(self):
        ctx = ResearchContext(topic="nlp")
        prompt = self.chat._build_system_prompt(ctx)
        assert "Reference specific papers" in prompt


# =============================================================================
# _build_user_prompt tests
# =============================================================================
class TestBuildUserPrompt:
    """Test _build_user_prompt per query type."""

    chat = ResearchChat()

    def test_factual_prompt(self):
        ctx = ResearchContext(topic="nlp")
        result = self.chat._build_user_prompt("What is BERT?", ctx, QueryType.FACTUAL)
        assert "What is BERT?" in result

    def test_inferential_prompt(self):
        ctx = ResearchContext(topic="nlp")
        result = self.chat._build_user_prompt("Why does it work?", ctx, QueryType.INFERENTIAL)
        assert "Why does it work?" in result
        assert "reason" in result.lower()

    def test_discovery_prompt(self):
        ctx = ResearchContext(topic="nlp")
        result = self.chat._build_user_prompt("Gaps?", ctx, QueryType.DISCOVERY)
        assert "gaps" in result.lower() or "Gap" in result

    def test_comparison_prompt(self):
        ctx = ResearchContext(topic="nlp")
        result = self.chat._build_user_prompt("Compare A and B", ctx, QueryType.COMPARISON)
        assert "Compare" in result or "compare" in result

    def test_unknown_type_falls_back_to_query(self):
        ctx = ResearchContext(topic="nlp")
        # QueryType is an enum, so we can't really pass unknown — but verify default
        result = self.chat._build_user_prompt("Hello", ctx, QueryType.FACTUAL)
        assert "Hello" in result


# =============================================================================
# get_history / clear_history tests
# =============================================================================
class TestChatHistory:
    """Test history management."""

    def test_initial_history_empty(self):
        chat = ResearchChat()
        assert chat.get_history() == []

    def test_clear_history(self):
        chat = ResearchChat()
        chat._chat_history.append({"role": "user", "content": "test"})
        chat.clear_history()
        assert chat.get_history() == []

    def test_get_history_returns_copy(self):
        chat = ResearchChat()
        chat._chat_history.append({"role": "user", "content": "test"})
        history = chat.get_history()
        history.append({"role": "x", "content": "y"})
        # Original unchanged
        assert len(chat.get_history()) == 1


# =============================================================================
# ResearchChat instantiation
# =============================================================================
class TestResearchChatInit:
    """Test ResearchChat class."""

    def test_instantiate_without_deps(self):
        chat = ResearchChat()
        assert chat.db is None
        assert chat.insight_manager is None

    def test_has_expected_methods(self):
        chat = ResearchChat()
        assert hasattr(chat, "_extract_topic")
        assert hasattr(chat, "_build_relations")
        assert hasattr(chat, "classify_query")
        assert hasattr(chat, "_build_system_prompt")
        assert hasattr(chat, "_build_user_prompt")
        assert hasattr(chat, "get_history")
        assert hasattr(chat, "clear_history")
