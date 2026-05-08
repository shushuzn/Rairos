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
from typing import List, Optional, Dict, TYPE_CHECKING

if TYPE_CHECKING:
    from research_loop.provenance import EquationSource, ClaimSource, AlgorithmSource


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
    # Cross-paper dedup: structural fingerprint of the algorithm
    algorithm_fingerprint: str = ""  # computed from equations + methods + hps
    # Provenance: source locations for extracted content (populated by _enrich_from_pdf)
    equation_sources: List["EquationSource"] = field(default_factory=list)
    claim_sources: List["ClaimSource"] = field(default_factory=list)
    algorithm_sources: List["AlgorithmSource"] = field(default_factory=list)


def compute_algorithm_fingerprint(content: "PaperContent") -> str:
    """Compute a structural fingerprint of an algorithm from paper content.

    Two papers implementing the same algorithm (e.g., "Attention is All You Need"
    variants) should produce the same fingerprint even with different notation.
    This enables cross-paper dedup: same algorithm → same fingerprint.

    Fingerprint is derived from:
    1. Equation structure (variables, operations, layout) — stripped of notation variants
    2. Method names (e.g., "self-attention", "feed-forward")
    3. Hyperparameter names (not values) — structural signature
    """
    import hashlib

    signals: list[str] = []

    # 1. Equations: extract structural skeleton (op signature only, no vars)
    for eq in content.equations:
        eq_lower = eq.lower()
        # Keep only operation keywords — strip all variable names and notation
        ops = re.findall(
            r"(?:softmax|attention|matmul|@|linear|layer.?norm|residual|dropout|encoder|decoder|self.?attention|cross.?attention|multi.?head|positional|embedding|relu|gelu|swiglu|feed.?forward|normalization|convolution|pooling|gru|lstm|rnn|transformer|cross.?entropy| BCE|CE|adam|sgd|rmsprop|weight)",
            eq_lower,
        )
        if ops:
            signals.append("eq:" + "|".join(sorted(set(ops))))

    # 2. Method names — canonical form with synonym collapsing
    for method in content.methods:
        # Normalize: lowercase, collapse non-alpha runs
        m = re.sub(r"[-_]?[0-9]+$", "", method.lower())
        m = re.sub(r"[^a-z]+", "", m)
        # Collapse method-family synonyms to canonical names
        for synonym_group in [
            [
                "feedforward",
                "feedforwardnetwork",
                "feedforwardlayer",
                "feedforwardblock",
                "feedforwardsublayer",
            ],
            ["selfattention", "selfattention"],
            ["multiheadattention", "multihead"],
            ["residual", "residualconnection", "skipconnection"],
            ["encoder", "encoderlayer", "encoderblock"],
            ["decoder", "decoderlayer", "decoderblock"],
            ["attention", "selfattention", "crossattention", "multiheadattention"],
            ["layer_norm", "layernorm", "ln"],
            ["convolution", "conv", "convlayer"],
        ]:
            if m in synonym_group:
                m = synonym_group[0]
                break
        if m:
            signals.append(f"method:{m}")

    # 3. Hyperparameter names (structural, not values)
    hp_names = sorted(content.hyperparameters.keys())
    if hp_names:
        signals.append("hpn:" + "|".join(hp_names))

    # 4. Datasets intentionally excluded — same algorithm can be evaluated on
    # different benchmarks (WMT, Wikitext, etc.). Dataset differences should NOT
    # make two implementations of the same algorithm look different.

    combined = ";".join(sorted(signals))
    return hashlib.sha256(combined.encode()).hexdigest()[:16]


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
        aid = re.sub(r"v\d+$", "", aid)  # strip version

        paper = fetch_arxiv_metadata(aid)

        # Extract content from PDF for richer analysis
        pdf_path = None
        if paper.pdf_url:
            try:
                import requests

                resp = requests.get(paper.pdf_url, timeout=30, stream=True)
                resp.raise_for_status()
                pdf_path = Path(f"{aid.replace('.', '_')}.pdf")
                with open(pdf_path, "wb") as f:
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
            categories=paper.categories.split(",") if paper.categories else [],
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
    """Extract algorithm descriptions, equations, claims from PDF text with provenance."""
    # Local imports to avoid circular reference
    from research_loop.provenance import (
        PaperLocation,
        EquationSource,
        ClaimSource,
        AlgorithmSource,
    )

    def _match_to_location(char_start: int, page_offsets: list[int]) -> PaperLocation:
        """Map a character offset to a page number via binary search on page_offsets."""
        page_idx = 0
        for i, offset in enumerate(page_offsets):
            if char_start >= offset:
                page_idx = i
            else:
                break
        return PaperLocation(
            section="unknown",
            page=page_idx + 1,
            char_start=char_start,
            char_end=char_start,
        )

    try:
        import fitz  # PyMuPDF
    except ImportError:
        try:
            import pymupdf as fitz
        except ImportError:
            return

    try:
        doc = fitz.open(pdf_path)
        pages_text: list[str] = []
        page_offsets: list[int] = []

        for page in doc:
            page_offsets.append(sum(len(t) for t in pages_text))
            pages_text.append(page.get_text())

        full_text = "".join(pages_text)
        text_lower = full_text.lower()

        # Algorithm descriptions: look for "algorithm", "method", "approach" sections
        algo_pattern = re.compile(
            r"(?:algorithm|method|approach|procedure|technique)s?[:\s]+([A-Z][^.!?\n]{50,500}?(?:\d+\.|\n){1,3})",
            re.IGNORECASE,
        )
        for match in algo_pattern.finditer(full_text[:10000]):  # First 10k chars
            desc = match.group(1).strip()
            if len(desc) > 30:
                idx = len(content.algorithm_descriptions)
                loc = _match_to_location(match.start(), page_offsets)
                content.algorithm_descriptions.append(desc[:300])
                content.algorithm_sources.append(
                    AlgorithmSource(index=idx, description=desc[:300], location=loc)
                )

        # Equations: look for display math
        eq_pattern = re.compile(r"\$\$(.+?)\$\$|\$(.+?)\$")
        for match in eq_pattern.finditer(full_text):
            eq = (match.group(1) or match.group(2) or "").strip()
            if eq and len(eq) > 5:
                idx = len(content.equations)
                loc = _match_to_location(match.start(), page_offsets)
                content.equations.append(eq[:200])
                content.equation_sources.append(
                    EquationSource(index=idx, equation=eq[:200], location=loc)
                )

        # Claims: look for "we show", "prove", "demonstrate", "our results"
        claim_patterns = [
            r"(?:we show|we prove|we demonstrate|our results? show)[^.!?\n]{10,200}",
            r"(?:the (?:model|method|algorithm) achieves?|performance reaches?)[^.!?\n]{10,200}",
        ]
        for pat in claim_patterns:
            for match in re.finditer(pat, text_lower):
                claim = match.group(0).strip()
                if len(claim) > 20:
                    idx = len(content.claims)
                    # map back to original offset (text_lower same length as full_text)
                    loc = _match_to_location(match.start(), page_offsets)
                    content.claims.append(claim[:300])
                    content.claim_sources.append(
                        ClaimSource(index=idx, claim=claim[:300], location=loc)
                    )

        # Hyperparameters: look for "learning rate", "batch size", etc.
        hp_patterns = [
            (r"learning\s*rate[:\s]+[\d.e\-]+", "learning_rate"),
            (r"batch\s*size[:\s]+\d+", "batch_size"),
            (r"epochs?[:\s]+\d+", "epochs"),
            (r"dropout[:\s]+[\d.]+", "dropout"),
            (r"hidden\s*layer[s]?[:\s]+\d+", "hidden_size"),
        ]
        for pat, name in hp_patterns:
            for match in re.finditer(pat, text_lower):
                val = match.group(0).split(":")[-1].strip()
                content.hyperparameters[name] = val

        # Datasets: look for common dataset names
        dataset_names = [
            "imagenet",
            "cifar-10",
            "cifar-100",
            "mnist",
            "wikitext",
            "glue",
            "squad",
            "arxiv",
            "pubmed",
            "openwebtext",
            "pile",
            "the pile",
            "alpaca",
            "dolly",
            "hh-rlhf",
        ]
        text_lower_for_ds = text_lower[:20000]
        for ds in dataset_names:
            if ds in text_lower_for_ds:
                content.datasets.append(ds)

    except Exception as e:
        import logging

        logging.getLogger("paper_parser").warning("Dataset detection failed: %s", e)


def _minimal_content(arxiv_id: str) -> PaperContent:
    """Return minimal PaperContent when full parsing fails."""
    aid = re.sub(r"v\d+$", "", arxiv_id.strip())
    return PaperContent(
        arxiv_id=aid,
        title=f"Paper {aid}",
        authors=[],
        abstract="",
    )
