     1|"""
     2|paper2code CLI Command
     3|
     4|Usage:
     5|    airos paper2code 2106.09685 --mode minimal --framework pytorch
     6|    airos paper2code https://arxiv.org/abs/2106.09685
     7|"""
# [LEGACY] Paper-to-code generator — depends on llm/paper2code/

     8|
     9|import re
    10|import click
    11|import sys
    12|from pathlib import Path
    13|
    14|# Add parent to path for imports
    15|sys.path.insert(0, str(Path(__file__).parent.parent.parent))
    16|
    17|

def _build_paper2code_parser(subparsers):
    """Register paper2code subcommand."""
    p = subparsers.add_parser("paper2code", help="Generate code from arXiv paper")
    p.add_argument("arxiv_id", help="arXiv ID or URL")
    p.add_argument("--mode", "-m", default="minimal", choices=["minimal", "full", "educational"])
    p.add_argument("--framework", "-f", default="pytorch", choices=["pytorch", "jax", "numpy"])
    p.add_argument("--description", "-d", default=None, help="Task description")
    p.add_argument("--output", "-o", default=None, help="Output directory")
    p.set_defaults(func=lambda a: paper2code(a))
    return p


def paper2code(args) -> int:
    """Run paper2code with lazy import."""
    try:
        from research_loop.paper2code_integration import PaperPipeline
    except ImportError:
        print_error("paper2code module not available (research_loop.paper2code_integration is missing)")
        print_info("This feature requires the paper2code integration module.")
        return 1

    pipeline = PaperPipeline()
    result = pipeline.run(
        arxiv_id=args.arxiv_id,
        mode=args.mode,
        framework=args.framework,
        description=args.description,
        output_dir=args.output,
    )
    print_success(f"Code generated: {result['implementation_dir']}")
    print_info(f"README: {result['readme']}")
    return 0
