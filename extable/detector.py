"""PDF table detection using PyMuPDF."""

import re
from pathlib import Path




_METRIC_KEYWORDS = [
    "accuracy", "precision", "recall", "f1", "bleu", "rouge", "perplexity",
    "loss", "auc", "map", "ndcg", "mrr", "cer", "wer", "beam", "latency",
    "throughput", "param", "bpc", "bits_per_char", "ppl",
    "glue", "super gl", "squad", "arc", "hella", "lambada",
]

_DATASET_KEYWORDS = [
    "squad", "glue", "coco", "imagenet", "mnist", "cifar", "wikitext",
    "openwebtext", "bookcorpus", "arxiv", "pubmed", "custom", "sst",
    "sst-2", "qqp", "mnli", "qnli", "rte", "cola",
]


class TableDetector:
    """Detect and extract tables from PDF pages using PyMuPDF (fitz)."""

    def __init__(self):
        self._has_fitz = True

    def detect_tables(self, page_source: str | Path | int,
                     pdf_path: Path | None = None) -> list[dict]:
        """Detect tables on a page.

        page_source: page number (int) OR path to page image (for OCR fallback)
        Returns list of {bbox, rows, cols, is_experiment_table}.
        """
        try:
            import fitz
        except ImportError:
            return []

        if isinstance(page_source, (int,)):
            page_num = int(page_source)
        else:
            page_num = 0

        doc = fitz.open(str(pdf_path))
        page = doc[page_num]
        blocks = page.get_text("dict")["blocks"]

        tables = []
        for block in blocks:
            if block.get("type") != 0:  # not text block
                continue
            table_bbox = block.get("bbox")
            if not table_bbox:
                continue

            # Extract table data
            table_data = self._extract_table_from_block(page, block)
            if not table_data or len(table_data) < 2:
                continue

            is_exp = self._is_experiment_table(table_data)
            tables.append({
                "bbox": table_bbox,
                "data": table_data,
                "is_experiment": is_exp,
                "page": page_num,
            })

        doc.close()
        return tables

    def _extract_table_from_block(self, page, block: dict) -> list[list[str]]:
        """Extract rows from a table block."""
        # Use tabula-style extraction: look for consistent horizontal separators
        lines = block.get("lines", [])
        if not lines:
            return []

        # Sort lines by y coordinate (row)
        rows_data: dict[float, list[tuple[float, str]]] = {}
        for line in lines:
            y0 = line["bbox"][1]
            row_key = round(y0, 1)
            if row_key not in rows_data:
                rows_data[row_key] = []
            for span in line.get("spans", []):
                x0 = span["bbox"][0]
                text = span["text"].strip()
                if text:
                    rows_data[row_key].append((x0, text))

        # Sort each row by x coordinate and flatten
        table = []
        for y in sorted(rows_data.keys()):
            cells = rows_data[y]
            cells.sort(key=lambda x: x[0])
            row = [c[1] for c in cells]
            table.append(row)

        return table

    def _is_experiment_table(self, table_data: list[list[str]]) -> bool:
        """Heuristic: is this an experiment results table?"""
        if len(table_data) < 2:
            return False

        header = " ".join(table_data[0]).lower()
        body = " ".join(" ".join(r) for r in table_data[1:]).lower()

        score_count = 0
        for kw in _METRIC_KEYWORDS:
            if kw in header or kw in body:
                score_count += 1

        dataset_count = 0
        for kw in _DATASET_KEYWORDS:
            if kw in body:
                dataset_count += 1

        # Look for numeric patterns (results tables have lots of numbers)
        numeric_cells = sum(
            1 for row in table_data[1:]
            for cell in row
            if re.search(r"\d+\.?\d*", cell)
        )

        return (
            score_count >= 1 and numeric_cells >= 4
        ) or (
            dataset_count >= 2 and numeric_cells >= 6
        )

    def extract_all_tables(self, pdf_path: str | Path,
                          max_pages: int = 0) -> list[dict]:
        """Extract all experiment tables from a PDF."""
        try:
            import fitz
        except ImportError:
            return []
        pdf_path = Path(pdf_path)
        doc = fitz.open(str(pdf_path))
        all_tables = []
        pages = range(len(doc)) if max_pages <= 0 else range(min(max_pages, len(doc)))

        for page_num in pages:
            tables = self.detect_tables(page_num, pdf_path)
            all_tables.extend(tables)

        doc.close()
        return all_tables
