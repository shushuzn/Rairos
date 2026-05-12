"""
Visual Extraction CLI Command

Usage:
    airos visual extract paper.pdf --output figures/
    airos visual extract paper.pdf --output figures/ --dpi 200
    airos visual extract paper.pdf --save-db 2604.22754
"""

from __future__ import annotations

import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import List, Optional, cast

sys.path.insert(0, str(Path(__file__).parent.parent.parent))

from pdf.visual import VisualExtractor
from db.database import Database, ExperimentTableRecord
from cli._shared import print_success, print_error, print_info
from cli.warp import WarpBlocks

# Chart KG integration
try:
    from pdf.chart_kg import ChartKGExtractor
    from kg.manager import KGManager
    from kg.integration import KGIntegration

    _HAS_CHART_KG = True
except ImportError:
    _HAS_CHART_KG = False


def _build_visual_parser(subparsers):
    p = subparsers.add_parser("visual", help="Extract visual content from PDFs")
    sub = p.add_subparsers(dest="visual_cmd", help="Visual commands")

    # extract command
    extract_p = sub.add_parser("extract", help="Extract figures, formulas, tables")
    extract_p.add_argument("pdf", help="Path to PDF file")
    extract_p.add_argument(
        "--output", "-o", default=None, help="Output directory for extracted images"
    )
    extract_p.add_argument(
        "--dpi", type=int, default=150, help="DPI for rendered formulas (default: 150)"
    )
    extract_p.add_argument(
        "--format",
        "-f",
        default="markdown",
        choices=["markdown", "json"],
        help="Output format for tables",
    )
    extract_p.add_argument(
        "--save-db",
        metavar="PAPER_ID",
        default=None,
        help="Save tables to database with this paper_id",
    )
    extract_p.set_defaults(
        func=lambda a: visual_extract.callback(
            pdf=a.pdf, output=a.output, dpi=a.dpi, format=a.format, save_db=a.save_db
        )
    )

    # query command - query stored tables
    query_p = sub.add_parser("query", help="Query stored tables from database")
    query_p.add_argument("paper_id", help="Paper ID to query tables for")
    query_p.add_argument("--page", type=int, default=None, help="Filter by page number")
    query_p.add_argument("--keyword", "-k", default=None, help="Search in table content")
    query_p.add_argument(
        "--format",
        "-f",
        default="markdown",
        choices=["markdown", "json", "csv"],
        help="Output format",
    )
    query_p.set_defaults(
        func=lambda a: visual_query.callback(
            paper_id=a.paper_id, page=a.page, keyword=a.keyword, format=a.format
        )
    )

    # list command - list papers with stored tables
    list_p = sub.add_parser("list", help="List papers with stored tables")
    list_p.add_argument(
        "--limit", type=int, default=20, help="Maximum number of results (default: 20)"
    )
    list_p.set_defaults(func=lambda a: visual_list.callback(limit=a.limit))

    # export command - export tables to file
    export_p = sub.add_parser("export", help="Export stored tables to file")
    export_p.add_argument("paper_id", help="Paper ID to export tables from")
    export_p.add_argument("output", help="Output file path")
    export_p.add_argument(
        "--format",
        "-f",
        default="csv",
        choices=["csv", "json", "markdown"],
        help="Output format (default: csv)",
    )
    export_p.add_argument("--page", type=int, default=None, help="Filter by page number")
    export_p.add_argument("--keyword", "-k", default=None, help="Search in table content")
    export_p.set_defaults(
        func=lambda a: visual_export.callback(
            paper_id=a.paper_id, output=a.output, format=a.format, page=a.page, keyword=a.keyword
        )
    )

    # chart command - index figures/tables to KG and query
    chart_p = sub.add_parser("chart", help="Index figures/tables to KG and query them")
    chart_p.add_argument("paper_id", nargs="?", default=None, help="Paper ID to index/query")
    chart_p.add_argument(
        "--index",
        "-i",
        metavar="PDF_PATH",
        default=None,
        help="Index figures/tables from PDF into KG",
    )
    chart_p.add_argument(
        "--list", "-l", action="store_true", help="List all indexed figures/tables for the paper"
    )
    chart_p.add_argument(
        "--figure",
        "-f",
        metavar="LABEL",
        default=None,
        help="Query specific figure, e.g. 'Figure 3'",
    )
    chart_p.add_argument(
        "--table", "-t", metavar="LABEL", default=None, help="Query specific table, e.g. 'Table 1'"
    )
    chart_p.set_defaults(
        func=lambda a: visual_chart.callback(
            paper_id=a.paper_id, index=a.index, list_charts=a.list, figure=a.figure, table=a.table
        )
    )

    p.set_defaults(func=lambda a: _show_visual_status())


def _show_visual_status():
    """Show visual extraction capabilities (used by CLI registry)."""
    from rich.console import Console

    c = Console()
    c.rule("[bold #FF8272]  Visual Extraction  [/]")
    print()
    rows = [
        ["[#A5D5FE]✓[/]", "Figure extraction", "PNG/JPG from PDF pages"],
        ["[#A5D5FE]✓[/]", "LaTeX rendering", "Formulas as high-DPI images"],
        ["[#A5D5FE]✓[/]", "Table extraction", "Markdown + CSV + JSON"],
    ]
    c.print(WarpBlocks.table(["", "Capability", "Format"], rows, title="Supported Types"))
    c.print()
    print(
        WarpBlocks.section(
            "Usage",
            "[#A5D5FE]airos visual extract[/] paper.pdf --output figures/",
            "[#A5D5FE]airos visual query[/] 2604.22754",
            "[#A5D5FE]airos visual list[/]",
        )
    )


def visual_extract(pdf: str, output: str, dpi: int, format: str, save_db: Optional[str] = None):
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
        print_success("\nExtraction complete!")
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
                    source="visual_extract",
                    title=f"Paper {save_db}",
                    abstract=f"Extracted from {pdf_path.name}",
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
                json.dump(
                    {
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
                            {
                                "headers": t.headers,
                                "rows": t.rows,
                                "page": t.page,
                                "caption": t.caption,
                            }
                            for t in result.tables_markdown
                        ],
                    },
                    f,
                    indent=2,
                    ensure_ascii=False,
                )
            print_success(f"\nJSON saved: {json_path}")

    except Exception as e:
        print_error(f"Extraction failed: {e}")
        sys.exit(1)


def visual_status():
    """Show visual extraction capabilities."""
    from rich.console import Console

    c = Console()
    c.rule("[bold #FF8272]  Visual Extraction  [/]")
    print()
    rows = [
        ["[#A5D5FE]✓[/]", "Figure extraction", "PNG/JPG from PDF pages"],
        ["[#A5D5FE]✓[/]", "LaTeX rendering", "Formulas as high-DPI images"],
        ["[#A5D5FE]✓[/]", "Table extraction", "Markdown + CSV + JSON"],
    ]
    c.print(WarpBlocks.table(["", "Capability", "Format"], rows, title="Supported Types"))
    c.print()
    print(
        WarpBlocks.section(
            "Usage",
            "[#A5D5FE]airos visual extract[/] paper.pdf --output figures/",
            "[#A5D5FE]airos visual query[/] 2604.22754",
            "[#A5D5FE]airos visual list[/]",
            width=60,
        )
    )


def visual_query(paper_id: str, page: int, keyword: str, format: str):
    """Query stored tables from database."""
    db = Database()
    try:
        tables = cast(List["ExperimentTableRecord"], db.get_experiment_tables(paper_id))

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
                t
                for t in tables
                if keyword_lower in t.table_caption.lower()
                or any(keyword_lower in h.lower() for h in t.headers)
                or any(keyword_lower in str(cell).lower() for row in t.rows for cell in row)
            ]

        if not tables:
            print_error("No tables match the query")
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
        cursor = db.conn.execute(
            """
            SELECT paper_id, COUNT(*) as table_count, MAX(page) + 1 as max_page
            FROM experiment_tables
            GROUP BY paper_id
            ORDER BY MAX(created_at) DESC
            LIMIT ?
        """,
            (limit,),
        )
        raw_rows = cursor.fetchall()

        if not raw_rows:
            from rich.console import Console

            c = Console()
            c.rule("[bold #FF8272]  Visual Tables  [/]")
            c.print()
            print(WarpBlocks.panel("No Results", "[#8E8E8E]No papers with stored tables found[/]"))
            return

        from rich.console import Console

        c = Console()
        c.rule("[bold #FF8272]  Visual Tables  [/]")
        c.print()
        rows = []
        for row in raw_rows:
            badge = "[#B4FA72]●[/]" if row[1] > 0 else "[#8E8E8E]○[/]"
            rows.append([badge, row[0], str(row[1]), str(row[2])])
        c.print(
            WarpBlocks.table(
                ["", "Paper ID", "Tables", "Pages"],
                rows,
                title=f"Papers with Stored Tables ({len(raw_rows)})",
            )
        )

    finally:
        db.close()


def visual_export(paper_id: str, output: str, format: str, page: int, keyword: str):
    """Export stored tables to a file."""
    db = Database()
    try:
        tables = cast(List["ExperimentTableRecord"], db.get_experiment_tables(paper_id))

        if not tables:
            print_error(f"No tables found for paper: {paper_id}")
            sys.exit(1)

        # Filter by page if specified
        if page is not None:
            tables = [t for t in tables if t.page == page - 1]

        # Filter by keyword if specified
        if keyword:
            keyword_lower = keyword.lower()
            tables = [
                t
                for t in tables
                if keyword_lower in t.table_caption.lower()
                or any(keyword_lower in h.lower() for h in t.headers)
                or any(keyword_lower in str(cell).lower() for row in t.rows for cell in row)
            ]

        if not tables:
            print_error("No tables match the query")
            sys.exit(1)

        # Generate content
        if format == "json":
            import json

            content = json.dumps(
                {
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
                },
                indent=2,
                ensure_ascii=False,
            )
        elif format == "csv":
            lines = []
            for t in tables:
                lines.append(f"# Table {t.id} (page {t.page + 1})")
                if t.table_caption:
                    lines.append(f"# Caption: {t.table_caption}")
                lines.append(",".join(f'"{h}"' for h in t.headers))
                for row in t.rows:
                    lines.append(",".join(f'"{c}"' for c in row))
                lines.append("")
            content = "\n".join(lines)
        else:  # markdown
            lines = []
            for i, t in enumerate(tables):
                lines.append(f"\n**Table {i + 1} (page {t.page + 1})**")
                if t.table_caption:
                    lines.append(f"*{t.table_caption}*")
                lines.append("| " + " | ".join(t.headers) + " |")
                lines.append("| " + " | ".join(["---"] * len(t.headers)) + " |")
                for row in t.rows:
                    lines.append("| " + " | ".join(str(c) for c in row) + " |")
            content = "\n".join(lines)

        # Write to file
        Path(output).parent.mkdir(parents=True, exist_ok=True)
        with open(output, "w", encoding="utf-8") as f:
            f.write(content)

        print_success(f"Exported {len(tables)} table(s) to: {output}")

    finally:
        db.close()


def visual_chart(paper_id: str, index: str, list_charts: bool, figure: str, table: str):
    """Index figures/tables to KG and query them."""
    if not _HAS_CHART_KG:
        print_error("Chart KG not available. Install required dependencies.")
        sys.exit(1)

    if not paper_id:
        print_error("Paper ID required. Usage: airos visual chart <paper_id>")
        sys.exit(1)

    kg = KGManager()
    integ = KGIntegration(kg)
    extractor = ChartKGExtractor(kg)

    # Index mode
    if index:
        pdf_path = Path(index)
        if not pdf_path.exists():
            print_error(f"PDF not found: {index}")
            sys.exit(1)

        # Get paper title from DB if available
        db = Database()
        paper = db.get_paper(paper_id)
        paper_title = paper.title if paper else paper_id
        db.close()

        print_info(f"Indexing charts from {pdf_path} for paper: {paper_id}")
        try:
            fig_nodes, tbl_nodes = extractor.extract_and_index(str(pdf_path), paper_id, paper_title)
            integ.on_charts_indexed(paper_id, fig_nodes, tbl_nodes)
            print_success(f"Indexed {len(fig_nodes)} figures and {len(tbl_nodes)} tables")
        except Exception as e:
            print_error(f"Indexing failed: {e}")
            sys.exit(1)
        return

    # Query mode - list
    if list_charts:
        figures = extractor.get_paper_figures(paper_id)
        tables = extractor.get_paper_tables(paper_id)

        from rich.console import Console

        c = Console()
        c.rule(f"[bold #FF8272]  Charts for {paper_id}  [/]")
        c.print()

        if figures:
            c.print("[bold]Figures:[/]")
            for fig in figures:
                props = fig.get("properties", {})
                page = props.get("page", "?")
                desc = props.get("description", "")[:80]
                c.print(f"  [#A5D5FE]•[/] {fig['label']} (p.{page + 1})")
                if desc:
                    c.print(f"    {desc}...")
                c.print()

        if tables:
            c.print("[bold]Tables:[/]")
            for tbl in tables:
                props = tbl.get("properties", {})
                page = props.get("page", "?")
                desc = props.get("description", "")[:80]
                c.print(f"  [#A5D5FE]•[/] {tbl['label']} (p.{page + 1})")
                if desc:
                    c.print(f"    {desc}...")
                c.print()

        if not figures and not tables:
            print_info(f"No charts indexed for {paper_id}")
            print_info("Run: airos visual chart <paper_id> --index paper.pdf")

        return

    # Query specific figure
    if figure:
        fig_node = extractor.query_figure(paper_id, figure)
        if fig_node:
            props = fig_node.get("properties", {})
            print_success(f"Figure: {fig_node['label']}")
            print(f"Page: {props.get('page', '?') + 1}")
            print(f"Caption: {props.get('caption', 'N/A')}")
            print(f"\nDescription:\n{props.get('description', 'N/A')}")
            if props.get("image_path"):
                print(f"\nImage: {props.get('image_path')}")
        else:
            print_error(f"Figure '{figure}' not found for {paper_id}")
        return

    # Query specific table
    if table:
        tables = extractor.get_paper_tables(paper_id)
        tbl_node = None
        for t in tables:
            if table.lower() in t["label"].lower():
                tbl_node = t
                break

        if tbl_node:
            props = tbl_node.get("properties", {})
            print_success(f"Table: {tbl_node['label']}")
            print(f"Page: {props.get('page', '?') + 1}")
            print(f"Caption: {props.get('caption', 'N/A')}")
            print(f"\nDescription:\n{props.get('description', 'N/A')}")
            print(f"\nMarkdown:\n{props.get('markdown', 'N/A')}")
        else:
            print_error(f"Table '{table}' not found for {paper_id}")
        return

    # No specific action - show help
    print_info("Usage:")
    print("  airos visual chart <paper_id> --index paper.pdf   # Index charts")
    print("  airos visual chart <paper_id> --list              # List all charts")
    print("  airos visual chart <paper_id> -f 'Figure 3'     # Query figure")
    print("  airos visual chart <paper_id> -t 'Table 1'       # Query table")
