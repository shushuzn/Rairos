"""
Paper Parser — extract structured content from arXiv papers for code generation.

Extracts:
- Title, authors, abstract
- Algorithm descriptions and pseudocode
- Key equations/formulas (as text)
- Claims and assumptions stated in the paper
- Experimental settings and hyperparameters
"""

from __future__ import annotations

import re
import arxiv
from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional

from pdf.extract import extract_pdf_text


@dataclass
class PaperContent:
    """Structured content extracted from a paper."""
    arxiv_id: str
    title: str
    authors: list[str]
    abstract: str
    pdf_path: Optional[Path] = None
    full_text: str = ""
    algorithm_descriptions: list[str] = field(default_factory=list)
    equations: list[str] = field(default_factory=list)
    claims: list[str] = field(default_factory=list)
    hyperparameters: dict = field(default_factory=dict)
    datasets: list[str] = field(default_factory=list)
    methods: list[str] = field(default_factory=list)


def download_and_parse(arxiv_id: str, work_dir: Optional[Path] = None) -> PaperContent:
    """Download paper and extract structured content."""
    # Normalize arxiv_id
    arxiv_id = normalize_arxiv_id(arxiv_id)

    # Download via arxiv API
    client = arxiv.Client()
    search = arxiv.Search(id_list=[arxiv_id])
    try:
        paper = next(client.results(search))
    except StopIteration:
        raise ValueError(f"arXiv paper not found: {arxiv_id}")

    # Download PDF
    pdf_path = Path(f"/tmp/{arxiv_id}.pdf")
    paper.download_pdf(dirpath=str(pdf_path.parent), filename=pdf_path.name)

    # Extract text
    full_text = extract_pdf_text(pdf_path, max_pages=None)

    content = PaperContent(
        arxiv_id=arxiv_id,
        title=paper.title or "",
        authors=[a.name for a in (paper.authors or [])],
        abstract=paper.summary or "",
        pdf_path=pdf_path,
        full_text=full_text,
    )

    # Extract structured content
    content.algorithm_descriptions = _extract_algorithms(full_text)
    content.equations = _extract_equations(full_text)
    content.claims = _extract_claims(full_text)
    content.hyperparameters = _extract_hyperparameters(full_text)
    content.datasets = _extract_datasets(full_text)
    content.methods = _extract_methods(full_text)

    return content


def parse_existing_pdf(pdf_path: Path, arxiv_id: str) -> PaperContent:
    """Parse a PDF already on disk."""
    full_text = extract_pdf_text(pdf_path, max_pages=None)

    content = PaperContent(
        arxiv_id=arxiv_id,
        title="",
        authors=[],
        abstract="",
        pdf_path=pdf_path,
        full_text=full_text,
    )

    content.algorithm_descriptions = _extract_algorithms(full_text)
    content.equations = _extract_equations(full_text)
    content.claims = _extract_claims(full_text)
    content.hyperparameters = _extract_hyperparameters(full_text)
    content.datasets = _extract_datasets(full_text)
    content.methods = _extract_methods(full_text)

    return content


def normalize_arxiv_id(raw: str) -> str:
    """Normalize arXiv ID from URL or bare ID."""
    # URL patterns
    m = re.search(r'arxiv\.org/abs/(\d+\.\d+)', raw)
    if m:
        return m.group(1)
    m = re.search(r'arxiv\.org/pdf/(\d+\.\d+)', raw)
    if m:
        return m.group(1)
    # Bare ID
    m = re.search(r'(\d+\.\d+)', raw)
    if m:
        return m.group(1)
    return raw.strip()


# ─── Extraction helpers ───────────────────────────────────────────────────────

def _extract_algorithms(text: str) -> list[str]:
    """Extract algorithm descriptions and pseudocode sections."""
    algorithms = []

    # Look for numbered algorithm blocks (Algorithm 1:, Algorithm 2:, etc.)
    algo_pattern = re.compile(
        r'(?:Algorithm \d+[:].*?)(?:\n(?:[A-Z].*?\n){3,})',
        re.DOTALL | re.IGNORECASE
    )
    for m in algo_pattern.finditer(text):
        snippet = m.group(0).strip()
        if len(snippet) > 100:
            algorithms.append(snippet)

    # Look for pseudocode sections (common LaTeX patterns)
    pseudo_patterns = [
        r'\\begin{algorithm}.*?\\end{algorithm}',
        r'\\begin{algorithmic}.*?\\end{algorithmic}',
        r'\\caption{.*?Algorithm.*?}(?:\n(?:.*?\n)*?)',
    ]
    for pat in pseudo_patterns:
        for m in re.finditer(pat, text, re.DOTALL):
            snippet = m.group(0).strip()
            if len(snippet) > 100:
                algorithms.append(snippet)

    # Input/Output/Parameters blocks
    io_pattern = re.compile(
        r'(?:INPUT|Output|Parameters)[:]\s*\n((?:.*?\n){3,})',
        re.IGNORECASE
    )
    for m in io_pattern.finditer(text):
        snippet = m.group(0).strip()
        if len(snippet) > 80:
            algorithms.append(snippet)

    return algorithms[:10]  # Cap at 10


def _extract_equations(text: str) -> list[str]:
    """Extract key equations as LaTeX strings."""
    equations = []

    # Display equations: \[ ... \] or $$ ... $$
    for m in re.finditer(r'\$\$(.+?)\$\$', text, re.DOTALL):
        eq = m.group(1).strip()
        if len(eq) > 10:
            equations.append(f"$${eq}$$")

    # Inline display equations: \[ ... \]
    for m in re.finditer(r'\\\[(.+?)\\\]', text, re.DOTALL):
        eq = m.group(1).strip()
        if len(eq) > 10:
            equations.append(f"$${eq}$$")

    # Numbered equations: \begin{equation} ... \end{equation}
    for m in re.finditer(r'\\begin\{equation\}(.+?)\\end\{equation\}', text, re.DOTALL):
        eq = m.group(1).strip()
        if len(eq) > 10:
            equations.append(eq)

    return equations[:20]


def _extract_claims(text: str) -> list[str]:
    """Extract key claims and assumptions."""
    claims = []

    claim_markers = [
        r'(?:Theorem|Lemma|Proposition|Corollary|Assumption|Hypothesis)\s+\d*[:\.]\s*(.{50,300}?)(?:\n|$)',
        r'We (?:prove|show|demonstrate|report|observe|find|conclude) that\s+(.{50,200}?)(?:\n|$)',
        r'(?:We assume|The key assumption)[:\.]\s*(.{30,200}?)(?:\n|$)',
    ]

    for pat_str in claim_markers:
        pat = re.compile(pat_str, re.IGNORECASE | re.DOTALL)
        for m in pat.finditer(text):
            snippet = m.group(0).strip()
            if len(snippet) > 30:
                claims.append(snippet)

    return claims[:15]


def _extract_hyperparameters(text: str) -> dict[str, str]:
    """Extract hyperparameters and experimental settings."""
    hp = {}

    # Common patterns: learning rate, batch size, epochs, etc.
    patterns = {
        r'learning rate[:\s]+([0-9e\-\.]+)': 'learning_rate',
        r'batch size[:\s]+(\d+)': 'batch_size',
        r'epochs?[:\s]+(\d+)': 'epochs',
        r'hidden size[:\s]+(\d+)': 'hidden_size',
        r'embedding dimension[:\s]+(\d+)': 'embedding_dim',
        r'attention heads?[:\s]+(\d+)': 'num_heads',
        r'weight decay[:\s]+([0-9e\-\.]+)': 'weight_decay',
        r'dropout[:\s]+([0-9e\-\.]+)': 'dropout',
        r'optimizer[:\s]+([A-Za-z]+)': 'optimizer',
    }

    for pat_str, key in patterns.items():
        m = re.search(pat_str, text, re.IGNORECASE)
        if m and key not in hp:
            hp[key] = m.group(1)

    return hp


def _extract_datasets(text: str) -> list[str]:
    """Extract dataset names mentioned in the paper."""
    datasets = []

    # Common benchmark names
    known = [
        r'(?:MNIST|CIFAR-10|CIFAR-100|ImageNet|WikiText|PTB|GLUE|SuperGLUE|SQuAD)',
        r'(?:CoNLL|SST|SNLI|MPQA|TREC|Reuters)',
        r'(?:MS MARCO|Natural Questions|TriviaQA)',
    ]

    for pat in known:
        for m in re.finditer(pat, text):
            name = m.group(0)
            if name not in datasets:
                datasets.append(name)

    # "datasets such as X, Y, Z"
    for m in re.finditer(r'datasets? (?:such as|included|used)[:\s]+([A-Za-z0-9,\s\-]+?)(?:\.|,)', text, re.IGNORECASE):
        names = m.group(1).split(',')
        for n in names:
            n = n.strip()
            if n and len(n) > 2:
                datasets.append(n)

    return list(dict.fromkeys(datasets))[:10]  # Dedupe, cap at 10


def _extract_methods(text: str) -> list[str]:
    """Extract key method/technique names."""
    methods = []

    # Section-level method names (look at section headings)
    for m in re.finditer(r'(?:^|\n)(?=.{0,30}(?:method|approach|model|technique|architecture))(?:\w+\s+){1,5}(?:method|approach|model|technique|architecture).{20,100}', text, re.IGNORECASE):
        snippet = m.group(0).strip()
        if len(snippet) > 20:
            methods.append(snippet)

    # Key technique names in bold/italic LaTeX
    for m in re.finditer(r'\\textbf\{([A-Z][a-z].{10,80})\}', text):
        methods.append(m.group(1).strip())

    return methods[:10]
