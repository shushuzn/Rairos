"""
paper2code CLI Command

Usage:
    airos paper2code 2106.09685 --framework pytorch
    airos paper2code https://arxiv.org/abs/2106.09685 --skip-tests
    airos paper2code 1706.03762 --framework pytorch --skip-gene-pool
"""

import re
import sys
from pathlib import Path

import click

sys.path.insert(0, str(Path(__file__).parent.parent.parent))

from research_loop.paper2code_integration import PaperPipeline
from cli._shared import print_success, print_error, print_info


@click.command("paper2code")
@click.argument("arxiv_id", type=str)
@click.option(
    "--mode",
    "-m",
    default="minimal",
    type=click.Choice(["minimal", "full", "educational"]),
    help="Implementation mode (reserved)",
)
@click.option(
    "--framework",
    "-f",
    default="pytorch",
    type=click.Choice(["pytorch", "jax", "numpy"]),
    help="Deep learning framework",
)
@click.option("--skip-tests", is_flag=True, help="Skip test generation and benchmark")
@click.option("--skip-gene-pool", is_flag=True, help="Don't encode results to Gene Pool")
@click.option("--install-deps", is_flag=True, help="Install paper2code dependencies")
def paper2code(
    arxiv_id: str,
    mode: str,
    framework: str,
    skip_tests: bool,
    skip_gene_pool: bool,
    install_deps: bool,
):
    """Generate code + tests + Gene Pool encoding from an arXiv paper.

    Runs the full闭环:
      1. Download & parse paper
      2. Generate code skeleton (LLM)
      3. Extract assertions → generate pytest tests
      4. Run benchmark → PASS encodes CapsuleGene to Gene Pool
    """

    # Normalize arXiv ID
    match = re.search(r"(\d+\.\d+)", arxiv_id)
    if match:
        arxiv_id = match.group(1)

    print_info(f"Running paper2code pipeline for arXiv:{arxiv_id}")

    try:
        if install_deps:
            print_info("Installing dependencies...")
            try:
                from research_loop.paper2code_integration import install_deps as _install

                _install()
            except Exception:
                pass  # Already installed or in venv

        pipeline = PaperPipeline()
        result = pipeline.run(
            arxiv_id,
            mode=mode,
            framework=framework,
            skip_tests=skip_tests,
            skip_gene_pool=skip_gene_pool,
        )

        print_success(f"Implementation: {result['src_dir']}")
        print_info(f"Module: {result['module_name']}")
        print_info(f"Tests: {result['test_dir']}")
        print_info(f"README: {result['readme']}")

        bench = result.get("benchmark")
        if bench:
            total = bench["passed"] + bench["failed"]
            pass_rate = bench["passed"] / total if total > 0 else 0
            print_info(
                f"Benchmark: {bench['passed']} passed, "
                f"{bench['failed']} failed, {bench['skipped']} skipped "
                f"({pass_rate:.0%}) in {bench['duration']:.1f}s"
            )

    except Exception as e:
        print_error(f"Pipeline failed: {e}")
        sys.exit(1)
