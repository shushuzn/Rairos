"""
Research Dashboard: Aggregated view of research progress.
"""
from dataclasses import dataclass, field
from typing import List, Optional, Dict, Tuple, Any
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
class HotPaper:
    """A paper with high citation velocity."""
    paper_id: str
    title: str
    year: int
    velocity: float
    forward_cites: int


@dataclass
class TrendKeyword:
    """A trending keyword from the corpus."""
    keyword: str
    direction: str  # rising / falling / emerging / stable
    paper_count: int
    growth: str  # e.g. "+25%" or "-10%"


@dataclass
class GapPreferenceStats:
    """Summary of user's gap_type and keyword preference profile."""
    total_events: int
    preferred_types: List[Tuple[str, float]]
    disliked_types: List[Tuple[str, float]]
    preferred_keywords: List[Tuple[str, float]] = field(default_factory=list)


@dataclass
class DashboardData:
    """Aggregated dashboard data."""
    generated_at: str = ""
    questions: List[QuestionSummary] = field(default_factory=list)
    experiments: List[ExperimentSummary] = field(default_factory=list)
    papers: Optional[PaperStats] = None
    hot_papers: List[HotPaper] = field(default_factory=list)
    trends: List[TrendKeyword] = field(default_factory=list)
    gap_preferences: Optional[GapPreferenceStats] = None
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

        # Papers + extended analytics
        if include_papers and self.db:
            try:
                self.db.init()
                data.papers = self._collect_paper_stats()
                data.hot_papers = self._collect_hot_papers()
                data.trends = self._collect_trends()
            except Exception:
                # Paper analytics are optional — dashboard still renders without them.
                data.papers = None

        # Gap type preferences
        try:
            from llm.insight_evolution import EvolutionTracker
            tracker = EvolutionTracker()
            profile = tracker.get_profile()
            if profile and profile.total_events > 0:
                prefs = profile.gap_type_preferences or {}
                kw_prefs = profile.keyword_preferences or {}
                preferred = [(gt, s) for gt, s in prefs.items() if s > 0.1]
                disliked = [(gt, s) for gt, s in prefs.items() if s < -0.05]
                kw_preferred = [(kw, s) for kw, s in kw_prefs.items() if s > 0.05]
                data.gap_preferences = GapPreferenceStats(
                    total_events=profile.total_events,
                    preferred_types=sorted(preferred, key=lambda x: x[1], reverse=True),
                    disliked_types=sorted(disliked, key=lambda x: x[1]),
                    preferred_keywords=sorted(kw_preferred, key=lambda x: x[1], reverse=True)[:5],
                )
        except Exception:
            # Gap preferences are optional — dashboard still renders without them.
            pass

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
            # Paper stats are optional — return partial stats without crashing.
            pass

        return stats

    def _collect_hot_papers(self) -> List[HotPaper]:
        """Collect top papers by citation velocity (from influence logic)."""
        hot = []
        try:
            current_year = 2026
            cur = self.db.conn.execute("""
                SELECT p.id, p.title, p.published, COUNT(c.id) AS forward_cites
                FROM papers p
                LEFT JOIN citations c ON c.target_id = p.id
                GROUP BY p.id
                HAVING forward_cites >= 1
            """)
            rows = cur.fetchall()
            scored = []
            for row in rows:
                paper_id, title, published, fwd = row[0], row[1] or "", row[2] or "", row[3]
                try:
                    year = int(published[:4])
                except (ValueError, TypeError):
                    continue
                if year < 2000 or year > current_year:
                    continue
                age = current_year - year + 1
                velocity = fwd / age
                scored.append((velocity, fwd, paper_id, title, year))

            scored.sort(key=lambda x: x[0], reverse=True)
            for velocity, fwd, pid, title, year in scored[:10]:
                hot.append(HotPaper(
                    paper_id=pid,
                    title=title[:60] + "…" if len(title) > 60 else title,
                    year=year,
                    velocity=round(velocity, 1),
                    forward_cites=fwd,
                ))
        except Exception:
            # Hot papers are optional — return empty list without crashing.
            pass
        return hot

    def _collect_trends(self) -> List[TrendKeyword]:
        """Collect keyword trends from paper corpus using TrendAnalyzer."""
        trends = []
        try:
            from llm.trend_analyzer import TrendAnalyzer
            analyzer = TrendAnalyzer(db=self.db)
            result = analyzer.analyze("", min_papers=3)
            direction_map = {
                "rising": "📈 rising",
                "falling": "📉 falling",
                "emerging": "✨ emerging",
                "stable": "➡️ stable",
            }
            for t in result.rising_trends[:5]:
                trends.append(TrendKeyword(
                    keyword=t.keyword,
                    direction=direction_map.get(t.direction.value, t.direction.value),
                    paper_count=t.current_year_count,
                    growth=f"+{max(int(t.growth_rate), 0)}%" if t.growth_rate > 0 else f"{int(t.growth_rate)}%",
                ))
            for t in result.falling_trends[:3]:
                trends.append(TrendKeyword(
                    keyword=t.keyword,
                    direction=direction_map.get(t.direction.value, t.direction.value),
                    paper_count=t.current_year_count,
                    growth=f"{int(t.growth_rate)}%",
                ))
            for t in result.emerging_trends[:3]:
                trends.append(TrendKeyword(
                    keyword=t.keyword,
                    direction=direction_map.get(t.direction.value, t.direction.value),
                    paper_count=t.current_year_count,
                    growth=f"+{max(int(t.growth_rate), 0)}%" if t.growth_rate > 0 else f"{int(t.growth_rate)}%",
                ))
        except Exception:
            # Trend collection is optional — return empty list without crashing.
            pass
        return trends

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
            "hot_papers_count": len(data.hot_papers),
            "trends_count": len(data.trends),
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
        if s.get('hot_papers_count', 0) > 0:
            lines.append(f"  Hot Papers: {s['hot_papers_count']} (citation velocity > 0)")
        if s.get('trends_count', 0) > 0:
            lines.append(f"  Trends: {s['trends_count']} keywords tracked")

        # Gap Type Preferences
        gp = data.gap_preferences
        if gp:
            lines.append("")
            lines.append("## 🧠 Research Gap Preferences")
            lines.append(f"  Based on {gp.total_events} exploration events")
            if gp.preferred_keywords:
                lines.append("  🔑 Preferred keywords:")
                for kw, score in gp.preferred_keywords[:5]:
                    bar = "█" * min(int(score * 5), 10)
                    lines.append(f"    {kw}: {score:+.2f} {bar}")
            if gp.preferred_types:
                lines.append("  🟢 Preferred types:")
                for gt, score in gp.preferred_types[:5]:
                    bar = "█" * min(int(score * 5), 10)
                    lines.append(f"    {gt}: {score:+.2f} {bar}")
            if gp.disliked_types:
                lines.append("  🔴 Avoided types:")
                for gt, score in gp.disliked_types[:3]:
                    bar = "█" * min(int(abs(score) * 5), 10)
                    lines.append(f"    {gt}: {score:+.2f} {bar}")
            if not gp.preferred_types and not gp.preferred_keywords and not gp.disliked_types:
                lines.append("  (no strong preferences yet — keep exploring!)")

        lines.append("")

        # Hot Papers
        if data.hot_papers:
            lines.append("## 🔥 Hot Papers (by Citation Velocity)")
            for i, p in enumerate(data.hot_papers[:5], 1):
                bar = "█" * min(int(p.velocity), 10)
                lines.append(
                    f"  {i}. {p.velocity:.1f}/y  {bar}  {p.title} ({p.year})"
                )
            if len(data.hot_papers) > 5:
                lines.append(f"  ... and {len(data.hot_papers) - 5} more")
            lines.append("")

        # Research Trends
        if data.trends:
            lines.append("## 📈 Research Trends")
            for t in data.trends[:10]:
                lines.append(
                    f"  {t.direction}  {t.keyword}  ({t.paper_count} papers, {t.growth})"
                )
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
                for q in questions[:3]:
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
            "hot_papers": [
                {
                    "paper_id": p.paper_id,
                    "title": p.title,
                    "year": p.year,
                    "velocity": p.velocity,
                    "forward_cites": p.forward_cites,
                }
                for p in data.hot_papers
            ],
            "trends": [
                {
                    "keyword": t.keyword,
                    "direction": t.direction,
                    "paper_count": t.paper_count,
                    "growth": t.growth,
                }
                for t in data.trends
            ],
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
            "gap_preferences": {
                "total_events": data.gap_preferences.total_events if data.gap_preferences else 0,
                "preferred_keywords": [
                    {"keyword": kw, "score": float(score)}
                    for kw, score in (data.gap_preferences.preferred_keywords if data.gap_preferences else [])
                ],
                "preferred_types": [
                    {"gap_type": gt, "score": float(score)}
                    for gt, score in (data.gap_preferences.preferred_types if data.gap_preferences else [])
                ],
                "disliked_types": [
                    {"gap_type": gt, "score": float(score)}
                    for gt, score in (data.gap_preferences.disliked_types if data.gap_preferences else [])
                ],
            } if data.gap_preferences else None,
        }, ensure_ascii=False, indent=2)
