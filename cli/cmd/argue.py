"""CLI command: argue — Build research arguments from evidence."""
from __future__ import annotations

import argparse

from cli._shared import get_db, print_info, print_error
from llm.argument_builder import ArgumentBuilder, render_argument


def _build_argue_parser(subparsers) -> argparse.ArgumentParser:
    """Build the argue subcommand parser."""
    p = subparsers.add_parser(
        "argue",
        help="Build structured research arguments",
        description="Build structured arguments from evidence with supporting and contradicting claims.",
    )
    p.add_argument(
        "thesis",
        nargs="*",
        default=[],
        help="Research thesis or claim to argue",
    )
    p.add_argument(
        "--evidence", "-e",
        action="store_true",
        help="Show detailed evidence",
    )
    p.add_argument(
        "--format", "-f",
        choices=["text", "markdown", "json"],
        default="text",
        help="Output format (default: text)",
    )
    p.add_argument(
        "--no-llm",
        action="store_true",
        help="Disable LLM generation (template only)",
    )
    p.add_argument(
        "--model", "-m",
        type=str,
        default=None,
        help="LLM model to use",
    )
    p.add_argument(
        "--interactive", "-i",
        action="store_true",
        help="Interactive argument building",
    )
    return p


def _run_argue(args: argparse.Namespace) -> int:
    """Run argue command."""
    db = get_db()
    db.init()

    # Interactive mode
    if args.interactive or not args.thesis:
        return _run_interactive(db, args)

    # Single thesis analysis
    thesis = " ".join(args.thesis)
    print_info(f"📝 构建论点：{thesis}")

    result = _build_argument(db, thesis, args)

    # Output
    if args.format == "json":
        import json
        print(json.dumps({
            "thesis": result.argument.thesis,
            "supporting": [
                {"source": e.source, "content": e.content[:200]}
                for e in result.argument.supporting_evidence
            ],
            "contradicting": [
                {"source": e.source, "content": e.content[:200]}
                for e in result.argument.contradicting_evidence
            ],
            "related_gaps": result.argument.related_gaps,
        }, indent=2, ensure_ascii=False))
    else:
        print()
        print(render_argument(result))

    return 0


def _build_argument(db, thesis: str, args) -> "ArgumentResult":
    """Build argument from thesis."""
    from llm.insight_cards import InsightManager
    from llm.gap_analyzer import GapAnalyzerV2

    # Initialize managers
    insight_manager = InsightManager()
    gap_analyzer = GapAnalyzerV2(db=db, insight_manager=insight_manager)

    # Build argument
    builder = ArgumentBuilder(
        db=db,
        insight_manager=insight_manager,
        gap_analyzer=gap_analyzer,
    )

    result = builder.build(
        thesis=thesis,
        use_llm=not args.no_llm,
        model=args.model,
    )

    return result


def _run_interactive(db, args: argparse.Namespace) -> int:
    """Interactive argument building mode."""
    print("📝 Research Argument Builder")
    print("  输入论点开始构建论证")
    print("  输入 q/quit 退出")
    print()

    use_llm = not args.no_llm

    while True:
        try:
            user_input = input("❯ ").strip()
        except (EOFError, KeyboardInterrupt):
            print()
            break

        if not user_input:
            continue

        cmd = user_input.lower()

        if cmd in ("q", "quit", "exit"):
            break

        if cmd == "llm":
            use_llm = not use_llm
            status = "禁用" if not use_llm else "启用"
            print(f"  ✓ LLM 生成已{status}")
            continue

        # Build argument
        print()
        print(f"📝 构建论点：{user_input}")

        result = _build_argument(db, user_input, args)
        print()
        print(render_argument(result))
        print()

    return 0
