"""Tests for research chat."""
import pytest
from unittest.mock import MagicMock, patch

from llm.research_chat import (
    ResearchChat,
    ResearchContext,
    PaperContext,
    QueryType,
)


class TestQueryClassification:
    """Test query classification."""

    @pytest.fixture
    def chat(self):
        return ResearchChat()

    def test_factual_query(self, chat):
        """Test factual query classification."""
        q = "RAG 的主要方法有哪些？"
        assert chat.classify_query(q) == QueryType.FACTUAL

    def test_comparison_query(self, chat):
        """Test comparison query classification."""
        q = "Self-RAG 和 Corrective-RAG 有什么区别？"
        assert chat.classify_query(q) == QueryType.COMPARISON

    def test_discovery_query(self, chat):
        """Test discovery query classification."""
        q = "RAG 还有什么未解决的问题？"
        assert chat.classify_query(q) == QueryType.DISCOVERY

    def test_inferential_query(self, chat):
        """Test inferential query classification."""
        q = "为什么检索增强能提高生成质量？"
        assert chat.classify_query(q) == QueryType.INFERENTIAL

    def test_english_queries(self, chat):
        """Test English query classification."""
        assert chat.classify_query("What is RAG?") == QueryType.FACTUAL
        assert chat.classify_query("Compare RAG and fine-tuning") == QueryType.COMPARISON


class TestTopicExtraction:
    """Test topic extraction from queries."""

    @pytest.fixture
    def chat(self):
        return ResearchChat()

    def test_extract_chinese_topic(self, chat):
        """Test Chinese topic extraction."""
        topic = chat._extract_topic("RAG 在长文档场景的主要挑战是什么？")
        assert "RAG" in topic
        # Chinese characters without spaces might stay together

    def test_extract_english_topic(self, chat):
        """Test English topic extraction."""
        topic = chat._extract_topic("What are the methods for RAG?")
        assert "RAG" in topic

    def test_extract_empty_query(self, chat):
        """Test empty query handling."""
        topic = chat._extract_topic("")
        assert len(topic) <= 20


class TestContextBuilding:
    """Test research context building."""

    @pytest.fixture
    def mock_db(self):
        """Create mock database."""
        db = MagicMock()
        mock_paper = MagicMock()
        mock_paper.uid = "p1"
        mock_paper.title = "Test Paper"
        mock_paper.abstract = "Test abstract"
        mock_paper.authors = "Author A"
        mock_paper.year = 2023
        db.search_papers.return_value = ([mock_paper], None)
        return db

    @pytest.fixture
    def mock_insight_manager(self):
        """Create mock insight manager."""
        manager = MagicMock()
        manager.search_cards.return_value = []
        return manager

    def test_build_context_with_db(self, mock_db):
        """Test context building with database."""
        chat = ResearchChat(db=mock_db)
        ctx = chat.build_context("RAG methods")

        assert ctx.topic is not None
        assert len(ctx.papers) == 1
        assert ctx.papers[0].title == "Test Paper"

    def test_build_context_without_db(self):
        """Test context building without database."""
        chat = ResearchChat(db=None)
        ctx = chat.build_context("RAG methods")

        assert ctx.papers == []

    def test_build_context_with_insights(self, mock_db, mock_insight_manager):
        """Test context building with insights."""
        mock_insight = MagicMock()
        mock_insight.content = "Key finding"
        mock_insight_manager.search_cards.return_value = [mock_insight]

        chat = ResearchChat(db=mock_db, insight_manager=mock_insight_manager)
        ctx = chat.build_context("RAG")

        assert len(ctx.insights) == 1

    def test_build_context_with_hint(self, mock_db):
        """Test context building with topic hint."""
        chat = ResearchChat(db=mock_db)
        ctx = chat.build_context("methods?", topic_hint="RAG")

        assert ctx.topic == "RAG"


class TestPromptBuilding:
    """Test prompt building."""

    @pytest.fixture
    def chat(self):
        return ResearchChat()

    @pytest.fixture
    def sample_context(self):
        """Create sample research context."""
        paper = PaperContext(
            uid="p1",
            title="Test Paper",
            abstract="Test abstract",
            authors=["Author A"],
            year=2023,
        )
        return ResearchContext(
            topic="RAG",
            papers=[paper],
            insights=[],
        )

    def test_system_prompt_includes_topic(self, chat, sample_context):
        """Test system prompt includes topic."""
        prompt = chat._build_system_prompt(sample_context)
        assert "RAG" in prompt

    def test_system_prompt_includes_papers(self, chat, sample_context):
        """Test system prompt includes papers."""
        prompt = chat._build_system_prompt(sample_context)
        assert "Test Paper" in prompt

    def test_user_prompt_factual(self, chat, sample_context):
        """Test factual user prompt."""
        prompt = chat._build_user_prompt(
            "What is RAG?",
            sample_context,
            QueryType.FACTUAL,
        )
        assert "What is RAG" in prompt

    def test_user_prompt_discovery(self, chat, sample_context):
        """Test discovery user prompt."""
        prompt = chat._build_user_prompt(
            "What are the gaps?",
            sample_context,
            QueryType.DISCOVERY,
        )
        assert "gap" in prompt.lower()


class TestChatHistory:
    """Test chat history management."""

    @pytest.fixture
    def chat(self):
        return ResearchChat()

    @pytest.fixture
    def mock_llm_response(self):
        """Mock LLM response."""
        with patch("llm.client.call_llm_chat_completions") as mock:
            mock.return_value = "Test response"
            yield mock

    def test_get_history_empty(self, chat):
        """Test empty history."""
        assert chat.get_history() == []

    def test_clear_history(self, chat):
        """Test clearing history."""
        chat._chat_history.append({"role": "user", "content": "test"})
        chat.clear_history()
        assert chat.get_history() == []

    def test_history_after_chat(self, chat, mock_llm_response):
        """Test history after chat."""
        chat.chat("What is RAG?")
        history = chat.get_history()

        assert len(history) == 2
        assert history[0]["role"] == "user"
        assert history[1]["role"] == "assistant"


class TestResearchChatIntegration:
    """Integration tests for ResearchChat."""

    @pytest.fixture
    def mock_db(self):
        """Create mock database."""
        db = MagicMock()
        mock_paper = MagicMock()
        mock_paper.uid = "p1"
        mock_paper.title = "RAG Paper"
        mock_paper.abstract = "Retrieval augmented generation"
        mock_paper.authors = "Author A, Author B"
        mock_paper.year = 2023
        db.search_papers.return_value = ([mock_paper], None)
        return db

    def test_chat_returns_response(self, mock_db):
        """Test chat returns a response."""
        chat = ResearchChat(db=mock_db)

        with patch("llm.client.call_llm_chat_completions") as mock:
            mock.return_value = "Test response"
            result = chat.chat("What is RAG?")

        assert result == "Test response"

    def test_chat_classifies_query(self, mock_db):
        """Test chat classifies query correctly."""
        chat = ResearchChat(db=mock_db)

        with patch("llm.client.call_llm_chat_completions"):
            # Comparison query
            chat.chat("Compare RAG and fine-tuning")
            assert chat.classify_query("Compare RAG and fine-tuning") == QueryType.COMPARISON

            # Discovery query
            chat.chat("What are the research gaps?")
            assert chat.classify_query("What are the research gaps?") == QueryType.DISCOVERY
