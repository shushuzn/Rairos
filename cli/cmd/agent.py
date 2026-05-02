"""CLI command: agent — Deep Research Agent with snapstate persistence.

Usage:
    airos agent deep-research "RLHF alignment" --iterations 3
    airos agent deep-research "transformer attention" --iterations 2 --verbose
    airos agent list-sessions
    airos agent resume <session_id>
    airos agent pause <session_id>
    airos agent delete <session_id>
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from cli._shared import Colors, colored, print_error, print_success

from research_loop.deep_research import DeepResearchAgent
from research_loop.snapstate import Snapstate


def _build_agent_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser(
        "agent",
        help="Deep Research Agent with gap-aware iterative research",
        prog="airos agent",
        description="Run autonomous deep research agent with gap detection and session persistence.",
    )

    sub = p.add_subparsers(dest="agent_command", metavar=" COMMAND")

    # deep-research subcommand
    dr = sub.add_parser("deep-research", help="Start a new deep research session")
    dr.add_argument("query", nargs="?", default="", help="Research topic/question")
    dr.add_argument(
        "--iterations", "-n", type=int, default=3,
        help="Max iterations (default 3). Each iteration: search → extract → analyze gaps"
    )
    dr.add_argument(
        "--papers", "-p", type=int, default=5,
        help="Max papers per iteration (default 5)"
    )
    dr.add_argument(
        "--verbose", "-v", action="store_true",
        help="Print verbose debug output"
    )
    dr.add_argument(
        "--resume", "-r", type=str, metavar="SESSION_ID",
        help="Resume an existing session"
    )
    dr.add_argument(
        "--output", "-o", type=str,
        help="Output file for the research report (default: print to stdout)"
    )
    dr.set_defaults(func=_run_deep_research)

    # list-sessions subcommand
    ls = sub.add_parser("list-sessions", help="List all saved research sessions")
    ls.add_argument("--limit", type=int, default=20, help="Max sessions to show (default 20)")
    ls.set_defaults(func=_list_sessions)

    # resume subcommand
    rs = sub.add_parser("resume", help="Resume a paused session")
    rs.add_argument("session_id", help="Session ID to resume")
    rs.add_argument("--verbose", "-v", action="store_true")
    rs.set_defaults(func=_resume_session)

    # pause subcommand
    ps = sub.add_parser("pause", help="Pause a running session")
    ps.add_argument("session_id", help="Session ID to pause")
    ps.set_defaults(func=_pause_session)

    # delete subcommand
    ds = sub.add_parser("delete", help="Delete a session")
    ds.add_argument("session_id", help="Session ID to delete")
    ds.add_argument("--force", "-f", action="store_true", help="Skip confirmation")
    ds.set_defaults(func=_delete_session)

    # status subcommand
    st = sub.add_parser("status", help="Show status of a session")
    st.add_argument("session_id", help="Session ID")
    st.set_defaults(func=_status_session)

    return p


def _run_deep_research(args) -> int:
    snapstate_dir = Path.home() / ".ai_research_os" / "sessions"
    agent = DeepResearchAgent(
        query=args.query,
        max_iterations=args.iterations,
        max_papers_per_iteration=args.papers,
        verbose=args.verbose,
        snapstate_dir=snapstate_dir,
    )

    if args.resume:
        session = agent.resume(args.resume)
        if not session:
            print_error(f"Session not found: {args.resume}")
            return 1
        print_success(f"Resumed session {args.resume} (iteration {session.iteration})")
    else:
        if not args.query:
            print_error("query is required for new sessions")
            return 1
        agent.start()

    result = agent.run()

    # Output
    if args.output:
        Path(args.output).write_text(result.report, encoding="utf-8")
        print_success(f"Report written to {args.output}")
    else:
        print()
        print(colored("=" * 60, Colors.BLUE))
        print(colored(f"  Deep Research Complete", Colors.BOLD))
        print(colored("=" * 60, Colors.BLUE))
        print(f"  Session: {result.session_id}")
        print(f"  Iterations: {result.iterations}")
        print(f"  Papers analyzed: {len(result.papers)}")
        print(f"  Gaps found: {len(result.gaps)}")
        print(f"  Duration: {result.duration_seconds:.1f}s")
        print()
        print(result.report)

    print_success(f"\nSession saved: {result.session_id}")
    return 0


def _list_sessions(args) -> int:
    snapstate = Snapstate()
    sessions = snapstate.list_sessions()

    if not sessions:
        print("No research sessions found.")
        print("Run: airos agent deep-research <query>")
        return 0

    print(colored(f"{'SESSION':12} {'STATUS':10} {'ITER':5} {'PAPERS':6} {'GAPS':5} {'DURATION':8}  QUERY", Colors.BOLD))
    print("-" * 80)
    for s in sessions[:args.limit]:
        status_color = Colors.GREEN if s["status"] == "completed" else Colors.YELLOW if s["status"] == "paused" else Colors.RED
        print(
            f"{s['session_id']:12} "
            f"{colored(s['status'].upper(), status_color):10} "
            f"{s['iteration']:5} "
            f"{len(s.get('papers', [])):6} "
            f"{len(s.get('gaps', [])):5} "
            f"{s['duration']:8.1f}s  "
            f"{s['query'][:40]}"
        )
    return 0


def _resume_session(args) -> int:
    snapstate = Snapstate()
    session = snapstate.load(args.session_id)
    if not session:
        print_error(f"Session not found: {args.session_id}")
        return 1

    if session.status == "completed":
        print(colored(f"Session {args.session_id} already completed. Re-running...", Colors.YELLOW))

    agent = DeepResearchAgent(
        query=session.query,
        max_iterations=session.max_iterations,
        verbose=args.verbose,
    )
    agent.resume(args.session_id)
    result = agent.run()

    print_success(f"Session {args.session_id} completed: {result.iterations} iterations, {len(result.papers)} papers")
    print()
    print(result.report[:500])
    return 0


def _pause_session(args) -> int:
    snapstate = Snapstate()
    session = snapstate.load(args.session_id)
    if not session:
        print_error(f"Session not found: {args.session_id}")
        return 1

    session.status = "paused"
    snapstate.save(session)
    print_success(f"Session {args.session_id} paused at iteration {session.iteration}")
    return 0


def _delete_session(args) -> int:
    snapstate = Snapstate()

    if not args.force:
        session = snapstate.load(args.session_id)
        query = session.query if session else "unknown"
        confirm = input(f"Delete session {args.session_id} (query: {query})? [y/N]: ")
        if confirm.lower() != "y":
            print("Aborted.")
            return 0

    deleted = snapstate.delete(args.session_id)
    if deleted:
        print_success(f"Deleted session {args.session_id}")
    else:
        print_error(f"Session not found: {args.session_id}")
    return 0


def _status_session(args) -> int:
    snapstate = Snapstate()
    session = snapstate.load(args.session_id)
    if not session:
        print_error(f"Session not found: {args.session_id}")
        return 1

    print(colored(f"Session: {session.session_id}", Colors.BOLD))
    print(f"  Query: {session.query}")
    print(f"  Status: {session.status}")
    print(f"  Iteration: {session.iteration} / {session.max_iterations}")
    print(f"  Papers: {len(session.papers)}")
    print(f"  Gaps: {len(session.gaps)}")
    print(f"  Findings: {len(session.findings)}")
    print(f"  Duration: {session.duration():.1f}s")
    print(f"  Created: {session.created_at}")
    print(f"  Updated: {session.updated_at}")

    if session.error:
        print(f"  Error: {session.error}")
    return 0


def run(args: argparse.Namespace) -> int:
    if not hasattr(args, "agent_command") or not args.agent_command:
        print_error("airos agent: no command specified. Run 'airos agent --help' for usage.")
        return 1

    return args.func(args)
