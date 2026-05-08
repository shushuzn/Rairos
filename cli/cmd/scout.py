"""CLI command: scout — proactively find papers matching Gene Pool interests.

Usage:
    rairos scout                     Scan ArXiv for papers matching Gene Pool
    rairos scout --topic "VLA robot"  Scan for a specific topic
    rairos scout --daemon             Run continuously (check every 6 hours)
    rairos scout --topic "LLM"        Save results to file
"""

from __future__ import annotations

import argparse
import json
import threading
import time
from pathlib import Path

from cli._shared import Colors, print_error, print_success


def _build_scout_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser(
        "scout",
        help="Proactively find papers matching your Gene Pool interests",
        description="Search ArXiv for recent papers, score them against your "
        "Gene Pool capsules, and recommend the most relevant ones.",
    )
    p.add_argument(
        "--topic",
        "-t",
        type=str,
        default="",
        help="Specific topic to search (default: auto-derived from Gene Pool)",
    )
    p.add_argument(
        "--sources",
        "-s",
        type=str,
        default="arxiv",
        choices=["arxiv", "news", "all"],
        help="Sources to search: arxiv, news, or all (default: arxiv)",
    )
    p.add_argument(
        "--limit",
        "-n",
        type=int,
        default=20,
        help="Maximum papers to return (default: 20)",
    )
    p.add_argument(
        "--min-score",
        type=float,
        default=0.15,
        help="Minimum match score (0.0-1.0, default: 0.15)",
    )
    p.add_argument(
        "--daemon",
        "-d",
        action="store_true",
        help="Run continuously, checking every 6 hours",
    )
    p.add_argument(
        "--interval",
        "-i",
        type=int,
        default=360,
        help="Daemon check interval in minutes (default: 360 = 6 hours)",
    )
    p.add_argument(
        "--output",
        "-o",
        type=str,
        default="",
        help="Save results to JSON file instead of printing",
    )
    p.set_defaults(func=_run_scout)
    return p  # type: ignore[no-any-return]


def _run_scout(args) -> None:
    from llm.scout import scout, render_scout_results

    topic = getattr(args, "topic", "")
    sources = getattr(args, "sources", "arxiv")
    limit = getattr(args, "limit", 20)
    min_score = getattr(args, "min_score", 0.15)
    daemon = getattr(args, "daemon", False)
    interval = getattr(args, "interval", 360)
    output = getattr(args, "output", "")

    if daemon:
        _run_daemon(topic, limit, min_score, interval, output, sources)
        return

    results = scout(
        topic=topic,
        sources=sources,
        max_results=limit,
        min_match_score=min_score,
    )

    if output:
        data = [
            {
                "rank": r.rank,
                "arxiv_id": r.arxiv_id,
                "title": r.title,
                "authors": r.authors,
                "published": r.published,
                "match_score": r.match_score,
                "matched_gap_type": r.matched_gap_type,
                "matched_gap_title": r.matched_gap_title,
                "reason": r.reason,
            }
            for r in results
        ]
        Path(output).write_text(json.dumps(data, indent=2, ensure_ascii=False), encoding="utf-8")
        print_success(f"Results saved to {output}")
    else:
        print(render_scout_results(results))


def _run_daemon(
    topic: str,
    limit: int,
    min_score: float,
    interval_minutes: int,
    output: str,
    sources: str = "arxiv",
) -> None:
    """Run scout in a continuous loop."""
    from llm.scout import scout, render_scout_results

    print(f"  Scout daemon started (interval={interval_minutes}min)")
    print("  Ctrl+C to stop\n")

    while True:
        try:
            results = scout(
                topic=topic, sources=sources, max_results=limit, min_match_score=min_score
            )
            ts = time.strftime("%Y-%m-%d %H:%M")

            if output:
                data = [
                    {
                        "rank": r.rank,
                        "arxiv_id": r.arxiv_id,
                        "title": r.title,
                        "match_score": r.match_score,
                        "matched_gap_type": r.matched_gap_type,
                    }
                    for r in results
                ]
                Path(output).write_text(
                    json.dumps(data, indent=2, ensure_ascii=False), encoding="utf-8"
                )

            print(f"[{ts}] Scout found {len(results)} matching papers")
            for r in results[:5]:
                print(f"  #{r.rank} {r.title[:60]} ({r.match_score:.2f})")
            if len(results) > 5:
                print(f"  ... and {len(results) - 5} more")
            print()
        except KeyboardInterrupt:
            print("\n  Scout daemon stopped.")
            break
        except Exception as e:
            print_error(f"Scout error: {e}")

        time.sleep(interval_minutes * 60)
