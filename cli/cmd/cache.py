"""CLI command: cache."""
from __future__ import annotations

import argparse
import orjson as json

from cli._shared import get_db


def _build_cache_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser("cache", help="Manage paper and LLM cache")
    p.add_argument("--get", metavar="UID", help="Get cached paper by UID")
    p.add_argument("--set", nargs=2, metavar=("UID", "PATH"), help="Cache a paper from JSON")
    p.add_argument("--clear", action="store_true", help="Clear all cache entries")
    p.add_argument("--stats", action="store_true", help="Show cache statistics")
    # LLM cache options
    p.add_argument("--llm", action="store_true", help="Operate on LLM response cache instead of paper cache")
    p.add_argument("--llm-clear", action="store_true", help="Clear LLM response cache")
    return p


def _run_cache(args: argparse.Namespace) -> int:
    # LLM cache operations
    if args.llm or args.llm_clear:
        from llm.client import clear_llm_cache, get_llm_cache_size, _cache_stats, get_cache_stats, reset_cache_stats
        if args.llm_clear:
            clear_llm_cache()
            reset_cache_stats()
            print("LLM cache cleared")
        elif args.llm:
            size = get_llm_cache_size()
            disk_stats = _cache_stats()
            hit_stats = get_cache_stats()
            print("LLM Response Cache:")
            print(f"  Entries:    {disk_stats.get('entries', size)}")
            print(f"  Expired:    {disk_stats.get('expired', 0)}")
            print(f"  Hits:       {hit_stats.get('hits', 0)}")
            print(f"  Misses:     {hit_stats.get('misses', 0)}")
            print(f"  Hit Rate:   {hit_stats.get('hit_rate', 0)}%")
        return 0

    db = get_db()
    db.init()

    if args.stats:
        entries = db.get_cached_paper("__stats__")
        print(f"Cache size: {entries}")
    elif args.clear:
        deleted = db.clear_cache()
        print(f"Cache cleared ({deleted} entries)")
    elif args.get:
        cached = db.get_cached_paper(args.get)
        if cached:
            print(json.dumps(cached, option=json.OPT_INDENT_2).decode())
        else:
            print(f"No cache entry for {args.get}")
    elif getattr(args, "set", None):
        uid, path = args.set
        try:
            with open(path, encoding="utf-8") as f:
                data = json.loads(f.read())
            db.set_cached_paper(uid, data)
            print(f"Cached {uid} from {path}")
        except Exception as e:
            print(f"Failed to cache {uid}: {e}")
            return 1
    else:
        print("Use --stats, --clear, --get UID, or --set UID PATH")

    return 0
