"""Tests for RAG pipeline module."""
import pytest
from pathlib import Path
import tempfile
import shutil


class TestRagPipeline:
    """Tests for RagPipeline class."""

    @pytest.fixture
    def temp_work_dir(self, tmp_path):
        """Create temporary work directory."""
        work_dir = tmp_path / ".rag_work"
        work_dir.mkdir()
        yield work_dir
        # Cleanup handled by tmp_path

    @pytest.fixture
    def mock_paper_dir(self, tmp_path):
        """Create mock paper directory with code."""
        paper_dir = tmp_path / "2106.09685"
        paper_dir.mkdir()

        # Create mock src directory
        src_dir = paper_dir / "src"
        src_dir.mkdir()

        # Create mock implementation file
        impl_file = src_dir / "attention.py"
        impl_file.write_text("""
\"\"\"
Attention implementation.
Example: attention_score = softmax(Q @ K.T / sqrt(d_k))
\"\"\"
import numpy as np


def softmax(x):
    \"\"\"Compute softmax values for each row.\"\"\"
    exp_x = np.exp(x - np.max(x, axis=-1, keepdims=True))
    return exp_x / np.sum(exp_x, axis=-1, keepdims=True)
""")

        # Create mock README
        readme = paper_dir / "README.md"
        readme.write_text("""
# Attention Is All You Need

Implementation of transformer attention mechanism.

## Example
```python
>>> import numpy as np
>>> from attention import softmax
>>> x = np.array([1.0, 2.0, 3.0])
>>> softmax(x)
array([0.09003057, 0.24472847, 0.66524096])
```
""")

        return paper_dir

    def test_extract_from_readme(self, temp_work_dir, mock_paper_dir):
        """Test extracting test cases from README."""
        from research_loop.rag_pipeline import RagPipeline

        pipeline = RagPipeline(work_dir=str(temp_work_dir))
        readme_path = mock_paper_dir / "README.md"
        content = readme_path.read_text(encoding="utf-8")
        test_cases = pipeline._parse_examples_from_readme(content)

        # Test cases can be empty if no patterns match
        assert isinstance(test_cases, list)
        if test_cases:
            assert all("question" in tc for tc in test_cases)
        assert all("expected_output" in tc for tc in test_cases)
        assert all("category" in tc for tc in test_cases)

    def test_generate_default_cases(self, temp_work_dir):
        """Test generating default test cases."""
        from research_loop.rag_pipeline import RagPipeline

        pipeline = RagPipeline(work_dir=str(temp_work_dir))
        cases = pipeline._generate_default_cases("2106.09685")

        assert len(cases) == 3
        assert all(c["category"] in ["general", "api", "io"] for c in cases)
        assert all("question" in c and "expected_output" in c for c in cases)

    def test_write_test_csv(self, temp_work_dir, mock_paper_dir):
        """Test writing test CSV."""
        from research_loop.rag_pipeline import RagPipeline

        pipeline = RagPipeline(work_dir=str(temp_work_dir))
        tests_dir = mock_paper_dir / "tests"
        tests_dir.mkdir(parents=True, exist_ok=True)
        csv_path = tests_dir / "test_cases.csv"

        test_cases = [
            {"question": "Test 1", "expected_output": "Output 1", "category": "general"},
            {"question": "Test 2", "expected_output": "Output 2", "category": "api"},
        ]

        pipeline._write_test_csv(csv_path, test_cases)

        assert csv_path.exists()

        # Verify CSV content
        import csv
        with open(csv_path, encoding="utf-8") as f:
            reader = csv.DictReader(f)
            rows = list(reader)

        assert len(rows) == 2
        assert rows[0]["question"] == "Test 1"
        assert rows[1]["category"] == "api"

    def test_generate_pytest_tests(self, temp_work_dir, mock_paper_dir):
        """Test generating pytest test files."""
        from research_loop.rag_pipeline import RagPipeline

        pipeline = RagPipeline(work_dir=str(temp_work_dir))
        csv_path = mock_paper_dir / "tests" / "test_cases.csv"
        csv_path.parent.mkdir(parents=True, exist_ok=True)

        pipeline._generate_pytest_tests(mock_paper_dir, csv_path)

        # Verify conftest.py
        conftest_path = mock_paper_dir / "tests" / "conftest.py"
        assert conftest_path.exists()
        assert "test_data_path" in conftest_path.read_text()

        # Verify test_impl.py
        test_impl_path = mock_paper_dir / "tests" / "test_impl.py"
        assert test_impl_path.exists()
        assert "TestPaperImplementation" in test_impl_path.read_text()

    def test_init_evoskill_benchmark(self, temp_work_dir):
        """Test initializing EvoSkill benchmark config."""
        from research_loop.rag_pipeline import RagPipeline

        pipeline = RagPipeline(work_dir=str(temp_work_dir))
        csv_path = "/path/to/test_cases.csv"

        result = pipeline._init_evoskill_benchmark("test_task", csv_path)

        assert "config" in result
        assert "task" in result

        config_path = Path(result["config"])
        assert config_path.exists()

        config_content = config_path.read_text()
        assert "test_task" in config_content
        assert csv_path in config_content
        assert "multi_tolerance" in config_content


class TestRagPipelineIntegration:
    """Integration tests for RAG pipeline."""

    @pytest.fixture
    def temp_work_dir(self, tmp_path):
        """Create temporary work directory."""
        work_dir = tmp_path / ".rag_work"
        work_dir.mkdir()
        yield work_dir
        shutil.rmtree(work_dir, ignore_errors=True)

    def test_pipeline_initialization(self, temp_work_dir):
        """Test RagPipeline initialization."""
        from research_loop.rag_pipeline import RagPipeline

        pipeline = RagPipeline(work_dir=str(temp_work_dir))

        assert pipeline.work_dir == temp_work_dir
        assert pipeline.paper_pipeline is not None
        assert pipeline.evoskill_pipeline is not None

    def test_extract_and_generate_tests(self, temp_work_dir):
        """Test test extraction and generation."""
        from research_loop.rag_pipeline import RagPipeline

        # Create mock paper dir
        paper_dir = temp_work_dir / "2106.09685"
        paper_dir.mkdir()

        readme = paper_dir / "README.md"
        readme.write_text("""
# Test Paper

Example code:
```python
def add(a, b):
    return a + b
```
""")

        pipeline = RagPipeline(work_dir=str(temp_work_dir))
        csv_path = pipeline._extract_and_generate_tests("2106.09685", paper_dir)

        assert csv_path.exists()
        assert csv_path.suffix == ".csv"
        assert "test_cases.csv" in str(csv_path)
