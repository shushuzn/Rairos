"""CLI command: merge."""
from __future__ import annotations

import argparse
import sys
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
    status_rank = {"completed": 4, "running": 3, "pending": 2, "failed": 1}

    def rank(p):
        return status_rank.get(p.parse_status, 0)

    winner = older if rank(older) >= rank(newer) else newer
    loser = newer if winner is older else older
    return (winner, loser)


def _build_merge_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser("merge", help="Merge a duplicate paper into a target paper")
    p.add_argument(
        "--keep",
        choices=["older", "newer", "parsed", "semantic"],
        default="older",
        help="Which paper to keep: 'older' (default), 'newer', 'parsed' (better parse_status), or 'semantic' (high similarity + parse_status)",
    )
    p.add_argument(
        "--dry-run",
        action="store_true",
        help="Show what would be merged without making changes",
    )
    p.add_argument(
        "--auto",
        action="store_true",
        help="Automatically find and merge all duplicate pairs with similarity >= 0.95",
    )
    p.add_argument("target_id", metavar="TARGET_ID", nargs="?", help="ID of the paper to keep")
    p.add_argument("duplicate_id", metavar="DUPLICATE_ID", nargs="?", help="ID of the duplicate paper to absorb and delete")
    return p


def _run_merge(args: argparse.Namespace) -> int:
    db = get_db()
    db.init()

    if getattr(args, "auto", False):
        # Auto-mode: scan all papers and merge high-similarity pairs
        papers, _ = db.list_papers(limit=10000)
        seen: set = set()
        merged_count = 0
        skipped_count = 0

        for paper in papers:
            if paper.id in seen or not paper.title:
                continue
            sims = db.find_similar(paper.id, threshold=0.95, limit=10)
            for sim_paper, _score in sims:
                pair_key = tuple(sorted([paper.id, sim_paper.id]))
                if pair_key in seen:
                    continue
                seen.add(pair_key)

                target, duplicate = paper, sim_paper
                sim = db.get_similarity(target.id, duplicate.id)
                if sim is None or sim < 0.95:
                    continue

                older = target if target.added_at <= duplicate.added_at else duplicate
                newer = duplicate if target.added_at <= duplicate.added_at else target
                keep, drop = _pick_keep(target, duplicate, "parsed")

                if args.dry_run:
                    print(f"Would merge {drop.id} into {keep.id}")
                    print(f"  keeping : [{keep.id}] {keep.title[:70]}")
                    print(f"  deleting: [{drop.id}] {drop.title[:70]}")
                    print(f"  semantic similarity: {sim:.3f}")
                    print()
                    skipped_count += 1
                else:
                    ok = db.merge_papers(keep.id, drop.id)
                    if ok:
                        db.log_dedup(keep.id, drop.id, "semantic-auto")
                        print(f"Merged {drop.id} into {keep.id} (similarity={sim:.3f})")
                        seen.add(tuple(sorted([keep.id, drop.id])))
                        merged_count += 1
                    else:
                        print(f"Merge failed for {drop.id} -> {keep.id}")

        if args.dry_run:
            print(f"({skipped_count} pair(s) would be merged, dry-run)")
        else:
            print(f"Auto-merge complete: {merged_count} pair(s) merged")
        return 0

    # Original two-paper merge
    if args.target_id is None or args.duplicate_id is None:
        print("merge requires TARGET_ID and DUPLICATE_ID (or use --auto)")
        return 1

    target = db.get_paper(args.target_id)
    if target is None:
        print(f"Target paper {args.target_id} not found")
        return 1

    duplicate = db.get_paper(args.duplicate_id)
    if duplicate is None:
        print(f"Duplicate paper {args.duplicate_id} not found")
        return 1

    older = target if target.added_at <= duplicate.added_at else duplicate
    newer = duplicate if target.added_at <= duplicate.added_at else target
    sim = db.get_similarity(target.id, duplicate.id)

    if args.keep == "semantic":
        if sim is None or sim < 0.8:
            print(f"Note: low similarity, falling back to 'parsed' (similarity: {f'{sim:.3f}' if sim is not None else 'N/A'})")
            keep, drop = _pick_keep(target, duplicate, "parsed")
        else:
            keep, drop = _pick_keep(target, duplicate, args.keep)
    else:
        keep, drop = _pick_keep(target, duplicate, args.keep)

    print(f"Merging {drop.id} into {keep.id}")
    print(f"  Keeping: [{keep.id}] {keep.title[:70]}")
    print(f"  Deleting: [{drop.id}] {drop.title[:70]}")
    if sim is not None:
        print(f"  Similarity: {sim:.3f}")

    if args.dry_run:
        return 0

    ok = db.merge_papers(keep.id, drop.id)
    if ok:
        db.log_dedup(keep.id, drop.id, args.keep)
        print_success(f"Merged {drop.id} into {keep.id}")
    else:
        print_error(f"Merge failed for {drop.id} -> {keep.id}")
        return 1

    return 0
