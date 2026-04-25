"""
Visual Extraction CLI Command

Usage:
    airos visual extract paper.pdf --output figures/
    airos visual extract paper.pdf --output figures/ --dpi 200
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent.parent))

import click
from pdf.visual import VisualExtractor
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
    extract_p.set_defaults(func=lambda a: visual_extract.callback(
        pdf=a.pdf, output=a.output, dpi=a.dpi, format=a.format))

    p.set_defaults(func=lambda a: visual_status.callback())


@click.command("visual")
@click.argument("pdf", required=False)
@click.option("--output", "-o", default=None)
@click.option("--dpi", type=int, default=150)
def visual(pdf: str, output: str, dpi: int):
    """Extract visual content from PDF."""
    if pdf:
        visual_extract.callback(pdf, output, dpi, "markdown")
    else:
        visual_status.callback()


def visual_extract(pdf: str, output: str, dpi: int, format: str):
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
