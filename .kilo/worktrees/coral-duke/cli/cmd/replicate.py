"""CLI command: replicate — Track paper replication attempts."""
from __future__ import annotations

import argparse

from cli._shared import get_db, print_info, print_error, print_success
from llm.replication_tracker import ReplicationTracker


def _build_replicate_parser(subparsers) -> argparse.ArgumentParser:
    """Build the replicate subcommand parser."""
    p = subparsers.add_parser(
        "replicate",
        help="Track replication attempts",
        description="Track paper replication attempts and results.",
    )
    p.add_argument("paper_id", nargs="?", help="Paper ID to track")
    p.add_argument("--status", "-s", choices=["success", "failed", "partial", "in_progress"],
                   help="Set replication status")
    p.add_argument("--note", "-n", help="Add a note")
    p.add_argument("--diff", "-d", action="append", help="Add a difference found")
    p.add_argument("--metric", help="Add a result metric (key=value)")
    p.add_argument("--env", help="Set environment (key=value)")
    p.add_argument("--report", "-r", help="Generate report for attempt ID")
    p.add_argument("--list", "-l", action="store_true", help="List all attempts")
    p.add_argument("--stats", action="store_true", help="Show statistics")
    p.add_argument("--markdown", "-m", action="store_true", help="Output as Markdown")
    return p


def _run_replicate(args: argparse.Namespace) -> int:
    """Run replicate command."""
    tracker = ReplicationTracker()

    # Generate report
    if args.report:
        report = tracker.generate_report(args.report)
        if not report:
            print_error(f"Attempt [{args.report}] not found")
            return 1

        print(f"# Replication Report: {report.attempt.paper_title}\n")
        print(f"**Status**: {report.attempt.status}")
        print(f"**Date**: {report.attempt.attempt_date}\n")

        if report.summary:
            print(f"## Summary\n{report.summary}\n")

        if report.methodology:
            print(report.methodology)
            print()

        if report.findings:
            print("## Findings")
            for f in report.findings:
                print(f"- {f}")
            print()

        if report.recommendations:
            print("## Recommendations")
            for r in report.recommendations:
                print(f"- {r}")
            print()

        return 0

    # Show statistics
    if args.stats:
        stats = tracker.get_statistics()
        print("🔬 Replication Statistics")
        print(f"  Total:       {stats['total']}")
        print(f"  Success:     {stats['success']}")
        print(f"  Failed:      {stats['failed']}")
        print(f"  Partial:     {stats['partial']}")
        print(f"  In Progress: {stats['in_progress']}")
        print(f"  Success Rate: {stats['success_rate']:.1f}%")
        return 0

    # List attempts
    if args.list:
        status_filter = args.status
        attempts = tracker.get_all_attempts(status=status_filter)

        if args.markdown:
            print(tracker.render_markdown(attempts))
        else:
            print(tracker.render_text(attempts))
        return 0

    # Update existing attempt
    if args.paper_id and not args.status:
        # Find attempt for this paper
        attempts = tracker.get_paper_attempts(args.paper_id)
        if not attempts:
            print_error(f"No attempts found for paper [{args.paper_id}]")
            print("Use with --status to create a new attempt")
            return 1

        # Use most recent
        attempt = attempts[0]
        print_info(f"Found attempt: {attempt.attempt_id}")

        updates = {}
        if args.note:
            updates["notes"] = args.note
        if args.diff:
            updates["differences"] = args.diff

        tracker.update_attempt(attempt.attempt_id, **updates)
        print_success("Attempt updated")
        return 0

    # Create or update attempt
    if not args.paper_id:
        print_error("Usage: replicate <paper_id> [options]")
        return 1

    # Get paper info if available
    paper_title = args.paper_id
    db = get_db()
    db.init()
    paper = db.get_paper(args.paper_id) if hasattr(db, 'get_paper') else None
    if paper:
        paper_title = getattr(paper, 'title', args.paper_id)

    # Check for existing attempt
    existing = tracker.get_paper_attempts(args.paper_id)

    if existing and args.status:
        # Update existing
        attempt_id = existing[0].attempt_id
        tracker.update_attempt(
            attempt_id,
            status=args.status,
            notes=args.note or "",
            differences=args.diff or [],
        )
        print_success(f"Updated attempt [{attempt_id}] with status: {args.status}")
        return 0

    if not args.status:
        # Create new attempt
        attempt = tracker.create_attempt(args.paper_id, paper_title)
        print_success(f"Created new attempt: {attempt.attempt_id}")
        return 0

    print_error("Invalid operation")
    return 1
