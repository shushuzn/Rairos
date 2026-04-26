"""Tests for literature review generator."""
import pytest

from llm.review_generator import (
    ReviewGenerator,
    ResearchStream,
    Controversy,
    ReviewSection,
    LiteratureReview,
)


class TestReviewGenerator:
    """Test ReviewGenerator."""

    def test_research_stream_creation(self):
        """Test ResearchStream dataclass."""
        stream = ResearchStream(
            name="检索增强型",
            papers=["paper1", "paper2"],
            methods=["method1", "method2"],
        )

        assert stream.name == "检索增强型"
        assert len(stream.papers) == 2
        assert len(stream.methods) == 2

    def test_controversy_creation(self):
        """Test Controversy dataclass."""
        controversy = Controversy(
            topic="效率 vs 质量",
            stream_a="检索增强型",
            stream_b="生成优化型",
            position_a="外部知识",
            position_b="端到端",
        )

        assert controversy.topic == "效率 vs 质量"
        assert controversy.stream_a == "检索增强型"

    def test_review_section_creation(self):
        """Test ReviewSection dataclass."""
        section = ReviewSection(
            title="概述",
            content="Test content",
        )

        assert section.title == "概述"
        assert section.content == "Test content"

    def test_literature_review_creation(self):
        """Test LiteratureReview dataclass."""
        review = LiteratureReview(
            topic="RAG",
            streams=[],
            controversies=[],
            open_problems=["problem1"],
        )

        assert review.topic == "RAG"
        assert len(review.streams) == 0
        assert len(review.open_problems) == 1

    def test_generator_initialization(self):
        """Test generator initialization."""
        generator = ReviewGenerator()
        assert generator.db is None

        generator = ReviewGenerator(db="mock_db")
        assert generator.db == "mock_db"

    def test_classify_streams(self):
        """Test stream classification from papers."""
        generator = ReviewGenerator()

        class MockPaper:
            uid = "paper1"
            title = "Retrieval Augmented Generation"
            abstract = "A method for retrieval augmented generation"

        class MockPaper2:
            uid = "paper2"
            title = "Fine-tuning LLMs"
            abstract = "Fine-tuning methods for large language models"

        streams = generator._classify_streams([MockPaper(), MockPaper2()])

        assert len(streams) >= 1
        stream_names = [s.name for s in streams]
        assert "检索增强型" in stream_names

    def test_detect_controversies(self):
        """Test controversy detection."""
        generator = ReviewGenerator()

        streams = [
            ResearchStream(name="检索增强型", papers=[]),
            ResearchStream(name="生成优化型", papers=[]),
        ]

        controversies = generator._detect_controversies(streams)

        assert len(controversies) >= 1
        assert any(c.topic == "效率 vs 质量" for c in controversies)

    def test_detect_controversies_single_stream(self):
        """Test controversy detection with single stream."""
        generator = ReviewGenerator()

        streams = [ResearchStream(name="检索增强型", papers=[])]
        controversies = generator._detect_controversies(streams)

        assert len(controversies) == 0

    def test_build_timeline(self):
        """Test timeline building."""
        generator = ReviewGenerator()

        class MockPaper:
            year = 2023
            title = "Test Paper 2023"

        class MockPaper2:
            year = 2022
            title = "Test Paper 2022"

        timeline = generator._build_timeline([MockPaper(), MockPaper2()])

        assert len(timeline) == 2
        assert timeline[0] == (2022, "Test Paper 2022")
        assert timeline[1] == (2023, "Test Paper 2023")

    def test_build_timeline_empty(self):
        """Test timeline with no papers."""
        generator = ReviewGenerator()
        timeline = generator._build_timeline([])
        assert len(timeline) == 0

    def test_identify_gaps(self):
        """Test gap identification."""
        generator = ReviewGenerator()

        class MockPaper:
            pass

        streams = [ResearchStream(name="检索增强型", papers=[])]
        gaps = generator._identify_gaps([MockPaper() for _ in range(5)], streams)

        assert len(gaps) > 0
        assert any("检索增强与生成优化" in g for g in gaps)

    def test_identify_gaps_diverse_streams(self):
        """Test gap identification with diverse streams."""
        generator = ReviewGenerator()

        streams = [
            ResearchStream(name="检索增强型", papers=[]),
            ResearchStream(name="生成优化型", papers=[]),
        ]

        gaps = generator._identify_gaps(
            [None] * 20,
            streams
        )

        assert len(gaps) > 0

    def test_generate_sections(self):
        """Test section generation."""
        generator = ReviewGenerator()

        streams = [ResearchStream(name="测试流派", papers=["p1"])]
        controversies = [Controversy(
            topic="测试争论",
            stream_a="A",
            stream_b="B",
            position_a="pos A",
            position_b="pos B",
        )]
        timeline = [(2023, "Test event")]
        open_problems = ["问题1", "问题2"]

        sections = generator._generate_sections(
            "测试主题",
            streams,
            controversies,
            timeline,
            open_problems,
            "full"
        )

        assert len(sections) >= 4
        titles = [s.title for s in sections]
        assert "概述" in titles
        assert "方法流派" in titles
        assert "核心争论" in titles

    def test_generate_sections_short_depth(self):
        """Test short depth skips timeline."""
        generator = ReviewGenerator()

        sections = generator._generate_sections(
            "测试",
            [],
            [],
            [(2023, "Event")],
            [],
            "short"
        )

        titles = [s.title for s in sections]
        assert "演化脉络" not in titles

    def test_render_markdown(self):
        """Test Markdown rendering."""
        generator = ReviewGenerator()

        review = LiteratureReview(
            topic="Test Topic",
            streams=[],
            controversies=[],
            sections=[
                ReviewSection(title="概述", content="Test overview"),
                ReviewSection(title="方法流派", content="Test streams"),
            ],
        )

        output = generator.render_markdown(review)

        assert "# Test Topic 文献综述" in output
        assert "## 概述" in output
        assert "Test overview" in output

    def test_render_json(self):
        """Test JSON rendering."""
        import json

        generator = ReviewGenerator()

        review = LiteratureReview(
            topic="Test",
            streams=[ResearchStream(name="流派A", papers=["p1"])],
            controversies=[],
            open_problems=["问题1"],
        )

        output = generator.render_json(review)
        data = json.loads(output)

        assert data["topic"] == "Test"
        assert len(data["streams"]) == 1
        assert data["streams"][0]["name"] == "流派A"
        assert len(data["open_problems"]) == 1

    def test_generate_with_mock_db(self):
        """Test full generation with mock database."""
        generator = ReviewGenerator()

        review = generator.generate(
            topic="RAG",
            max_papers=10,
            depth="full",
        )

        assert review.topic == "RAG"
        assert isinstance(review.streams, list)
        assert isinstance(review.controversies, list)
        assert isinstance(review.sections, list)
        assert len(review.sections) > 0
