"""
Citation Chain: Build and visualize citation relationships.
Research family clustering and silent citation detection.
"""
from dataclasses import dataclass, field, asdict
from typing import List, Optional, Dict, Set, Tuple, Any, cast
from collections import deque
import json
import re
import uuid


@dataclass
class CitationNode:
    """A paper in the citation chain."""
    paper_id: str
    title: str
    year: int = 0
    authors: List[str] = field(default_factory=list)
    abstract: str = ""
    citations: List[str] = field(default_factory=list)  # Papers this paper cites
    cited_by: List[str] = field(default_factory=list)   # Papers citing this
    citation_count: int = 0


@dataclass
class CitationChain:
    """A chain of citations."""
    nodes: List[CitationNode] = field(default_factory=list)
    edges: List[Tuple[str, str]] = field(default_factory=list)  # (from, to)


@dataclass
class ResearchFamily:
    """A cluster of papers sharing a common ancestor or theme."""
    family_id: str
    ancestor_id: str
    ancestor_title: str
    papers: List[Dict[str, Any]] = field(default_factory=list)
    common_theme: str = ""
    size: int = 0

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


class CitationChainBuilder:
    """Build citation chains from papers with family clustering and silent citation detection."""

    def __init__(self, db=None):
        self.db = db
        self.nodes: Dict[str, CitationNode] = {}

    def add_paper(
        self,
        paper_id: str,
        title: str,
        year: int = 0,
        authors: Optional[List[str]] = None,
        references: Optional[List[str]] = None,
        abstract: str = "",
        citation_count: int = 0,
    ) -> CitationNode:
        """Add a paper to the chain."""
        if paper_id not in self.nodes:
            self.nodes[paper_id] = CitationNode(
                paper_id=paper_id,
                title=title,
                year=year,
                authors=authors or [],
                citations=references or [],
                abstract=abstract,
                citation_count=citation_count,
            )
        return self.nodes[paper_id]

    def link_citations(self, from_id: str, to_id: str):
        """Link two papers with a citation relationship."""
        if from_id in self.nodes and to_id in self.nodes:
            if to_id not in self.nodes[from_id].citations:
                self.nodes[from_id].citations.append(to_id)
            if from_id not in self.nodes[to_id].cited_by:
                self.nodes[to_id].cited_by.append(from_id)

    def build_from_db(self, paper_id: str, depth: int = 2) -> CitationChain:
        """Build chain from database."""
        if not self.db:
            return CitationChain()

        self.nodes.clear()
        visited: Set[str] = set()
        queue = deque([(paper_id, 0)])  # (paper_id, depth)

        while queue:
            pid, d = queue.popleft()
            if pid in visited or d > depth:
                continue
            visited.add(pid)

            # Fetch paper
            paper = self.db.get_paper(pid) if hasattr(self.db, 'get_paper') else None
            if paper:
                refs = getattr(paper, 'references', []) or []
                ref_ids = [r if isinstance(r, str) else getattr(r, 'id', '') for r in refs]
                self.add_paper(
                    paper_id=pid,
                    title=getattr(paper, 'title', pid),
                    year=getattr(paper, 'year', 0) or 0,
                    authors=[],
                    references=ref_ids,
                    abstract=getattr(paper, 'abstract', ''),
                    citation_count=getattr(paper, 'citation_count', 0) or 0,
                )

                # Queue references
                if d < depth:
                    for ref_id in ref_ids:
                        if ref_id and ref_id not in visited:
                            queue.append((ref_id, d + 1))

        # Build edges
        edges = []
        for node in self.nodes.values():
            for cited in node.citations:
                if cited in self.nodes:
                    edges.append((node.paper_id, cited))

        return CitationChain(nodes=list(self.nodes.values()), edges=edges)

    def build_chain(
        self,
        seed_arxiv_id: str,
        max_depth: int = 2,
    ) -> CitationChain:
        """Build chain using Semantic Scholar API for citation data."""
        from parsers.semantic_scholar import get_references, get_citations

        self.nodes.clear()
        visited: Set[str] = set()
        queue = deque([(seed_arxiv_id, 0, "seed")])  # (paper_id, depth, source)

        while queue:
            pid, d, source = queue.popleft()
            if pid in visited or d > max_depth:
                continue
            visited.add(pid)

            try:
                # Get paper metadata from local DB first
                paper_info = self._get_paper_info(pid)

                if paper_info:
                    self.add_paper(
                        paper_id=pid,
                        title=paper_info.get("title", pid),
                        year=int(paper_info.get("published", "0")[:4]) if paper_info.get("published") else 0,
                        authors=paper_info.get("authors", []),
                        abstract=paper_info.get("abstract", ""),
                        citation_count=paper_info.get("citation_count", 0) or 0,
                    )
                else:
                    self.add_paper(paper_id=pid, title=pid)

                # Fetch references (backward)
                if d < max_depth:
                    refs = get_references(pid, limit=8) or []
                    for ref in refs:
                        ref_id = ref.get("arxivId", "")
                        if ref_id and ref_id not in visited:
                            self.add_paper(
                                paper_id=ref_id,
                                title=ref.get("title", "Unknown"),
                                year=ref.get("year", 0) or 0,
                                authors=[a.get("name", "") for a in ref.get("authors", [])],
                                citation_count=ref.get("citationCount", 0) or 0,
                            )
                            self.link_citations(pid, ref_id)
                            queue.append((ref_id, d + 1, "backward"))

                # Fetch citations (forward)
                if d < max_depth:
                    cites = get_citations(pid, limit=8) or []
                    for cite in cites:
                        cite_id = cite.get("arxivId", "")
                        if cite_id and cite_id not in visited:
                            self.add_paper(
                                paper_id=cite_id,
                                title=cite.get("title", "Unknown"),
                                year=cite.get("year", 0) or 0,
                                authors=[a.get("name", "") for a in cite.get("authors", [])],
                                citation_count=cite.get("citationCount", 0) or 0,
                            )
                            self.link_citations(cite_id, pid)
                            queue.append((cite_id, d + 1, "forward"))

            except Exception:
                continue

        # Build edges
        edges = []
        for node in self.nodes.values():
            for cited in node.citations:
                if cited in self.nodes:
                    edges.append((node.paper_id, cited))

        return CitationChain(nodes=list(self.nodes.values()), edges=edges)

    def _get_paper_info(self, arxiv_id: str) -> Optional[Dict[str, Any]]:
        """Get paper from local DB."""
        if not self.db:
            try:
                from db.database import Database
                db = Database()
                db.init()
                self.db = db
            except Exception:
                return None

        try:
            rows, _ = self.db.search_papers(arxiv_id, limit=1)
            if not rows:
                return None
            r = rows[0]
            return {
                "arxiv_id": getattr(r, "paper_id", "") or getattr(r, "arxiv_id", ""),
                "title": getattr(r, "title", ""),
                "abstract": getattr(r, "abstract", "") or "",
                "authors": getattr(r, "authors", []) or [],
                "published": getattr(r, "published", "") or "",
                "citation_count": getattr(r, "citation_count", 0) or 0,
            }
        except Exception:
            return None

    def find_path(self, from_id: str, to_id: str) -> Optional[List[str]]:
        """Find shortest path between two papers."""
        if from_id not in self.nodes or to_id not in self.nodes:
            return None
        if from_id == to_id:
            return [from_id]

        visited = {from_id}
        queue = deque([[from_id]])

        while queue:
            path = queue.popleft()
            current = path[-1]

            for neighbor in self.nodes[current].citations:
                if neighbor == to_id:
                    return path + [neighbor]
                if neighbor not in visited:
                    visited.add(neighbor)
                    queue.append(path + [neighbor])

            for neighbor in self.nodes[current].cited_by:
                if neighbor == to_id:
                    return path + [neighbor]
                if neighbor not in visited:
                    visited.add(neighbor)
                    queue.append(path + [neighbor])

        return None

    def find_influencers(self, paper_id: str, depth: int = 2) -> List[CitationNode]:
        """Find papers that influenced this paper (ancestors)."""
        if paper_id not in self.nodes:
            return []

        visited = {paper_id}
        queue = deque([(paper_id, 0)])
        ancestors = []

        while queue:
            pid, d = queue.popleft()
            if d > depth:
                continue

            for ancestor_id in self.nodes[pid].cited_by:
                if ancestor_id not in visited:
                    visited.add(ancestor_id)
                    if ancestor_id in self.nodes:
                        ancestors.append(self.nodes[ancestor_id])
                    queue.append((ancestor_id, d + 1))

        return ancestors

    def find_impact(self, paper_id: str, depth: int = 2) -> List[CitationNode]:
        """Find papers influenced by this paper (descendants)."""
        if paper_id not in self.nodes:
            return []

        visited = {paper_id}
        queue = deque([(paper_id, 0)])
        descendants = []

        while queue:
            pid, d = queue.popleft()
            if d > depth:
                continue

            for descendant_id in self.nodes[pid].citations:
                if descendant_id not in visited:
                    visited.add(descendant_id)
                    if descendant_id in self.nodes:
                        descendants.append(self.nodes[descendant_id])
                    queue.append((descendant_id, d + 1))

        return descendants

    def find_similar_papers(
        self,
        paper_id: str,
        limit: int = 10,
        threshold: float = 0.85,
    ) -> List[Tuple[Any, float]]:
        """Find semantically similar papers using embeddings."""
        if not self.db:
            return []

        try:
            return cast(List[Tuple[Any, float]], self.db.find_similar(paper_id, threshold=threshold, limit=limit))
        except Exception:
            return []

    def suggest_related_work(
        self,
        paper_id: str,
        limit: int = 5,
    ) -> List[Dict]:
        """Suggest related work not in current chain."""
        similar = self.find_similar_papers(paper_id, limit=limit + len(self.nodes))

        # Filter out papers already in chain
        existing_ids = set(self.nodes.keys())
        suggestions = []
        for paper, score in similar:
            if paper.id not in existing_ids:
                suggestions.append({
                    "paper_id": paper.id,
                    "title": getattr(paper, 'title', paper.id),
                    "similarity": score,
                    "reason": "semantic similarity",
                })
            if len(suggestions) >= limit:
                break

        return suggestions

    # ── Research Family Clustering ──────────────────────────────────────

    def cluster_families(self) -> List[ResearchFamily]:
        """Cluster papers in current chain into research families by shared citations."""
        families: List[ResearchFamily] = []

        # Group by year
        by_year: Dict[int, List[CitationNode]] = {}
        for node in self.nodes.values():
            if node.year:
                by_year.setdefault(node.year, []).append(node)

        # Papers citing the same references = same family
        for node in self.nodes.values():
            if not node.citations:
                continue

            # Find other papers that cite overlapping references
            for other in self.nodes.values():
                if other.paper_id == node.paper_id:
                    continue
                if not other.citations:
                    continue

                shared = set(node.citations) & set(other.citations)
                if len(shared) >= 2:  # At least 2 shared references
                    family_id = str(uuid.uuid4())[:6]
                    families.append(ResearchFamily(
                        family_id=family_id,
                        ancestor_id=node.paper_id,
                        ancestor_title=f"Family sharing: {', '.join(list(shared)[:3])}",
                        papers=[
                            {"paper_id": node.paper_id, "title": node.title, "year": node.year},
                            {"paper_id": other.paper_id, "title": other.title, "year": other.year},
                        ],
                        common_theme=f"Shared references: {', '.join(list(shared)[:3])}",
                        size=2,
                    ))

        # Deduplicate by family_id
        seen = set()
        unique = []
        for f in families:
            if f.family_id not in seen:
                seen.add(f.family_id)
                unique.append(f)

        return unique[:10]  # Limit to top 10 families

    # ── Silent Citation Detection ──────────────────────────────────────

    METHOD_TERMS: Set[str] = {
        "transformer", "attention", "neural", "network", "embedding", "latent",
        "fine-tuning", "pretraining", "gradient", "loss", "optimization",
        "encoder", "decoder", "architecture", "layer", "token",
        "rag", "retrieval", "knowledge", "distillation", "quantization",
        "chain-of-thought", "prompting", "few-shot", "zero-shot", "in-context",
        "reinforcement", "reward", "policy", "rlhf", "dpo",
        "graph", "neural network", "convolutional", "recurrent",
        "generative", "diffusion", "gan", "vae", "autoencoder",
    }

    def _extract_terms(self, text: str) -> Set[str]:
        """Extract significant terms from text."""
        words = re.findall(r'\b[a-z][a-z0-9-]{3,}\b', text.lower())
        return set(words) & self.METHOD_TERMS

    def detect_silent_citations(self) -> List[Dict[str, Any]]:
        """Detect potential silent citations — papers using similar methods without explicit citation."""
        silent: List[Dict[str, Any]] = []
        nodes = list(self.nodes.values())

        for i, node in enumerate(nodes):
            if not node.abstract:
                continue

            node_terms = self._extract_terms(node.abstract)

            for j, other in enumerate(nodes):
                if i >= j or not other.abstract:
                    continue

                # Skip if already citing each other
                if other.paper_id in node.citations or node.paper_id in other.citations:
                    continue

                other_terms = self._extract_terms(other.abstract)
                shared = node_terms & other_terms

                # High method overlap but no citation
                if len(shared) >= 4:
                    # Determine which is newer (likely the copier)
                    newer = other if (other.year or 0) > (node.year or 0) else node
                    older = node if newer == other else other

                    silent.append({
                        "newer_arxiv_id": newer.paper_id,
                        "newer_title": newer.title,
                        "newer_year": newer.year,
                        "older_arxiv_id": older.paper_id,
                        "older_title": older.title,
                        "older_year": older.year,
                        "shared_methods": list(shared),
                        "confidence": min(len(shared) / 10.0, 0.95),
                        "note": f"{newer.paper_id[:8]} uses similar methods to {older.paper_id[:8]} but may not cite it",
                    })

        # Sort by confidence
        silent.sort(key=lambda x: x["confidence"], reverse=True)
        return silent[:10]

    # ── Rendering ────────────────────────────────────────────────────

    def render_text(self, chain: CitationChain, max_nodes: int = 20) -> str:
        """Render chain as ASCII tree."""
        if not chain.nodes:
            return "No citation chain."

        lines = ["=" * 60, "📚 Citation Chain", "=" * 60, ""]

        # Sort by year
        sorted_nodes = sorted(chain.nodes, key=lambda x: -x.year if x.year else 0)

        for _i, node in enumerate(sorted_nodes[:max_nodes]):
            lines.append(f"[{node.paper_id[:8]}] {node.title[:50]}")
            lines.append(f"  Year: {node.year or '?'} | Cites: {len(node.citations)} | Cited by: {len(node.cited_by)}")
            lines.append("")

        if len(chain.nodes) > max_nodes:
            lines.append(f"... and {len(chain.nodes) - max_nodes} more papers")

        lines.append("")
        lines.append(f"Total: {len(chain.nodes)} papers, {len(chain.edges)} connections")
        lines.append("=" * 60)

        return '\n'.join(lines)

    def render_graphviz(self, chain: CitationChain) -> str:
        """Render chain as Graphviz DOT format."""
        lines = ["digraph citations {", "  rankdir=LR;", "  node [shape=box];"]

        for node in chain.nodes:
            label = f'{node.title[:30]}...\\n({node.year})' if node.year else node.title[:30]
            lines.append(f'  "{node.paper_id}" [label="{label}"];')

        for from_id, to_id in chain.edges:
            lines.append(f'  "{from_id}" -> "{to_id}";')

        lines.append("}")
        return '\n'.join(lines)

    def render_mermaid(self, chain: CitationChain) -> str:
        """Render chain as Mermaid flowchart."""
        lines = ["```mermaid", "flowchart LR"]

        for node in chain.nodes:
            year_str = f"({node.year})" if node.year else ""
            lines.append(f'    {node.paper_id[:8]}[{node.title[:30]}{year_str}]')

        for from_id, to_id in chain.edges:
            lines.append(f"    {from_id[:8]} --> {to_id[:8]}")

        lines.append("```")
        return '\n'.join(lines)

    def render_families(self, families: List[ResearchFamily]) -> str:
        """Render research families as readable text."""
        if not families:
            return "No research families detected."

        lines = ["=" * 60, "🔬 Research Families", "=" * 60, ""]

        for i, fam in enumerate(families, 1):
            lines.append(f"[{i}] Family: {fam.common_theme}")
            lines.append(f"  Size: {fam.size} papers")
            for p in fam.papers[:5]:
                lines.append(f"  - [{p['paper_id'][:8]}] {p['title'][:50]} ({p.get('year', '?')})")
            lines.append("")

        return '\n'.join(lines)

    def render_silent_citations(self, silent: List[Dict[str, Any]]) -> str:
        """Render detected silent citations as readable text."""
        if not silent:
            return "No silent citations detected."

        lines = ["=" * 60, "⚠️ Silent Citations (suspected)", "=" * 60, ""]

        for s in silent:
            conf = f"{s['confidence']:.0%}"
            lines.append(f"[{conf} confidence] {s['newer_arxiv_id'][:8]}")
            lines.append(f"  NEWER: {s['newer_title'][:60]} ({s['newer_year']})")
            lines.append(f"  OLDER: {s['older_title'][:60]} ({s['older_year']})")
            lines.append(f"  SHARED: {', '.join(s['shared_methods'][:5])}")
            lines.append("")

        return '\n'.join(lines)
