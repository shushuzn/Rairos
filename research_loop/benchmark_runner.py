"""
Benchmark Runner — run pytest tests and encode results to Gene Pool.

闭环核心:
- 运行 pytest 测试
- 通过 → encode CapsuleGene(successful implementation pattern)
- 失败 → 反馈给 GapAnalyzer, 标记为低质量路径
- 成功后 → 触发 InsightEvolution feedback-descent 进化
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
import time
import uuid
from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional

from llm.insight.gene import CapsuleGene
from llm.insight.preferences import ExplorationAction
from llm.insight.tracker import EvolutionTracker

from research_loop.lsp_diagnostics import check_ruff, Diagnostic


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
    ruff_diagnostics: list = field(default_factory=list)  # LSP diagnostics
    # Core Claim Coverage — quality signal for paper2code
    numerical_claims_total: int = 0  # paper extracted numerical claims count
    numerical_claims_covered: int = 0  # claims with real assertions (not stubs)
    coverage_ratio: float = 0.0  # covered/total, 0.0~1.0
    covered_claims: list[str] = field(default_factory=list)
    uncovered_claims: list[str] = field(default_factory=list)


@dataclass
class BenchmarkConfig:
    """Configuration for a benchmark run."""

    arxiv_id: str
    paper_topic: str
    algorithm_description: str
    test_dir: Path
    code_path: Path
    code_quality: float = 0.5  # estimated quality of generated code
    min_pass_rate: float = 0.0  # minimum pass rate to encode as success (0 = record everything)
    algorithm_fingerprint: str = ""  # cross-paper dedup via structural fingerprint
    generated_code: str = ""  # raw code string for provenance comment parsing
    paper_section_refs: list = field(default_factory=list)  # resolved paper_section_refs for archetype
    min_coverage_ratio: float = 0.0  # minimum coverage to encode (0 = no gate)
    numerical_claims_total: int = 0  # total numerical claims from paper


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
        sys.executable,
        "-m",
        "pytest",
        str(test_dir),
        "-v",
        "--tb=short",
        "--no-header",
        "-q",
    ]

    # Prepend src_dir to PYTHONPATH so pytest can import the generated module
    env = {
        **os.environ,
        "PYTHONPATH": f"{config.code_path.parent}{os.pathsep}{os.environ.get('PYTHONPATH', '')}",
    }

    result = BenchmarkResult(
        arxiv_id=config.arxiv_id,
        test_dir=test_dir,
        passed=0,
        failed=0,
        skipped=0,
        duration_seconds=0,
    )

    # Fast lint check — ruff runs synchronously, captures import/syntax issues
    # before the slower pytest run. Progressive enhancement: ruff (<1s) then pyright.
    ruff_diagnostics = check_ruff(config.code_path)
    result.ruff_diagnostics = ruff_diagnostics
    if ruff_diagnostics:
        _log_diagnostics(ruff_diagnostics, config.code_path)

    try:
        proc = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            timeout=300,
            encoding="utf-8",
            errors="replace",
            env=env,
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

    # Populate Core Claim Coverage fields
    _populate_coverage_fields(result, config, test_dir)

    # Encode to Gene Pool based on results
    if tracker and result.passed > 0:
        _encode_to_gene_pool(config, result, tracker, config.algorithm_fingerprint)

    # If all tests passed, record as successful implementation
    if result.passed > 0 and result.failed == 0 and tracker:
        _record_successful_implementation(config, result, tracker)

    # If tests failed, record the failure signal for gap analyzer
    if result.failed > 0 and tracker:
        _record_failed_implementation(config, result, tracker)

    # Trigger InsightEvolution feedback-descent cycle for the paper's topic.
    # Closes the loop: benchmark pass -> Gene Pool write -> capsule audit ->
    # V2 candidate propose -> pairwise evaluate -> apply improvements.
    if tracker and result.passed > 0:
        _trigger_evolution(tracker, config.paper_topic)

    return result


def _parse_pytest_output(result: BenchmarkResult, output: str) -> None:
    """Parse pytest stdout/stderr for pass/fail counts."""
    import re

    # Matches: "N passed", "N passed, M failed", "N passed, M failed, K skipped"
    # Also handles all-skipped case: "N skipped"
    m = re.search(
        r"(\d+)\s+passed"
        r"(?:,\s+(\d+)\s+failed)?"
        r"(?:,\s+(\d+)\s+skipped)?",
        output,
    )
    if m:
        result.passed = int(m.group(1))
        result.failed = int(m.group(2)) if m.group(2) else 0
        result.skipped = int(m.group(3)) if m.group(3) else 0
    else:
        # Handle all-skipped or all-failed output: "N skipped in Xs" or "N failed in Xs"
        skipped_m = re.search(r"(\d+)\s+skipped", output)
        if skipped_m:
            result.skipped = int(skipped_m.group(1))
        failed_m = re.search(r"(\d+)\s+failed", output)
        if failed_m:
            result.failed = int(failed_m.group(1))
        # Handle collection errors: "N error" during collection
        error_m = re.search(r"(\d+)\s+error", output)
        if error_m:
            result.failed = int(error_m.group(1))


def _parse_json_report(result: BenchmarkResult, report_path: Path) -> None:
    """Parse pytest-json-report output."""
    try:
        data = json.loads(report_path.read_text(encoding="utf-8"))
        result.passed = data.get("summary", {}).get("passed", result.passed)
        result.failed = data.get("summary", {}).get("failed", result.failed)
        result.skipped = data.get("summary", {}).get("skipped", result.skipped)

        for node in data.get("results", []):
            test_name = node.get("nodeid", "")
            outcome = node.get("outcome", "")
            if outcome == "passed":
                result.passed_tests.append(test_name)
            elif outcome == "failed":
                result.failed_tests.append(test_name)
    except Exception:
        pass  # Non-critical, we already have data from stdout


def _populate_coverage_fields(
    result: BenchmarkResult, config: BenchmarkConfig, test_dir: Path
) -> None:
    """Populate Core Claim Coverage fields from generated test files.

    A numerical claim is "covered" if the test executes a real assertion
    (not a skip). Skips indicate the model couldn't be evaluated.
    Any pytest.skip → uncovered (stub or couldn't evaluate).
    """
    import re

    result.numerical_claims_total = config.numerical_claims_total

    skip_pattern = re.compile(r"pytest\.skip\(", re.IGNORECASE)

    covered = []
    uncovered = []

    for test_file in test_dir.glob("test_claims.py"):
        try:
            content = test_file.read_text(encoding="utf-8")
            for func_match in re.finditer(r"^def (test_numerical_claim_\d+.*?):", content, re.MULTILINE):
                func_name = func_match.group(1)
                func_start = func_match.end()
                next_def = content.find("\ndef ", func_start)
                func_body = content[func_start:next_def if next_def != -1 else len(content)]

                if skip_pattern.search(func_body):
                    uncovered.append(func_name)
                else:
                    covered.append(func_name)
        except Exception:
            pass

    result.numerical_claims_covered = len(covered)
    result.covered_claims = covered
    result.uncovered_claims = uncovered
    total = config.numerical_claims_total
    result.coverage_ratio = result.numerical_claims_covered / total if total > 0 else 0.0


def _encode_to_gene_pool(
    config: BenchmarkConfig,
    result: BenchmarkResult,
    tracker: EvolutionTracker,
    algorithm_fingerprint: str = "",
) -> None:
    """Encode a successful implementation pattern to Gene Pool.

    Called when tests pass. The fact that this paper's algorithm
    was implementable AND passed tests is worth recording as a CapsuleGene.
    """
    pass_rate = (
        result.passed / (result.passed + result.failed)
        if (result.passed + result.failed) > 0
        else 0
    )

    if pass_rate < config.min_pass_rate:
        return  # Not successful enough to encode

    try:
        from llm.gene_pool_io import paper_exists_in_pool, fingerprint_exists_in_pool

        # Same-paper dedup: re-running on same arXiv ID
        if paper_exists_in_pool(config.arxiv_id, gap_type="implementation"):
            return

        # Cross-paper dedup: same algorithm (fingerprint) from a different paper.
        # Only check if fingerprint is available and non-empty.
        if algorithm_fingerprint and fingerprint_exists_in_pool(
            algorithm_fingerprint, gap_type="implementation"
        ):
            return  # same algorithm already encoded from another paper, skip

    except Exception:
        pass  # non-critical: encoding proceeds if dedup check fails

    # The success_score is derived from pass rate + coverage bonus
    # coverage_ratio ∈ [0.0, 1.0]; coverage_bonus adds up to +0.2
    coverage_bonus = result.coverage_ratio * 0.2
    success_score = (pass_rate * config.code_quality) + coverage_bonus
    success_score = min(success_score, 1.0)

    # Build archetype with algorithm fingerprint and paper section refs for traceability
    archetype = {}
    if algorithm_fingerprint:
        archetype["algorithm_fingerprint"] = algorithm_fingerprint
    if config.paper_section_refs:
        archetype["paper_section_refs"] = config.paper_section_refs

    _capsule = CapsuleGene(
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
        archetype=archetype,
    )

    # Persist via tracker with source_paper_id for Gene Pool → paper linkage
    tracker.encode_capsule(
        topic=config.paper_topic,
        gap_type="implementation",
        gap_title=f"{config.arxiv_id} implementation",
        gap_description=f"Passed {result.passed}/{result.passed + result.failed} tests",
        success_score=success_score,
        source_paper_id=config.arxiv_id,
        capsule_archetype=archetype,
    )


def _record_successful_implementation(
    config: BenchmarkConfig,
    result: BenchmarkResult,
    tracker: EvolutionTracker,
) -> None:
    """Record that this paper's implementation passed all tests."""
    tracker.record_event(
        topic=config.paper_topic,
        action=ExplorationAction.IMPLEMENTATION_PASS,
        paper_ids=[config.arxiv_id],
        notes=f"Passed {result.passed}/{result.passed + result.failed} tests in {result.duration_seconds:.1f}s",
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
        topic=config.paper_topic,
        action=ExplorationAction.IMPLEMENTATION_FAIL,
        paper_ids=[config.arxiv_id],
        notes=f"Failed {result.failed}/{result.passed + result.failed} tests. Errors: {result.error_message[:300]}",
    )


def _trigger_evolution(
    tracker: EvolutionTracker,
    paper_topic: str,
) -> None:
    """Trigger InsightEvolution feedback-descent cycle for the paper's topic.

    Reads capsules from Gene Pool matching this topic, runs audit/propose/evaluate/apply,
    and persists any V2 capsule improvements back to the Gene Pool.
    """
    try:
        from llm.insight.evolution import InsightEvolution

        evo = InsightEvolution(tracker=tracker)
        summary = evo.evolve(topic=paper_topic)

        # Log summary to stderr for visibility
        improved = summary.get("applied", [])
        if improved:
            print(
                f"[evolution] Applied {len(improved)} V2 capsule(s) for topic: {paper_topic[:60]}"
            )
        else:
            print(f"[evolution] No improvements applied for topic: {paper_topic[:60]}")
    except Exception as e:
        # Never let evolution errors crash the benchmark pipeline
        print(f"[evolution] Warning: evolution cycle failed: {e}")


def _extract_keywords(text: str) -> list[str]:
    """Simple keyword extraction from text."""
    import re

    stopwords = {
        "the",
        "a",
        "an",
        "and",
        "or",
        "but",
        "in",
        "on",
        "at",
        "to",
        "for",
        "of",
        "with",
        "by",
        "from",
        "is",
        "are",
        "was",
        "were",
        "be",
        "been",
        "being",
        "have",
        "has",
        "had",
        "do",
        "does",
        "did",
        "will",
        "would",
        "could",
        "should",
        "may",
        "might",
        "can",
        "this",
        "that",
        "these",
        "those",
        "it",
        "its",
        "we",
        "our",
        "you",
        "your",
        "i",
        "my",
    }
    words = re.findall(r"[a-zA-Z]{3,}", text.lower())
    keywords = [w for w in words if w not in stopwords]
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
        sys.executable,
        "-m",
        "pytest",
        str(test_dir),
        "-v" if verbose else "-q",
        "--tb=short",
    ]
    return subprocess.run(cmd, capture_output=True, text=True, encoding="utf-8", errors="replace")


def _log_diagnostics(diagnostics: list[Diagnostic], code_path: Path) -> None:
    """Print ruff diagnostics to stderr for visibility before pytest runs."""
    import sys

    lines = [f"\n[ruff] {len(diagnostics)} issue(s) in {code_path.name}:"]
    for d in diagnostics:
        loc = f"{d.file}:{d.line}:{d.column}"
        lines.append(f"  [{d.severity.upper()}] {loc} {d.code}: {d.message}")
    lines.append("  (running pytest anyway...)")
    print("\n".join(lines), file=sys.stderr)


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
