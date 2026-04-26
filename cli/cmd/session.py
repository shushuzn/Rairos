"""CLI command: session — Research session management."""
from __future__ import annotations

import argparse
import sys

from cli._shared import get_db, print_info, print_error
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
    sessions = tracker.get_recent_sessions(days=args.days, limit=args.limit)

    if not sessions:
        print("暂无研究会话记录")
        return 0

    print(f"📚 最近 {len(sessions)} 个会话 (过去 {args.days} 天)")
    print()

    for s in sessions:
        date = s.started_at[:10]
        intent_icon = {
            ResearchIntent.LEARNING: "📖",
            ResearchIntent.EXPLORING: "🔍",
            ResearchIntent.IMPROVING: "🚀",
            ResearchIntent.COMPARING: "⚖️",
            ResearchIntent.REPRODUCING: "🔧",
            ResearchIntent.CITING: "📝",
        }.get(s.intent, "📚")

        print(f"{intent_icon} {date} | {s.title}")
        print(f"   {len(s.queries)} 个问答 | {s.duration_minutes} 分钟")
        if s.tags:
            print(f"   标签: {', '.join(s.tags[:3])}")
        if s.insights:
            print(f"   💡 {s.insights[0][:50]}")
        print()

    return 0


def _session_current(tracker: ResearchSessionTracker) -> int:
    """Show current session."""
    session = tracker.get_current_session()

    if not session:
        print("当前没有活跃的会话")
        print()
        print("使用 'airos session start' 启动新会话")
        return 0

    print("📚 当前会话")
    print(f"   标题: {session.title}")
    print(f"   ID: {session.id}")
    print(f"   时长: {session.duration_minutes} 分钟")
    print(f"   问答: {len(session.queries)}")
    print(f"   意图: {session.intent.value}")

    if session.tags:
        print(f"   标签: {', '.join(session.tags[:5])}")

    if session.insights:
        print("   洞察:")
        for insight in session.insights:
            print(f"      • {insight}")

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
