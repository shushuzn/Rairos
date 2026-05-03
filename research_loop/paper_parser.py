"""
Paper Parser — Download and parse research papers into structured PaperContent.

Used by the paper2code integration pipeline:
  download_and_parse(arxiv_id) → PaperContent
  parse_existing_pdf(pdf_path, arxiv_id) → PaperContent

PaperContent feeds into:
  - code_generator: generate code skeleton from paper content
  - test_extractor: extract testable assertions from paper content
"""
from __future__ import annotations

import re
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import List, Optional, Dict


@dataclass
class PaperContent:
    """Structured paper content for code generation and testing."""
    arxiv_id: str
    title: str
    authors: List[str] = field(default_factory=list)
    abstract: str = ""
    published: str = ""
    updated: str = ""
    # Algorithm/paper-specific fields
    algorithm_descriptions: List[str] = field(default_factory=list)
    equations: List[str] = field(default_factory=list)
    claims: List[str] = field(default_factory=list)
    hyperparameters: Dict[str, str] = field(default_factory=dict)
    datasets: List[str] = field(default_factory=list)
    methods: List[str] = field(default_factory=list)
    categories: List[str] = field(default_factory=list)


def download_and_parse(arxiv_id: str) -> PaperContent:
    """Download paper metadata from arXiv API and parse into PaperContent.

    Uses parsers.arxiv.fetch_arxiv_metadata for metadata extraction.
    Falls back to minimal metadata if API is unavailable.
    """
    try:
        from parsers.arxiv import fetch_arxiv_metadata

        # Normalize arxiv_id
        aid = arxiv_id.strip()
        if ".org/abs/" in aid:
            aid = aid.split(".org/abs/")[-1]
        aid = re.sub(r'v\d+$', '', aid)  # strip version

        paper = fetch_arxiv_metadata(aid)

        # Extract content from PDF for richer analysis
        pdf_path = None
        if paper.pdf_url:
            try:
                import requests
                resp = requests.get(paper.pdf_url, timeout=30, stream=True)
                resp.raise_for_status()
                pdf_path = Path(f'{aid.replace(".", "_")}.pdf')
                with open(pdf_path, 'wb') as f:
                    for chunk in resp.iter_content(chunk_size=8192):
                        f.write(chunk)
            except Exception:
                pdf_path = None

        content = PaperContent(
            arxiv_id=aid,
            title=paper.title,
            authors=paper.authors,
            abstract=paper.abstract,
            published=paper.published,
            updated=paper.updated,
            categories=paper.categories.split(',') if paper.categories else [],
        )

        # If PDF available, extract richer content
        if pdf_path and pdf_path.exists():
            _enrich_from_pdf(content, pdf_path)
            try:
                pdf_path.unlink()
            except Exception:
                pass

        return content

    except Exception:
        return _minimal_content(arxiv_id)


def parse_existing_pdf(pdf_path: str, arxiv_id: str) -> PaperContent:
    """Parse an already-downloaded PDF into PaperContent."""
    path = Path(pdf_path)
    if not path.exists():
        return _minimal_content(arxiv_id)

    content = _minimal_content(arxiv_id)
    _enrich_from_pdf(content, path)
    return content


def _enrich_from_pdf(content: PaperContent, pdf_path: Path) -> None:
    """Extract algorithm descriptions, equations, claims from PDF text."""
    try:
        import fitz  # PyMuPDF
    except ImportError:
        try:
            import pymupdf as fitz
        except ImportError:
            return

    try:
        doc = fitz.open(pdf_path)
        text = ""
        for page in doc:
            text += page.get_text()

        # Extract sections
        text_lower = text.lower()

        # Algorithm descriptions: look for "algorithm", "method", "approach" sections
        algo_pattern = re.compile(
            r'(?:algorithm|method|approach|procedure|technique)s?[:\s]+([A-Z][^.!?\n]{50,500}?(?:\d+\.|\n){1,3})',
            re.IGNORECASE
        )
        for match in algo_pattern.finditer(text[:10000]):  # First 10k chars
            desc = match.group(1).strip()
            if len(desc) > 30:
                content.algorithm_descriptions.append(desc[:300])

        # Equations: look for display math
        eq_pattern = re.compile(r'\$\$(.+?)\$\$|\$(.+?)\$')
        for match in eq_pattern.finditer(text):
            eq = (match.group(1) or match.group(2) or "").strip()
            if eq and len(eq) > 5:
                content.equations.append(eq[:200])

        # Claims: look for "we show", "prove", "demonstrate", "our results"
        claim_patterns = [
            r'(?:we show|we prove|we demonstrate|our results? show)[^.!?\n]{10,200}',
            r'(?:the (?:model|method|algorithm) achieves?|performance reaches?)[^.!?\n]{10,200}',
        ]
        for pat in claim_patterns:
            for match in re.finditer(pat, text_lower):
                claim = match.group(0).strip()
                if len(claim) > 20:
                    content.claims.append(claim[:300])

        # Hyperparameters: look for "learning rate", "batch size", etc.
        hp_patterns = [
            (r'learning\s*rate[:\s]+[\d.e\-]+', 'learning_rate'),
            (r'batch\s*size[:\s]+\d+', 'batch_size'),
            (r'epochs?[:\s]+\d+', 'epochs'),
            (r'dropout[:\s]+[\d.]+', 'dropout'),
            (r'hidden\s*layer[s]?[:\s]+\d+', 'hidden_size'),
        ]
        for pat, name in hp_patterns:
            for match in re.finditer(pat, text_lower):
                val = match.group(0).split(':')[-1].strip()
                content.hyperparameters[name] = val

        # Datasets: look for common dataset names
        dataset_names = [
            'imagenet', 'cifar-10', 'cifar-100', 'mnist', 'wikitext',
            'glue', 'squad', 'arxiv', 'pubmed', 'openwebtext',
            'pile', 'the pile', 'alpaca', 'dolly', 'hh-rlhf'
        ]
        text_lower_for_ds = text_lower[:20000]
        for ds in dataset_names:
            if ds in text_lower_for_ds:
                content.datasets.append(ds)

    except Exception:
        pass


def _minimal_content(arxiv_id: str) -> PaperContent:
    """Return minimal PaperContent when full parsing fails."""
    aid = re.sub(r'v\d+$', '', arxiv_id.strip())
    return PaperContent(
        arxiv_id=aid,
        title=f"Paper {aid}",
        authors=[],
        abstract="",
    )
