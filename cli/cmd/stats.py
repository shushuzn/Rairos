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
    p.add_argument(
        "--format",
        "-f",
        choices=["table", "warp"],
        default="table",
        help="Output format (default: table)",
    )
    return p  # type: ignore[no-any-return]


def _run_stats(args: argparse.Namespace) -> int:
    db = get_db()
    db.init()
    s = db.get_stats()

    if args.json:
        print(json.dumps(s, option=json.OPT_INDENT_2).decode())
        return 0  # type: ignore[no-any-return]

    if getattr(args, "format", "table") == "warp":
        _run_stats_warp(s)
        return 0  # type: ignore[no-any-return]

    # Default table format
    print_header("Papers:")
    print(f"  total : {colored(s['total_papers'], Colors.BOLD)}")
    print(
        f"  by source : {', '.join(f'{colored(k, Colors.OKBLUE)}={v}' for k, v in sorted(s['by_source'].items()))}"
    )
    print(
        f"  by status : {', '.join(f'{colored(k, Colors.OKGREEN)}={v}' for k, v in sorted(s['by_status'].items()))}"
    )
    print_header("Queue:")
    print(f"  queued  : {s['queue_queued']}")
    print(f"  running : {s['queue_running']}")
    print_header("Cache:")
    print(f"  entries : {s['cache_entries']}")
    print_header("Dedup:")
    print(f"  records : {s['dedup_records']}")
    return 0  # type: ignore[no-any-return]


def _run_stats_warp(s) -> None:
    """Render stats using Warp-style blocks."""
    from cli.warp import WarpBlocks
    from llm.client import get_llm_cache_size, _cache_stats, get_cache_stats

    blocks = []

    # Papers overview table
    paper_rows = [
        ["Total", str(s["total_papers"])],
        *[[k, str(v)] for k, v in sorted(s["by_source"].items())],
        *[[k, str(v)] for k, v in sorted(s["by_status"].items())],
    ]
    blocks.append(WarpBlocks.table(["Papers", "Count"], paper_rows, title="Database Overview"))

    # LLM cache stats
    llm_size = get_llm_cache_size()
    llm_disk = _cache_stats()
    llm_hit = get_cache_stats()
    llm_rate = llm_hit.get("hit_rate", 0)

    # Color-coded hit rate
    if llm_rate >= 80:
        rate_str = f"[#B4FA72]✓ {llm_rate}%[/]"
    elif llm_rate >= 50:
        rate_str = f"[#FEFDC2]⚠ {llm_rate}%[/]"
    else:
        rate_str = f"[#FF5555]✗ {llm_rate}%[/]"

    llm_lines = [
        f"  Entries : [#A5D5FE]{llm_disk.get('entries', llm_size)}[/]",
        f"  Expired : [#8E8E8E]{llm_disk.get('expired', 0)}[/]",
        f"  Hits    : [#B4FA72]{llm_hit.get('hits', 0)}[/]",
        f"  Misses  : [#FF8272]{llm_hit.get('misses', 0)}[/]",
        f"  Hit Rate: {rate_str}",
    ]
    blocks.append(WarpBlocks.panel("LLM Cache", "\n".join(llm_lines)))

    # Queue + Dedup panel
    status_lines = [
        f"Queued  : [#A5D5FE]{s['queue_queued']}[/]",
        f"Running : [#FEFDC2]{s['queue_running']}[/]",
        f"Paper Cache: [#8E8E8E]{s['cache_entries']}[/] entries",
        f"Dedup   : [#8E8E8E]{s['dedup_records']}[/] records",
    ]
    blocks.append(WarpBlocks.panel("Queue & Dedup", "\n".join(status_lines)))

    print("\n\n".join(blocks))
