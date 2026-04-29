"""CLI command: queue."""
from __future__ import annotations

import argparse

from cli._shared import get_db


def _build_queue_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser("queue", help="Manage job queue")
    p.add_argument("--add", metavar="UID", help="Add a paper UID to the queue")
    p.add_argument("--list", action="store_true", help="List pending jobs")
    p.add_argument("--dequeue", action="store_true", help="Pop next job from queue")
    p.add_argument("--cancel", metavar="JOB_ID", type=int, help="Cancel a queued job by id")
    p.add_argument("--clear", action="store_true", help="Clear all queued jobs")
    p.add_argument(
        "--format", "-f",
        choices=["text", "warp"],
        default="text",
        help="Output format (default: text)",
    )
    return p


def _run_queue(args: argparse.Namespace) -> int:
    db = get_db()
    db.init()

    if args.list:
        jobs = db.get_papers(limit=100)
        pending = [p.id for p in jobs if p.parse_status == "pending"]
        if args.format == "warp":
            _run_queue_warp(pending, len(jobs))
        elif pending:
            print("Pending:", ", ".join(pending))
        else:
            print("Queue empty")
    elif args.dequeue:
        job = db.dequeue_job()
        if job:
            print(f"Dequeued: {job['paper_id']} (id={job['id']})")
        else:
            print("Queue empty")
    elif args.add:
        db.enqueue_job(args.add, "parse")
        print(f"Added {args.add} to queue")
    elif args.cancel is not None:
        removed = db.cancel_job(args.cancel)
        if removed:
            print(f"Cancelled job {args.cancel}")
        else:
            print(f"No such job {args.cancel}")
    elif args.clear:
        n = db.clear_pending_papers()
        print(f"Cleared {n} pending paper(s)")
    else:
        print("Use --list, --dequeue, --add UID, --cancel JOB_ID, or --clear")

    return 0


def _run_queue_warp(pending: list, total: int) -> None:
    """Render queue status using Warp-style blocks."""
    from cli.warp import WarpBlocks

    blocks = []

    # Header panel
    status = "[#B4FA72]✓ Empty[#/]" if not pending else f"[#FEFDC2]{len(pending)} pending[#/]"
    blocks.append(WarpBlocks.panel(
        "Job Queue",
        f"Status: {status} · {total} papers total",
    ))

    if pending:
        rows = [[uid, "pending", "parse"] for uid in pending]
        blocks.append(WarpBlocks.table(
            ["Paper ID", "Status", "Job Type"],
            rows,
            title=f"Pending ({len(pending)})",
        ))
    else:
        blocks.append(WarpBlocks.panel(
            "Queue Empty",
            "[#8E8E8E]No pending jobs in the queue.[#8E8E8E]",
        ))

    print("\n\n".join(blocks))
