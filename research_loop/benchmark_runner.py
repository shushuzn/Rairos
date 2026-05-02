"""
Benchmark Runner — run pytest tests and encode results to Gene Pool.

闭环核心：
- 运行 pytest 测试
- 通过 → encode CapsuleGene(successful implementation pattern)
- 失败 → 反馈给 GapAnalyzer，标记为低质量路径
"""

from __future__ import annotations

import json
import subprocess
import sys
import time
import uuid
from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional

from llm.insight.gene import CapsuleGene
from llm.insight.tracker import EvolutionTracker


@dataclass
class BenchmarkResult:
    """Result of a single benchmark run."""
    arxiv_id: str
    test_dir: Path
    passed: int
    failed: int
    skipped: int
    duration_seconds: float
    passed_tests: list[str] = field(default_factory=list)
    failed_tests: list[str] = field(default_factory=list)
    error_message: str = ""
    gene_pool_entry: Optional[CapsuleGene] = None


@dataclass
class BenchmarkConfig:
    """Configuration for a benchmark run."""
    arxiv_id: str
    paper_topic: str
    algorithm_description: str
    test_dir: Path
    code_path: Path
    code_quality: float = 0.5  # estimated quality of generated code
    min_pass_rate: float = 0.5  # minimum pass rate to encode as success


def run_benchmark(
    config: BenchmarkConfig,
    tracker: Optional[EvolutionTracker] = None,
) -> BenchmarkResult:
    """Run pytest on the generated test suite.

    Args:
        config: Benchmark configuration
        tracker: EvolutionTracker for Gene Pool encoding

    Returns:
        BenchmarkResult with pass/fail details
    """
    start = time.time()

    # Run pytest with JSON report
    test_dir = config.test_dir
    json_report = test_dir / "report.json"

    cmd = [
        sys.executable, "-m", "pytest",
        str(test_dir),
        "-v",
        "--tb=short",
        "--no-header",
        "-q",
        f"--json-report",
        f"--json-report-file={json_report}",
    ]

    result = BenchmarkResult(
        arxiv_id=config.arxiv_id,
        test_dir=test_dir,
        passed=0,
        failed=0,
        skipped=0,
        duration_seconds=0,
    )

    try:
        proc = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            timeout=300,
            encoding="utf-8",
            errors="replace",
        )
        result.duration_seconds = time.time() - start
        result.error_message = proc.stdout + proc.stderr

    except subprocess.TimeoutExpired:
        result.duration_seconds = time.time() - start
        result.error_message = f"Timeout after {result.duration_seconds:.0f}s"
        return result
    except Exception as e:
        result.duration_seconds = time.time() - start
        result.error_message = str(e)
        return result

    # Parse pytest output for pass/fail counts
    _parse_pytest_output(result, proc.stdout + proc.stderr)

    # Parse JSON report if available
    if json_report.exists():
        _parse_json_report(result, json_report)

    # Encode to Gene Pool based on results
    if tracker and result.passed > 0:
        _encode_to_gene_pool(config, result, tracker)

    # If all tests passed and pass rate is high, also record as successful implementation
    if result.passed > 0 and result.failed == 0 and tracker:
        _record_successful_implementation(config, result, tracker)

    # If tests failed, record the failure signal for gap analyzer
    if result.failed > 0 and tracker:
        _record_failed_implementation(config, result, tracker)

    return result


def _parse_pytest_output(result: BenchmarkResult, output: str) -> None:
    """Parse pytest stdout/stderr for pass/fail counts."""
    # Look for patterns like "5 passed, 2 failed, 1 skipped in 2.34s"
    import re
    m = re.search(
        r'(\d+)\s+passed'
        r'(?:,\s+(\d+)\s+failed)?'
        r'(?:,\s+(\d+)\s+skipped)?',
        output
    )
    if m:
        result.passed = int(m.group(1))
        result.failed = int(m.group(2)) if m.group(2) else 0
        result.skipped = int(m.group(3)) if m.group(3) else 0


def _parse_json_report(result: BenchmarkResult, report_path: Path) -> None:
    """Parse pytest-json-report output."""
    try:
        data = json.loads(report_path.read_text(encoding="utf-8"))
        result.passed = data.get("summary", {}).get("passed", result.passed)
        result.failed = data.get("summary", {}).get("failed", result.failed)
        result.skipped = data.get("summary", {}).get("skipped", result.skipped)

        # Collect test names
        for node in data.get("results", []):
            test_name = node.get("nodeid", "")
            outcome = node.get("outcome", "")
            if outcome == "passed":
                result.passed_tests.append(test_name)
            elif outcome == "failed":
                result.failed_tests.append(test_name)
    except Exception:
        pass  # Non-critical, we already have data from stdout


def _encode_to_gene_pool(
    config: BenchmarkConfig,
    result: BenchmarkResult,
    tracker: EvolutionTracker,
) -> None:
    """Encode a successful implementation pattern to Gene Pool.

    Called when tests pass. The fact that this paper's algorithm
    was implementable AND passed tests is worth recording as a CapsuleGene.
    """
    pass_rate = result.passed / (result.passed + result.failed) if (result.passed + result.failed) > 0 else 0

    if pass_rate < config.min_pass_rate:
        return  # Not successful enough to encode

    # The success_score is derived from pass rate
    success_score = pass_rate * config.code_quality

    capsule = CapsuleGene(
        capsule_id=f"impl_{config.arxiv_id.replace('.', '_')}_{uuid.uuid4().hex[:6]}",
        created_at=_timestamp(),
        trigger_topic=config.paper_topic,
        trigger_gap_type="implementation",
        trigger_keywords=_extract_keywords(config.algorithm_description),
        action_gap_type="implementation",
        action_gap_title=f"{config.arxiv_id} implementation",
        outcome_success_score=success_score,
        feedback_count=1,
        evolved_generation=0,
        archetype={},
    )

    # Persist via tracker
    tracker.encode_capsule(
        topic=config.paper_topic,
        gap_type="implementation",
        gap_title=f"{config.arxiv_id} implementation",
        gap_description=f"Passed {result.passed}/{result.passed + result.failed} tests",
        success_score=success_score,
    )


def _record_successful_implementation(
    config: BenchmarkConfig,
    result: BenchmarkResult,
    tracker: EvolutionTracker,
) -> None:
    """Record that this paper's implementation passed all tests."""
    tracker.record_event(
        event_type="implementation_pass",
        paper_id=config.arxiv_id,
        details={
            "passed": result.passed,
            "failed": result.failed,
            "duration": result.duration_seconds,
            "pass_rate": result.passed / (result.passed + result.failed),
        },
    )


def _record_failed_implementation(
    config: BenchmarkConfig,
    result: BenchmarkResult,
    tracker: EvolutionTracker,
) -> None:
    """Record that this paper's implementation had test failures.

    This feeds back to GapAnalyzer to inform future gap scoring
    (failed implementations are lower quality paths).
    """
    tracker.record_event(
        event_type="implementation_fail",
        paper_id=config.arxiv_id,
        details={
            "passed": result.passed,
            "failed": result.failed,
            "failed_tests": result.failed_tests[:5],  # First 5 only
            "duration": result.duration_seconds,
            "error": result.error_message[:500],
        },
    )


def _extract_keywords(text: str) -> list[str]:
    """Simple keyword extraction from text."""
    import re
    # Remove common stopwords
    stopwords = {
        "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for",
        "of", "with", "by", "from", "is", "are", "was", "were", "be", "been",
        "being", "have", "has", "had", "do", "does", "did", "will", "would",
        "could", "should", "may", "might", "can", "this", "that", "these",
        "those", "it", "its", "we", "our", "you", "your", "i", "my",
    }
    words = re.findall(r'[a-zA-Z]{3,}', text.lower())
    keywords = [w for w in words if w not in stopwords]
    # Return unique preserving order
    seen = set()
    unique = []
    for w in keywords:
        if w not in seen:
            seen.add(w)
            unique.append(w)
    return unique[:20]


def _timestamp() -> str:
    """Return ISO timestamp."""
    from datetime import datetime
    return datetime.utcnow().isoformat()


# ─── Test run utilities ───────────────────────────────────────────────────────

def run_tests_locally(test_dir: Path, verbose: bool = True) -> subprocess.CompletedProcess:
    """Run tests and return the subprocess result (for CLI use)."""
    cmd = [
        sys.executable, "-m", "pytest",
        str(test_dir),
        "-v" if verbose else "-q",
        "--tb=short",
    ]
    return subprocess.run(cmd, capture_output=True, text=True, encoding="utf-8", errors="replace")


def summarize_result(result: BenchmarkResult) -> str:
    """Human-readable summary of benchmark result."""
    total = result.passed + result.failed
    pass_rate = result.passed / total if total > 0 else 0

    lines = [
        f"arXiv: {result.arxiv_id}",
        f"Tests: {result.passed} passed, {result.failed} failed, {result.skipped} skipped",
        f"Duration: {result.duration_seconds:.2f}s",
        f"Pass rate: {pass_rate:.1%}",
    ]

    if result.gene_pool_entry:
        lines.append(f"Gene Pool: encoded capsule {result.gene_pool_entry.capsule_id}")

    if result.error_message and result.failed > 0:
        lines.append(f"\nError (first 300 chars):\n{result.error_message[:300]}")

    return "\n".join(lines)
