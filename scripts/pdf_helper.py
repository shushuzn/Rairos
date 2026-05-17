#!/usr/bin/env python3
"""
PDF Generation Helper for Rairos MCP Tools.

Generates PDFs from structured content using fpdf2.
Requires: fpdf2

Usage:
    python pdf_helper.py --type review --data '{"title": "...", "sections": [...]}'
    python pdf_helper.py --type abstract --data '{"title": "...", "abstract": "...", "authors": [...]}'
"""

import argparse
import json
import sys
from typing import Dict, Any, List, Optional
from datetime import datetime

try:
    from fpdf import FPDF
except ImportError:
    print("ERROR: fpdf2 not installed. Run: uv pip install fpdf2 --system")
    sys.exit(1)


class RairosPDF(FPDF):
    def __init__(self):
        super().__init__()
        self.set_auto_page_break(auto=True, margin=15)

    def header(self):
        self.set_font('Helvetica', 'I', 8)
        self.set_text_color(128, 128, 128)
        self.cell(0, 10, 'Rairos Research OS', align='R')
        self.ln(5)

    def footer(self):
        self.set_y(-15)
        self.set_font('Helvetica', 'I', 8)
        self.set_text_color(128, 128, 128)
        self.cell(0, 10, f'Page {self.page_no()}', align='C')

    def section_title(self, title: str, level: int = 1):
        if level == 1:
            self.set_font('Helvetica', 'B', 14)
            self.set_text_color(26, 26, 46)
            self.ln(4)
            self.cell(0, 8, title)
            self.ln(8)
            self.set_draw_color(26, 26, 46)
            self.set_line_width(0.5)
            self.line(10, self.get_y(), 200, self.get_y())
            self.ln(4)
        elif level == 2:
            self.set_font('Helvetica', 'B', 12)
            self.set_text_color(45, 45, 68)
            self.ln(2)
            self.cell(0, 7, title)
            self.ln(7)
        else:
            self.set_font('Helvetica', 'B', 11)
            self.set_text_color(61, 61, 92)
            self.ln(2)
            self.cell(0, 6, title)
            self.ln(6)

    def body_text(self, text: str):
        self.set_font('Helvetica', '', 10)
        self.set_text_color(51, 51, 51)
        self.multi_cell(0, 5, text)
        self.ln(2)

    def bullet_point(self, text: str, indent: int = 10):
        self.set_font('Helvetica', '', 10)
        self.set_text_color(51, 51, 51)
        self.set_x(indent)
        self.cell(5, 5, chr(149))
        self.multi_cell(0, 5, text)

    def citation(self, text: str):
        self.set_font('Helvetica', 'I', 9)
        self.set_text_color(85, 85, 85)
        self.set_x(15)
        self.multi_cell(0, 4, text)

    def abstract_box(self, title: str, abstract: str):
        self.set_draw_color(0, 102, 204)
        self.set_line_width(0.5)
        self.set_fill_color(240, 247, 255)
        y_start = self.get_y()
        self.rect(10, y_start, 190, 30, 'DF')
        self.set_xy(12, y_start + 2)
        self.set_font('Helvetica', 'B', 10)
        self.set_text_color(0, 102, 204)
        self.cell(0, 5, 'ABSTRACT')
        self.set_xy(12, y_start + 8)
        self.set_font('Helvetica', '', 9)
        self.set_text_color(51, 51, 51)
        self.multi_cell(186, 4, abstract)
        self.set_y(y_start + 32)


def generate_literature_review_pdf(data: Dict[str, Any], output: str) -> bool:
    """Generate a literature review PDF from structured data."""
    try:
        pdf = RairosPDF()
        pdf.add_page()

        # Title
        pdf.set_font('Helvetica', 'B', 18)
        pdf.set_text_color(26, 26, 46)
        title = data.get('title', 'Literature Review')
        pdf.multi_cell(0, 8, title)
        pdf.ln(3)

        # Subtitle / Topic
        if 'topic' in data:
            pdf.set_font('Helvetica', 'I', 11)
            pdf.set_text_color(85, 85, 85)
            pdf.cell(0, 6, f"Topic: {data['topic']}")
            pdf.ln(10)

        # Date and stats
        pdf.set_font('Helvetica', '', 9)
        pdf.set_text_color(102, 102, 102)
        pdf.cell(0, 5, f"Generated: {datetime.now().strftime('%Y-%m-%d')}")
        pdf.ln(5)
        if 'papers_reviewed' in data:
            pdf.cell(0, 5, f"Papers Reviewed: {data['papers_reviewed']}")
            pdf.ln(5)
        pdf.ln(5)

        # Abstract
        if 'abstract' in data:
            pdf.abstract_box('Abstract', data['abstract'])

        # Sections
        sections = data.get('sections', [])
        for section in sections:
            section_title = section.get('title', section.get('heading', 'Section'))
            section_level = section.get('level', 1)
            pdf.section_title(section_title, section_level)

            # Content
            content = section.get('content', '')
            if isinstance(content, list):
                for item in content:
                    if isinstance(item, dict):
                        if item.get('type') == 'bullet':
                            pdf.bullet_point(item.get('text', ''))
                        elif item.get('type') == 'citation':
                            pdf.citation(item.get('text', ''))
                        else:
                            pdf.body_text(str(item))
                    else:
                        pdf.body_text(str(item))
            else:
                pdf.body_text(str(content))

            pdf.ln(3)

        # References
        if 'references' in data:
            pdf.add_page()
            pdf.section_title('References', 1)
            for i, ref in enumerate(data['references'], 1):
                if isinstance(ref, dict):
                    ref_text = ref.get('formatted', ref.get('text', str(ref)))
                else:
                    ref_text = str(ref)
                pdf.set_font('Helvetica', '', 9)
                pdf.set_text_color(51, 51, 51)
                pdf.set_x(15)
                pdf.multi_cell(0, 4, f"[{i}] {ref_text}")
                pdf.ln(2)

        pdf.output(output)
        return True
    except Exception as e:
        print(f"ERROR: {e}")
        return False


def generate_research_brief_pdf(data: Dict[str, Any], output: str) -> bool:
    """Generate a research brief PDF from structured data."""
    try:
        pdf = RairosPDF()
        pdf.add_page()

        # Title
        pdf.set_font('Helvetica', 'B', 16)
        pdf.set_text_color(26, 26, 46)
        pdf.multi_cell(0, 8, data.get('title', 'Research Brief'))
        pdf.ln(3)

        # arXiv ID
        if 'arxiv_id' in data:
            pdf.set_font('Helvetica', '', 9)
            pdf.set_text_color(102, 102, 102)
            pdf.cell(0, 5, f"arXiv: {data['arxiv_id']}")
            pdf.ln(8)

        # Authors
        if 'authors' in data:
            authors = data['authors']
            if isinstance(authors, list):
                authors = ', '.join(authors)
            pdf.set_font('Helvetica', 'I', 10)
            pdf.set_text_color(51, 51, 51)
            pdf.multi_cell(0, 5, authors)
            pdf.ln(3)

        # Summary
        if 'summary' in data:
            pdf.section_title('Summary', 2)
            pdf.body_text(data['summary'])

        # Key Contributions
        if 'key_contributions' in data:
            pdf.section_title('Key Contributions', 2)
            contribs = data['key_contributions']
            if isinstance(contribs, list):
                for c in contribs:
                    pdf.bullet_point(c)
            else:
                pdf.body_text(str(contribs))
            pdf.ln(2)

        # Methodology
        if 'methodology' in data:
            pdf.section_title('Methodology', 2)
            pdf.body_text(data['methodology'])

        # Results
        if 'results' in data:
            pdf.section_title('Results', 2)
            pdf.body_text(data['results'])

        # Verdict
        if 'verdict' in data:
            pdf.section_title('Verdict', 2)
            pdf.body_text(data['verdict'])

        pdf.output(output)
        return True
    except Exception as e:
        print(f"ERROR: {e}")
        return False


def generate_markdown_pdf(content: str, output: str, title: str = "Document") -> bool:
    """Generate a simple PDF from markdown text (basic formatting only)."""
    try:
        pdf = RairosPDF()
        pdf.add_page()

        lines = content.split('\n')
        in_code_block = False

        for line in lines:
            stripped = line.strip()

            # Code block
            if stripped.startswith('```'):
                in_code_block = not in_code_block
                if in_code_block:
                    pdf.ln(2)
                continue

            if in_code_block:
                pdf.set_font('Courier', '', 9)
                pdf.set_text_color(51, 51, 51)
                pdf.set_x(15)
                pdf.cell(0, 4, stripped[:120])
                pdf.ln(4)
                continue

            # Headers
            if stripped.startswith('# '):
                pdf.section_title(stripped[2:], 1)
            elif stripped.startswith('## '):
                pdf.section_title(stripped[3:], 2)
            elif stripped.startswith('### '):
                pdf.section_title(stripped[4:], 3)
            # Bullet points
            elif stripped.startswith('- ') or stripped.startswith('* '):
                pdf.bullet_point(stripped[2:])
            # Empty lines
            elif not stripped:
                continue
            # Regular text
            else:
                # Remove markdown formatting
                clean = stripped.replace('**', '').replace('*', '').replace('`', '')
                pdf.body_text(clean)

        pdf.output(output)
        return True
    except Exception as e:
        print(f"ERROR: {e}")
        return False


def main():
    parser = argparse.ArgumentParser(description='Rairos PDF Generation Helper')
    parser.add_argument('--type', required=True,
                       choices=['review', 'brief', 'markdown'],
                       help='Document type')
    parser.add_argument('--data', type=str,
                       help='JSON data for the document')
    parser.add_argument('--file', type=str,
                       help='Markdown file (for markdown type)')
    parser.add_argument('--output', required=True,
                       help='Output PDF path')

    args = parser.parse_args()

    try:
        if args.type == 'markdown':
            if not args.file:
                print("ERROR: --file required for markdown type")
                sys.exit(1)
            with open(args.file, 'r', encoding='utf-8') as f:
                content = f.read()
            success = generate_markdown_pdf(content, args.output)
        else:
            if not args.data:
                print("ERROR: --data required for review/brief types")
                sys.exit(1)
            data = json.loads(args.data)

            if args.type == 'review':
                success = generate_literature_review_pdf(data, args.output)
            else:
                success = generate_research_brief_pdf(data, args.output)

        if success:
            print(f"SUCCESS: {args.output}")
            sys.exit(0)
        else:
            sys.exit(1)
    except json.JSONDecodeError as e:
        print(f"ERROR: Invalid JSON: {e}")
        sys.exit(1)
    except Exception as e:
        print(f"ERROR: {e}")
        sys.exit(1)


if __name__ == '__main__':
    main()
