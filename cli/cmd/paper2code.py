"""
paper2code CLI Command

Usage:
    airos paper2code 2106.09685 --mode minimal --framework pytorch
    airos paper2code https://arxiv.org/abs/2106.09685
"""

import click
import sys
from pathlib import Path

# Add parent to path for imports
sys.path.insert(0, str(Path(__file__).parent.parent.parent))

from research_loop.paper2code_integration import PaperPipeline
from cli._shared import print_success, print_error, print_info


def _build_paper2code_parser(subparsers):
    """Register paper2code subcommand."""
    p = subparsers.add_parser("paper2code", help="Generate code from arXiv paper")
    p.add_argument("arxiv_id", help="arXiv ID or URL")
    p.add_argument("--mode", "-m", default="minimal",
                   choices=["minimal", "full", "educational"])
    p.add_argument("--framework", "-f", default="pytorch",
                   choices=["pytorch", "jax", "numpy"])
    p.add_argument("--install-deps", action="store_true")
    p.set_defaults(func=lambda a: paper2code.callback(
        arxiv_id=a.arxiv_id, mode=a.mode, framework=a.framework,
        install_deps=a.install_deps))


@click.command("paper2code")
@click.argument("arxiv_id", type=str)
@click.option("--mode", "-m", default="minimal",
              type=click.Choice(["minimal", "full", "educational"]),
              help="Implementation mode")
@click.option("--framework", "-f", default="pytorch",
              type=click.Choice(["pytorch", "jax", "numpy"]),
              help="Deep learning framework")
@click.option("--install-deps", is_flag=True,
              help="Install paper2code dependencies")
def paper2code(arxiv_id: str, mode: str, framework: str, install_deps: bool):
    """Generate citation-anchored implementation from arXiv paper."""

    # Clean arxiv ID from URL if needed
    import re
    match = re.search(r'(\d+\.\d+)', arxiv_id)
    if match:
        arxiv_id = match.group(1)

    print_info(f"Running paper2code pipeline for arXiv:{arxiv_id}")

    try:
        pipeline = PaperPipeline()

        if install_deps:
            print_info("Installing dependencies...")
            pipeline.install_deps()

        print_info(f"Mode: {mode}, Framework: {framework}")
        result = pipeline.run(arxiv_id, mode=mode, framework=framework)

        print_success(f"✓ Implementation generated: {result['implementation_dir']}")
        print_info(f"  README: {result['readme']}")

    except FileNotFoundError as e:
        print_error(f"Error: {e}")
        print_info("Hint: Clone paper2code skill:")
        print_info("  git clone https://github.com/PrathamLearnsToCode/paper2code.git ~/.claude/skills/")
        sys.exit(1)
    except Exception as e:
        print_error(f"Pipeline failed: {e}")
        sys.exit(1)
