"""CLI command: gap — Research gap detection."""
from __future__ import annotations

import argparse

from cli._shared import get_db, print_info, print_error
from llm.gap_detector import GapDetector


def _build_gap_parser(subparsers) -> argparse.ArgumentParser:
    """Build the gap subcommand parser."""
    p = subparsers.add_parser(
        "gap",
        help="Detect research gaps and generate research questions",
        description="Analyze papers to identify research gaps and suggest questions.",
    )
    p.add_argument(
        "topic",
        nargs="?",
        default=None,
        help="Research topic to analyze",
    )
    p.add_argument(
        "--no-llm",
        action="store_true",
        help="Disable LLM analysis (rule-based only)",
    )
    p.add_argument(
        "--json", "-j",
        action="store_true",
        help="Output as JSON",
    )
    p.add_argument(
        "--min-papers", "-n",
        type=int,
        default=3,
        help="Minimum papers needed (default: 3)",
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
        help="Interactive exploration mode",
    )
    return p


def _run_gap(args: argparse.Namespace) -> int:
    """Run gap detection command."""
    db = get_db()
    db.init()

    detector = GapDetector(db=db)

    # Interactive mode
    if args.interactive or not args.topic:
        return _run_interactive(detector, args)

    # Single topic analysis
    print_info(f"🔬 Analyzing gaps for: {args.topic}")

    result = detector.analyze(
        topic=args.topic,
        use_llm=not args.no_llm,
        model=args.model,
        min_papers=args.min_papers,
    )

    if args.json:
        print(detector.render_json(result))
    else:
        print()
        print(detector.render_result(result))

    return 0


def _run_interactive(detector: GapDetector, args: argparse.Namespace) -> int:
    """Interactive gap exploration mode."""
    print("🔬 Research Gap Detector")
    print("  输入 topic 开始分析")
    print("  输入 no-llm 禁用 LLM 分析")
    print("  输入 json 输出 JSON 格式")
    print("  输入 q/quit 退出")
    print()

    use_llm = True

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

        if cmd == "no-llm":
            use_llm = not use_llm
            status = "禁用" if not use_llm else "启用"
            print(f"  ✓ LLM 分析已{status}")
            continue

        if cmd == "json":
            print("  请先输入 topic 进行分析")
            continue

        # Treat as topic
        topic = user_input
        print()
        print(f"🔬 Analyzing: {topic}")
        print(f"   LLM: {'启用' if use_llm else '禁用'}")

        result = detector.analyze(
            topic=topic,
            use_llm=use_llm,
        )

        if not result.analyzed_papers_count:
            print("  未找到相关论文")
        else:
            print()
            print(detector.render_result(result))
        print()

    return 0
