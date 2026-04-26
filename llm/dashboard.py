"""
Research Dashboard: Aggregated view of research progress.
"""
from dataclasses import dataclass, field
from typing import List, Optional, Dict, Any
from datetime import datetime


@dataclass
class QuestionSummary:
    """Summary of a research question."""
    id: str
    question: str
    status: str
    priority: str
    hypotheses_count: int
    roadmap_id: str


@dataclass
class ExperimentSummary:
    """Summary of an experiment."""
    id: str
    name: str
    status: str
    milestone: str
    metrics_count: int


@dataclass
class PaperStats:
    """Paper reading statistics."""
    total_papers: int
    recent_papers: int  # Last 30 days
    by_year: Dict[str, int]
    by_tag: Dict[str, int]


@dataclass
class DashboardData:
    """Aggregated dashboard data."""
    generated_at: str = ""
    questions: List[QuestionSummary] = field(default_factory=list)
    experiments: List[ExperimentSummary] = field(default_factory=list)
    papers: Optional[PaperStats] = None
    summary: Dict[str, Any] = field(default_factory=dict)

    def __post_init__(self):
        if not self.generated_at:
            self.generated_at = datetime.now().isoformat()


class Dashboard:
    """Aggregate research progress data."""

    def __init__(self, db=None):
        self.db = db

    def collect(self, include_papers: bool = True) -> DashboardData:
        """Collect all dashboard data."""
        data = DashboardData()

        # Questions
        from llm.question_tracker import QuestionTracker
        tracker = QuestionTracker()
        questions = tracker.list_questions()
        for q in questions:
            data.questions.append(QuestionSummary(
                id=q.id,
                question=q.question,
                status=q.status,
                priority=q.priority,
                hypotheses_count=len(q.hypotheses),
                roadmap_id=q.roadmap_id or "",
            ))

        # Experiments
        from llm.experiment_tracker import ExperimentTracker
        exp_tracker = ExperimentTracker()
        exps = exp_tracker.list_experiments()
        for e in exps:
            data.experiments.append(ExperimentSummary(
                id=e.id,
                name=e.name,
                status=e.status,
                milestone=e.roadmap_milestone,
                metrics_count=len(e.metrics),
            ))

        # Papers
        if include_papers and self.db:
            try:
                self.db.init()
                data.papers = self._collect_paper_stats()
            except Exception:
                data.papers = None

        # Summary
        data.summary = self._build_summary(data)

        return data

    def _collect_paper_stats(self) -> PaperStats:
        """Collect paper statistics."""
        stats = PaperStats(total_papers=0, recent_papers=0, by_year={}, by_tag={})

        try:
            papers = self.db.list_papers(limit=10000)
            stats.total_papers = len(papers)

            from datetime import timedelta
            now = datetime.now()
            thirty_days_ago = (now - timedelta(days=30)).isoformat()

            for p in papers:
                # By year
                year = getattr(p, 'year', None) or 'unknown'
                stats.by_year[year] = stats.by_year.get(year, 0) + 1

                # Recent papers
                created = getattr(p, 'created_at', None) or ''
                if created and created > thirty_days_ago:
                    stats.recent_papers += 1

                # By tag
                tags = getattr(p, 'tags', []) or []
                for tag in tags:
                    stats.by_tag[tag] = stats.by_tag.get(tag, 0) + 1
        except Exception:
            pass

        return stats

    def _build_summary(self, data: DashboardData) -> Dict[str, Any]:
        """Build summary statistics."""
        questions_by_status = {}
        for q in data.questions:
            questions_by_status[q.status] = questions_by_status.get(q.status, 0) + 1

        experiments_by_status = {}
        for e in data.experiments:
            experiments_by_status[e.status] = experiments_by_status.get(e.status, 0) + 1

        return {
            "total_questions": len(data.questions),
            "questions_by_status": questions_by_status,
            "total_experiments": len(data.experiments),
            "experiments_by_status": experiments_by_status,
            "total_papers": data.papers.total_papers if data.papers else 0,
            "papers_this_month": data.papers.recent_papers if data.papers else 0,
        }

    def render_text(self, data: DashboardData) -> str:
        """Render dashboard as text."""
        lines = [
            "=" * 60,
            "📊 Research Dashboard",
            f"Generated: {data.generated_at[:19]}",
            "=" * 60,
            "",
        ]

        # Summary
        s = data.summary
        lines.append("## Summary")
        lines.append(f"  Questions: {s.get('total_questions', 0)}")
        lines.append(f"  Experiments: {s.get('total_experiments', 0)}")
        lines.append(f"  Papers: {s.get('total_papers', 0)} (this month: {s.get('papers_this_month', 0)})")
        lines.append("")

        # Questions
        lines.append("## Questions")
        if not data.questions:
            lines.append("  (none)")
        else:
            q_by_status = {}
            for q in data.questions:
                status = q.status or "unknown"
                q_by_status.setdefault(status, []).append(q)

            for status, questions in sorted(q_by_status.items()):
                icon = {"open": "📝", "in_progress": "🔄", "resolved": "✅"}.get(status, "❓")
                lines.append(f"  {icon} {status.upper()} ({len(questions)})")
                for q in questions[:3]:  # Show first 3
                    lines.append(f"    - [{q.id}] {q.question[:50]}...")
                if len(questions) > 3:
                    lines.append(f"    ... and {len(questions) - 3} more")
        lines.append("")

        # Experiments
        lines.append("## Experiments")
        if not data.experiments:
            lines.append("  (none)")
        else:
            e_by_status = {}
            for e in data.experiments:
                e_by_status.setdefault(e.status, []).append(e)

            for status, exps in sorted(e_by_status.items()):
                icon = {"running": "⚡", "completed": "✅", "failed": "❌"}.get(status, "❓")
                lines.append(f"  {icon} {status.upper()} ({len(exps)})")
                for e in exps[:3]:
                    metrics = f" [{e.metrics_count} metrics]" if e.metrics_count else ""
                    lines.append(f"    - [{e.id}] {e.name}{metrics}")
                if len(exps) > 3:
                    lines.append(f"    ... and {len(exps) - 3} more")
        lines.append("")

        # Papers
        if data.papers and data.papers.total_papers > 0:
            lines.append("## Papers")
            lines.append(f"  Total: {data.papers.total_papers}")
            lines.append(f"  This month: {data.papers.recent_papers}")
            if data.papers.by_year:
                lines.append("  By year:")
                for year in sorted(data.papers.by_year.keys(), reverse=True)[:5]:
                    lines.append(f"    {year}: {data.papers.by_year[year]}")
            if data.papers.by_tag:
                lines.append("  Top tags:")
                top_tags = sorted(data.papers.by_tag.items(), key=lambda x: -x[1])[:5]
                for tag, count in top_tags:
                    lines.append(f"    {tag}: {count}")
        lines.append("")
        lines.append("=" * 60)

        return '\n'.join(lines)

    def render_json(self, data: DashboardData) -> str:
        """Render dashboard as JSON."""
        import json
        return json.dumps({
            "generated_at": data.generated_at,
            "summary": data.summary,
            "questions": [
                {
                    "id": q.id,
                    "question": q.question,
                    "status": q.status,
                    "priority": q.priority,
                    "hypotheses_count": q.hypotheses_count,
                }
                for q in data.questions
            ],
            "experiments": [
                {
                    "id": e.id,
                    "name": e.name,
                    "status": e.status,
                    "milestone": e.milestone,
                    "metrics_count": e.metrics_count,
                }
                for e in data.experiments
            ],
            "papers": {
                "total": data.papers.total_papers if data.papers else 0,
                "this_month": data.papers.recent_papers if data.papers else 0,
                "by_year": data.papers.by_year if data.papers else {},
                "by_tag": data.papers.by_tag if data.papers else {},
            } if data.papers else None,
        }, ensure_ascii=False, indent=2)
