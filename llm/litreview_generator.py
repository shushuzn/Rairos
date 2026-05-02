"""LLM-Powered Literature Review Generator.

Generates structured, narrative literature reviews from a paper corpus.
Unlike renderers/litreview.py (which sorts/groups papers), this uses an LLM
to produce thematic sections, consensus/disagreement analysis, and narrative flow.

Workflow:
    1. Collect papers from local DB for a topic
    2. Extract structured context (titles, abstracts, methods, results)
    3. Prompt LLM to generate a narrative literature review
    4. Return structured sections + save as markdown file
"""
from __future__ import annotations

import json
import uuid
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any, Dict, List, Optional

from llm.constants import LLM_BASE_URL, LLM_MODEL


_LIT_REVIEW_SYSTEM_PROMPT = """You are an expert research literature reviewer.
Given a collection of papers on a topic, write a comprehensive, structured literature review.

Structure your review as follows:
1. **Overview** — scope, scale, and timeline of research in this area
2. **Thematic Categories** — organize papers into 3-5 coherent themes/families
3. **Consensus Points** — what the field agrees on
4. **Controversies & Disagreements** — where researchers differ
5. **Research Gaps** — underexplored areas, contradictions, future directions
6. **Key Papers** — must-read papers per theme with 1-line justification

Be critical, analytical, and concise. Use academic tone.
Reference papers by their short titles in brackets.
Do NOT list every paper — synthesize and prioritize."""


_LIT_REVIEW_USER_TEMPLATE = """Topic: {topic}

Papers ({count} total):
{papers_text}

Please generate a comprehensive literature review following the required structure."""


@dataclass
class LitReviewSection:
    """A section of the literature review."""
    title: str
    content: str
    paper_refs: List[str] = field(default_factory=list)  # short titles referenced
    subsection_titles: List[str] = field(default_factory=list)


@dataclass
class LitReview:
    """A complete generated literature review."""
    topic: str
    sections: List[LitReviewSection] = field(default_factory=list)
    papers_used: List[str] = field(default_factory=list)  # arxiv_ids
    total_papers: int = 0
    generated_at: str = ""


@dataclass
class LitReviewResult:
    """Result of lit review generation."""
    success: bool
    topic: str
    review: Optional[LitReview] = None
    markdown: str = ""
    error: str = ""


class LitReviewGenerator:
    """Generate narrative literature reviews using LLM."""

    def __init__(self, db=None):
        self.db = db

    def generate(
        self,
        topic: str,
        limit: int = 30,
        use_llm: bool = True,
        api_key: Optional[str] = None,
        base_url: Optional[str] = None,
        model: Optional[str] = None,
        output_dir: Optional[Path] = None,
    ) -> LitReviewResult:
        """Generate a literature review for a topic.

        Args:
            topic: Research topic to review
            limit: Max papers to include (default 30)
            use_llm: Use LLM for generation (False = template-only)
            api_key: LLM API key
            base_url: LLM API base URL
            model: Model name
            output_dir: Directory to save the review markdown

        Returns:
            LitReviewResult with the generated review
        """
        import os
        import datetime

        # Step 1: Collect papers
        papers = self._collect_papers(topic, limit)
        if not papers:
            return LitReviewResult(
                success=False,
                topic=topic,
                error=f"No papers found for topic: {topic}",
            )

        # Step 2: Build papers text for LLM
        papers_text = self._build_papers_text(papers)
        count = len(papers)

        if not use_llm:
            # Template-only mode: basic structured review without LLM
            review = self._generate_template_review(topic, papers)
            markdown = self._render_markdown(review)
            result = LitReviewResult(success=True, topic=topic, review=review, markdown=markdown)
        else:
            # LLM-powered generation
            api_key = api_key or os.getenv("OPENAI_API_KEY", "")
            if not api_key:
                return LitReviewResult(
                    success=False,
                    topic=topic,
                    error="OPENAI_API_KEY not set",
                )

            try:
                review = self._generate_llm_review(
                    topic=topic,
                    papers_text=papers_text,
                    count=count,
                    api_key=api_key,
                    base_url=base_url,
                    model=model,
                )
                markdown = self._render_markdown(review)
                result = LitReviewResult(success=True, topic=topic, review=review, markdown=markdown)
            except Exception as e:
                # Fallback to template on LLM failure
                review = self._generate_template_review(topic, papers)
                markdown = self._render_markdown(review)
                result = LitReviewResult(
                    success=True, topic=topic, review=review, markdown=markdown
                )

        # Step 3: Save to file
        if output_dir and result.success:
            self._save_review(result.review, markdown, output_dir)

        return result

    def _collect_papers(self, topic: str, limit: int) -> List[Dict[str, Any]]:
        """Collect papers from local DB for a topic."""
        if self.db is None:
            try:
                from db.database import Database
                db = Database()
                db.init()
                self.db = db
            except Exception:
                return []

        try:
            rows, _ = self.db.search_papers(topic, limit=limit)
            papers = []
            for r in rows:
                papers.append({
                    "arxiv_id": getattr(r, "paper_id", "") or getattr(r, "arxiv_id", ""),
                    "title": getattr(r, "title", ""),
                    "abstract": getattr(r, "abstract", "") or "",
                    "authors": getattr(r, "authors", []) or [],
                    "published": getattr(r, "published", "") or "",
                    "score": getattr(r, "score", 0) or 0,
                })
            return papers
        except Exception:
            return []

    def _build_papers_text(self, papers: List[Dict[str, Any]]) -> str:
        """Build papers text for LLM context."""
        lines = []
        for i, p in enumerate(papers, 1):
            authors = ", ".join(p.get("authors", [])[:3]) if p.get("authors") else "Unknown"
            year = p.get("published", "")[:4] if p.get("published") else "?"
            title = p.get("title", "Untitled")
            abstract = p.get("abstract", "")[:300]
            lines.append(
                f"[{i}] ({year}) {title}\n"
                f"    Authors: {authors}\n"
                f"    Abstract: {abstract}..."
            )
        return "\n\n".join(lines)

    def _generate_llm_review(
        self,
        topic: str,
        papers_text: str,
        count: int,
        api_key: str,
        base_url: Optional[str],
        model: Optional[str],
    ) -> LitReview:
        """Generate lit review using LLM."""
        import os

        try:
            from llm.chat import call_llm_chat_completions
        except ImportError:
            from llm.client import call_llm_chat_completions

        user_prompt = _LIT_REVIEW_USER_TEMPLATE.format(
            topic=topic,
            count=count,
            papers_text=papers_text,
        )

        response = call_llm_chat_completions(
            base_url=base_url or os.getenv("OPENAI_BASE_URL", "") or LLM_BASE_URL,
            api_key=api_key,
            model=model or os.getenv("LLM_MODEL", "") or LLM_MODEL,
            system_prompt=_LIT_REVIEW_SYSTEM_PROMPT,
            user_prompt=user_prompt,
        )

        # Parse LLM response into structured sections
        sections = self._parse_llm_response(response)
        now = datetime.datetime.now().isoformat()

        return LitReview(
            topic=topic,
            sections=sections,
            papers_used=[p.get("arxiv_id", "") for p in []],
            total_papers=count,
            generated_at=now,
        )

    def _parse_llm_response(self, response: str) -> List[LitReviewSection]:
        """Parse LLM response into structured sections."""
        sections = []
        current_section: Optional[LitReviewSection] = None
        current_content: List[str] = []

        for line in response.split("\n"):
            line = line.strip()
            if not line:
                if current_content:
                    current_content.append("")
                continue

            # Detect section headers
            is_header = False
            for prefix in ["## ", "# ", "**", "*", "##"]:
                if line.startswith(prefix) and len(line) > 3:
                    is_header = True
                    break

            if line.startswith("## ") or (line.startswith("# ") and "literature review" not in line.lower()):
                # Save previous section
                if current_section and current_content:
                    current_section.content = "\n".join(current_content).strip()
                    sections.append(current_section)

                title = line.lstrip("#* ").rstrip(":").strip()
                current_section = LitReviewSection(title=title, content="")
                current_content = []
            elif line.startswith("**") and ":" in line and current_section is None:
                # First section without ## prefix
                title = line.lstrip("*").rstrip(":").strip()
                current_section = LitReviewSection(title=title, content="")
                current_content = []
            else:
                if current_content is None:
                    current_content = []
                current_content.append(line)

        # Save last section
        if current_section and current_content:
            current_section.content = "\n".join(current_content).strip()
            sections.append(current_section)

        # If no sections parsed, create a single section with full content
        if not sections and response.strip():
            sections.append(LitReviewSection(
                title="Literature Review",
                content=response.strip()
            ))

        return sections

    def _generate_template_review(self, topic: str, papers: List[Dict[str, Any]]) -> LitReview:
        """Generate a template-based review without LLM."""
        import datetime

        now = datetime.datetime.now().isoformat()

        # Group papers by year
        by_year: Dict[str, List] = {}
        for p in papers:
            year = p.get("published", "")[:4] or "unknown"
            by_year.setdefault(year, []).append(p)

        # Top papers by score
        top = sorted(papers, key=lambda x: x.get("score", 0), reverse=True)[:10]

        sections = [
            LitReviewSection(
                title="Overview",
                content=f"This review covers {len(papers)} papers on **{topic}**. "
                        f"Research spans {min(by_year.keys(), default='N/A')} to {max(by_year.keys(), default='N/A')}.",
            ),
            LitReviewSection(
                title="Top Papers by Relevance",
                content="\n".join(
                    f"- **{p['title'][:60]}** ({p.get('published', '')[:4]})"
                    for p in top
                ),
            ),
            LitReviewSection(
                title="Timeline",
                content="\n".join(
                    f"- **{year}**: {len(plist)} paper(s)"
                    for year, plist in sorted(by_year.items(), reverse=True)
                ),
            ),
        ]

        return LitReview(
            topic=topic,
            sections=sections,
            papers_used=[p.get("arxiv_id", "") for p in papers],
            total_papers=len(papers),
            generated_at=now,
        )

    def _render_markdown(self, review: LitReview) -> str:
        """Render LitReview as markdown string."""
        import datetime

        now = datetime.datetime.now().strftime("%Y-%m-%d")
        lines = [
            f"# Literature Review: {review.topic}",
            "",
            f"**Generated:** {now} | **Papers reviewed:** {review.total_papers}",
            "",
            "---",
            "",
        ]

        for section in review.sections:
            lines.append(f"## {section.title}")
            lines.append("")
            if section.content:
                lines.append(section.content)
            lines.append("")

        lines.append("---")
        lines.append(f"_Generated by Rairos LitReviewGenerator on {now}_")

        return "\n".join(lines)

    def _save_review(
        self,
        review: LitReview,
        markdown: str,
        output_dir: Path,
    ) -> Path:
        """Save review as markdown file."""
        output_dir = Path(output_dir)
        output_dir.mkdir(parents=True, exist_ok=True)

        slug = "".join(c if c.isalnum() else "_" for c in review.topic.lower())[:40]
        filepath = output_dir / f"litreview_{slug}_{uuid.uuid4().hex[:6]}.md"
        filepath.write_text(markdown, encoding="utf-8")
        return filepath
