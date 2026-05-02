"""CLI command: export."""
from __future__ import annotations

import argparse
import csv as _csv
import io as _io
import json as _json
import re
from pathlib import Path

from cli._shared import get_db


def _sanitize_bibtex(value: str) -> str:
    """Escape special characters for BibTeX field values."""
    if not value:
        return ""
    # Escape braces and backslashes
    value = value.replace("\\", "\\\\")
    value = value.replace("{", "\\{").replace("}", "\\}")
    # Handle accented characters (keep as-is for biber)
    return value


def _make_bibtex_key(paper) -> str:
    """Generate a BibTeX citation key from paper data."""
    # Use first author surname + year + first meaningful word
    authors = paper.authors if hasattr(paper, "authors") else []
    first_author = "Unknown"
    if authors:
        raw = authors[0].strip()
        parts = raw.split()
        first_author = parts[-1] if parts else "Unknown"
        # Remove non-alphabetic characters
        first_author = re.sub(r"[^a-zA-Z]", "", first_author)
    year = paper.published[:4] if paper.published and len(paper.published) >= 4 else "0000"
    # Get first content word from title
    title_words = re.findall(r"[a-zA-Z]+", paper.title or "")
    first_word = title_words[0].lower() if title_words else "paper"
    return f"{first_author.lower()}{year}{first_word}"


def _paper_to_bibtex(paper) -> str:
    """Convert a PaperRecord to BibTeX entry string."""
    key = _make_bibtex_key(paper)
    lines = [f"@article{{{key},"]

    if paper.title:
        lines.append(f"  title = {{{_sanitize_bibtex(paper.title)}}},")

    if paper.authors:
        # Format authors: "Last, First and Last2, First2"
        author_str = " and ".join(a.strip() for a in paper.authors if a.strip())
        lines.append(f"  author = {{{_sanitize_bibtex(author_str)}}}, ")

    if paper.published:
        lines.append(f"  year = {{{paper.published[:4]}}},")

    if paper.journal:
        lines.append(f"  journal = {{{_sanitize_bibtex(paper.journal)}}},")

    if paper.doi:
        lines.append(f"  doi = {{{paper.doi}}},")

    if paper.abs_url:
        lines.append(f"  url = {{{paper.abs_url}}},")

    if paper.abstract:
        lines.append(f"  abstract = {{{_sanitize_bibtex(paper.abstract)}}},")

    if paper.primary_category:
        lines.append(f"  categories = {{{paper.primary_category}}},")

    # arXiv ID as note
    if paper.id:
        lines.append(f"  note = {{{paper.id}}},")

    lines.append("}")
    return "\n".join(lines)


def _build_export_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser("export", help="Export papers to CSV, JSON, or BibTeX")
    p.add_argument("--format", choices=["csv", "json", "bibtex"], default="csv",
                   help="Output format (default: csv)")
    p.add_argument("--limit", type=int, default=0, help="Limit number of papers (0 = all)")
    p.add_argument("--out", metavar="FILE", help="Write to file instead of stdout")
    p.add_argument("--paper", metavar="ID", help="Export single paper by ID (overrides --limit)")
    return p


def _run_export(args: argparse.Namespace) -> int:
    db = get_db()
    db.init()

    if getattr(args, "paper", None):
        # Export single paper
        if not db.paper_exists(args.paper):
            print(f"Paper not found: {args.paper}", file=__import__("sys").stderr)
            return 1
        paper = db.get_paper(args.paper)
        if args.format == "bibtex":
            content = _paper_to_bibtex(paper)
        elif args.format == "json":
            content = _json.dumps({
                "id": paper.id, "title": paper.title, "authors": paper.authors,
                "abstract": paper.abstract, "published": paper.published,
                "doi": paper.doi, "journal": paper.journal,
                "primary_category": paper.primary_category,
            }, indent=2, ensure_ascii=False)
        else:
            # CSV for single paper
            fields = ["id", "title", "authors", "abstract", "published", "doi"]
            output = _io.StringIO()
            writer = _csv.DictWriter(output, fieldnames=fields)
            writer.writeheader()
            row = {k: getattr(paper, k, "") for k in fields}
            row["authors"] = ", ".join(paper.authors) if paper.authors else ""
            writer.writerow(row)
            content = output.getvalue()
    else:
        fields, rows = db.export_papers(format=args.format, limit=args.limit)

        if args.format == "bibtex":
            # rows are dicts from export_papers
            output = _io.StringIO()
            for row in rows:
                # Reconstruct minimal paper-like object for _paper_to_bibtex
                class FakePaper:
                    pass
                p = FakePaper()
                p.id = row.get("id", "")
                p.title = row.get("title", "")
                p.authors = row.get("authors", "").split(" and ") if row.get("authors") else []
                p.published = row.get("published", "")
                p.journal = row.get("journal", "")
                p.doi = row.get("doi", "")
                p.abs_url = row.get("abs_url", "")
                p.abstract = row.get("abstract", "")
                p.primary_category = row.get("primary_category", "")
                output.write(_paper_to_bibtex(p))
                output.write("\n\n")
            content = output.getvalue()
        elif args.format == "csv":
            output = _io.StringIO()
            writer = _csv.DictWriter(output, fieldnames=fields)
            writer.writeheader()
            writer.writerows(rows)
            content = output.getvalue()
        else:
            content = _json.dumps(rows, indent=2)

    if args.out:
        Path(args.out).write_text(content, encoding="utf-8")
        count = 1 if getattr(args, "paper", None) else len(rows)
        print(f"Exported {count} paper{'s' if count != 1 else ''} to {args.out}")
    else:
        print(content)

    return 0
