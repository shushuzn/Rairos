"""
Weekly Digest: Generate weekly research summaries.
"""
from dataclasses import dataclass, field
from datetime import datetime, timedelta
from typing import List, Optional


@dataclass
class WeekData:
    """Data for a week."""
    start_date: str
    end_date: str
    journal_entries: int = 0
    experiments_started: int = 0
    experiments_completed: int = 0
    questions_new: int = 0
    questions_resolved: int = 0
    papers_added: int = 0
    mood_breakdown: dict = field(default_factory=dict)
    top_tags: list = field(default_factory=list)
    highlights: List[str] = field(default_factory=list)


class WeeklyDigest:
    """Generate weekly research summaries."""

    def __init__(self):
        pass

    def collect_week_data(self, days: int = 7) -> WeekData:
        """Collect data for the past N days."""
        now = datetime.now()
        start = now - timedelta(days=days)
        start_str = start.strftime("%Y-%m-%d")
        end_str = now.strftime("%Y-%m-%d")

        week_data = WeekData(start_date=start_str, end_date=end_str)

        # Journal entries
        from llm.journal import Journal
        journal = Journal()
        entries = journal.list_entries(days=days)
        week_data.journal_entries = len(entries)

        # Mood breakdown
        mood_count = {}
        tag_count = {}
        for e in entries:
            if e.mood:
                mood_count[e.mood] = mood_count.get(e.mood, 0) + 1
            for tag in e.tags:
                tag_count[tag] = tag_count.get(tag, 0) + 1
        week_data.mood_breakdown = mood_count
        week_data.top_tags = sorted(tag_count.items(), key=lambda x: -x[1])[:5]

        # Experiments
        from llm.experiment_tracker import ExperimentTracker
        exp_tracker = ExperimentTracker()
        exps = exp_tracker.list_experiments()
        for e in exps:
            if e.created_at >= start.isoformat():
                week_data.experiments_started += 1
            if e.status == "completed" and e.completed_at >= start.isoformat():
                week_data.experiments_completed += 1

        # Questions
        from llm.question_tracker import QuestionTracker
        q_tracker = QuestionTracker()
        questions = q_tracker.list_questions()
        for q in questions:
            if q.created_at >= start.isoformat():
                week_data.questions_new += 1
            if q.status == "resolved" and q.updated_at and q.updated_at >= start.isoformat():
                week_data.questions_resolved += 1

        return week_data

    def generate_summary(self, data: WeekData) -> str:
        """Generate a text summary."""
        lines = [
            "=" * 60,
            f"📊 Weekly Research Digest",
            f"   {data.start_date} ~ {data.end_date}",
            "=" * 60,
            "",
        ]

        # Activity stats
        lines.append("## 📈 Activity")
        lines.append(f"  Journal entries: {data.journal_entries}")
        lines.append(f"  Experiments started: {data.experiments_started}")
        lines.append(f"  Experiments completed: {data.experiments_completed}")
        lines.append(f"  New questions: {data.questions_new}")
        lines.append(f"  Questions resolved: {data.questions_resolved}")
        lines.append("")

        # Mood
        if data.mood_breakdown:
            lines.append("## 💭 Mood")
            mood_icons = {"productive": "⚡", "stuck": "😓", "excited": "🎉", "neutral": "📝"}
            for mood, count in data.mood_breakdown.items():
                icon = mood_icons.get(mood, "📝")
                lines.append(f"  {icon} {mood}: {count}")
            lines.append("")

        # Tags
        if data.top_tags:
            lines.append("## 🏷️ Top Topics")
            for tag, count in data.top_tags[:5]:
                lines.append(f"  {tag}: {count}")
            lines.append("")

        # Highlights
        if data.highlights:
            lines.append("## ⭐ Highlights")
            for h in data.highlights:
                lines.append(f"  • {h}")
            lines.append("")

        # Week's productivity score
        score = self._calculate_productivity_score(data)
        lines.append(f"## 📅 Productivity Score: {score}/100")
        lines.append("")
        lines.append("=" * 60)

        return '\n'.join(lines)

    def _calculate_productivity_score(self, data: WeekData) -> int:
        """Calculate a simple productivity score."""
        score = 0
        score += min(data.journal_entries * 5, 25)  # Max 25 pts
        score += min(data.experiments_completed * 20, 40)  # Max 40 pts
        score += min(data.questions_resolved * 15, 30)  # Max 30 pts
        if data.mood_breakdown.get("excited", 0) > 0:
            score += 5  # Bonus for excitement
        return min(score, 100)

    def render_markdown(self, data: WeekData) -> str:
        """Render as Markdown."""
        lines = [
            f"# Weekly Research Digest",
            f"**Period**: {data.start_date} ~ {data.end_date}",
            "",
        ]

        # Stats table
        lines.append("## Stats")
        lines.append("| Metric | Value |")
        lines.append("|--------|-------|")
        lines.append(f"| Journal entries | {data.journal_entries} |")
        lines.append(f"| Experiments started | {data.experiments_started} |")
        lines.append(f"| Experiments completed | {data.experiments_completed} |")
        lines.append(f"| New questions | {data.questions_new} |")
        lines.append(f"| Questions resolved | {data.questions_resolved} |")
        lines.append("")

        # Mood
        if data.mood_breakdown:
            lines.append("## Mood Distribution")
            for mood, count in data.mood_breakdown.items():
                lines.append(f"- {mood}: {count}")
            lines.append("")

        # Tags
        if data.top_tags:
            lines.append("## Top Topics")
            for tag, count in data.top_tags:
                lines.append(f"- **{tag}**: {count}")
            lines.append("")

        # Score
        score = self._calculate_productivity_score(data)
        lines.append(f"**Productivity Score**: {score}/100")

        return '\n'.join(lines)
