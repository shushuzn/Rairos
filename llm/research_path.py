"""
Research Path Planner: Generate optimal reading order from knowledge graph.

基于引用图和知识图谱，自动生成最优论文阅读顺序。

使用 PageRank + 拓扑排序 + 意图检测来推荐个性化阅读路径。
"""

from __future__ import annotations

import re
from collections import defaultdict
from dataclasses import dataclass, field
from enum import Enum
from typing import Optional, List, Dict, Set, Tuple

import numpy as np


class ReadingLevel(Enum):
    """Reading level / depth preference."""
    INTRO = "intro"      # 入门：基础概念
    INTERMEDIATE = "intermediate"  # 进阶：核心论文
    ADVANCED = "advanced"  # 深入：最新进展


@dataclass
class PaperNode:
    """Represents a paper in the reading path."""
    paper_id: str
    title: str
    year: int = 0
    authors: List[str] = field(default_factory=list)
    cited_by: List[str] = field(default_factory=list)  # Papers citing this
    cites: List[str] = field(default_factory=list)  # Papers this cites
    relevance_score: float = 0.0
    pagerank: float = 0.0
    is_foundational: bool = False  # Groundbreaking paper
    is_milestone: bool = False  # Important intermediate paper


@dataclass
class ReadingStep:
    """A single step in the reading path."""
    order: int
    paper: PaperNode
    role: str  # "foundation", "core", "improvement", "variant", "latest"
    reason: str  # Why this paper is recommended here
    estimated_read_time_minutes: int = 15


@dataclass
class ReadingPath:
    """A complete reading path recommendation."""
    topic: str
    level: ReadingLevel
    total_papers: int
    total_reading_time_minutes: int
    steps: List[ReadingStep]
    alternative_paths: List[List[ReadingStep]] = field(default_factory=list)
    skipped_papers: List[str] = field(default_factory=list)  # Papers that exist but not included


class ResearchPathPlanner:
    """Generate optimal research reading paths from citation graph."""

    def __init__(self, kg_manager=None, db=None):
        self.kg = kg_manager
        self.db = db

    def plan_path(
        self,
        topic: str,
        level: ReadingLevel = ReadingLevel.INTERMEDIATE,
        max_papers: int = 8,
        min_year: Optional[int] = None,
        max_year: Optional[int] = None,
    ) -> ReadingPath:
        """
        Generate an optimal reading path for a topic.

        Args:
            topic: Research topic/keyword
            level: Reading level (intro/intermediate/advanced)
            max_papers: Maximum papers to recommend
            min_year: Filter by minimum year
            max_year: Filter by maximum year

        Returns:
            ReadingPath with ordered reading steps
        """
        # 1. Find papers related to topic
        papers = self._find_topic_papers(topic, min_year, max_year)
        if not papers:
            return self._empty_path(topic, level)

        # 2. Build citation graph
        graph = self._build_citation_graph(papers)

        # 3. Calculate PageRank to find important papers
        self._calculate_pagerank(graph)

        # 4. Identify foundational and milestone papers
        self._identify_key_papers(graph, level)

        # 5. Topological sort for optimal reading order
        ordered_papers = self._topological_sort(graph, level)

        # 6. Generate reading steps
        steps = self._generate_steps(ordered_papers, topic, level)

        # Limit to max_papers
        if len(steps) > max_papers:
            skipped = steps[max_papers:]
            steps = steps[:max_papers]
        else:
            skipped = []

        total_time = sum(s.estimated_read_time_minutes for s in steps)

        return ReadingPath(
            topic=topic,
            level=level,
            total_papers=len(steps),
            total_reading_time_minutes=total_time,
            steps=steps,
            skipped_papers=[s.paper.title for s in skipped],
        )

    def _find_topic_papers(
        self,
        topic: str,
        min_year: Optional[int] = None,
        max_year: Optional[int] = None,
    ) -> List[PaperNode]:
        """Find papers related to topic from KG and DB."""
        papers = {}

        # Search in KG by tag
        if self.kg:
            kg_papers = self._search_kg_by_tag(topic)
            for p in kg_papers:
                papers[p.paper_id] = p

        # Search in DB by text
        if self.db:
            db_papers = self._search_db_by_text(topic, min_year, max_year)
            for p in db_papers:
                papers[p.paper_id] = p

        return list(papers.values())

    def _search_kg_by_tag(self, topic: str) -> List[PaperNode]:
        """Search papers in KG by tag."""
        if not self.kg:
            return []

        results = []
        try:
            nodes = self.kg.find_papers_by_tag(topic.lower())
            for node in nodes:
                paper = PaperNode(
                    paper_id=node["entity_id"],
                    title=node["label"],
                )
                # Get citation edges
                edges = self.kg.get_edges_by_node(node["id"], direction="both")
                for edge in edges:
                    if edge["relation_type"] == "cite":
                        if edge["source_id"] == node["id"]:
                            paper.cites.append(edge["target_id"])
                        else:
                            paper.cited_by.append(edge["source_id"])
                results.append(paper)
        except Exception:
            # Knowledge graph build failed — return partial results without crashing.
            pass

        return results

    def _search_db_by_text(
        self,
        topic: str,
        min_year: Optional[int] = None,
        max_year: Optional[int] = None,
    ) -> List[PaperNode]:
        """Search papers in DB by title/abstract."""
        if not self.db:
            return []

        results = []
        try:
            # Use FTS search
            rows, _ = self.db.search_papers(topic, limit=50)
            for row in rows:
                year = getattr(row, 'year', 0) or 0
                if min_year and year < min_year:
                    continue
                if max_year and year > max_year:
                    continue

                paper = PaperNode(
                    paper_id=str(getattr(row, 'id', '')),
                    title=getattr(row, 'title', topic) or topic,
                    year=year,
                    authors=self._parse_authors(getattr(row, 'authors', '')),
                    relevance_score=getattr(row, 'bm25_score', 0) or 0.5,
                )
                results.append(paper)
        except Exception:
            # DB text search failed — return partial results without crashing.
            pass

        return results

    def _parse_authors(self, authors_str: str) -> List[str]:
        """Parse authors string to list."""
        if not authors_str:
            return []
        # Handle common formats
        authors_str = authors_str.replace(' and ', ', ')
        return [a.strip() for a in authors_str.split(',') if a.strip()]

    def _build_citation_graph(self, papers: List[PaperNode]) -> Dict[str, PaperNode]:
        """Build citation graph from papers."""
        graph = {p.paper_id: p for p in papers}

        # If we have citation data in KG, enrich it
        if self.kg:
            self._enrich_from_kg(graph)

        return graph

    def _enrich_from_kg(self, graph: Dict[str, PaperNode]):
        """Enrich graph with citation data from KG."""
        try:
            # Get all Paper nodes
            paper_nodes = self.kg.get_all_nodes(node_type="Paper")
            paper_id_map = {
                n["entity_id"]: n["id"]
                for n in paper_nodes
                if n["entity_id"] in graph
            }

            for paper_id, kg_id in paper_id_map.items():
                edges = self.kg.get_edges_by_node(kg_id, direction="both", rel_type="cite")
                for edge in edges:
                    if edge["source_id"] == kg_id:
                        # This paper cites another
                        target = self._get_entity_id_from_kg_id(edge["target_id"])
                        if target and target in graph:
                            graph[paper_id].cites.append(target)
                    else:
                        # This paper is cited by another
                        source = self._get_entity_id_from_kg_id(edge["source_id"])
                        if source and source in graph:
                            graph[paper_id].cited_by.append(source)
        except Exception:
            # Knowledge graph citation mapping failed — continue without crashing.
            pass

    def _get_entity_id_from_kg_id(self, kg_id: str) -> Optional[str]:
        """Convert KG node ID to entity ID."""
        if not self.kg:
            return None
        node = self.kg.get_node(kg_id)
        return node["entity_id"] if node else None

    def _calculate_pagerank(self, graph: Dict[str, PaperNode], damping: float = 0.85, iterations: int = 30):
        """
        Calculate PageRank for papers.

        In citation graph:
        - Outgoing edge (A cites B) means B influences A
        - Higher PageRank = more influential paper
        """
        if not graph:
            return

        n = len(graph)
        if n == 0:
            return

        paper_ids = list(graph.keys())
        idx_map = {pid: i for i, pid in enumerate(paper_ids)}

        # Build adjacency matrix (citation relationships)
        # If A cites B, then there's an edge from A to B
        # For PageRank, we want edges in reverse direction (influenced by)
        adj = np.zeros((n, n))

        for paper in graph.values():
            if paper.paper_id not in idx_map:
                continue
            i = idx_map[paper.paper_id]
            for cited in paper.cited_by:
                if cited in idx_map:
                    j = idx_map[cited]
                    adj[j, i] = 1.0  # cited_by -> influence

        # Normalize by out-degree
        for j in range(n):
            out_degree = sum(adj[j, :])
            if out_degree > 0:
                adj[j, :] /= out_degree

        # PageRank iteration
        pr = np.ones(n) / n
        for _ in range(iterations):
            new_pr = (1 - damping) / n + damping * adj.T @ pr
            if np.allclose(pr, new_pr, atol=1e-6):
                break
            pr = new_pr

        # Assign PageRank scores
        for i, pid in enumerate(paper_ids):
            graph[pid].pagerank = pr[i]

    def _identify_key_papers(self, graph: Dict[str, PaperNode], level: ReadingLevel):
        """Identify foundational and milestone papers."""
        if not graph:
            return

        papers = list(graph.values())
        if not papers:
            return

        # Sort by PageRank
        papers.sort(key=lambda p: p.pagerank, reverse=True)

        # Top papers by PageRank are foundational
        top_count = max(1, len(papers) // 4)
        for p in papers[:top_count]:
            p.is_foundational = True

        # If we have year data, earliest papers with high PageRank are milestones
        papers_with_year = [p for p in papers if p.year > 0]
        if papers_with_year:
            papers_with_year.sort(key=lambda p: p.year)
            # First 25% by year with decent PageRank
            mid_rank = len(papers) // 2
            for p in papers_with_year[:mid_rank]:
                if p.pagerank > 0.5 / len(graph):
                    p.is_milestone = True

    def _topological_sort(
        self,
        graph: Dict[str, PaperNode],
        level: ReadingLevel,
    ) -> List[PaperNode]:
        """
        Topological sort for reading order.

        Key insight: Papers should be read before papers they are cited by.
        A cites B -> read B before A.
        """
        if not graph:
            return []

        # Build in-degree count (how many papers cite this paper)
        in_degree = {pid: 0 for pid in graph}
        for paper in graph.values():
            for cited in paper.cited_by:
                if cited in graph:  # Only count if in our graph
                    in_degree[paper.paper_id] += 1

        # Papers with no incoming edges (not cited by others in graph) go first
        queue = [pid for pid, deg in in_degree.items() if deg == 0]
        result = []

        while queue:
            # Prefer foundational papers and early-year papers
            queue.sort(
                key=lambda pid: (
                    -graph[pid].is_foundational,
                    -graph[pid].pagerank,
                    graph[pid].year if graph[pid].year else 9999,
                )
            )
            pid = queue.pop(0)
            result.append(graph[pid])

            # Reduce in-degree for papers this paper cites
            for cited_pid in graph[pid].cites:
                if cited_pid in in_degree:
                    in_degree[cited_pid] -= 1
                    if in_degree[cited_pid] == 0:
                        queue.append(cited_pid)

        # If we have cycles or missing edges, add remaining papers by PageRank
        if len(result) < len(graph):
            remaining = [graph[pid] for pid in graph if pid not in [p.paper_id for p in result]]
            remaining.sort(key=lambda p: (-p.pagerank, p.year if p.year else 9999))
            result.extend(remaining)

        return result

    def _generate_steps(
        self,
        papers: List[PaperNode],
        topic: str,
        level: ReadingLevel,
    ) -> List[ReadingStep]:
        """Generate reading steps with roles and reasons."""
        steps = []
        seen_years = set()

        for i, paper in enumerate(papers):
            role, reason = self._assign_role(paper, i, level, seen_years)
            read_time = self._estimate_read_time(paper)

            step = ReadingStep(
                order=i + 1,
                paper=paper,
                role=role,
                reason=reason,
                estimated_read_time_minutes=read_time,
            )
            steps.append(step)
            if paper.year > 0:
                seen_years.add(paper.year)

        return steps

    def _assign_role(
        self,
        paper: PaperNode,
        position: int,
        level: ReadingLevel,
        seen_years: Set[int],
    ) -> Tuple[str, str]:
        """Assign role and reason for including this paper."""
        year = paper.year

        if paper.is_foundational and (not year or year < 2018):
            return "foundation", "开创性工作，奠定了该领域的基础"

        if paper.is_foundational:
            return "core", "高影响力核心论文，必读"

        if position == 0:
            return "core", "作为入口论文，适合建立整体认知"

        # Check if this is a newer paper
        if year and year >= 2022:
            return "latest", f"最新进展（{year}年）"

        # Check for relationship to earlier papers
        if paper.cited_by and len(paper.cited_by) > 2:
            return "improvement", f"被多篇后续论文引用，影响力较高"

        if level == ReadingLevel.INTRO:
            if not year or year < 2020:
                return "core", "适合入门的核心论文"
            else:
                return "variant", f"{year}年的方法变体"

        return "improvement", "该领域的改进/应用"

    def _estimate_read_time(self, paper: PaperNode) -> int:
        """Estimate reading time in minutes based on available info."""
        # Base time
        base = 15

        # Title length heuristic
        if paper.title:
            base += len(paper.title) // 50

        # Citation count heuristic (more citations = more complex/important)
        if len(paper.cited_by) > 5:
            base += 10
        elif len(paper.cited_by) > 2:
            base += 5

        return min(base, 45)  # Cap at 45 minutes

    def _empty_path(self, topic: str, level: ReadingLevel) -> ReadingPath:
        """Return empty path when no papers found."""
        return ReadingPath(
            topic=topic,
            level=level,
            total_papers=0,
            total_reading_time_minutes=0,
            steps=[],
            alternative_paths=[],
            skipped_papers=[],
        )

    def render_path(self, path: ReadingPath) -> str:
        """Render reading path as formatted string."""
        if not path.steps:
            return f"📚 未找到关于「{path.topic}」的相关论文"

        level_labels = {
            ReadingLevel.INTRO: "入门",
            ReadingLevel.INTERMEDIATE: "进阶",
            ReadingLevel.ADVANCED: "深入",
        }

        lines = [
            f"📚 《{path.topic}》阅读路径推荐",
            f"   难度: {level_labels.get(path.level, '进阶')} | "
            f"共 {path.total_papers} 篇 | "
            f"预计 {path.total_reading_time_minutes} 分钟",
            "",
        ]

        role_icons = {
            "foundation": "🏛️",
            "core": "📖",
            "improvement": "⚡",
            "variant": "🔄",
            "latest": "✨",
        }

        for step in path.steps:
            icon = role_icons.get(step.role, "📄")
            year_str = f"[{step.paper.year}]" if step.paper.year else ""
            title = step.paper.title[:50] + ("..." if len(step.paper.title) > 50 else "")

            lines.append(f"{step.order}. {icon} {year_str} {title}")
            lines.append(f"   💡 {step.reason}")
            if step.paper.authors:
                authors_str = ", ".join(step.paper.authors[:2])
                lines.append(f"   👥 {authors_str}" + (" et al." if len(step.paper.authors) > 2 else ""))
            lines.append(f"   ⏱️ {step.estimated_read_time_minutes} min")
            lines.append("")

        if path.skipped_papers:
            lines.append(f"💡 还有 {len(path.skipped_papers)} 篇相关论文未显示")

        return "\n".join(lines)

    def render_mermaid(self, path: ReadingPath) -> str:
        """Render reading path as Mermaid graph."""
        if not path.steps:
            return "graph TD\n    Empty[\"No papers found\"]"

        lines = ["graph TD"]
        lines.append('    subgraph "Reading Path"')

        for step in path.steps:
            pid = step.paper.paper_id.replace("-", "_")
            title = step.paper.title[:30].replace('"', "'")
            role = step.role
            lines.append(f'    {step.order}_{pid}["{step.order}. {title}"]:::{role}')

        # Add arrows between steps
        for i in range(len(path.steps) - 1):
            curr = path.steps[i].paper.paper_id.replace("-", "_")
            next_pid = path.steps[i + 1].paper.paper_id.replace("-", "_")
            lines.append(f"    {i+1}_{curr} --> {i+2}_{next_pid}")

        lines.append("    end")
        lines.append("")
        lines.append("    classDef foundation fill:#f9f,stroke:#333")
        lines.append("    classDef core fill:#ff9,stroke:#333")
        lines.append("    classDef improvement fill:#9f9,stroke:#333")
        lines.append("    classDef latest fill:#9ff,stroke:#333")

        return "\n".join(lines)
