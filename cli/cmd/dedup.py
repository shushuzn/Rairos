"""CLI command: dedup."""
from __future__ import annotations

import argparse
from typing import Any, Tuple

from cli._shared import get_db
from cli._shared import (
    Colors, colored, print_success, print_error, print_warning, print_info, print_header,
)
from cli.warp import WarpBlocks


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
    from rich.console import Console
    c = Console()

    db = get_db()
    db.init()

    if args.report:
        logs = db.get_dedup_log()
        if not logs:
            c.print(WarpBlocks.panel("Dedup History", "[#8E8E8E]No dedup history[/]"))
            return 0
        rows = []
        for r in logs:
            rows.append([
                f"[#D0D1FE][{r['id']}][/]",
                f"[#FF8272]{r['logged_at'][:10]}[/]",
                f"[#FEFDC2]{r['keep_policy']}[/]",
                f"[#A5D5FE]{r['target_id']}[/]",
                f"[#FF8272]{r['duplicate_id']}[/]",
                f"[#8E8E8E]{r['target_title'][:45]}[/]",
            ])
        c.print(WarpBlocks.panel(f"Dedup History — [#FF8272]{len(logs)}[/] record(s)", ""))
        if rows:
            c.print(WarpBlocks.table(
                ["#", "Date", "Keep", "Target", "Merged", "Title"],
                rows,
                title=f"Dedup Log ({len(rows)})"
            ))
        return 0

    pairs = db.find_duplicates(since=args.since or None)

    if not pairs:
        c.print(WarpBlocks.panel("No Duplicates Found", "[#8E8E8E]No duplicate pairs in the database[/]"))
        return 0

    pair_rows = []
    dry_rows = []
    parsed_rank = {"completed": 4, "running": 3, "pending": 2, "failed": 1}

    for older, newer in pairs:
        def rank(p, _rank=parsed_rank):
            return _rank.get(p.parse_status, 0)

        parsed_winner = older if rank(older) >= rank(newer) else newer
        winner_status = f"[#B4FA72]{parsed_winner.parse_status}[/]"
        title_short = (older.title[:55] + "...") if len(older.title) > 58 else older.title
        pair_rows.append([
            f"[#FF8272]{older.id}[/]",
            f"[#D0D1FE]{newer.id}[/]",
            f"[#A5D5FE]{title_short}[/]",
            f"[#FEFDC2]{older.doi or '(none)'}[/]",
            winner_status,
            f"[#8E8E8E]{older.added_at[:10]}[/]",
        ])

        if args.dry_run:
            target, dup = _pick_keep(older, newer, args.keep)
            dry_rows.append([
                f"[#FF8272]{older.id}[/] / [#FF8272]{newer.id}[/]",
                f"[#B4FA72]{target.id}[/] ← keep[/]",
                f"[#FF5555]{dup.id}[/]",
                f"[#A5D5FE]{args.keep}[/]",
                f"[#8E8E8E]winner: {parsed_winner.id}[/]",
            ])

    c.print(WarpBlocks.panel(
        f"Duplicate Pairs — [#FF8272]{len(pairs)}[/] pair(s)",
        f"[#8E8E8E]Scanned since: {args.since or 'all'}[/]"
    ))
    if pair_rows:
        c.print(WarpBlocks.table(
            ["Older", "Newer", "Title", "DOI", "Winner", "Added"],
            pair_rows,
            title=f"Duplicate Pairs ({len(pair_rows)})"
        ))
    if args.dry_run and dry_rows:
        c.print(WarpBlocks.table(
            ["Pair", "Keep", "Merge", "Strategy", "Parsed Winner"],
            dry_rows,
            title=f"Dry Run — Would Merge ({len(dry_rows)})"
        ))
        c.print(WarpBlocks.panel(
            "Dry Run",
            f"[#FEFDC2]{len(pairs)} duplicate pair(s) — no changes made[/]"
        ))
    c.print()

    if args.dry_run:
        return 0

    if args.auto:
        merged = 0
        for older, newer in pairs:
            target, duplicate = _pick_keep(older, newer, args.keep)
            ok = db.merge_papers(target.id, duplicate.id)
            if ok:
                db.log_dedup(target.id, duplicate.id, args.keep)
                merged += 1
        c.print(WarpBlocks.panel(
            "Auto-Merge Complete",
            f"[#B4FA72]Merged [#FF8272]{merged}[/] / [#FF8272]{len(pairs)}[/] pair(s)"
        ))
        return 0

    if args.batch:
        merged, skipped = 0, 0
        for older, newer in pairs:
            if older.doi and older.doi == newer.doi:
                target, duplicate = _pick_keep(older, newer, args.keep)
                ok = db.merge_papers(target.id, duplicate.id)
                if ok:
                    db.log_dedup(target.id, duplicate.id, args.keep)
                    merged += 1
                else:
                    skipped += 1
            else:
                skipped += 1
        c.print(WarpBlocks.panel(
            "Batch Merge Complete",
            f"[#B4FA72]Merged [#FF8272]{merged}[/]  [#FF5555]Skipped [#FEFDC2]{skipped}[/]  ([#FF8272]{len(pairs)}[/] total)"
        ))
        return 0

    return 0
