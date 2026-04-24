"""CLI command: dedup."""
from __future__ import annotations

import argparse
from typing import Any, Tuple

from cli._shared import get_db
from cli._shared import (
    Colors, colored, print_success, print_error, print_warning, print_info, print_header,
)


def _pick_keep(older: Any, newer: Any, strategy: str) -> Tuple[Any, Any]:
    """Return (target, duplicate) based on keep strategy."""
    if strategy == "older":
        return (older, newer)
    if strategy == "newer":
        return (newer, older)
    # "parsed": keep the one with better parse_status
    # Ranking: completed > running > pending > failed
    status_rank = {"completed": 4, "running": 3, "pending": 2, "failed": 1}

    def rank(p):
        return status_rank.get(p.parse_status, 0)

    winner = older if rank(older) >= rank(newer) else newer
    loser = newer if winner is older else older
    return (winner, loser)


def _build_dedup_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser("dedup", help="Find duplicate papers in the database")
    g = p.add_mutually_exclusive_group()
    g.add_argument("--dry-run", action="store_true", help="Show duplicates without merging")
    g.add_argument("--auto", action="store_true", help="Automatically merge every duplicate pair")
    g.add_argument("--batch", action="store_true", help="Auto-merge safe pairs (same DOI), skip the rest")
    p.add_argument(
        "--keep",
        choices=["older", "newer", "parsed"],
        default="older",
        help="Which paper to keep: 'older' (default, keeps paper with earlier added_at), "
             "'newer' (keeps paper with later added_at), or 'parsed' (keeps paper with better parse_status)",
    )
    p.add_argument("--report", action="store_true", help="Show dedup history log")
    p.add_argument(
        "--since",
        metavar="YYYY-MM-DD",
        default="",
        help="Only consider papers added on or after this date",
    )
    return p


def _run_dedup(args: argparse.Namespace) -> int:
    db = get_db()
    db.init()

    if args.report:
        logs = db.get_dedup_log()
        if not logs:
            print("No dedup history")
            return 0
        print(f"Dedup history ({len(logs)} record(s)):")
        for r in logs:
            print(f"  [{r['id']}] {r['logged_at']}  keep={r['keep_policy']}  kept={r['target_id']}  merged={r['duplicate_id']}")
            print(f"    kept title:   {r['target_title'][:70]}")
            print(f"    merged title: {r['duplicate_title'][:70]}")
        return 0

    pairs = db.find_duplicates(since=args.since or None)

    if not pairs:
        print("No duplicates found")
        return 0

    for older, newer in pairs:
        parsed_rank = {"completed": 4, "running": 3, "pending": 2, "failed": 1}

        def rank(p, _rank=parsed_rank):
            return _rank.get(p.parse_status, 0)

        parsed_winner = older if rank(older) >= rank(newer) else newer
        print(f"Duplicate pair: {older.id} / {newer.id}")
        print(f"  Title: {older.title[:80]}")
        print(f"  DOI: {older.doi or '(none)'}")
        print(f"  [{older.id}] status={older.parse_status:<10} added_at={older.added_at}")
        print(f"  [{newer.id}] status={newer.parse_status:<10} added_at={newer.added_at}")

        if args.dry_run:
            target, dup = _pick_keep(older, newer, args.keep)
            print(f"  --> would keep [{target.id}], merge [{dup.id}] (--keep={args.keep})")
            print(f"  --> parsed winner: [{parsed_winner.id}] (status={parsed_winner.parse_status})")
        print()

    if args.dry_run:
        print(f"({len(pairs)} duplicate pair(s), dry-run — no changes made)")
        return 0

    if args.auto:
        merged = 0
        for older, newer in pairs:
            target, duplicate = _pick_keep(older, newer, args.keep)
            ok = db.merge_papers(target.id, duplicate.id)
            if ok:
                db.log_dedup(target.id, duplicate.id, args.keep)
                print(f"Auto-merged {duplicate.id} into {target.id} (--keep={args.keep})")
                merged += 1
            else:
                print(f"Failed to merge {duplicate.id} into {target.id}")
        print(f"\nAuto-merged {merged}/{len(pairs)} pair(s)")
        return 0

    if args.batch:
        merged, skipped = 0, 0
        for older, newer in pairs:
            if older.doi and older.doi == newer.doi:
                target, duplicate = _pick_keep(older, newer, args.keep)
                ok = db.merge_papers(target.id, duplicate.id)
                if ok:
                    db.log_dedup(target.id, duplicate.id, args.keep)
                    print(f"Auto-merged {duplicate.id} into {target.id} (same DOI, --keep={args.keep})")
                    merged += 1
                else:
                    print(f"Failed to merge {duplicate.id} into {target.id}")
            else:
                skipped += 1
                print(f"Skipped (no same DOI): {older.id} / {newer.id}")
        print(f"\nAuto-merged {merged}/{len(pairs)} pair(s), {skipped} skipped (no same DOI)")
        return 0

    return 0
