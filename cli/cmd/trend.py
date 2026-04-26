"""CLI command: trend — Research trend analysis."""
from __future__ import annotations

import argparse

from cli._shared import get_db, print_info
from llm.trend_analyzer import TrendAnalyzer


def _build_trend_parser(subparsers) -> argparse.ArgumentParser:
    """Build the trend subcommand parser."""
    p = subparsers.add_parser(
        "trend",
        help="Analyze research trends over time",
        description="Analyze paper distribution, keyword trends, and citation velocity.",
    )
    p.add_argument(
        "topic",
        nargs="?",
        default=None,
        help="Research topic to analyze",
    )
    p.add_argument(
        "--year-start", "-s",
        type=int,
        default=None,
        help="Start year for analysis (default: 6 years ago)",
    )
    p.add_argument(
        "--year-end", "-e",
        type=int,
        default=None,
        help="End year for analysis (default: current year)",
    )
    p.add_argument(
        "--min-papers", "-n",
        type=int,
        default=10,
        help="Minimum papers needed (default: 10)",
    )
    p.add_argument(
        "--mermaid", "-m",
        action="store_true",
        help="Output as Mermaid timeline",
    )
    p.add_argument(
        "--json", "-j",
        action="store_true",
        help="Output as JSON",
    )
    p.add_argument(
        "--interactive", "-i",
        action="store_true",
        help="Interactive exploration mode",
    )
    return p


def _run_trend(args: argparse.Namespace) -> int:
    """Run trend analysis command."""
    db = get_db()
    db.init()

    analyzer = TrendAnalyzer(db=db)

    # Interactive mode
    if args.interactive or not args.topic:
        return _run_interactive(analyzer, args)

    # Build year range
    year_range = None
    if args.year_start or args.year_end:
        year_range = (args.year_start or 2019, args.year_end or 2025)

    # Single topic analysis
    print_info(f"📈 Analyzing trends for: {args.topic}")

    result = analyzer.analyze(
        topic=args.topic,
        year_range=year_range,
        min_papers=args.min_papers,
    )

    if args.json:
        import json
        data = {
            "topic": result.topic,
            "year_range": result.year_range,
            "total_papers": result.total_papers,
            "growth_rate": result.growth_rate,
            "rising_keywords": [t.keyword for t in result.rising_trends[:5]],
            "emerging_keywords": [t.keyword for t in result.emerging_trends[:5]],
            "falling_keywords": [t.keyword for t in result.falling_trends[:5]],
        }
        print(json.dumps(data, ensure_ascii=False, indent=2))
    elif args.mermaid:
        print(analyzer.render_mermaid_timeline(result))
    else:
        print()
        print(analyzer.render_result(result))

    return 0


def _run_interactive(analyzer: TrendAnalyzer, args: argparse.Namespace) -> int:
    """Interactive trend exploration mode."""
    print("📈 Research Trend Analyzer")
    print("  输入 topic 开始分析")
    print("  输入 mermaid 切换 Mermaid 输出")
    print("  输入 json 切换 JSON 输出")
    print("  输入 q/quit 退出")
    print()

    use_mermaid = False
    use_json = False

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

        if cmd == "mermaid":
            use_mermaid = not use_mermaid
            status = "启用" if use_mermaid else "禁用"
            print(f"  ✓ Mermaid 输出已{status}")
            continue

        if cmd == "json":
            use_json = not use_json
            status = "启用" if use_json else "禁用"
            print(f"  ✓ JSON 输出已{status}")
            continue

        # Treat as topic
        topic = user_input
        print()
        print(f"📈 Analyzing: {topic}")

        result = analyzer.analyze(topic=topic, min_papers=args.min_papers)

        if result.total_papers == 0:
            print("  未找到足够的相关论文")
        elif use_json:
            import json
            data = {
                "topic": result.topic,
                "year_range": result.year_range,
                "total_papers": result.total_papers,
                "growth_rate": result.growth_rate,
                "rising_keywords": [t.keyword for t in result.rising_trends[:5]],
                "emerging_keywords": [t.keyword for t in result.emerging_trends[:5]],
            }
            print(json.dumps(data, ensure_ascii=False, indent=2))
        elif use_mermaid:
            print(analyzer.render_mermaid_timeline(result))
        else:
            print(analyzer.render_result(result))

        print()

    return 0
