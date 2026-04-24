"""CLI command: export."""
from __future__ import annotations

import argparse
import csv as _csv
import io as _io
import json as _json

from pathlib import Path

from cli._shared import get_db


def _build_export_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser("export", help="Export all papers to CSV or JSON")
    p.add_argument("--format", choices=["csv", "json"], default="csv", help="Output format (default: csv)")
    p.add_argument("--limit", type=int, default=0, help="Limit number of papers (0 = all)")
    p.add_argument("--out", metavar="FILE", help="Write to file instead of stdout")
    return p


def _run_export(args: argparse.Namespace) -> int:
    db = get_db()
    db.init()
    fields, rows = db.export_papers(format=args.format, limit=args.limit)

    output = _io.StringIO()

    if args.format == "csv":
        writer = _csv.DictWriter(output, fieldnames=fields)
        writer.writeheader()
        writer.writerows(rows)
        content = output.getvalue()
    else:
        content = _json.dumps(rows, indent=2)

    if args.out:
        Path(args.out).write_text(content, encoding="utf-8")
        print(f"Exported {len(rows)} papers to {args.out}")
    else:
        print(content)

    return 0
