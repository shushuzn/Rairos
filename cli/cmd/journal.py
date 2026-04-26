"""CLI command: journal — Research journal."""
from __future__ import annotations

import argparse

from cli._shared import print_info, print_error
from llm.journal import Journal


def _build_journal_parser(subparsers) -> argparse.ArgumentParser:
    """Build the journal subcommand parser."""
    p = subparsers.add_parser(
        "journal",
        help="Research journal",
        description="Track research activities and thoughts.",
    )
    sub = p.add_subparsers(dest="action", help="Actions")

    # Add
    add_p = sub.add_parser("add", help="Add entry")
    add_p.add_argument("content", help="Journal content")
    add_p.add_argument("--tag", "-t", action="append", help="Tags")
    add_p.add_argument("--question", "-q", help="Question ID to link")
    add_p.add_argument("--experiment", "-e", help="Experiment ID to link")
    add_p.add_argument("--paper", help="Paper ID to link")
    add_p.add_argument("--mood", choices=["productive", "stuck", "excited", "neutral"], help="Mood")

    # List
    list_p = sub.add_parser("list", help="List entries")
    list_p.add_argument("--limit", "-n", type=int, default=20)
    list_p.add_argument("--tag", help="Filter by tag")
    list_p.add_argument("--question", help="Filter by question ID")
    list_p.add_argument("--experiment", help="Filter by experiment ID")
    list_p.add_argument("--today", action="store_true", help="Today's entries")
    list_p.add_argument("--days", type=int, help="Entries from last N days")
    list_p.add_argument("--verbose", "-v", action="store_true")

    # Search
    search_p = sub.add_parser("search", help="Search entries")
    search_p.add_argument("query", help="Search query")
    search_p.add_argument("--limit", "-n", type=int, default=20)

    # Stats
    sub.add_parser("stats", help="Show statistics")

    # Delete
    del_p = sub.add_parser("delete", help="Delete entry")
    del_p.add_argument("entry_id", help="Entry ID to delete")

    return p


def _run_journal(args: argparse.Namespace) -> int:
    """Run journal command."""
    journal = Journal()

    if not args.action or args.action == "add":
        if not getattr(args, 'content', None):
            print_error("Usage: journal add <content> [options]")
            return 1

        entry = journal.add(
            content=args.content,
            tags=getattr(args, 'tag', None),
            question_id=getattr(args, 'question', ''),
            experiment_id=getattr(args, 'experiment', ''),
            paper_id=getattr(args, 'paper', ''),
            mood=getattr(args, 'mood', ''),
        )
        print_info(f"✓ Entry [{entry.id}] added")
        return 0

    elif args.action == "list":
        entries = journal.list_entries(
            limit=args.limit,
            tag=getattr(args, 'tag', '') or '',
            question_id=getattr(args, 'question', '') or '',
            experiment_id=getattr(args, 'experiment', '') or '',
            today=getattr(args, 'today', False),
            days=getattr(args, 'days', 0) or 0,
        )
        print(journal.render_list(entries, verbose=getattr(args, 'verbose', False)))
        return 0

    elif args.action == "search":
        entries = journal.search(args.query, limit=args.limit)
        print(journal.render_list(entries, verbose=True))
        return 0

    elif args.action == "stats":
        stats = journal.stats()
        print()
        print(f"📊 Journal Statistics")
        print(f"   Total entries: {stats['total']}")
        print(f"   This week: {stats['this_week']}")
        print(f"   This month: {stats['this_month']}")
        if stats.get('top_tags'):
            print("   Top tags:")
            for tag, count in stats['top_tags'][:5]:
                print(f"     {tag}: {count}")
        return 0

    elif args.action == "delete":
        if journal.delete(args.entry_id):
            print_info(f"✓ Entry [{args.entry_id}] deleted")
            return 0
        else:
            print_error(f"Entry [{args.entry_id}] not found")
            return 1

    return 0
