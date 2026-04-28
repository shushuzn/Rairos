"""
Visual Extraction CLI Command

Usage:
    airos visual extract paper.pdf --output figures/
    airos visual extract paper.pdf --output figures/ --dpi 200
    airos visual extract paper.pdf --save-db 2604.22754
"""
from __future__ import annotations

import argparse
import sys
from datetime import datetime, timezone
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent.parent))

import click
from pdf.visual import VisualExtractor
from db.database import Database, ExperimentTableRecord
from cli._shared import print_success, print_error, print_info


def _build_visual_parser(subparsers):
    p = subparsers.add_parser("visual", help="Extract visual content from PDFs")
    sub = p.add_subparsers(dest="visual_cmd", help="Visual commands")

    # extract command
    extract_p = sub.add_parser("extract", help="Extract figures, formulas, tables")
    extract_p.add_argument("pdf", help="Path to PDF file")
    extract_p.add_argument("--output", "-o", default=None,
                          help="Output directory for extracted images")
    extract_p.add_argument("--dpi", type=int, default=150,
                          help="DPI for rendered formulas (default: 150)")
    extract_p.add_argument("--format", "-f", default="markdown",
                          choices=["markdown", "json"],
                          help="Output format for tables")
    extract_p.add_argument("--save-db", metavar="PAPER_ID", default=None,
                          help="Save tables to database with this paper_id")
    extract_p.set_defaults(func=lambda a: visual_extract.callback(
        pdf=a.pdf, output=a.output, dpi=a.dpi, format=a.format, save_db=a.save_db))

    # query command - query stored tables
    query_p = sub.add_parser("query", help="Query stored tables from database")
    query_p.add_argument("paper_id", help="Paper ID to query tables for")
    query_p.add_argument("--page", type=int, default=None,
                        help="Filter by page number")
    query_p.add_argument("--keyword", "-k", default=None,
                        help="Search in table content")
    query_p.add_argument("--format", "-f", default="markdown",
                        choices=["markdown", "json", "csv"],
                        help="Output format")
    query_p.set_defaults(func=lambda a: visual_query.callback(
        paper_id=a.paper_id, page=a.page, keyword=a.keyword, format=a.format))

    # list command - list papers with stored tables
    list_p = sub.add_parser("list", help="List papers with stored tables")
    list_p.add_argument("--limit", type=int, default=20,
                       help="Maximum number of results (default: 20)")
    list_p.set_defaults(func=lambda a: visual_list.callback(limit=a.limit))

    p.set_defaults(func=lambda a: visual_status.callback())


@click.command("visual")
@click.argument("pdf", required=False)
@click.option("--output", "-o", default=None)
@click.option("--dpi", type=int, default=150)
@click.option("--save-db", default=None, help="Save tables to DB with this paper_id")
def visual(pdf: str, output: str, dpi: int, save_db: str):
    """Extract visual content from PDF."""
    if pdf:
        visual_extract.callback(pdf, output, dpi, "markdown", save_db)
    else:
        visual_status.callback()


def visual_extract(pdf: str, output: str, dpi: int, format: str, save_db: str = None):
    """Extract figures, formulas, and tables from PDF."""
    pdf_path = Path(pdf)

    if not pdf_path.exists():
        print_error(f"PDF not found: {pdf}")
        sys.exit(1)

    output_dir = Path(output) if output else None
    paper_id = pdf_path.stem

    print_info(f"Extracting visual content from: {pdf}")
    print_info(f"Output directory: {output_dir or 'memory only'}")

    try:
        extractor = VisualExtractor(output_dir=str(output_dir) if output_dir else None, dpi=dpi)
        result = extractor.extract_visual_content(str(pdf_path), paper_id)

        # Summary
        print_success(f"\nExtraction complete!")
        print_info(f"  Figures: {len(result.figures)}")
        print_info(f"  Formulas: {len(result.rendered_formulas)}")
        print_info(f"  Tables: {len(result.tables_markdown)}")

        # Save tables to database
        if save_db and result.tables_markdown:
            db = Database()

            # Auto-create paper record if it doesn't exist
            existing = db.get_paper(save_db)
            if not existing:
                db.upsert_paper(
                    paper_id=save_db,
                    source='visual_extract',
                    title=f'Paper {save_db}',
                    abstract=f'Extracted from {pdf_path.name}',
                )
                print_info(f"Created paper record: {save_db}")

            tables = [
                ExperimentTableRecord(
                    id=0,  # Auto-assigned
                    paper_id=save_db,
                    table_caption=t.caption,
                    page=t.page,
                    headers=t.headers,
                    rows=t.rows,
                    bbox_x0=0,
                    bbox_y0=0,
                    bbox_x1=0,
                    bbox_y1=0,
                    created_at=datetime.now(timezone.utc).isoformat(),
                )
                for t in result.tables_markdown
            ]
            db.upsert_experiment_tables(save_db, tables)
            db.close()
            print_success(f"Saved {len(tables)} tables to database as: {save_db}")

        # Print tables as markdown
        if result.tables_markdown:
            print_success("\n--- Tables ---")
            for i, table in enumerate(result.tables_markdown):
                print(f"\n**Table {i + 1} (page {table.page + 1})**")
                if table.caption:
                    print(f"*{table.caption}*")
                print(table.markdown)

        # Save results as JSON if requested
        if format == "json" and output_dir:
            import json
            json_path = output_dir / f"{paper_id}_visual.json"
            with open(json_path, "w", encoding="utf-8") as f:
                json.dump({
                    "paper_id": result.paper_id,
                    "figures": [
                        {"page": fig.page, "caption": fig.caption, "image_path": fig.image_path}
                        for fig in result.figures
                    ],
                    "formulas": [
                        {"latex": f.latex, "is_display": f.is_display, "page": f.page}
                        for f in result.rendered_formulas
                    ],
                    "tables": [
                        {"headers": t.headers, "rows": t.rows, "page": t.page, "caption": t.caption}
                        for t in result.tables_markdown
                    ],
                }, f, indent=2, ensure_ascii=False)
            print_success(f"\nJSON saved: {json_path}")

    except Exception as e:
        print_error(f"Extraction failed: {e}")
        sys.exit(1)


def visual_status():
    """Show visual extraction capabilities."""
    print_info("Visual Extraction Module")
    print_info("  Extracts: figures, LaTeX formulas, tables")
    print_info("\nUsage:")
    print_info("  airos visual extract paper.pdf --output figures/")
    print_info("  airos visual query 2604.22754")
    print_info("  airos visual list")


def visual_query(paper_id: str, page: int, keyword: str, format: str):
    """Query stored tables from database."""
    db = Database()
    try:
        tables = db.get_experiment_tables(paper_id)

        if not tables:
            print_error(f"No tables found for paper: {paper_id}")
            sys.exit(1)

        # Filter by page if specified
        if page is not None:
            tables = [t for t in tables if t.page == page - 1]  # 0-indexed

        # Filter by keyword if specified
        if keyword:
            keyword_lower = keyword.lower()
            tables = [
                t for t in tables
                if keyword_lower in t.table_caption.lower()
                or any(keyword_lower in h.lower() for h in t.headers)
                or any(keyword_lower in str(cell).lower() for row in t.rows for cell in row)
            ]

        if not tables:
            print_error(f"No tables match the query")
            sys.exit(1)

        print_success(f"Found {len(tables)} table(s) for paper: {paper_id}")

        if format == "json":
            import json
            result = {
                "paper_id": paper_id,
                "tables": [
                    {
                        "id": t.id,
                        "page": t.page + 1,
                        "caption": t.table_caption,
                        "headers": t.headers,
                        "rows": t.rows,
                    }
                    for t in tables
                ],
            }
            print(json.dumps(result, indent=2, ensure_ascii=False))
        elif format == "csv":
            for t in tables:
                print(f"\n# Table {t.id} (page {t.page + 1})")
                if t.table_caption:
                    print(f"# Caption: {t.table_caption}")
                print(",".join(t.headers))
                for row in t.rows:
                    print(",".join(f'"{str(c)}"' for c in row))
        else:  # markdown
            for i, t in enumerate(tables):
                print(f"\n**Table {i + 1} (page {t.page + 1})**")
                if t.table_caption:
                    print(f"*{t.table_caption}*")
                print("| " + " | ".join(t.headers) + " |")
                print("| " + " | ".join(["---"] * len(t.headers)) + " |")
                for row in t.rows:
                    print("| " + " | ".join(str(c) for c in row) + " |")

    finally:
        db.close()


def visual_list(limit: int):
    """List papers with stored tables."""
    db = Database()
    try:
        cursor = db.conn.execute("""
            SELECT paper_id, COUNT(*) as table_count, MAX(page) + 1 as max_page
            FROM experiment_tables
            GROUP BY paper_id
            ORDER BY MAX(created_at) DESC
            LIMIT ?
        """, (limit,))
        rows = cursor.fetchall()

        if not rows:
            print_info("No papers with stored tables found.")
            return

        print_success(f"Papers with stored tables ({len(rows)} results):\n")
        print(f"{'Paper ID':<15} {'Tables':<8} {'Pages'}")
        print("-" * 40)
        for row in rows:
            print(f"{row[0]:<15} {row[1]:<8} {row[2]}")

    finally:
        db.close()
