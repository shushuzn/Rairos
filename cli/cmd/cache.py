"""CLI command: cache."""
from __future__ import annotations

import argparse
import orjson as json

from cli._shared import get_db
from cli._shared import (
    Colors, colored, print_success, print_error, print_warning, print_info, print_header,
)


def _build_cache_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser("cache", help="Manage paper cache")
    p.add_argument("--get", metavar="UID", help="Get cached paper by UID")
    p.add_argument("--set", nargs=2, metavar=("UID", "PATH"), help="Cache a paper from JSON")
    p.add_argument("--clear", action="store_true", help="Clear all cache entries")
    p.add_argument("--stats", action="store_true", help="Show cache statistics")
    return p


def _run_cache(args: argparse.Namespace) -> int:
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
