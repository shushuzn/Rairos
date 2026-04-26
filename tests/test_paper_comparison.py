"""Tests for paper comparison."""
import pytest
from llm.paper_comparison import (
    PaperComparator,
    ComparisonColumn,
    ComparisonResult,
)


class TestPaperComparator:
    """Test PaperComparator."""

    @pytest.fixture
    def comparator(self):
        return PaperComparator()

    def test_add_paper(self, comparator):
        """Test adding a paper."""
        class MockPaper:
            uid = "p1"
            title = "Test Paper"
            year = 2023
            authors = ["Author A", "Author B"]
            abstract = "This is a test abstract."
            method = ""
            dataset = ""

        col = comparator.add_paper(MockPaper())
        assert col.paper_id == "p1"
        assert col.title == "Test Paper"
        assert col.year == 2023

    def test_extract_methods(self, comparator):
        """Test method extraction from paper text."""
        class MockPaper:
            title = "Transformer based model"
            abstract = "We use BERT and attention mechanisms"
            method = ""

        col = comparator.add_paper(MockPaper())
        assert "Transformer" in col.methods
        assert "BERT" in col.methods
        assert "Attention" in col.methods

    def test_extract_datasets(self, comparator):
        """Test dataset extraction from paper text."""
        class MockPaper:
            title = "GLUE benchmark evaluation"
            abstract = "We evaluate on SQuAD and MNLI"
            dataset = ""

        col = comparator.add_paper(MockPaper())
        assert "GLUE" in col.datasets
        assert "SQuAD" in col.datasets
        assert "MNLI" in col.datasets

    def test_compare_papers(self, comparator):
        """Test comparing multiple papers."""
        class Paper1:
            uid = "p1"
            title = "Paper One"
            year = 2023
            authors = ["Author A"]
            abstract = "We use BERT on GLUE"
            method = "BERT"
            dataset = "GLUE"
            metrics = ""

        class Paper2:
            uid = "p2"
            title = "Paper Two"
            year = 2024
            authors = ["Author B"]
            abstract = "We use GPT on SuperGLUE"
            method = "GPT"
            dataset = "SuperGLUE"
            metrics = ""

        result = comparator.compare(["p1", "p2"])
        assert len(result.columns) == 2
        assert len(result.aspect_rows) > 0

    def test_compare_with_aspects(self, comparator):
        """Test comparing with specific aspects."""
        class Paper1:
            uid = "p1"
            title = "Paper One"
            year = 2023
            authors = []
            abstract = ""
            method = ""
            dataset = ""
            metrics = ""

        result = comparator.compare(["p1"], aspects=["methods", "datasets"])
        assert len(result.aspect_rows) == 2

    def test_render_text(self, comparator):
        """Test text rendering."""
        class Paper1:
            uid = "p1"
            title = "Paper One"
            year = 2023
            authors = []
            abstract = ""
            method = "Transformer"
            dataset = "GLUE"
            metrics = ""

        col = comparator.add_paper(Paper1())
        result = ComparisonResult(columns=[col], aspect_rows=[
            {"aspect": "Methods", "p1": "Transformer"},
        ])

        output = comparator.render_text(result)
        assert "Paper Comparison" in output
        assert "Paper One" in output

    def test_render_markdown(self, comparator):
        """Test Markdown rendering."""
        class Paper1:
            uid = "p1"
            title = "Paper One"
            year = 2023
            authors = []
            abstract = ""
            method = ""
            dataset = ""
            metrics = ""

        col = comparator.add_paper(Paper1())
        result = ComparisonResult(columns=[col], aspect_rows=[
            {"aspect": "Methods", "p1": "Transformer"},
        ])

        output = comparator.render_markdown(result)
        assert "# Paper Comparison" in output
        assert "| Methods |" in output

    def test_render_diff(self, comparator):
        """Test diff rendering."""
        class Paper1:
            title = "Paper One"
            methods = ["BERT", "Transformer"]

        class Paper2:
            title = "Paper Two"
            methods = ["GPT", "Transformer"]

        output = comparator.render_diff(Paper1(), Paper2(), "methods")
        assert "Paper One" in output
        assert "Paper Two" in output


class TestComparisonColumn:
    """Test ComparisonColumn."""

    def test_creation(self):
        """Test creating a column."""
        col = ComparisonColumn(
            paper_id="p1",
            title="Test",
            year=2023,
            methods=["BERT"],
            datasets=["GLUE"],
        )
        assert col.paper_id == "p1"
        assert col.title == "Test"
        assert col.methods == ["BERT"]
        assert col.datasets == ["GLUE"]


class TestComparisonResult:
    """Test ComparisonResult."""

    def test_creation(self):
        """Test creating a result."""
        col = ComparisonColumn(paper_id="p1", title="Test")
        result = ComparisonResult(columns=[col])
        assert len(result.columns) == 1
        assert len(result.aspect_rows) == 0
