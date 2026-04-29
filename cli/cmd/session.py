"""CLI command: session — Research session management."""
from __future__ import annotations

import argparse
import sys

from cli._shared import get_db, print_info, print_error
from cli.warp import WarpBlocks
from llm.research_session import ResearchSessionTracker, ResearchIntent


def _build_session_parser(subparsers) -> argparse.ArgumentParser:
    """Build the session subcommand parser."""
    p = subparsers.add_parser(
        "session",
        help="Manage research sessions",
        description="Start, list, and manage research sessions for context-aware conversations.",
    )
    sub = p.add_subparsers(dest="action", help="Session actions")

    # start
    sp = sub.add_parser("start", help="Start a new research session")
    sp.add_argument("title", nargs="?", default=None, help="Session title")
    sp.add_argument("--topic", "-t", help="Initial topic")

    # list
    sp = sub.add_parser("list", help="List recent sessions")
    sp.add_argument("--days", "-d", type=int, default=7, help="Days to look back (default: 7)")
    sp.add_argument("--limit", "-n", type=int, default=10, help="Max sessions to show (default: 10)")

    # current
    sub.add_parser("current", help="Show current session")

    # end
    sub.add_parser("end", help="End current session")

    # interactive
    sp = sub.add_parser("chat", help="Interactive research chat within session")
    sp.add_argument("query", nargs="*", help="Initial query (optional)")
    sp.add_argument("--topic", help="Override topic context")

    return p


def _run_session(args: argparse.Namespace) -> int:
    """Run session command."""
    tracker = ResearchSessionTracker()

    if args.action == "start":
        return _session_start(tracker, args)
    elif args.action == "list":
        return _session_list(tracker, args)
    elif args.action == "current":
        return _session_current(tracker)
    elif args.action == "end":
        return _session_end(tracker)
    elif args.action == "chat":
        return _session_chat(tracker, args)
    else:
        # Default: show current session
        return _session_current(tracker)


def _session_start(tracker: ResearchSessionTracker, args) -> int:
    """Start a new session."""
    session = tracker.start_session(title=args.title)

    print_info(f"📚 会话已启动: {session.title}")
    print(f"   ID: {session.id}")
    print(f"   时长: 0 分钟")

    if args.topic:
        print(f"   主题: {args.topic}")

    return 0


def _session_list(tracker: ResearchSessionTracker, args) -> int:
    """List recent sessions."""
    from rich.console import Console
    c = Console()

    sessions = tracker.get_recent_sessions(days=args.days, limit=args.limit)

    if not sessions:
        c.print(WarpBlocks.panel("Sessions", "[#8E8E8E]No recent sessions found[/]"))
        return 0

    intent_names = {
        ResearchIntent.LEARNING: "LEARNING",
        ResearchIntent.EXPLORING: "EXPLORING",
        ResearchIntent.IMPROVING: "IMPROVING",
        ResearchIntent.COMPARING: "COMPARING",
        ResearchIntent.REPRODUCING: "REPRODUCING",
        ResearchIntent.CITING: "CITING",
    }
    intent_icons = {
        ResearchIntent.LEARNING: "📖",
        ResearchIntent.EXPLORING: "🔍",
        ResearchIntent.IMPROVING: "🚀",
        ResearchIntent.COMPARING: "⚖️",
        ResearchIntent.REPRODUCING: "🔧",
        ResearchIntent.CITING: "📝",
    }

    rows = []
    for s in sessions:
        date = s.started_at[:10]
        icon = intent_icons.get(s.intent, "📚")
        intent_label = intent_names.get(s.intent, "—")
        tags_str = ", ".join(s.tags[:3]) if s.tags else ""
        insight_preview = (s.insights[0][:45] + "...") if s.insights else ""
        rows.append([
            icon,
            f"[#FF8272]{date}[/]",
            f"[#A5D5FE]{s.title[:40]}[/]",
            f"[#B4FA72]{len(s.queries)} Q&A[/]",
            f"[#D0D1FE]{s.duration_minutes}m[/]",
            f"[#FEFDC2]{intent_label}[/]",
        ])

    c.print(WarpBlocks.panel(
        f"Recent Sessions — [#FF8272]{len(sessions)}[/] (last [#FF8272]{args.days}[/] days)",
        "[#8E8E8E]Use airos session start to begin a research session[/]"
    ))
    if rows:
        c.print(WarpBlocks.table(
            ["", "Date", "Title", "Q&A", "Min", "Intent"],
            rows,
            title=f"Sessions ({len(rows)})"
        ))
    c.print()
    return 0


def _session_current(tracker: ResearchSessionTracker) -> int:
    """Show current session."""
    from rich.console import Console
    c = Console()

    session = tracker.get_current_session()

    if not session:
        c.print(WarpBlocks.panel(
            "Current Session",
            "[#8E8E8E]No active session[/]\n\n"
            "[#A5D5FE]Use airos session start to begin[/]"
        ))
        return 0

    intent_label = {
        ResearchIntent.LEARNING: "📖 Learning",
        ResearchIntent.EXPLORING: "🔍 Exploring",
        ResearchIntent.IMPROVING: "🚀 Improving",
        ResearchIntent.COMPARING: "⚖️ Comparing",
        ResearchIntent.REPRODUCING: "🔧 Reproducing",
        ResearchIntent.CITING: "📝 Citing",
    }.get(session.intent, f"📚 {session.intent.value}")

    body_lines = [
        f"[#A5D5FE]Title:[/]  [#FF8272]{session.title}[/]",
        f"[#A5D5FE]ID:[/]     [#D0D1FE][{session.id}][/]",
        f"[#A5D5FE]Duration:[/] [#B4FA72]{session.duration_minutes} minutes[/]",
        f"[#A5D5FE]Q&A:[/]    [#B4FA72]{len(session.queries)}[/]",
        f"[#A5D5FE]Intent:[/] {intent_label}",
    ]
    if session.tags:
        tags_str = ", ".join(session.tags[:5])
        body_lines.append(f"[#A5D5FE]Tags:[/]   [#FEFDC2]{tags_str}[/]")
    if session.insights:
        body_lines.append("")
        body_lines.append("[#A5D5FE]Insights:[/]")
        for insight in session.insights:
            body_lines.append(f"  • [#B4FA72]{insight[:70]}[/]")

    c.print(WarpBlocks.panel("Current Session", "\n".join(body_lines)))
    return 0


def _session_end(tracker: ResearchSessionTracker) -> int:
    """End current session."""
    session = tracker.end_session()

    if not session:
        print("没有活跃的会话需要结束")
        return 0

    print(f"✅ 会话已结束: {session.title}")
    print(f"   时长: {session.duration_minutes} 分钟")
    print(f"   问答: {len(session.queries)}")

    return 0


def _session_chat(tracker: ResearchSessionTracker, args) -> int:
    """Interactive chat within session."""
    db = get_db()
    db.init()

    # Start session if not active
    session = tracker.get_current_session()
    if not session:
        title = args.query[0] if args.query else None
        session = tracker.start_session(title=title)
        print_info(f"📚 新会话已启动: {session.title}")
    else:
        print_info(f"📚 继续会话: {session.title}")

    # Get initial query
    if args.query:
        query = " ".join(args.query)
        _process_chat_query(tracker, db, query)
    else:
        print("💬 研究助手 (输入 q/quit 退出)")
        print("   输入 topic 开始分析")
        print("   输入 gaps 查看发现的研究空白")
        print("   输入 hypothesis 生成研究假说")
        print()

    # Interactive loop
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

        if cmd in ("gaps", "gap"):
            _show_session_gaps(tracker, db)
            continue

        if cmd in ("hypothesis", "hyp"):
            _show_session_hypothesis(tracker, db)
            continue

        # Regular query
        _process_chat_query(tracker, db, user_input)
        print()

    # Offer to end session
    print()
    print(f"会话已暂停 (ID: {session.id})")
    print("使用 'airos session end' 结束会话")

    return 0


def _process_chat_query(tracker, db, query: str):
    """Process a chat query and add to session."""
    from llm.research_chat import ResearchChat
    from llm.insight_cards import InsightManager

    print(f"🔍 {query}")
    print()

    # Build context with session awareness
    session = tracker.get_current_session()
    topic_hint = None

    if session and session.tags:
        # Use session tags as topic hints
        topic_hint = session.tags[0]

    # Create chat with session context
    insight_manager = InsightManager()
    chat = ResearchChat(db=db, insight_manager=insight_manager)

    # Build enhanced context
    ctx = chat.build_context(query, topic_hint=topic_hint)

    # Get response
    response = chat.chat(query, context=ctx)

    # Extract paper info for session tracking
    paper_ids = [p.uid for p in ctx.papers]
    paper_titles = [p.title for p in ctx.papers]

    # Add to session
    tracker.add_query(
        question=query,
        answer=response[:500],  # Truncate for storage
        paper_ids=paper_ids,
        paper_titles=paper_titles,
    )

    print(response)


def _show_session_gaps(tracker, db):
    """Show gaps based on session context."""
    from llm.gap_analyzer import GapAnalyzerV2
    from llm.insight_cards import InsightManager

    session = tracker.get_current_session()
    if not session or not session.tags:
        print("请先进行一些研究问答")
        return

    topic = session.tags[0]
    print_info(f"🔬 分析 gaps for: {topic}")

    insight_manager = InsightManager()
    analyzer = GapAnalyzerV2(db=db, insight_manager=insight_manager)

    result = analyzer.analyze(
        topic=topic,
        use_insights=True,
        use_llm=True,
    )

    if result.gaps:
        from llm.gap_analyzer import render_gap_report
        print(render_gap_report(result))
    else:
        print(f"未发现 {topic} 的研究空白")


def _show_session_hypothesis(tracker, db):
    """Show hypotheses based on session context."""
    from llm.gap_analyzer import GapAnalyzerV2, render_combined_report
    from llm.insight_cards import InsightManager
    from llm.hypothesis_generator import HypothesisGenerator

    session = tracker.get_current_session()
    if not session or not session.tags:
        print("请先进行一些研究问答")
        return

    topic = session.tags[0]
    print_info(f"💡 分析 gaps & 生成 hypothesis for: {topic}")

    insight_manager = InsightManager()
    analyzer = GapAnalyzerV2(db=db, insight_manager=insight_manager)

    gap_result, hyp_result = analyzer.analyze_with_hypotheses(
        topic=topic,
        use_insights=True,
        use_llm=True,
    )

    if gap_result.gaps or hyp_result.hypotheses:
        print(render_combined_report(gap_result, hyp_result))
        print()
        print(hyp_result.render_result(hyp_result))
    else:
        print(f"无法为 {topic} 生成假说")
