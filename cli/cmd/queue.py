"""CLI command: queue."""
from __future__ import annotations

import argparse

from cli._shared import get_db


def _build_queue_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser("queue", help="Manage job queue")
    p.add_argument("--add", metavar="UID", help="Add a paper UID to the queue")
    p.add_argument("--list", action="store_true", help="List queued jobs from job_queue table")
    p.add_argument("--pending", action="store_true", help="Show papers awaiting processing (parse_status=pending)")
    p.add_argument("--dequeue", action="store_true", help="Pop next job from queue")
    p.add_argument("--cancel", metavar="JOB_ID", type=int, help="Cancel a queued job by id")
    p.add_argument("--clear", action="store_true", help="Clear all queued jobs")
    p.add_argument(
        "--format", "-f",
        choices=["text", "warp"],
        default="text",
        help="Output format (default: text)",
    )
    return p  # type: ignore[no-any-return]


def _run_queue(args: argparse.Namespace) -> int:
    db = get_db()
    db.init()

    if args.list:
        # Use db.get_queue_jobs() which is mockable in tests
        rows = db.get_queue_jobs(limit=100)
        if args.format == "warp":
            _run_queue_warp_jq(rows)
        elif rows:
            for r in rows:
                print(f"[{r['id']}] {r['paper_id']} ({r['job_type']}) priority={r['priority']} status={r['status']}")
        else:
            print("Queue empty")
    elif getattr(args, 'pending', False):
        # Show papers with pending parse_status (not in job_queue)
        with db.conn as conn:
            cur = conn.execute(
                "SELECT id, parse_status, source FROM papers "
                "WHERE parse_status = 'pending' ORDER BY id LIMIT 200"
            )
            rows = list(cur.fetchall())
        if rows:
            print(f"{len(rows)} paper(s) awaiting processing:")
            for r in rows:
                print(f"  {r['id']} [{r['source']}]")
        else:
            print("No pending papers")
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

    return 0  # type: ignore[no-any-return]


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

def _run_queue_warp_jq(rows: list) -> None:
    """Render queue status from job_queue table using Warp-style blocks."""
    from cli.warp import WarpBlocks

    blocks = []

    # Header panel
    status = "[#B4FA72]✓ Empty[#/]" if not rows else f"[#FEFDC2]{len(rows)} job(s)[#/]"
    blocks.append(WarpBlocks.panel(
        "Job Queue",
        f"Status: {status}",
    ))

    if rows:
        table_rows = [[str(r['id']), r['paper_id'], r['job_type'], str(r['priority']), r['status']] for r in rows]
        blocks.append(WarpBlocks.table(
            ["ID", "Paper ID", "Type", "Priority", "Status"],
            table_rows,
            title=f"Jobs ({len(rows)})",
        ))
    else:
        blocks.append(WarpBlocks.panel(
            "Queue Empty",
            "[#8E8E8E]No jobs in the queue.[#8E8E8E]",
        ))

    print("\n\n".join(blocks))

