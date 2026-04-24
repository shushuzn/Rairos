"""CLI command: stats."""
from __future__ import annotations

import argparse

import orjson as json

from cli._shared import get_db

from cli._shared import (
    Colors,
    colored,
    print_header,
)


def _build_stats_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser("stats", help="Show database statistics summary")
    p.add_argument("--json", action="store_true", help="Output as JSON")
    return p


def _run_stats(args: argparse.Namespace) -> int:
    db = get_db()
    db.init()
    s = db.get_stats()
    if args.json:
        print(json.dumps(s, option=json.OPT_INDENT_2).decode())
    else:
        print_header("Papers:")
        print(f"  total : {colored(s['total_papers'], Colors.BOLD)}")
        print(f"  by source : {', '.join(f'{colored(k, Colors.OKBLUE)}={v}' for k, v in sorted(s['by_source'].items()))}")
        print(f"  by status : {', '.join(f'{colored(k, Colors.OKGREEN)}={v}' for k, v in sorted(s['by_status'].items()))}")
        print_header("Queue:")
        print(f"  queued  : {s['queue_queued']}")
        print(f"  running : {s['queue_running']}")
        print_header("Cache:")
        print(f"  entries : {s['cache_entries']}")
    print_header("Dedup:")
    print(f"  records : {s['dedup_records']}")
    return 0
