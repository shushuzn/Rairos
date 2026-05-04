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

    # Subcommands: list, extract
    sub = p.add_subparsers(dest="gap_cmd", help="Gap subcommands")

    # gap list — show Gene Pool capsules
    list_p = sub.add_parser("list", help="List Gene Pool capsules")
    list_p.add_argument(
        "--status",
        choices=["all", "active", "consumed", "archived"],
        default="active",
        help="Filter by status (default: active)",
    )
    list_p.add_argument("--json", "-j", action="store_true", help="Output as JSON")

    # gap extract — extract gap from a paper
    ext_p = sub.add_parser("extract", help="Extract a research gap from a paper")
    ext_p.add_argument("paper_id", help="Paper ID (arXiv ID or internal ID)")
    ext_p.add_argument("--json", "-j", action="store_true", help="Output as JSON")

    # gap watch — monitor arXiv for papers matching Gene Pool
    watch_p = sub.add_parser("watch", help="Monitor arXiv for papers matching Gene Pool entries")
    watch_p.add_argument(
        "--interval", type=int, default=30, help="Check interval in minutes (default: 30)"
    )
    watch_p.add_argument("--daemon", action="store_true", help="Run as background daemon")
    watch_p.add_argument(
        "--new-only", action="store_true", help="Only report papers not yet in database"
    )

    # gap contradictions — find Gene Pool pairs with same gap_type but opposite polarity
    con_p = sub.add_parser(
        "contradictions", help="Find Gene Pool capsule pairs with opposite polarity"
    )
    con_p.add_argument("--json", "-j", action="store_true", help="Output as JSON")

    # gap path — trace citation path from paper to Gene Pool capsule
    path_p = sub.add_parser("path", help="Trace citation path from paper to Gene Pool capsule")
    path_p.add_argument("paper_id", help="Starting paper ID")
    path_p.add_argument("--depth", type=int, default=3, help="Max path depth (default: 3)")
    path_p.add_argument("--json", "-j", action="store_true", help="Output as JSON")

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
        "--json",
        "-j",
        action="store_true",
        help="Output as JSON",
    )
    p.add_argument(
        "--min-papers",
        "-n",
        type=int,
        default=3,
        help="Minimum papers needed (default: 3)",
    )
    p.add_argument(
        "--model",
        "-m",
        type=str,
        default=None,
        help="LLM model to use",
    )
    p.add_argument(
        "--interactive",
        "-i",
        action="store_true",
        help="Interactive exploration mode",
    )
    p.add_argument(
        "--enhanced",
        "-e",
        action="store_true",
        help="Use enhanced analysis with user insights",
    )
    p.add_argument(
        "--no-insights",
        action="store_true",
        help="Don't use user insights in enhanced mode",
    )
    p.add_argument(
        "--hypothesis",
        "-H",
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
    p.add_argument(
        "--prefs-history",
        action="store_true",
        help="Show gap_type preference evolution timeline",
    )
    p.add_argument(
        "--feedback",
        action="store_true",
        help="After gap analysis, prompt for accept/reject feedback on each gap",
    )
    return p  # type: ignore[no-any-return]


def _run_gap(args: argparse.Namespace) -> int:
    """Run gap detection command."""
    db = get_db()
    db.init()

    # gap list — show Gene Pool capsules
    if args.gap_cmd == "list":
        return _run_gap_list(args)

    # gap extract — extract gap from a paper
    if args.gap_cmd == "extract":
        return _run_gap_extract(args)

    # gap watch — monitor arXiv for Gene Pool matches
    if args.gap_cmd == "watch":
        return _run_gap_watch(args)

    # gap contradictions — find Gene Pool capsule pairs with opposite polarity
    if args.gap_cmd == "contradictions":
        return _run_gap_contradictions(args)

    # gap path — trace citation path from paper to Gene Pool capsule
    if args.gap_cmd == "path":
        return _run_gap_path(args)

    # Profile/history/stats/prefs-history commands
    if args.profile or args.history or args.stats or args.prefs_history:
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

    # Preference feedback loop — close the insight evolution loop
    if args.feedback:
        print()
        tracker = EvolutionTracker()
        _collect_gap_feedback(args.topic, result.gaps, tracker)

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

    if args.prefs_history:
        print()
        print(tracker.render_gap_type_preferences_history())
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
    tracker.get_profile()

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

            print(
                json.dumps(
                    {
                        "topic": gap_result.topic,
                        "gaps": [
                            {
                                "title": g.title,
                                "type": g.gap_type.value,
                                "severity": g.severity.name,
                            }
                            for g in gap_result.gaps
                        ],
                        "hypotheses": [
                            {"statement": h.core_statement, "type": h.hypothesis_type.value}
                            for h in hypothesis_result.hypotheses
                        ],
                    },
                    indent=2,
                )
            )
        else:
            print()
            print(render_combined_report(gap_result, hypothesis_result))

        # Preference feedback loop
        if args.feedback:
            print()
            _collect_gap_feedback(args.topic, gap_result.gaps, tracker)

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

        print(
            json.dumps(
                {
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
                    },
                },
                indent=2,
            )
        )
    else:
        print()
        print(render_gap_report(result))

    # Preference feedback loop
    if args.feedback:
        print()
        _collect_gap_feedback(args.topic, result.gaps, tracker)

    return 0


def _run_interactive(detector: GapDetector, args: argparse.Namespace) -> int:
    """Interactive gap exploration mode."""
    print("🔬 Research Gap Detector")
    print("  输入 topic 开始分析")
    print("  输入 no-llm 禁用 LLM 分析")
    print("  输入 json 输出 JSON 格式")
    print("  输入 accept <n> / reject <n> 对第 N 个 gap 标记偏好")
    print("  输入 q/quit 退出")
    print()

    use_llm = True
    last_gaps = []

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

        # accept/reject commands in interactive mode
        parts = cmd.split()
        if len(parts) == 2 and parts[0] in ("accept", "reject"):
            if not last_gaps:
                print("  请先分析一个 topic")
                continue
            try:
                idx = int(parts[1]) - 1
                if idx < 0 or idx >= len(last_gaps):
                    print(f"  无效编号，有效范围 1-{len(last_gaps)}")
                    continue
                gap = last_gaps[idx]
                action_name = "采纳" if parts[0] == "accept" else "忽略"
                print(f"  {action_name}: [{idx + 1}] {gap.title}")
                # Tracker is not available in interactive mode without args
                # Just acknowledge — the user can re-run with --feedback
            except ValueError:
                print("  用法: accept <n> / reject <n>")
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
            last_gaps = result.gaps
        print()

    return 0


def _run_gap_list(args: argparse.Namespace) -> int:
    """List Gene Pool capsules (unified storage via EvolutionTracker)."""
    import json
    from pathlib import Path
    from llm.insight.tracker import EvolutionTracker

    tracker = EvolutionTracker()
    capsules = tracker._load_capsules()

    status_filter = args.status if hasattr(args, "status") else "active"
    if status_filter != "all":
        capsules = [c for c in capsules if c.status == status_filter]

    # Also load legacy capsules from JSON for backward compat
    legacy_path = Path.home() / ".ai_research_os" / "gene_pool" / "capsules.json"
    if legacy_path.exists():
        try:
            legacy = json.loads(legacy_path.read_text(encoding="utf-8")).get("capsules", [])
        except Exception:
            legacy = []
    else:
        legacy = []

    if args.json:
        output = [c.to_dict() for c in capsules] + legacy
        print(json.dumps(output, indent=2, ensure_ascii=False))
        return 0

    if not capsules and not legacy:
        print_info("Gene Pool is empty.")
        return 0

    total = len(capsules) + len(legacy)
    print(f"🧬 Gene Pool — {len(capsules)} active + {len(legacy)} legacy = {total} total\n")

    for c in capsules:
        print(f"  [{'active':8}] score={c.outcome_success_score:.2f} [{c.credibility_badge.upper()}] "
              f"cred={c.credibility_score:.2f} {c.action_gap_title[:55]}")
        print(f"           type={c.action_gap_type}  keywords={', '.join(c.trigger_keywords[:5])}")
        print(f"           id={c.capsule_id}  created={c.created_at[:10]}")
        print()

    if legacy:
        print(f"  --- Legacy capsules ({len(legacy)}) ---\n")
        for c in legacy:
            status = c.get("status", "active")
            score = c.get("outcome_success_score", 0)
            gap_type = c.get("action_gap_type") or c.get("trigger_gap_type", "?")
            title = c.get("action_gap_title") or c.get("trigger_gap_title", "?")
            print(f"  [{status:8}] score={score:.2f} {title[:55]}")
            print(f"           type={gap_type}")
            print()
    return 0


def _run_gap_extract(args: argparse.Namespace) -> int:
    """Extract a research gap from a paper and save to Gene Pool."""
    import json

    db = get_db()
    db.init()
    paper = db.get_paper(args.paper_id)

    if not paper:
        print_error(f"Paper not found: {args.paper_id}")
        return 1

    print_info(f"Extracting gap from: {paper.title[:60]}")

    from llm.paper_gap_extractor import extract_gap_from_paper, save_gap_to_gene_pool

    result = extract_gap_from_paper(
        paper_id=paper.id,
        title=paper.title,
        abstract=paper.abstract or "",
        authors=paper.authors,
    )

    if result.get("error"):
        print_error(f"Extraction failed: {result.get('error')}")
        return 1

    if args.json:
        print(json.dumps(result, indent=2, ensure_ascii=False))
        return 0

    polarity = result.get("polarity", "positive")
    print()
    print(f"  Gap Type: {result.get('gap_type', '?')}")
    print(f"  Title:    {result.get('gap_title', '?')}")
    print(f"  Summary:  {result.get('summary', '?')}")
    print(f"  Keywords: {', '.join(result.get('keywords', []))}")
    print(f"  Polarity: {polarity}")
    print()

    # Ask confirmation before saving
    try:
        confirm = input("Save to Gene Pool? [y/N] ").strip().lower()
    except (EOFError, KeyboardInterrupt):
        print()
        confirm = "n"

    if confirm == "y":
        ok = save_gap_to_gene_pool(
            paper_id=paper.id,
            title=paper.title,
            gap_type=result.get("gap_type", "unknown"),
            gap_title=result.get("gap_title", ""),
            keywords=result.get("keywords", []),
            summary=result.get("summary", ""),
            polarity=polarity,
        )
        if ok:
            print_info("✓ Saved to Gene Pool.")
        else:
            print_error("Failed to save.")
            return 1
    else:
        print_info("Skipped.")

    return 0


def _run_gap_watch(args):
    import threading
    import urllib.request
    import xml.etree.ElementTree as ET
    from llm.briefing_generator import _match_gene_pool

    db = get_db()
    db.init()

    def _check_cycle():
        try:
            url = "https://export.arxiv.org/api/query?search_query=all:lastUploadDate&start=0&max_results=20&sortBy=submittedDate&sortOrder=descending"
            with urllib.request.urlopen(url, timeout=30) as resp:
                root = ET.fromstring(resp.read())
                ns = {"a": "http://www.w3.org/2005/Atom"}
                entries = root.findall("a:entry", ns)
        except Exception as e:
            print_info(f"arXiv feed error: {e}")
            return

        matched = []
        for entry in entries:
            t = entry.find("a:title", ns)
            s = entry.find("a:summary", ns)
            i = entry.find("a:id", ns)
            if t is None or s is None or i is None:
                continue
            title = (t.text or "").strip().replace("\n", " ")
            abstract = (s.text or "").strip().replace("\n", " ")
            arxiv_id = (i.text or "").strip().split("/")[-1]

            if args.new_only and db.get_paper(arxiv_id):
                continue

            matches = _match_gene_pool(arxiv_id, title, abstract)
            if matches:
                matched.append((arxiv_id, title, matches))

        if matched:
            print(f"\n  Found {len(matched)} paper(s) matching Gene Pool:")
            for arxiv_id, title, matches in matched:
                print(f"  [{arxiv_id}] {title[:60]}")
                for m in matches:
                    print(
                        f"    -> {m['gap_title'][:60]} type={m['gap_type']} score={m['outcome_score']:.2f}"
                    )
        else:
            print("  (no matches this cycle)")

    if args.daemon:
        print_info(f"Gene Pool watch running (interval={args.interval}min, daemon)...")
        _check_cycle()
        stop_event = threading.Event()
        while not stop_event.wait(args.interval * 60):
            _check_cycle()
        return 0
    else:
        print_info("Checking arXiv for Gene Pool matches...")
        _check_cycle()
        return 0


def _run_gap_contradictions(args: argparse.Namespace) -> int:
    """Find Gene Pool capsule pairs with same gap_type but opposite polarity."""
    import json
    from pathlib import Path

    capsule_path = Path.home() / ".ai_research_os" / "gene_pool" / "capsules.json"
    if not capsule_path.exists():
        print_info("Gene Pool is empty.")
        return 0

    data = json.loads(capsule_path.read_text(encoding="utf-8"))
    capsules = data.get("capsules", [])

    from llm.paper_gap_extractor import detect_contradictions

    contradictions = detect_contradictions(capsules)

    if args.json:
        print(json.dumps(contradictions, indent=2, ensure_ascii=False))
        return 0

    if not contradictions:
        print_info("No contradictions found in Gene Pool.")
        return 0

    print(f"⚡ Found {len(contradictions)} contradiction(s) in Gene Pool:")
    print()
    for i, c in enumerate(contradictions):
        gap_type = c["gap_type"]
        shared = c["shared_keywords"]
        pc = c["positive_capsule"]
        nc = c["negative_capsule"]
        print(f"  [{i + 1}] {gap_type}")
        print(f"      ADVANCE: {pc.get('action_gap_title', pc.get('trigger_gap_title', '?'))[:60]}")
        print(f"             ↕ {pc.get('capsule_id', '?')}")
        print(
            f"      CHALLENGE: {nc.get('action_gap_title', nc.get('trigger_gap_title', '?'))[:60]}"
        )
        print(f"             ↕ {nc.get('capsule_id', '?')}")
        print(f"      Shared keywords: {', '.join(shared[:6])}")
        print()
    return 0


def _run_gap_path(args: argparse.Namespace) -> int:
    """Trace citation path from paper to Gene Pool capsule."""
    import json

    db = get_db()
    db.init()

    paper = db.get_paper(args.paper_id)
    if not paper:
        print_error(f"Paper not found: {args.paper_id}")
        return 1

    from llm.citation_chain import CitationChainBuilder

    builder = CitationChainBuilder(db=db)
    paths = builder.find_paths_to_gene_pool(
        seed_paper_id=args.paper_id,
        depth=args.depth,
    )

    if args.json:
        print(json.dumps(paths, indent=2, ensure_ascii=False))
        return 0

    if not paths:
        print_info(f"No Gene Pool capsules found within {args.depth} hops of [{args.paper_id}]")
        print(f"  Title: {paper.title[:60]}")
        return 0

    print(f"🧬 {len(paths)} path(s) from [{args.paper_id}] to Gene Pool:")
    print(f"  Seed:  {paper.title[:60]}")
    print()

    for i, result in enumerate(paths):
        path = result["path"]
        gap_type = result["gap_type"]
        _gap_title = result["gap_title"]
        polarity = result["polarity"]
        capsule_id = result["capsule_id"]
        score = result["outcome_score"]
        source = result["source_paper_id"]

        arrow = "→" if polarity == "positive" else "⇄"
        print(f"  [{i + 1}] {gap_type} {arrow}")
        for j, pid in enumerate(path):
            p = db.get_paper(pid) if hasattr(db, "get_paper") else None
            title = getattr(p, "title", pid)[:50] if p else pid[:50]
            indent = "  " if j > 0 else ""
            suffix = " ◂ Gene Pool" if pid == source else ""
            print(f"    {indent}{'└─ ' if j == len(path) - 1 else '├─ '}{pid[:12]} {title}{suffix}")
        print(f"       score={score:.2f}  cap={capsule_id[:16]}")
        print()
    return 0


def _collect_gap_feedback(topic: str, gaps, tracker: EvolutionTracker) -> None:
    """Prompt user for accept/reject feedback on each gap.

    Closes the preference loop: user signals directly which gaps they care about,
    and these signals feed back into gap_type_preferences.
    """
    from llm.insight_evolution import ExplorationAction

    if not gaps:
        print("  (无 gaps 可反馈)")
        return

    print("  📊 偏好反馈 — 输入编号采纳 / reject 编号忽略 / skip 跳过 / done 完成:")
    print()

    for i, gap in enumerate(gaps):
        print(f"  [{i + 1}] {gap.title}")
        print(f"      type={gap.gap_type.value}  severity={gap.severity.name}")
        if gap.description:
            desc = gap.description[:80] + ("..." if len(gap.description) > 80 else "")
            print(f"      {desc}")
        print()

    print("  示例: accept 1  reject 2  skip  done")
    print()

    # Collect feedback
    accepted = []
    rejected = []

    while True:
        try:
            user_input = input("❯ ").strip()
        except (EOFError, KeyboardInterrupt):
            print()
            break

        if not user_input:
            continue

        parts = user_input.lower().split()
        cmd = parts[0]

        if cmd in ("done", "skip", "q", "quit"):
            break

        if len(parts) < 2:
            print("  用法: accept <n>  /  reject <n>  /  done")
            continue

        action = parts[0]
        try:
            idx = int(parts[1]) - 1
        except ValueError:
            print("  编号需为数字")
            continue

        if idx < 0 or idx >= len(gaps):
            print(f"  编号无效 ({1}-{len(gaps)})")
            continue

        gap = gaps[idx]

        if action == "accept":
            tracker.record_gap_accept(
                topic=topic,
                gap_type=gap.gap_type.value,
                gap_title=gap.title,
                gap_description=gap.description,
            )
            accepted.append(gap.title)
            print(f"  ✓ 采纳: [{idx + 1}] {gap.title}")
        elif action == "reject":
            tracker.record_gap_reject(
                topic=topic,
                gap_type=gap.gap_type.value,
                gap_title=gap.title,
                reason="user_rejected",
            )
            rejected.append(gap.title)
            print(f"  ✗ 忽略: [{idx + 1}] {gap.title}")
        else:
            print("  未知命令: accept / reject / done")

    print()
    if accepted or rejected:
        print(f"  📈 偏好已更新 — 采纳 {len(accepted)}  忽略 {len(rejected)}")
        profile = tracker.get_profile()
        if profile.gap_type_preferences:
            print("  当前 gap_type 偏好:")
            for gt, score in sorted(profile.gap_type_preferences.items(), key=lambda x: -x[1])[:5]:
                print(f"    {gt}: {score:.2f}")
    else:
        print("  (无偏好更新)")
