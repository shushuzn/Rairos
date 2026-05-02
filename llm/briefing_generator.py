"""Research Briefing Generator.

Generates structured research briefings from papers, enriched with
Gene Pool and Research Memory context for decision-relevant intelligence.
"""
from __future__ import annotations

import json
import uuid
from dataclasses import asdict, dataclass, field
from datetime import datetime
from pathlib import Path
from typing import Any, Dict, List, Optional

from llm.constants import LLM_BASE_URL, LLM_MODEL


@dataclass
class BriefingSection:
    """A section of the briefing."""
    title: str
    content: str
    level: int = 2  # markdown heading level


@dataclass
class Briefing:
    """A complete research briefing."""
    paper_arxiv_id: str
    paper_title: str
    sections: List[BriefingSection] = field(default_factory=list)
    gene_pool_matches: List[Dict[str, Any]] = field(default_factory=list)
    memory_stances: List[Dict[str, Any]] = field(default_factory=list)
    verdict: str = ""   # validates / contradicts / neutral / irrelevant
    verdict_reason: str = ""
    generated_at: str = ""


@dataclass
class BriefingResult:
    """Result of briefing generation."""
    success: bool
    briefing: Optional[Briefing] = None
    markdown: str = ""
    error: str = ""


def _load_gene_pool() -> List[Dict[str, Any]]:
    """Load Gene Pool entries for context."""
    try:
        from pathlib import Path
        path = Path.home() / ".ai_research_os" / "gene_pool" / "capsules.json"
        if path.exists():
            data = json.loads(path.read_text(encoding="utf-8"))
            # Support both raw list and {"version":1,"capsules":[...]} format
            if isinstance(data, list):
                return data
            return data.get("capsules", [])
    except Exception:
        pass
    return []


def _load_research_memory() -> List[Dict[str, Any]]:
    """Load Research Memory stances for context."""
    try:
        from pathlib import Path
        path = Path.home() / ".ai_research_os" / "research_memory" / "stances.json"
        if path.exists():
            return json.loads(path.read_text(encoding="utf-8"))
    except Exception:
        pass
    return []


def _match_gene_pool(topic: str, title: str, abstract: str) -> List[Dict[str, Any]]:
    """Find Gene Pool entries relevant to this paper."""
    gene_pool = _load_gene_pool()
    if not gene_pool:
        return []

    text = (title + " " + abstract).lower()
    topic_lower = topic.lower()

    matches = []
    for capsule in gene_pool:
        # Support multiple field name variants from different versions
        gap_title = (capsule.get("gap_title") or capsule.get("action_gap_title") or capsule.get("trigger_gap_title") or "").lower()
        gap_type = capsule.get("gap_type") or capsule.get("trigger_gap_type") or capsule.get("action_gap_type") or ""
        keywords = [k.lower() for k in (capsule.get("keywords") or capsule.get("trigger_keywords") or [])]
        outcome_score = capsule.get("outcome_success_score") or capsule.get("score") or 0.0

        # Simple keyword overlap
        overlap = sum(1 for kw in keywords if kw in text)
        if overlap >= 1 or any(kw in topic_lower for kw in keywords) or gap_title and overlap > 0:
            matches.append({
                "gap_title": gap_title,
                "gap_type": gap_type,
                "outcome_score": outcome_score,
                "match_reason": f"keyword overlap: {overlap}" if overlap else "topic match",
            })

    # Sort by score, return top 3
    matches.sort(key=lambda x: x.get("outcome_score", 0), reverse=True)
    return matches[:3]


def _match_research_memory(topic: str, title: str, abstract: str) -> List[Dict[str, Any]]:
    """Find Research Memory stances relevant to this paper."""
    stances = _load_research_memory()
    if not stances:
        return []

    text = (title + " " + abstract).lower()
    topic_lower = topic.lower()

    matches = []
    for stance in stances:
        claim = stance.get("claim", "").lower()
        topic_s = stance.get("topic", "").lower()

        # Check keyword overlap
        claim_words = set(claim.split()) & set(text.split())
        topic_words = set(topic_s.split()) & set(text.split())

        if claim_words or topic_words:
            matches.append({
                "stance_id": stance.get("stance_id", ""),
                "topic": stance.get("topic", ""),
                "claim": stance.get("claim", ""),
                "stance_type": stance.get("stance", ""),
                "confidence": stance.get("confidence", 0.0),
                "evidence_refs": stance.get("evidence_refs", []),
            })

    return matches[:3]


class BriefingGenerator:
    """Generate structured research briefings enriched with Gene Pool + Memory context."""

    def __init__(self, db=None):
        self.db = db

    def generate(
        self,
        arxiv_id: str,
        use_llm: bool = True,
        api_key: Optional[str] = None,
        base_url: Optional[str] = None,
        model: Optional[str] = None,
        output_dir: Optional[Path] = None,
    ) -> BriefingResult:
        """Generate a briefing for a paper.

        Args:
            arxiv_id: arXiv ID of the paper
            use_llm: Use LLM for generation (False = metadata-only)
            api_key: LLM API key
            base_url: LLM API base URL
            model: Model name
            output_dir: Directory to save the briefing markdown

        Returns:
            BriefingResult with the generated briefing
        """
        import os
        import datetime

        # Step 1: Fetch paper from DB
        paper = self._fetch_paper(arxiv_id)
        if not paper:
            return BriefingResult(success=False, error=f"Paper {arxiv_id} not found")

        title = paper.get("title", "Unknown")
        abstract = paper.get("abstract", "")
        authors = paper.get("authors", [])

        # Step 2: Match against Gene Pool and Research Memory
        gene_pool_matches = _match_gene_pool(arxiv_id, title, abstract)
        memory_stances = _match_research_memory(arxiv_id, title, abstract)

        # Step 3: Determine verdict
        verdict, verdict_reason = self._compute_verdict(
            title, abstract, gene_pool_matches, memory_stances
        )

        # Step 4: Generate content
        if use_llm:
            sections = self._generate_llm_briefing(
                title=title,
                abstract=abstract,
                authors=authors,
                arxiv_id=arxiv_id,
                gene_pool_matches=gene_pool_matches,
                memory_stances=memory_stances,
                verdict=verdict,
                verdict_reason=verdict_reason,
                api_key=api_key,
                base_url=base_url,
                model=model,
            )
        else:
            sections = self._generate_metadata_briefing(
                title, abstract, authors, arxiv_id, gene_pool_matches, memory_stances
            )

        now = datetime.datetime.now().isoformat()
        briefing = Briefing(
            paper_arxiv_id=arxiv_id,
            paper_title=title,
            sections=sections,
            gene_pool_matches=gene_pool_matches,
            memory_stances=memory_stances,
            verdict=verdict,
            verdict_reason=verdict_reason,
            generated_at=now,
        )

        markdown = self._render_markdown(briefing, authors)

        result = BriefingResult(success=True, briefing=briefing, markdown=markdown)

        # Step 5: Save to file
        if output_dir:
            self._save_briefing(briefing, markdown, output_dir)

        return result

    def _fetch_paper(self, arxiv_id: str) -> Optional[Dict[str, Any]]:
        """Fetch paper from local DB."""
        if self.db is None:
            try:
                from db.database import Database
                db = Database()
                db.init()
                self.db = db
            except Exception:
                return None

        try:
            # Try direct ID lookup first (exact match for arxiv_id)
            paper = self.db.get_paper(arxiv_id)
            if paper:
                return {
                    "arxiv_id": paper.id,
                    "title": paper.title,
                    "abstract": paper.abstract or "",
                    "authors": paper.authors or [],
                    "published": paper.published or "",
                    "venue": paper.journal or "",
                    "citation_count": 0,
                    "tags": paper.tags or [],
                }
            # Fall back to full-text search
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
                "venue": getattr(r, "venue", "") or "",
                "citation_count": getattr(r, "citation_count", 0) or 0,
                "tags": getattr(r, "tags", []) or [],
            }
        except Exception:
            return None

    def _compute_verdict(
        self,
        title: str,
        abstract: str,
        gene_pool_matches: List[Dict[str, Any]],
        memory_stances: List[Dict[str, Any]],
    ) -> tuple:
        """Compute whether paper validates, contradicts, or is neutral to existing knowledge."""
        if not gene_pool_matches and not memory_stances:
            return "neutral", "No matching Gene Pool entries or Research Memory stances"

        text = (title + " " + abstract).lower()

        # Check Gene Pool gaps
        validates_gaps = any(
            m.get("outcome_score", 0) > 0.5 for m in gene_pool_matches
        )

        # Check Research Memory stances
        contradiction_signals = ["fail to", "does not", "cannot", "ineffective", "worse than", "no evidence", "contrary to", "challenges"]
        contradicts = any(
            sig in text for sig in contradiction_signals
        )

        if contradicts:
            return "contradicts", "Paper contains language suggesting it challenges existing approaches"
        if validates_gaps:
            return "validates", "Paper addresses Gene Pool gaps with high outcome scores"
        return "neutral", "Paper is related but does not directly validate or contradict existing knowledge"

    def _generate_llm_briefing(
        self,
        title: str,
        abstract: str,
        authors: List[str],
        arxiv_id: str,
        gene_pool_matches: List[Dict[str, Any]],
        memory_stances: List[Dict[str, Any]],
        verdict: str,
        verdict_reason: str,
        api_key: Optional[str],
        base_url: Optional[str],
        model: Optional[str],
    ) -> List[BriefingSection]:
        """Generate briefing using LLM."""
        import os

        try:
            from llm.chat import call_llm_chat_completions
        except ImportError:
            from llm.client import call_llm_chat_completions

        gene_context = ""
        if gene_pool_matches:
            gene_context += "\n\n**Relevant Gene Pool Gaps:**\n"
            for m in gene_pool_matches:
                gene_context += f"- {m['gap_title']} (type: {m['gap_type']}, score: {m['outcome_score']:.2f})\n"

        memory_context = ""
        if memory_stances:
            memory_context += "\n\n**Relevant Research Memory Stances:**\n"
            for s in memory_stances:
                memory_context += f"- [{s['stance_type'].upper()}] {s['topic']}: {s['claim'][:80]}\n"

        prompt = f"""Generate a structured research briefing for the following paper.

**Paper:** {title}
**Authors:** {", ".join(authors[:3]) if authors else "Unknown"}
**arXiv:** {arxiv_id}

**Abstract:**
{abstract[:1000]}

{gene_context}{memory_context}

**Verdict:** {verdict.upper()} — {verdict_reason}

Generate a briefing with these sections:
1. **TL;DR** — 2-3 sentence summary of what the paper does and why it matters
2. **Key Claims** — the 2-3 most important claims (bullet points)
3. **Method Summary** — how the paper approaches the problem (3-4 sentences)
4. **Strengths** — genuine strengths of the approach
5. **Weaknesses** — honest weaknesses / limitations
6. **Gene Pool Relevance** — how this paper relates to your open research gaps
7. **Research Memory Alignment** — does it validate or contradict your prior stances?
8. **Action Items** — what you should do next (read in depth / run experiment / discuss)

Respond with ONLY valid markdown. Be concise and critical."""

        try:
            response = call_llm_chat_completions(
                base_url=base_url or os.getenv("OPENAI_BASE_URL", "") or LLM_BASE_URL,
                api_key=api_key or os.getenv("OPENAI_API_KEY", ""),
                model=model or os.getenv("LLM_MODEL", "") or LLM_MODEL,
                system_prompt="You are a research intelligence analyst. Produce concise, critical briefings.",
                user_prompt=prompt,
            )
        except Exception as e:
            return self._generate_metadata_briefing(
                title, abstract, authors, arxiv_id, gene_pool_matches, memory_stances
            )

        # Parse response into sections
        sections = self._parse_sections(response)
        return sections

    def _parse_sections(self, response: str) -> List[BriefingSection]:
        """Parse LLM markdown response into structured sections."""
        sections = []
        current_title = "Summary"
        current_lines = []

        for line in response.split("\n"):
            line = line.strip()
            if not line:
                current_lines.append("")
                continue

            # Detect markdown headers
            if line.startswith("## ") or line.startswith("**") and line.endswith("**"):
                # Save previous section
                if current_lines:
                    sections.append(BriefingSection(
                        title=current_title,
                        content="\n".join(current_lines).strip(),
                    ))
                    current_lines = []

                title = line.lstrip("#* ").rstrip("*").strip()
                if title.lower().startswith("tldr") or title.lower().startswith("tl;dr"):
                    title = "TL;DR"
                current_title = title
            else:
                current_lines.append(line)

        if current_lines:
            sections.append(BriefingSection(
                title=current_title,
                content="\n".join(current_lines).strip(),
            ))

        return sections

    def _generate_metadata_briefing(
        self,
        title: str,
        abstract: str,
        authors: List[str],
        arxiv_id: str,
        gene_pool_matches: List[Dict[str, Any]],
        memory_stances: List[Dict[str, Any]],
    ) -> List[BriefingSection]:
        """Generate a metadata-only briefing without LLM."""
        import datetime

        sections = [
            BriefingSection(
                title="TL;DR",
                content=f"**{title}**\n\nAuthors: {', '.join(authors[:3]) if authors else 'Unknown'}\n\n{abstract[:300]}...",
            ),
            BriefingSection(
                title="Key Metadata",
                content=f"- arXiv: {arxiv_id}\n- Authors: {', '.join(authors) if authors else 'Unknown'}\n- Gene Pool Matches: {len(gene_pool_matches)}\n- Research Memory Stances: {len(memory_stances)}",
            ),
        ]

        if gene_pool_matches:
            lines = [f"**Relevant Gene Pool Gaps ({len(gene_pool_matches)}):**"]
            for m in gene_pool_matches:
                lines.append(f"- [{m['gap_type']}] {m['gap_title']} (score: {m['outcome_score']:.2f})")
            sections.append(BriefingSection(title="Gene Pool Relevance", content="\n".join(lines)))

        if memory_stances:
            lines = [f"**Relevant Research Memory Stances ({len(memory_stances)}):**"]
            for s in memory_stances:
                lines.append(f"- [{s['stance_type'].upper()}] {s['topic']}: {s['claim'][:80]}")
            sections.append(BriefingSection(title="Memory Alignment", content="\n".join(lines)))

        return sections

    def _render_markdown(self, briefing: Briefing, authors: List[str]) -> str:
        """Render Briefing as markdown string."""
        import datetime

        now = datetime.datetime.now().strftime("%Y-%m-%d")

        verdict_emoji = {"validates": "✅", "contradicts": "❌", "neutral": "⚪", "irrelevant": "🚫"}
        emoji = verdict_emoji.get(briefing.verdict, "⚪")

        lines = [
            f"# Research Briefing: {briefing.paper_title}",
            "",
            f"**arXiv:** [{briefing.paper_arxiv_id}](https://arxiv.org/abs/{briefing.paper_arxiv_id}) | "
            f"**Authors:** {', '.join(authors[:3]) if authors else 'Unknown'} | "
            f"**Generated:** {now}",
            "",
            f"**Verdict:** {emoji} **{briefing.verdict.upper()}** — {briefing.verdict_reason}",
            "",
        ]

        for section in briefing.sections:
            lines.append(f"## {section.title}")
            lines.append("")
            lines.append(section.content)
            lines.append("")

        if briefing.gene_pool_matches:
            lines.append("## Gene Pool Matches")
            lines.append("")
            for m in briefing.gene_pool_matches:
                lines.append(f"- **[{m['gap_type']}]** {m['gap_title']} (score: {m['outcome_score']:.2f}) — {m['match_reason']}")
            lines.append("")

        if briefing.memory_stances:
            lines.append("## Research Memory Alignment")
            lines.append("")
            for s in briefing.memory_stances:
                lines.append(f"- **[{s['stance_type'].upper()}]** {s['topic']}: {s['claim'][:80]}")
            lines.append("")

        lines.append("---")
        lines.append(f"_Generated by Rairos BriefingGenerator on {now}_")

        return "\n".join(lines)

    def _save_briefing(self, briefing: Briefing, markdown: str, output_dir: Path) -> Path:
        """Save briefing as markdown file."""
        output_dir = Path(output_dir)
        output_dir.mkdir(parents=True, exist_ok=True)
        slug = "".join(c if c.isalnum() else "_" for c in briefing.paper_arxiv_id.lower())
        filepath = output_dir / f"briefing_{slug}.md"
        filepath.write_text(markdown, encoding="utf-8")
        return filepath
