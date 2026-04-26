"""
Citation Chain: Build and visualize citation relationships.
"""
from dataclasses import dataclass, field
from typing import List, Optional, Dict, Set, Tuple
from collections import deque


@dataclass
class CitationNode:
    """A paper in the citation chain."""
    paper_id: str
    title: str
    year: int = 0
    authors: List[str] = field(default_factory=list)
    citations: List[str] = field(default_factory=list)  # Papers this paper cites
    cited_by: List[str] = field(default_factory=list)   # Papers citing this


@dataclass
class CitationChain:
    """A chain of citations."""
    nodes: List[CitationNode] = field(default_factory=list)
    edges: List[Tuple[str, str]] = field(default_factory=list)  # (from, to)


class CitationChainBuilder:
    """Build citation chains from papers."""

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
    ) -> CitationNode:
        """Add a paper to the chain."""
        if paper_id not in self.nodes:
            self.nodes[paper_id] = CitationNode(
                paper_id=paper_id,
                title=title,
                year=year,
                authors=authors or [],
                citations=references or [],
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
                    authors=[],  # Would need to parse
                    references=ref_ids,
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

    def render_text(self, chain: CitationChain, max_nodes: int = 20) -> str:
        """Render chain as ASCII tree."""
        if not chain.nodes:
            return "No citation chain."

        lines = ["=" * 60, "📚 Citation Chain", "=" * 60, ""]

        # Sort by year
        sorted_nodes = sorted(chain.nodes, key=lambda x: -x.year if x.year else 0)

        for i, node in enumerate(sorted_nodes[:max_nodes]):
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
