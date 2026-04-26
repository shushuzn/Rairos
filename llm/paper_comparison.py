"""
Paper Comparison: Compare multiple papers side-by-side.
"""
from dataclasses import dataclass, field
from typing import List, Optional, Dict, Any
from difflib import unified_diff


@dataclass
class ComparisonColumn:
    """A column in the comparison table."""
    paper_id: str
    title: str
    year: int = 0
    authors: List[str] = field(default_factory=list)
    methods: List[str] = field(default_factory=list)
    datasets: List[str] = field(default_factory=list)
    metrics: Dict[str, str] = field(default_factory=dict)
    abstract: str = ""


@dataclass
class ComparisonResult:
    """Result of paper comparison."""
    columns: List[ComparisonColumn]
    aspect_rows: List[Dict[str, Any]] = field(default_factory=list)


class PaperComparator:
    """Compare papers side-by-side."""

    def __init__(self, db=None):
        self.db = db

    def add_paper(self, paper: Any) -> ComparisonColumn:
        """Convert a paper object to ComparisonColumn."""
        return ComparisonColumn(
            paper_id=getattr(paper, 'uid', '') or getattr(paper, 'id', ''),
            title=getattr(paper, 'title', 'Unknown'),
            year=int(getattr(paper, 'year', 0) or 0),
            authors=self._parse_authors(paper),
            methods=self._extract_methods(paper),
            datasets=self._extract_datasets(paper),
            metrics=self._extract_metrics(paper),
            abstract=getattr(paper, 'abstract', '') or '',
        )

    def _parse_authors(self, paper: Any) -> List[str]:
        """Parse authors from paper."""
        authors = getattr(paper, 'authors', [])
        if isinstance(authors, str):
            return [a.strip() for a in authors.split(',')[:5]]
        return list(authors)[:5]

    def _extract_methods(self, paper: Any) -> List[str]:
        """Extract methods from paper text."""
        text = (
            getattr(paper, 'title', '') + ' ' +
            getattr(paper, 'abstract', '') + ' ' +
            getattr(paper, 'method', '')
        ).lower()

        keywords = {
            'transformer': 'Transformer',
            'bert': 'BERT',
            'gpt': 'GPT',
            'lstm': 'LSTM',
            'cnn': 'CNN',
            'gan': 'GAN',
            'rl': 'Reinforcement Learning',
            'attention': 'Attention',
            'embedding': 'Embedding',
            'retrieval': 'Retrieval',
            'rag': 'RAG',
            'fine-tun': 'Fine-tuning',
            'rlhf': 'RLHF',
            'chain-of-thought': 'Chain-of-Thought',
            'prompt': 'Prompting',
        }

        found = []
        for kw, name in keywords.items():
            if kw in text and name not in found:
                found.append(name)
        return found[:5]

    def _extract_datasets(self, paper: Any) -> List[str]:
        """Extract datasets from paper text."""
        text = (
            getattr(paper, 'title', '') + ' ' +
            getattr(paper, 'abstract', '') + ' ' +
            getattr(paper, 'dataset', '')
        ).lower()

        datasets = {
            'glue': 'GLUE',
            'super.glue': 'SuperGLUE',
            'squad': 'SQuAD',
            'natural questions': 'NQ',
            'triviaqa': 'TriviaQA',
            'mmlu': 'MMLU',
            'humaneval': 'HumanEval',
            'mbpp': 'MBPP',
            ' AlpacaEval': 'AlpacaEval',
            'coqa': 'CoQA',
            'hotpotqa': 'HotpotQA',
            ' DROP': 'DROP',
            'fever': 'FEVER',
            'mnli': 'MNLI',
            'qnli': 'QNLI',
            'cola': 'CoLA',
            'sst': 'SST',
            'stsb': 'STS-B',
            'qqp': 'QQP',
            'mrpc': 'MRPC',
        }

        found = []
        for kw, name in datasets.items():
            if kw in text and name not in found:
                found.append(name)
        return found[:5]

    def _extract_metrics(self, paper: Any) -> Dict[str, str]:
        """Extract metrics from paper text."""
        text = (
            getattr(paper, 'abstract', '') + ' ' +
            getattr(paper, 'result', '') + ' ' +
            getattr(paper, 'metrics', '')
        ).lower()

        metrics = {
            'accuracy': 'Acc',
            'precision': 'Prec',
            'recall': 'Rec',
            'f1': 'F1',
            'bleu': 'BLEU',
            'rouge': 'ROUGE',
            'perplexity': 'PPL',
            'latency': 'Latency',
            'throughput': 'Throughput',
        }

        found = {}
        for kw, name in metrics.items():
            if kw in text:
                found[name] = '✓'

        # Try to extract specific values
        import re
        patterns = [
            (r'(\d+\.?\d*)\s*%?\s*(accuracy)', r'\1%'),
            (r'(\d+\.?\d*)\s*(bleu)', r'\1'),
            (r'(\d+\.?\d*)\s*(f1)', r'\1'),
        ]

        for pattern, replacement in patterns:
            match = re.search(pattern, text)
            if match:
                key = match.group(2).title()
                val = match.group(1)
                found[key] = val

        return dict(list(found.items())[:5])

    def compare(
        self,
        paper_ids: List[str],
        aspects: Optional[List[str]] = None,
    ) -> ComparisonResult:
        """Compare papers by paper IDs."""
        if aspects is None:
            aspects = ['methods', 'datasets', 'metrics', 'authors']

        columns = []

        for pid in paper_ids:
            if self.db and hasattr(self.db, 'get_paper'):
                paper = self.db.get_paper(pid)
                if paper:
                    columns.append(self.add_paper(paper))
            else:
                columns.append(ComparisonColumn(
                    paper_id=pid,
                    title=pid,
                ))

        # Build aspect rows
        aspect_rows = []
        for aspect in aspects:
            row = {'aspect': aspect.capitalize()}
            for col in columns:
                if aspect == 'methods':
                    row[col.paper_id] = ', '.join(col.methods) or '-'
                elif aspect == 'datasets':
                    row[col.paper_id] = ', '.join(col.datasets) or '-'
                elif aspect == 'metrics':
                    row[col.paper_id] = ', '.join(
                        f"{k}={v}" for k, v in col.metrics.items()
                    ) or '-'
                elif aspect == 'authors':
                    row[col.paper_id] = ', '.join(col.authors[:2]) + ('+' if len(col.authors) > 2 else '') or '-'
                elif aspect == 'year':
                    row[col.paper_id] = str(col.year) if col.year else '-'
                elif aspect == 'abstract':
                    row[col.paper_id] = col.abstract[:100] + '...' if len(col.abstract) > 100 else col.abstract or '-'
            aspect_rows.append(row)

        return ComparisonResult(columns=columns, aspect_rows=aspect_rows)

    def render_text(self, result: ComparisonResult) -> str:
        """Render comparison as ASCII table."""
        if not result.columns:
            return "No papers to compare."

        lines = ["=" * 80, "📊 Paper Comparison", "=" * 80, ""]

        # Header row
        header = ["Aspect"]
        for col in result.columns:
            title = col.title[:25] if len(col.title) > 25 else col.title
            header.append(title)
        lines.append(' | '.join(f"{h:^25}" for h in header))
        lines.append("-" * 80)

        # Data rows
        for row in result.aspect_rows:
            row_str = [f"{row['aspect']:12}"]
            for col in result.columns:
                val = row.get(col.paper_id, '-')
                if len(val) > 25:
                    val = val[:22] + '...'
                row_str.append(f"{val:^25}")
            lines.append(' | '.join(row_str))

        lines.append("-" * 80)
        lines.append("")
        return '\n'.join(lines)

    def render_markdown(self, result: ComparisonResult) -> str:
        """Render comparison as Markdown table."""
        lines = ["# Paper Comparison\n"]

        if not result.columns:
            return '\n'.join(lines) + "\nNo papers to compare."

        # Header
        header = ["| Aspect |"] + [
            f"| {col.title[:40]} |" for col in result.columns
        ]
        lines.append(''.join(header))
        lines.append('|' + '|'.join(['---' for _ in range(len(result.columns) + 1)]) + '|')

        # Rows
        for row in result.aspect_rows:
            cells = [f"| {row['aspect']} |"]
            for col in result.columns:
                val = row.get(col.paper_id, '-')
                cells.append(f" {val} |")
            lines.append(''.join(cells))

        return '\n'.join(lines)

    def render_diff(self, paper_a: Any, paper_b: Any, field: str = 'methods') -> str:
        """Generate diff between two papers on a specific field."""
        a_field = getattr(paper_a, field, []) or []
        b_field = getattr(paper_b, field, []) or []

        if isinstance(a_field, str):
            a_field = [a_field]
        if isinstance(b_field, str):
            b_field = [b_field]

        lines = [
            f"=== Diff: {getattr(paper_a, 'title', 'Paper A')[:30]} vs {getattr(paper_b, 'title', 'Paper B')[:30]} ===",
            f"--- {field} ---",
        ]

        a_str = '\n'.join(sorted(a_field))
        b_str = '\n'.join(sorted(b_field))

        diff = list(unified_diff(
            a_str.splitlines(),
            b_str.splitlines(),
            fromfile='Paper A',
            tofile='Paper B',
            lineterm='',
        ))

        if diff:
            lines.extend(diff)
        else:
            lines.append("(No differences)")

        return '\n'.join(lines)
