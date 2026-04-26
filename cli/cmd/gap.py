"""CLI command: gap — Research gap detection."""
from __future__ import annotations

import argparse

from cli._shared import get_db, print_info, print_error
from llm.gap_detector import GapDetector
from llm.insight_evolution import EvolutionTracker


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
    p.add_argument(
        "--enhanced", "-e",
        action="store_true",
        help="Use enhanced analysis with user insights",
    )
    p.add_argument(
        "--no-insights",
        action="store_true",
        help="Don't use user insights in enhanced mode",
    )
    p.add_argument(
        "--hypothesis", "-H",
        action="store_true",
        help="Generate hypotheses from gaps",
    )
    p.add_argument(
        "--profile",
        action="store_true",
        help="Show research preference profile",
    )
    p.add_argument(
        "--history",
        action="store_true",
        help="Show exploration history for topic",
    )
    p.add_argument(
        "--stats",
        action="store_true",
        help="Show exploration statistics overview",
    )
    return p


def _run_gap(args: argparse.Namespace) -> int:
    """Run gap detection command."""
    db = get_db()
    db.init()

    # Profile/history/stats commands
    if args.profile or args.history or args.stats:
        return _run_profile_or_history(args)

    # Enhanced mode with insights (auto-enable for --hypothesis)
    if args.enhanced or args.hypothesis:
        return _run_gap_enhanced(args)

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


def _run_profile_or_history(args: argparse.Namespace) -> int:
    """Show user research profile or exploration history."""
    tracker = EvolutionTracker()

    if args.profile:
        print()
        print(tracker.render_profile())
        return 0

    if args.history:
        if not args.topic:
            print_error("Error: --history requires a topic argument")
            return 1
        print()
        print(tracker.render_topic_history(args.topic))
        return 0

    if args.stats:
        print()
        print(tracker.render_stats())
        return 0

    return 0


def _run_gap_enhanced(args: argparse.Namespace) -> int:
    """Run enhanced gap detection with insights."""
    from llm.gap_analyzer import GapAnalyzerV2, render_gap_report, render_combined_report
    from llm.insight_cards import InsightManager
    from llm.insight_evolution import EvolutionTracker, ExplorationAction

    db = get_db()
    db.init()
    tracker = EvolutionTracker()
    print_info(f"🔬 Enhanced gap analysis for: {args.topic}")

    # Check if user has preferences
    profile = tracker.get_profile()
    has_preferences = profile.total_events > 0

    # Initialize managers
    insight_manager = None if args.no_insights else InsightManager()

    # Pass tracker for preference-based reordering + trend analyzer for trend-aware sorting
    from llm.trend_analyzer import TrendAnalyzer
    trend_analyzer = TrendAnalyzer(db=db)
    analyzer = GapAnalyzerV2(
        db=db,
        insight_manager=insight_manager,
        evolution_tracker=tracker,
        trend_analyzer=trend_analyzer,
    )

    # Hypothesis generation mode
    if args.hypothesis:
        print_info("💡 Generating hypotheses from gaps...")
        gap_result, hypothesis_result = analyzer.analyze_with_hypotheses(
            topic=args.topic,
            use_insights=not args.no_insights,
            use_llm=not args.no_llm,
            model=args.model,
            min_papers=args.min_papers,
        )

        # Record hypothesis generation events
        for gap in gap_result.gaps:
            tracker.record_event(
                topic=args.topic,
                action=ExplorationAction.VIEWED,
                gap_type=gap.gap_type.value,
                gap_title=gap.title,
                gap_description=gap.description,
            )

        # Record HYPOTHESIZED event for each hypothesis (not just the first)
        for h in hypothesis_result.hypotheses:
            tracker.record_hypothesis_generated(
                topic=args.topic,
                gap_type=h.gap_type,
                gap_title=h.title,
                hypothesis_id=h.id,
            )

        if args.json:
            import json
            print(json.dumps({
                "topic": gap_result.topic,
                "gaps": [{"title": g.title, "type": g.gap_type.value, "severity": g.severity.name} for g in gap_result.gaps],
                "hypotheses": [{"statement": h.core_statement, "type": h.hypothesis_type.value} for h in hypothesis_result.hypotheses],
            }, indent=2))
        else:
            print()
            print(render_combined_report(gap_result, hypothesis_result))
        return 0

    # Standard enhanced analysis
    result = analyzer.analyze(
        topic=args.topic,
        use_insights=not args.no_insights,
        use_llm=not args.no_llm,
        model=args.model,
        min_papers=args.min_papers,
    )

    # Record gap view events
    for gap in result.gaps:
        tracker.record_event(
            topic=args.topic,
            action=ExplorationAction.VIEWED,
            gap_type=gap.gap_type.value,
            gap_title=gap.title,
            gap_description=gap.description,
        )

    if args.json:
        import json
        print(json.dumps({
            "topic": result.topic,
            "gaps": [
                {
                    "title": g.title,
                    "type": g.gap_type.value,
                    "severity": g.severity.name,
                    "insights": g.user_insights,
                    "priority": g.priority,
                }
                for g in result.gaps
            ],
            "stats": {
                "papers": result.total_papers_analyzed,
                "insights": result.total_insights_used,
            }
        }, indent=2))
    else:
        print()
        print(render_gap_report(result))

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
