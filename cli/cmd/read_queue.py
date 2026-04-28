"""CLI command: read-queue — Smart reading priority queue."""
from __future__ import annotations

import argparse
from dataclasses import dataclass
from typing import Any, Dict, List, Optional

import os
import warnings

from cli._shared import get_db, Colors, colored, print_info


@dataclass
class QueuedPaper:
    """A paper in the reading queue with scoring breakdown."""
    paper_id: str
    title: str
    score: float
    semantic_score: float
    citation_score: float
    tag_score: float
    recency_score: float
    authors: list[str]
    published: str
    primary_category: str


class ReadQueueScorer:
    """Score papers for reading priority based on multiple signals."""

    def __init__(self, db):
        self.db = db
        # Default weights (can be adjusted)
        self.alpha = 0.4   # semantic similarity weight
        self.beta = 0.3     # citation relationship weight
        self.gamma = 0.2    # tag overlap weight
        self.delta = 0.1    # recency weight

    def _get_read_papers(self, limit: int = 50) -> list:
        """Get papers the user has already read (parsed/completed)."""
        results, _ = self.db.search_papers(
            query="",
            limit=limit,
            parse_status="parsed"  # Only fully parsed papers
        )
        return results

    def get_read_papers_context(self, limit: int = 10) -> list[dict]:
        """Return list of dicts for LLM context."""
        results, _ = self.db.search_papers(
            query="",
            limit=limit,
            parse_status="parsed"
        )
        return [
            {
                "title": p.title or "",
                "authors": list(p.authors) if p.authors else [],
                "year": (p.published[:4] if p.published and len(p.published) >= 4 else "N/A"),
                "category": (p.primary_category or ""),
            }
            for p in results
        ]

    def _compute_semantic_scores(self, read_papers: list, candidates: list) -> dict:
        """Compute average similarity to read papers."""
        scores = {}
        if not read_papers:
            return scores

        for candidate in candidates:
            total_sim = 0.0
            sim_count = 0
            for read_paper in read_papers:
                try:
                    sim = self.db.get_similarity(read_paper.paper_id, candidate.paper_id)
                    if sim is not None:
                        total_sim += sim
                        sim_count += 1
                except Exception:
                    pass
            scores[candidate.paper_id] = total_sim / sim_count if sim_count > 0 else 0.0
        return scores

    def _compute_citation_scores(self, read_papers: list, candidates: list) -> dict:
        """Score papers that cite or are cited by read papers."""
        scores = {}
        read_ids = {p.paper_id for p in read_papers}

        for candidate in candidates:
            score = 0.0
            # Check if this candidate cites any read paper
            try:
                edges = self.db.get_edges_by_node(candidate.paper_id, direction="out", rel_type="cite")
                for edge in edges:
                    if edge.get("target_id") in read_ids:
                        score += 1.0
                # Check if any read paper cites this candidate
                edges_in = self.db.get_edges_by_node(candidate.paper_id, direction="in", rel_type="cite")
                for edge in edges_in:
                    if edge.get("source_id") in read_ids:
                        score += 0.5  # Slightly lower weight for being cited
            except Exception:
                pass
            scores[candidate.paper_id] = min(score / 5.0, 1.0)  # Normalize to 0-1
        return scores

    def _compute_tag_scores(self, read_papers: list, candidates: list) -> dict:
        """Score based on tag overlap with read papers."""
        scores = {}
        # Get tags from read papers
        read_tags = set()
        for p in read_papers:
            if hasattr(p, "categories") and p.categories:
                for tag in p.categories.split(","):
                    read_tags.add(tag.strip().lower())

        for candidate in candidates:
            if hasattr(candidate, "categories") and candidate.categories:
                cand_tags = set(c.lower() for c in candidate.categories.split(","))
                overlap = len(cand_tags & read_tags)
                scores[candidate.paper_id] = min(overlap / 3.0, 1.0)  # 3+ overlapping tags = max
            else:
                scores[candidate.paper_id] = 0.0
        return scores

    def _compute_recency_scores(self, candidates: list) -> dict:
        """Score based on paper recency (prefer recent papers)."""
        import time
        scores = {}
        current_year = int(time.strftime("%Y"))

        for candidate in candidates:
            year = current_year
            if hasattr(candidate, "published") and candidate.published:
                try:
                    year = int(candidate.published[:4])
                except (ValueError, TypeError):
                    pass
            # Score: papers from current year = 1.0, decreasing 0.1 per year older
            score = max(0.0, 1.0 - (current_year - year) * 0.1)
            scores[candidate.paper_id] = min(score, 1.0)
        return scores

    def score_papers(self, candidates: list, limit: int = 20) -> list[QueuedPaper]:
        """Score and rank candidate papers for reading priority."""
        read_papers = self._get_read_papers(limit=30)

        # Compute individual scores
        semantic = self._compute_semantic_scores(read_papers, candidates)
        citation = self._compute_citation_scores(read_papers, candidates)
        tag = self._compute_tag_scores(read_papers, candidates)
        recency = self._compute_recency_scores(candidates)

        # Combine scores
        results = []
        for candidate in candidates:
            s_sem = semantic.get(candidate.paper_id, 0.0)
            s_cit = citation.get(candidate.paper_id, 0.0)
            s_tag = tag.get(candidate.paper_id, 0.0)
            s_rec = recency.get(candidate.paper_id, 0.0)

            # Weighted combination
            combined = (
                self.alpha * s_sem +
                self.beta * s_cit +
                self.gamma * s_tag +
                self.delta * s_rec
            )

            # Boost if we have any strong signal
            if max(s_sem, s_cit, s_tag) > 0.5:
                combined = min(combined * 1.2, 1.0)

            results.append(QueuedPaper(
                paper_id=candidate.paper_id,
                title=candidate.title,
                score=combined,
                semantic_score=s_sem,
                citation_score=s_cit,
                tag_score=s_tag,
                recency_score=s_rec,
                authors=candidate.authors if hasattr(candidate, "authors") else [],
                published=candidate.published if hasattr(candidate, "published") else "",
                primary_category=candidate.primary_category if hasattr(candidate, "primary_category") else "",
            ))

        # Sort by combined score
        results.sort(key=lambda x: x.score, reverse=True)
        return results[:limit]


def _build_read_queue_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser(
        "read-queue",
        help="Smart reading priority queue based on your reading history",
    )
    p.add_argument(
        "--limit", type=int, default=10,
        help="Max papers to return (default: 10)",
    )
    p.add_argument(
        "--tag", action="append",
        help="Filter by tag/field of interest (repeatable)",
    )
    p.add_argument(
        "--year", type=int,
        help="Filter by minimum year",
    )
    p.add_argument(
        "--min-similarity", type=float, default=0.0,
        help="Minimum semantic similarity threshold (default: 0.0)",
    )
    p.add_argument(
        "--format", choices=["table", "json", "score-breakdown"], default="table",
        help="Output format (default: table)",
    )
    p.add_argument(
        "--explain", action="store_true",
        help="Generate LLM explanations for why each paper is recommended",
    )
    p.add_argument(
        "--explain-model", type=str, default=None,
        help="LLM model for explanations (default: from config)",
    )
    # Reading status management
    p.add_argument(
        "--start", metavar="PAPER_ID",
        help="Start reading a paper (set status to 'reading')",
    )
    p.add_argument(
        "--done", metavar="PAPER_ID",
        help="Mark a paper as completed",
    )
    p.add_argument(
        "--status", metavar="PAPER_ID", nargs="?", const="",
        help="Check reading status of a paper, or list all reading papers",
    )
    p.add_argument(
        "--reset", metavar="PAPER_ID",
        help="Reset reading status to 'unread'",
    )
    return p


def _handle_status_action(args: argparse.Namespace, db) -> Optional[int]:
    """Handle --start, --done, --status, --reset actions. Returns exit code or None to continue."""
    from cli._shared import print_success, print_error, print_info

    # --status: show status
    if args.status is not None:
        if args.status == "":
            # List all reading papers
            reading = db.get_papers_by_reading_status("reading", limit=50)
            completed = db.get_papers_by_reading_status("completed", limit=50)
            if not reading and not completed:
                print_info("No papers currently reading or completed.")
            if reading:
                print(colored("📖 正在阅读", Colors.HEADER))
                print("-" * 60)
                for i, p in enumerate(reading, 1):
                    started = p.reading_started_at or "N/A"
                    print(f"  {i}. {p.id} — {p.title[:50]}")
                    print(f"     开始于: {started}")
                print()
            if completed:
                print(colored("✅ 已完成", Colors.HEADER))
                print("-" * 60)
                for i, p in enumerate(completed, 1):
                    completed_at = p.reading_completed_at or "N/A"
                    print(f"  {i}. {p.id} — {p.title[:50]}")
                    print(f"     完成于: {completed_at}")
            return 0
        else:
            # Show specific paper status
            info = db.get_reading_status(args.status)
            if info is None:
                print_error(f"Paper not found: {args.status}")
                return 1
            title = db.get_paper_title(args.status)
            status_map = {"unread": "未读", "reading": "📖 正在阅读", "completed": "✅ 已完成"}
            status_text = status_map.get(info["status"], info["status"])
            print(f"📄 {args.status}")
            print(f"   标题: {title[:60] if title else 'N/A'}")
            print(f"   状态: {status_text}")
            if info["started_at"]:
                print(f"   开始于: {info['started_at']}")
            if info["completed_at"]:
                print(f"   完成于: {info['completed_at']}")
            return 0

    # --start: begin reading
    if args.start:
        if not db.paper_exists(args.start):
            print_error(f"Paper not found: {args.start}")
            return 1
        title = db.get_paper_title(args.start)
        db.update_reading_status(args.start, "reading")
        print_success(f"Started reading: {args.start}")
        if title:
            print(f"   {title[:60]}")
        return 0

    # --done: mark completed
    if args.done:
        if not db.paper_exists(args.done):
            print_error(f"Paper not found: {args.done}")
            return 1
        title = db.get_paper_title(args.done)
        db.update_reading_status(args.done, "completed")
        print_success(f"Marked as completed: {args.done}")
        if title:
            print(f"   {title[:60]}")
        return 0

    # --reset: reset to unread
    if args.reset:
        if not db.paper_exists(args.reset):
            print_error(f"Paper not found: {args.reset}")
            return 1
        title = db.get_paper_title(args.reset)
        db.update_reading_status(args.reset, "unread")
        print_success(f"Reset reading status: {args.reset}")
        if title:
            print(f"   {title[:60]}")
        return 0

    return None  # No status action taken, continue with normal flow


def _run_read_queue(args: argparse.Namespace) -> int:
    from cli._shared import load_dotenv
    load_dotenv()

    db = get_db()
    db.init()

    # Handle reading status actions first
    result = _handle_status_action(args, db)
    if result is not None:
        return result

    # Get candidate papers — exclude completed papers from reading queue
    candidates, total = db.search_papers(
        query="",
        limit=200,
    )

    # Filter out completed papers from recommendations
    completed_ids = {p.id for p in db.get_papers_by_reading_status("completed", limit=1000)}
    candidates = [p for p in candidates if p.paper_id not in completed_ids]

    # Apply filters
    if args.tag:
        filtered = []
        for p in candidates:
            if hasattr(p, "categories") and p.categories:
                p_tags = set(c.lower() for c in p.categories.split(","))
                if any(t.lower() in p_tags for t in args.tag):
                    filtered.append(p)
        candidates = filtered

    if args.year:
        candidates = [p for p in candidates
                     if hasattr(p, "published") and p.published
                     and p.published[:4].isdigit()
                     and int(p.published[:4]) >= args.year]

    # Score and rank
    scorer = ReadQueueScorer(db)
    results = scorer.score_papers(candidates, limit=args.limit)

    # Filter by min similarity if specified
    if args.min_similarity > 0:
        results = [r for r in results if r.semantic_score >= args.min_similarity]

    if not results:
        print_info("No papers match your criteria. Try relaxing filters or add more papers to your library.")
        return 0

    # Generate LLM explanations if requested
    explanations = {}
    if args.explain:
        api_key = os.getenv("OPENAI_API_KEY")
        if not api_key:
            print_info("OPENAI_API_KEY not set. Skipping explanations. Set it to enable LLM explanations.")
        else:
            from config import DEFAULT_LLM_MODEL_CLI, DEFAULT_OPENAI_BASE_URL
            from llm.generate import ai_generate_reading_recommendation_explanation

            model = args.explain_model or DEFAULT_LLM_MODEL_CLI
            base_url = DEFAULT_OPENAI_BASE_URL
            read_context = scorer.get_read_papers_context(limit=10)

            for r in results:
                try:
                    explanation = ai_generate_reading_recommendation_explanation(
                        paper_title=r.title,
                        paper_authors=r.authors,
                        paper_year=r.published[:4] if r.published else "N/A",
                        paper_category=r.primary_category,
                        score=r.score,
                        semantic_score=r.semantic_score,
                        citation_score=r.citation_score,
                        tag_score=r.tag_score,
                        recency_score=r.recency_score,
                        read_papers_context=read_context,
                        base_url=base_url,
                        api_key=api_key,
                        model=model,
                    )
                    explanations[r.paper_id] = explanation
                except Exception as e:
                    warnings.warn(f"LLM explanation failed for {r.paper_id}: {e}")
                    explanations[r.paper_id] = None

    if args.format == "json":
        import json
        output = [{
            "paper_id": r.paper_id,
            "title": r.title,
            "score": round(r.score, 3),
            "authors": r.authors,
            "published": r.published,
            "category": r.primary_category,
            "explanation": explanations.get(r.paper_id),
        } for r in results]
        print(json.dumps(output, indent=2, ensure_ascii=False))
    elif args.format == "score-breakdown":
        print(colored("=== Reading Priority Queue ===", Colors.HEADER))
        print(f"Scoring: semantic={scorer.alpha}, citation={scorer.beta}, tag={scorer.gamma}, recency={scorer.delta}\n")
        for i, r in enumerate(results, 1):
            bar_len = int(r.score * 20)
            bar = "█" * bar_len + "░" * (20 - bar_len)
            color = Colors.OKGREEN if r.score > 0.5 else Colors.WARNING if r.score > 0.2 else Colors.OKBLUE
            print(f"{i:2}. [{bar}] {r.score:.2f}  {r.paper_id}")
            print(f"    {r.title[:70]}")
            print(f"    Sem:{r.semantic_score:.2f} Cit:{r.citation_score:.2f} Tag:{r.tag_score:.2f} New:{r.recency_score:.2f}")
            if r.published:
                print(f"    {r.published[:4]} | {r.primary_category or 'N/A'}")

            # Show LLM explanation if available
            if args.explain and explanations.get(r.paper_id):
                exp = explanations[r.paper_id]
                print()
                print(colored("    ╔═ 推荐理由 ═╗", Colors.OKBLUE))
                for line in exp.strip().split("\n"):
                    if line.strip():
                        print(f"    ║ {line}")
                print("    ╚═════════════╝")
            print()
    else:  # table format
        print(colored("=== Recommended Reading Queue ===", Colors.HEADER))
        print(f"{'#':>2}  {'Score':>5}  {'ID':<15}  {'Title'}")
        print("-" * 80)
        for i, r in enumerate(results, 1):
            score_str = colored(f"{r.score:.2f}", Colors.OKGREEN if r.score > 0.5 else Colors.WARNING)
            print(f"{i:2}.  {score_str}  {r.paper_id:<15}  {r.title[:45]}")

            # Show brief LLM explanation if available
            if args.explain and explanations.get(r.paper_id):
                exp = explanations[r.paper_id]
                if exp:
                    # Extract first meaningful line
                    lines = [l.strip() for l in exp.strip().split("\n") if l.strip() and not l.strip().startswith("#")]
                    brief = lines[0][:80] if lines else ""
                    if brief:
                        print(f"       {colored('→', Colors.OKBLUE)} {brief}...")

        print()
        print_info(f"Showing {len(results)} of {total} candidates")

    return 0
