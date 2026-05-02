"""
paper2code Integration Module — Self-Contained Pipeline

Complete闭环: paper → parse → code skeleton → extract assertions → generate tests → run benchmark → encode to Gene Pool

Flow:
  1. Download & parse paper (arxiv API + PDF extract)
  2. Generate code skeleton (LLM, based on paper content)
  3. Extract testable assertions (IO examples, numerical claims, properties)
  4. Generate pytest test suite
  5. Run benchmark — PASS → encode CapsuleGene to Gene Pool
                              FAIL → record for GapAnalyzer feedback

Usage:
    from research_loop.paper2code_integration import PaperPipeline
    pipeline = PaperPipeline()
    result = pipeline.run("1706.03762")

    # Or step-by-step:
    from research_loop.paper_parser import download_and_parse
    from research_loop.code_generator import generate_code
    from research_loop.test_extractor import extract_tests
    from research_loop.benchmark_runner import run_benchmark, BenchmarkConfig

    content = download_and_parse("1706.03762")
    code = generate_code(content)
    tests = extract_tests(content, code)
    result = run_benchmark(BenchmarkConfig(...))
"""

from __future__ import annotations

import shutil
import subprocess
import sys
from pathlib import Path
from typing import Optional

# Import our new modules
from research_loop.paper_parser import download_and_parse, parse_existing_pdf, PaperContent
from research_loop.code_generator import generate_code, save_code
from research_loop.test_extractor import extract_tests, save_tests, TestSuite
from research_loop.benchmark_runner import (
    run_benchmark,
    BenchmarkConfig,
    BenchmarkResult,
    summarize_result,
)


class PaperPipeline:
    """Complete paper → code → tests → Gene Pool闭环 pipeline."""

    def __init__(
        self,
        work_dir: str = ".paper2code_work",
        tracker_data_dir: Optional[str] = None,
    ):
        self.work_dir = Path(work_dir)
        # Default tracker dir: ~/.ai_research_os/evolution
        self.tracker_data_dir = Path(tracker_data_dir) if tracker_data_dir else None

    def run(
        self,
        arxiv_id: str,
        mode: str = "minimal",
        framework: str = "pytorch",
        skip_tests: bool = False,
        skip_gene_pool: bool = False,
    ) -> dict:
        """
        Execute full pipeline.

        Args:
            arxiv_id: e.g. "2106.09685" or full URL
            mode: minimal | full | educational  (currently unused, reserved)
            framework: pytorch | jax | numpy
            skip_tests: skip test generation and benchmark
            skip_gene_pool: don't encode to Gene Pool

        Returns:
            dict with paths to all generated artifacts
        """
        # Normalize
        from research_loop.paper_parser import normalize_arxiv_id
        arxiv_id = normalize_arxiv_id(arxiv_id)

        self.work_dir.mkdir(parents=True, exist_ok=True)
        paper_dir = self.work_dir / arxiv_id.replace(".", "_")

        # Stage 1: Download & parse paper
        print(f"[paper2code] Downloading paper {arxiv_id}...")
        try:
            content = download_and_parse(arxiv_id)
        except Exception as e:
            print(f"[paper2code] Download failed: {e}, trying PDF parse...")
            # Fallback: try to find existing PDF
            pdf_path = self._find_existing_pdf(arxiv_id)
            if pdf_path and pdf_path.exists():
                content = parse_existing_pdf(pdf_path, arxiv_id)
            else:
                raise RuntimeError(f"Could not fetch paper {arxiv_id}: {e}")

        print(f"[paper2code] Paper title: {content.title[:80]}")

        # Stage 2: Generate code skeleton
        print(f"[paper2code] Generating {framework} code skeleton...")
        code = generate_code(content, framework=framework)
        src_dir = paper_dir / "src"
        module_name = self._suggest_module_name(content.title)
        code_path = save_code(code, src_dir, module_name=module_name)
        print(f"[paper2code] Code saved: {code_path}")

        # Stage 3: Generate tests
        test_dir = paper_dir / "tests"
        benchmark_result: Optional[BenchmarkResult] = None

        if not skip_tests:
            print(f"[paper2code] Extracting assertions and generating tests...")
            try:
                suite = extract_tests(content, code, module_name=module_name)
                save_tests(suite, test_dir)
                print(f"[paper2code] Tests: {len(suite.test_cases)} test cases")

                # Stage 4: Run benchmark
                print(f"[paper2code] Running benchmark...")
                config = BenchmarkConfig(
                    arxiv_id=arxiv_id,
                    paper_topic=content.title,
                    algorithm_description="; ".join(content.algorithm_descriptions[:1]) if content.algorithm_descriptions else content.abstract[:200],
                    test_dir=test_dir,
                    code_path=code_path,
                )

                tracker = self._get_tracker(skip_gene_pool)
                benchmark_result = run_benchmark(config, tracker=tracker)

                print(f"[paper2code] Benchmark: {benchmark_result.passed} passed, "
                      f"{benchmark_result.failed} failed, {benchmark_result.skipped} skipped")
                print(summarize_result(benchmark_result))

            except Exception as e:
                print(f"[paper2code] Test/benchmark stage failed: {e}")
                # Continue anyway — code was generated successfully
        else:
            print(f"[paper2code] Skipping tests (--skip-tests)")

        # Stage 5: Write README
        readme = self._generate_readme(content, framework, benchmark_result)
        readme_path = paper_dir / "README.md"
        readme_path.write_text(readme, encoding="utf-8")

        return {
            "arxiv_id": arxiv_id,
            "paper_dir": str(paper_dir),
            "src_dir": str(src_dir),
            "code_path": str(code_path),
            "module_name": module_name,
            "test_dir": str(test_dir),
            "readme": str(readme_path),
            "benchmark": {
                "passed": benchmark_result.passed if benchmark_result else 0,
                "failed": benchmark_result.failed if benchmark_result else 0,
                "skipped": benchmark_result.skipped if benchmark_result else 0,
                "duration": benchmark_result.duration_seconds if benchmark_result else 0,
            } if benchmark_result else None,
        }

    def _get_tracker(self, skip_gene_pool: bool):
        """Get EvolutionTracker for Gene Pool encoding."""
        if skip_gene_pool:
            return None

        try:
            from llm.insight.tracker import EvolutionTracker
            data_dir = self.tracker_data_dir or Path.home() / ".ai_research_os" / "evolution"
            return EvolutionTracker(data_dir=data_dir)
        except Exception as e:
            print(f"[paper2code] Warning: could not init EvolutionTracker: {e}")
            return None

    def _find_existing_pdf(self, arxiv_id: str) -> Optional[Path]:
        """Search common locations for existing PDF."""
        import os
        candidates = [
            Path("data") / f"{arxiv_id}.pdf",
            Path("data/arxiv") / f"{arxiv_id}.pdf",
            Path.home() / ".ai_research_os" / "papers" / f"{arxiv_id}.pdf",
        ]
        for p in candidates:
            if p.exists():
                return p
        return None

    def _suggest_module_name(self, title: str) -> str:
        """Suggest a Python module name from paper title."""
        import re
        # Strip LaTeX
        title = re.sub(r'\$.*?\$', '', title)
        # Keep alphanumeric
        name = re.sub(r'[^a-zA-Z0-9]', '_', title.lower())
        # Strip duplicate underscores
        name = re.sub(r'_+', '_', name).strip('_')
        # Limit length
        return name[:40] or "paper_model"

    def _generate_readme(self, content: PaperContent, framework: str, result: Optional[BenchmarkResult]) -> str:
        """Generate README for the paper implementation."""
        benchmark_str = ""
        if result:
            total = result.passed + result.failed
            pass_rate = result.passed / total if total > 0 else 0
            benchmark_str = f"""
## Benchmark Results

- **Tests:** {result.passed} passed, {result.failed} failed, {result.skipped} skipped
- **Pass rate:** {pass_rate:.0%}
- **Duration:** {result.duration_seconds:.1f}s

This pass rate informs the Gene Pool about this implementation's quality.
"""

        return f"""# {content.title}

**arXiv:** [{content.arxiv_id}](https://arxiv.org/abs/{content.arxiv_id})

{content.abstract[:500]}

## Framework

{framework.upper()}

## Contents

- `src/` — Generated implementation skeleton
- `tests/` — Auto-generated test suite
- `README.md` — This file

## Gene Pool Integration

When tests pass, the implementation pattern is encoded as a CapsuleGene
and stored in the Gene Pool. This enables the self-evolving system to
remember successful paper-to-code mappings for future retrieval.

## Paper Metadata

- **Authors:** {', '.join(content.authors[:3])}{' et al.' if len(content.authors) > 3 else ''}
- **Datasets:** {', '.join(content.datasets) if content.datasets else 'Not specified'}
- **Hyperparameters:** {', '.join(f'{k}={v}' for k, v in content.hyperparameters.items()) if content.hyperparameters else 'Not specified'}

{benchmark_str}
"""


def install_deps() -> None:
    """Install runtime dependencies for paper2code pipeline."""
    subprocess.run(
        [sys.executable, "-m", "pip", "install", "arxiv", "pymupdf"],
        check=True,
    )
