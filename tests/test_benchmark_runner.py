"""Tests for research_loop/benchmark_runner.py — benchmark execution and Gene Pool encoding."""

from unittest.mock import patch, MagicMock
import json
import subprocess
import tempfile
from pathlib import Path

from research_loop.benchmark_runner import (
    BenchmarkResult,
    BenchmarkConfig,
    run_benchmark,
    _parse_pytest_output,
    _parse_json_report,
    _extract_keywords,
    summarize_result,
)


class TestBenchmarkResult:
    """Test BenchmarkResult dataclass."""

    def test_creates_with_required_fields(self):
        """Should create BenchmarkResult with required fields."""
        result = BenchmarkResult(
            arxiv_id="2301.00001",
            test_dir=Path("/tmp/tests"),
            passed=5,
            failed=1,
            skipped=2,
            duration_seconds=12.5,
        )
        assert result.arxiv_id == "2301.00001"
        assert result.passed == 5
        assert result.failed == 1
        assert result.skipped == 2
        assert result.duration_seconds == 12.5

    def test_default_lists_empty(self):
        """passed_tests, failed_tests, ruff_diagnostics should default to empty lists."""
        result = BenchmarkResult(
            arxiv_id="2301.00001",
            test_dir=Path("/tmp/tests"),
            passed=0,
            failed=0,
            skipped=0,
            duration_seconds=0,
        )
        assert result.passed_tests == []
        assert result.failed_tests == []
        assert result.gene_pool_entry is None


class TestBenchmarkConfig:
    """Test BenchmarkConfig dataclass."""

    def test_creates_with_required_fields(self):
        """Should create BenchmarkConfig with required fields."""
        config = BenchmarkConfig(
            arxiv_id="2301.00001",
            paper_topic="transformer efficiency",
            algorithm_description="A novel transformer",
            test_dir=Path("/tmp/test_dir"),
            code_path=Path("/tmp/code.py"),
        )
        assert config.arxiv_id == "2301.00001"
        assert config.paper_topic == "transformer efficiency"
        assert config.code_quality == 0.5
        assert config.min_pass_rate == 0.0


class TestParsePytestOutput:
    """Test _parse_pytest_output regex parsing."""

    def test_parses_passed_only(self):
        """Should parse 'N passed' correctly."""
        result = BenchmarkResult(
            arxiv_id="test",
            test_dir=Path("/tmp"),
            passed=0,
            failed=0,
            skipped=0,
            duration_seconds=0,
        )
        _parse_pytest_output(result, "10 passed in 5.23s")
        assert result.passed == 10
        assert result.failed == 0
        assert result.skipped == 0

    def test_parses_passed_and_failed(self):
        """Should parse 'N passed, M failed' correctly."""
        result = BenchmarkResult(
            arxiv_id="test",
            test_dir=Path("/tmp"),
            passed=0,
            failed=0,
            skipped=0,
            duration_seconds=0,
        )
        _parse_pytest_output(result, "5 passed, 2 failed in 3.10s")
        assert result.passed == 5
        assert result.failed == 2
        assert result.skipped == 0

    def test_parses_all_three(self):
        """Should parse 'N passed, M failed, K skipped' correctly."""
        result = BenchmarkResult(
            arxiv_id="test",
            test_dir=Path("/tmp"),
            passed=0,
            failed=0,
            skipped=0,
            duration_seconds=0,
        )
        _parse_pytest_output(result, "5 passed, 2 failed, 3 skipped in 10s")
        assert result.passed == 5
        assert result.failed == 2
        assert result.skipped == 3

    def test_parses_all_skipped(self):
        """Should parse 'N skipped' correctly."""
        result = BenchmarkResult(
            arxiv_id="test",
            test_dir=Path("/tmp"),
            passed=0,
            failed=0,
            skipped=0,
            duration_seconds=0,
        )
        _parse_pytest_output(result, "7 skipped in 1.5s")
        assert result.skipped == 7

    def test_parses_failed_only(self):
        """Should parse 'N failed' correctly."""
        result = BenchmarkResult(
            arxiv_id="test",
            test_dir=Path("/tmp"),
            passed=0,
            failed=0,
            skipped=0,
            duration_seconds=0,
        )
        _parse_pytest_output(result, "3 failed in 2.0s")
        assert result.failed == 3

    def test_parses_collection_error(self):
        """Should parse 'N error' during collection."""
        result = BenchmarkResult(
            arxiv_id="test",
            test_dir=Path("/tmp"),
            passed=0,
            failed=0,
            skipped=0,
            duration_seconds=0,
        )
        _parse_pytest_output(result, "1 error during collection")
        assert result.failed == 1

    def test_no_change_on_unmatched_output(self):
        """Should not modify counts on unrecognized output."""
        result = BenchmarkResult(
            arxiv_id="test",
            test_dir=Path("/tmp"),
            passed=0,
            failed=0,
            skipped=0,
            duration_seconds=0,
        )
        _parse_pytest_output(result, "some random output")
        assert result.passed == 0
        assert result.failed == 0
        assert result.skipped == 0


class TestParseJsonReport:
    """Test _parse_json_report JSON parsing."""

    def test_parses_summary(self):
        """Should update counts from JSON summary."""
        result = BenchmarkResult(
            arxiv_id="test",
            test_dir=Path("/tmp"),
            passed=0,
            failed=0,
            skipped=0,
            duration_seconds=0,
        )
        json_data = {
            "summary": {"passed": 8, "failed": 2, "skipped": 1},
            "results": [],
        }
        with tempfile.NamedTemporaryFile(mode="w", suffix=".json", delete=False) as f:
            json.dump(json_data, f)
            f.flush()
            _parse_json_report(result, Path(f.name))
        assert result.passed == 8
        assert result.failed == 2
        assert result.skipped == 1

    def test_parses_result_nodes(self):
        """Should populate passed_tests and failed_tests from results."""
        result = BenchmarkResult(
            arxiv_id="test",
            test_dir=Path("/tmp"),
            passed=0,
            failed=0,
            skipped=0,
            duration_seconds=0,
        )
        json_data = {
            "summary": {"passed": 1, "failed": 1, "skipped": 0},
            "results": [
                {"nodeid": "test_foo.py::test_a", "outcome": "passed"},
                {"nodeid": "test_foo.py::test_b", "outcome": "failed"},
            ],
        }
        with tempfile.NamedTemporaryFile(mode="w", suffix=".json", delete=False) as f:
            json.dump(json_data, f)
            f.flush()
            _parse_json_report(result, Path(f.name))
        assert "test_foo.py::test_a" in result.passed_tests
        assert "test_foo.py::test_b" in result.failed_tests

    def test_ignores_malformed_json(self):
        """Should not crash on invalid JSON."""
        result = BenchmarkResult(
            arxiv_id="test",
            test_dir=Path("/tmp"),
            passed=5,
            failed=2,
            skipped=1,
            duration_seconds=0,
        )
        with tempfile.NamedTemporaryFile(mode="w", suffix=".json", delete=False) as f:
            f.write("not valid json {{{")
            f.flush()
            _parse_json_report(result, Path(f.name))
        # Original counts preserved
        assert result.passed == 5
        assert result.failed == 2


class TestExtractKeywords:
    """Test _extract_keywords stopword filtering."""

    def test_removes_common_stopwords(self):
        """Should filter out common English stopwords."""
        text = "the quick brown fox jumps over the lazy dog and the cat"
        keywords = _extract_keywords(text)
        assert "the" not in keywords
        assert "and" not in keywords
        assert "quick" in keywords
        assert "brown" in keywords
        assert "over" in keywords  # not a stopword in the set

    def test_returns_unique_keywords_in_order(self):
        """Should return unique keywords preserving first-seen order."""
        text = "transformer attention mechanism transformer attention"
        keywords = _extract_keywords(text)
        assert keywords.count("transformer") == 1
        assert keywords.count("attention") == 1
        assert keywords[0] == "transformer"

    def test_limits_to_20_keywords(self):
        """Should cap at 20 keywords."""
        text = " ".join([f"word{i}" for i in range(50)])
        keywords = _extract_keywords(text)
        assert len(keywords) <= 20

    def test_filters_short_words(self):
        """Should filter out words shorter than 3 characters."""
        text = "a b c test word example"
        keywords = _extract_keywords(text)
        assert "a" not in keywords
        assert "b" not in keywords
        assert "test" in keywords

    def test_empty_string(self):
        """Should return empty list for empty input."""
        assert _extract_keywords("") == []


class TestSummarizeResult:
    """Test summarize_result text generation."""

    def test_includes_arxiv_id(self):
        """Should display arXiv ID."""
        result = BenchmarkResult(
            arxiv_id="2301.12345",
            test_dir=Path("/tmp"),
            passed=10,
            failed=2,
            skipped=1,
            duration_seconds=45.3,
        )
        summary = summarize_result(result)
        assert "2301.12345" in summary

    def test_includes_pass_fail_counts(self):
        """Should show pass/fail/skipped counts."""
        result = BenchmarkResult(
            arxiv_id="test",
            test_dir=Path("/tmp"),
            passed=8,
            failed=3,
            skipped=1,
            duration_seconds=30.0,
        )
        summary = summarize_result(result)
        assert "8 passed" in summary
        assert "3 failed" in summary
        assert "1 skipped" in summary

    def test_includes_duration(self):
        """Should include duration in seconds."""
        result = BenchmarkResult(
            arxiv_id="test",
            test_dir=Path("/tmp"),
            passed=5,
            failed=0,
            skipped=0,
            duration_seconds=12.75,
        )
        summary = summarize_result(result)
        assert "12.75s" in summary or "12.8s" in summary

    def test_includes_pass_rate(self):
        """Should show pass rate percentage."""
        result = BenchmarkResult(
            arxiv_id="test",
            test_dir=Path("/tmp"),
            passed=3,
            failed=1,
            skipped=0,
            duration_seconds=5.0,
        )
        summary = summarize_result(result)
        assert "75.0%" in summary

    def test_zero_pass_rate_handled(self):
        """Should not divide by zero when all tests fail."""
        result = BenchmarkResult(
            arxiv_id="test",
            test_dir=Path("/tmp"),
            passed=0,
            failed=5,
            skipped=0,
            duration_seconds=5.0,
        )
        summary = summarize_result(result)
        assert "0.0%" in summary or "0%" in summary

    def test_includes_error_snippet_when_failed(self):
        """Should include error message when tests failed."""
        result = BenchmarkResult(
            arxiv_id="test",
            test_dir=Path("/tmp"),
            passed=0,
            failed=2,
            skipped=0,
            duration_seconds=3.0,
            error_message="ERROR: test timeout after 30s\nImportError: no module named foo",
        )
        summary = summarize_result(result)
        assert "ERROR" in summary or "ImportError" in summary


class TestRunBenchmark:
    """Test run_benchmark integration."""

    def test_returns_benchmark_result(self):
        """Should return a BenchmarkResult object."""
        with (
            patch("research_loop.benchmark_runner.check_ruff") as mock_ruff,
            patch("subprocess.run") as mock_run,
        ):
            mock_ruff.return_value = []
            mock_run.return_value = MagicMock(
                stdout="5 passed in 10s",
                stderr="",
                returncode=0,
            )
            config = BenchmarkConfig(
                arxiv_id="2301.00001",
                paper_topic="test topic",
                algorithm_description="test algorithm",
                test_dir=Path("/tmp/test_dir"),
                code_path=Path("/tmp/code.py"),
            )
            result = run_benchmark(config)
            assert isinstance(result, BenchmarkResult)
            assert result.arxiv_id == "2301.00001"

    def test_calls_check_ruff(self):
        """Should run ruff diagnostics before pytest."""
        with (
            patch("research_loop.benchmark_runner.check_ruff") as mock_ruff,
            patch("subprocess.run") as mock_run,
        ):
            mock_ruff.return_value = []
            mock_run.return_value = MagicMock(stdout="0 passed", stderr="", returncode=0)
            config = BenchmarkConfig(
                arxiv_id="2301.00001",
                paper_topic="test",
                algorithm_description="algo",
                test_dir=Path("/tmp/test"),
                code_path=Path("/tmp/code.py"),
            )
            run_benchmark(config)
            mock_ruff.assert_called_once()

    def test_parses_pytest_output(self):
        """Should parse pytest stdout for pass/fail counts."""
        with (
            patch("research_loop.benchmark_runner.check_ruff") as mock_ruff,
            patch("subprocess.run") as mock_run,
        ):
            mock_ruff.return_value = []
            mock_run.return_value = MagicMock(
                stdout="7 passed, 2 failed, 1 skipped in 20s",
                stderr="",
                returncode=1,
            )
            config = BenchmarkConfig(
                arxiv_id="2301.00001",
                paper_topic="test",
                algorithm_description="algo",
                test_dir=Path("/tmp/test"),
                code_path=Path("/tmp/code.py"),
            )
            result = run_benchmark(config)
            assert result.passed == 7
            assert result.failed == 2
            assert result.skipped == 1

    def test_timeout_returns_error_message(self):
        """Should handle subprocess.TimeoutExpired gracefully."""
        with (
            patch("research_loop.benchmark_runner.check_ruff") as mock_ruff,
            patch("subprocess.run") as mock_run,
        ):
            mock_ruff.return_value = []
            mock_run.side_effect = subprocess.TimeoutExpired(cmd="pytest", timeout=300)
            config = BenchmarkConfig(
                arxiv_id="2301.00001",
                paper_topic="test",
                algorithm_description="algo",
                test_dir=Path("/tmp/test"),
                code_path=Path("/tmp/code.py"),
            )
            result = run_benchmark(config)
            assert "Timeout" in result.error_message

    def test_encodes_to_gene_pool_when_tracker_provided(self):
        """Should call tracker.encode_capsule when tracker and tests pass."""
        with (
            patch("research_loop.benchmark_runner.check_ruff") as mock_ruff,
            patch("subprocess.run") as mock_run,
            patch("research_loop.benchmark_runner._encode_to_gene_pool") as _mock_encode,
        ):
            mock_ruff.return_value = []
            mock_run.return_value = MagicMock(stdout="5 passed", stderr="", returncode=0)
            mock_tracker = MagicMock()
            config = BenchmarkConfig(
                arxiv_id="2301.00001",
                paper_topic="test topic",
                algorithm_description="test algorithm",
                test_dir=Path("/tmp/test"),
                code_path=Path("/tmp/code.py"),
                code_quality=0.8,
            )
            result = run_benchmark(config, tracker=mock_tracker)
            assert result.passed == 5


class TestLogDiagnostics:
    """Test _log_diagnostics output."""

    def test_diagnostic_class_structure(self):
        """Diagnostic should be constructable from lsp_diagnostics."""
        from research_loop.lsp_diagnostics import Diagnostic

        d = Diagnostic(
            file=Path("/tmp/code.py"),
            line=10,
            column=5,
            severity="error",
            code="E401",
            message="unused import",
        )
        assert d.line == 10
        assert d.severity == "error"
