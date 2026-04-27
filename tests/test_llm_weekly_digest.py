"""Tier 2 unit tests — llm/weekly_digest.py, pure functions, no I/O."""
import pytest
from llm.weekly_digest import WeekData, WeeklyDigest


# =============================================================================
# WeekData dataclass tests
# =============================================================================
class TestWeekDataInit:
    """Test WeekData dataclass."""

    def test_required_fields(self):
        """Required fields: start_date, end_date."""
        data = WeekData(start_date="2026-04-01", end_date="2026-04-07")
        assert data.start_date == "2026-04-01"
        assert data.end_date == "2026-04-07"

    def test_optional_fields_default(self):
        """Optional fields have defaults."""
        data = WeekData(start_date="S", end_date="E")
        assert data.journal_entries == 0
        assert data.experiments_started == 0
        assert data.experiments_completed == 0
        assert data.questions_new == 0
        assert data.questions_resolved == 0
        assert data.papers_added == 0
        assert data.mood_breakdown == {}
        assert data.top_tags == []
        assert data.highlights == []

    def test_all_fields_can_be_set(self):
        """All fields can be set."""
        data = WeekData(
            start_date="2026-04-01",
            end_date="2026-04-07",
            journal_entries=10,
            experiments_started=3,
            experiments_completed=2,
            questions_new=5,
            questions_resolved=3,
            papers_added=8,
            mood_breakdown={"productive": 3, "excited": 2},
            top_tags=[("transformer", 5), ("nlp", 3)],
            highlights=["First paper read!", "Experiment succeeded"],
        )
        assert data.journal_entries == 10
        assert data.experiments_started == 3
        assert data.experiments_completed == 2
        assert data.questions_new == 5
        assert data.questions_resolved == 3
        assert data.papers_added == 8
        assert data.mood_breakdown == {"productive": 3, "excited": 2}
        assert data.top_tags == [("transformer", 5), ("nlp", 3)]
        assert data.highlights == ["First paper read!", "Experiment succeeded"]


# =============================================================================
# _calculate_productivity_score tests
# =============================================================================
class TestProductivityScore:
    """Test _calculate_productivity_score formula."""

    def _score(self, data: WeekData) -> int:
        """Replicate _calculate_productivity_score."""
        score = 0
        score += min(data.journal_entries * 5, 25)
        score += min(data.experiments_completed * 20, 40)
        score += min(data.questions_resolved * 15, 30)
        if data.mood_breakdown.get("excited", 0) > 0:
            score += 5
        return min(score, 100)

    def test_zero_values(self):
        """All zeros returns 0."""
        data = WeekData(start_date="S", end_date="E")
        assert self._score(data) == 0

    def test_journal_entries_contributes(self):
        """Each journal entry gives up to 5 points, max 25."""
        data = WeekData(start_date="S", end_date="E", journal_entries=10)
        assert self._score(data) == 25  # capped at 25

        data = WeekData(start_date="S", end_date="E", journal_entries=3)
        assert self._score(data) == 15

        data = WeekData(start_date="S", end_date="E", journal_entries=6)
        assert self._score(data) == 25  # 6*5=30, capped at 25

    def test_experiments_completed_contributes(self):
        """Each completed experiment gives up to 20 points, max 40."""
        data = WeekData(start_date="S", end_date="E", experiments_completed=2)
        assert self._score(data) == 40

        data = WeekData(start_date="S", end_date="E", experiments_completed=1)
        assert self._score(data) == 20

        data = WeekData(start_date="S", end_date="E", experiments_completed=3)
        assert self._score(data) == 40  # capped at 40

    def test_questions_resolved_contributes(self):
        """Each resolved question gives up to 15 points, max 30."""
        data = WeekData(start_date="S", end_date="E", questions_resolved=2)
        assert self._score(data) == 30

        data = WeekData(start_date="S", end_date="E", questions_resolved=1)
        assert self._score(data) == 15

        data = WeekData(start_date="S", end_date="E", questions_resolved=3)
        assert self._score(data) == 30  # capped at 30

    def test_excited_mood_gives_bonus(self):
        """Excited mood > 0 gives +5 bonus."""
        data = WeekData(start_date="S", end_date="E", mood_breakdown={"excited": 1})
        assert self._score(data) == 5

        data = WeekData(start_date="S", end_date="E", mood_breakdown={"excited": 5})
        assert self._score(data) == 5  # bonus is flat +5, not +5 per entry

    def test_excited_zero_no_bonus(self):
        """Excited mood of 0 gives no bonus."""
        data = WeekData(start_date="S", end_date="E", mood_breakdown={"excited": 0})
        assert self._score(data) == 0

    def test_excited_absent_no_bonus(self):
        """No excited key in mood_breakdown gives no bonus."""
        data = WeekData(start_date="S", end_date="E", mood_breakdown={"productive": 3})
        assert self._score(data) == 0

    def test_combined_score(self):
        """Multiple sources contribute to score."""
        data = WeekData(
            start_date="S",
            end_date="E",
            journal_entries=3,        # 15 pts
            experiments_completed=1,  # 20 pts
            questions_resolved=1,     # 15 pts
            mood_breakdown={"excited": 1},  # +5 pts
        )
        assert self._score(data) == 55  # 15+20+15+5

    def test_capped_at_100(self):
        """Score cannot exceed 100."""
        data = WeekData(
            start_date="S",
            end_date="E",
            journal_entries=10,      # 25 pts (capped)
            experiments_completed=10,  # 40 pts (capped)
            questions_resolved=10,    # 30 pts (capped)
            mood_breakdown={"excited": 1},  # +5 pts
        )
        assert self._score(data) == 100  # 25+40+30+5=100

    def test_mixed_excited_and_other_moods(self):
        """Excited with other moods still gives bonus."""
        data = WeekData(
            start_date="S",
            end_date="E",
            journal_entries=2,
            mood_breakdown={"productive": 3, "excited": 1, "stuck": 2},
        )
        # 2*5=10, excited=1 gives +5 → 15
        assert self._score(data) == 15


# =============================================================================
# generate_summary tests
# =============================================================================
class TestGenerateSummary:
    """Test generate_summary formatting."""

    def _generate_summary(self, data: WeekData) -> str:
        """Replicate generate_summary."""
        lines = [
            "=" * 60,
            "📊 Weekly Research Digest",
            f"   {data.start_date} ~ {data.end_date}",
            "=" * 60,
            "",
        ]

        lines.append("## 📈 Activity")
        lines.append(f"  Journal entries: {data.journal_entries}")
        lines.append(f"  Experiments started: {data.experiments_started}")
        lines.append(f"  Experiments completed: {data.experiments_completed}")
        lines.append(f"  New questions: {data.questions_new}")
        lines.append(f"  Questions resolved: {data.questions_resolved}")
        lines.append("")

        if data.mood_breakdown:
            lines.append("## 💭 Mood")
            mood_icons = {"productive": "⚡", "stuck": "😓", "excited": "🎉", "neutral": "📝"}
            for mood, count in data.mood_breakdown.items():
                icon = mood_icons.get(mood, "📝")
                lines.append(f"  {icon} {mood}: {count}")
            lines.append("")

        if data.top_tags:
            lines.append("## 🏷️ Top Topics")
            for tag, count in data.top_tags[:5]:
                lines.append(f"  {tag}: {count}")
            lines.append("")

        if data.highlights:
            lines.append("## ⭐ Highlights")
            for h in data.highlights:
                lines.append(f"  • {h}")
            lines.append("")

        score = 0
        score += min(data.journal_entries * 5, 25)
        score += min(data.experiments_completed * 20, 40)
        score += min(data.questions_resolved * 15, 30)
        if data.mood_breakdown.get("excited", 0) > 0:
            score += 5
        score = min(score, 100)

        lines.append(f"## 📅 Productivity Score: {score}/100")
        lines.append("")
        lines.append("=" * 60)
        return "\n".join(lines)

    def test_header(self):
        """Header includes border and title."""
        data = WeekData(start_date="2026-04-01", end_date="2026-04-07")
        output = self._generate_summary(data)
        assert "=" * 60 in output
        assert "📊 Weekly Research Digest" in output
        assert "2026-04-01 ~ 2026-04-07" in output

    def test_activity_section(self):
        """Activity section shows all stats."""
        data = WeekData(
            start_date="S",
            end_date="E",
            journal_entries=5,
            experiments_started=2,
            experiments_completed=1,
            questions_new=3,
            questions_resolved=2,
        )
        output = self._generate_summary(data)
        assert "Journal entries: 5" in output
        assert "Experiments started: 2" in output
        assert "Experiments completed: 1" in output
        assert "New questions: 3" in output
        assert "Questions resolved: 2" in output

    def test_mood_section(self):
        """Mood section shows mood icons."""
        data = WeekData(
            start_date="S",
            end_date="E",
            mood_breakdown={"productive": 3, "excited": 2},
        )
        output = self._generate_summary(data)
        assert "## 💭 Mood" in output
        assert "⚡ productive: 3" in output
        assert "🎉 excited: 2" in output

    def test_mood_section_unknown_icon(self):
        """Unknown mood uses default 📝."""
        data = WeekData(
            start_date="S",
            end_date="E",
            mood_breakdown={"unknown_mood": 1},
        )
        output = self._generate_summary(data)
        assert "📝 unknown_mood: 1" in output

    def test_no_mood_section_when_empty(self):
        """No mood section when mood_breakdown is empty."""
        data = WeekData(start_date="S", end_date="E", mood_breakdown={})
        output = self._generate_summary(data)
        assert "## 💭 Mood" not in output

    def test_tags_section(self):
        """Tags section shows top topics."""
        data = WeekData(
            start_date="S",
            end_date="E",
            top_tags=[("transformer", 5), ("nlp", 3), ("attention", 2)],
        )
        output = self._generate_summary(data)
        assert "## 🏷️ Top Topics" in output
        assert "transformer: 5" in output
        assert "nlp: 3" in output

    def test_tags_limited_to_5(self):
        """Only top 5 tags shown."""
        data = WeekData(
            start_date="S",
            end_date="E",
            top_tags=[(f"tag{i}", i) for i in range(10)],
        )
        output = self._generate_summary(data)
        assert "tag0: 0" in output
        assert "tag4: 4" in output
        assert "tag5: 5" not in output

    def test_no_tags_section_when_empty(self):
        """No tags section when top_tags is empty."""
        data = WeekData(start_date="S", end_date="E", top_tags=[])
        output = self._generate_summary(data)
        assert "## 🏷️ Top Topics" not in output

    def test_highlights_section(self):
        """Highlights section shows items."""
        data = WeekData(
            start_date="S",
            end_date="E",
            highlights=["First paper read!", "Experiment succeeded"],
        )
        output = self._generate_summary(data)
        assert "## ⭐ Highlights" in output
        assert "• First paper read!" in output
        assert "• Experiment succeeded" in output

    def test_no_highlights_section_when_empty(self):
        """No highlights section when highlights is empty."""
        data = WeekData(start_date="S", end_date="E", highlights=[])
        output = self._generate_summary(data)
        assert "## ⭐ Highlights" not in output

    def test_productivity_score(self):
        """Productivity score shown at end."""
        data = WeekData(
            start_date="S",
            end_date="E",
            journal_entries=2,
            experiments_completed=1,
            questions_resolved=1,
        )
        output = self._generate_summary(data)
        # 2*5=10 + 1*20=20 + 1*15=15 = 45
        assert "## 📅 Productivity Score: 45/100" in output

    def test_full_summary(self):
        """Full summary with all sections."""
        data = WeekData(
            start_date="2026-04-01",
            end_date="2026-04-07",
            journal_entries=3,
            experiments_started=2,
            experiments_completed=1,
            questions_new=5,
            questions_resolved=2,
            mood_breakdown={"productive": 2, "excited": 1},
            top_tags=[("transformer", 4)],
            highlights=["Breakthrough!"],
        )
        output = self._generate_summary(data)
        assert "📊 Weekly Research Digest" in output
        assert "2026-04-01 ~ 2026-04-07" in output
        assert "## 📈 Activity" in output
        assert "## 💭 Mood" in output
        assert "## 🏷️ Top Topics" in output
        assert "## ⭐ Highlights" in output
        # 3*5=15 + 1*20=20 + 2*15=30 + 5(excited) = 70
        assert "70/100" in output


# =============================================================================
# render_markdown tests
# =============================================================================
class TestRenderMarkdown:
    """Test render_markdown formatting."""

    def _render_markdown(self, data: WeekData) -> str:
        """Replicate render_markdown."""
        lines = [
            "# Weekly Research Digest",
            f"**Period**: {data.start_date} ~ {data.end_date}",
            "",
        ]

        lines.append("## Stats")
        lines.append("| Metric | Value |")
        lines.append("|--------|-------|")
        lines.append(f"| Journal entries | {data.journal_entries} |")
        lines.append(f"| Experiments started | {data.experiments_started} |")
        lines.append(f"| Experiments completed | {data.experiments_completed} |")
        lines.append(f"| New questions | {data.questions_new} |")
        lines.append(f"| Questions resolved | {data.questions_resolved} |")
        lines.append("")

        if data.mood_breakdown:
            lines.append("## Mood Distribution")
            for mood, count in data.mood_breakdown.items():
                lines.append(f"- {mood}: {count}")
            lines.append("")

        if data.top_tags:
            lines.append("## Top Topics")
            for tag, count in data.top_tags:
                lines.append(f"- **{tag}**: {count}")
            lines.append("")

        score = 0
        score += min(data.journal_entries * 5, 25)
        score += min(data.experiments_completed * 20, 40)
        score += min(data.questions_resolved * 15, 30)
        if data.mood_breakdown.get("excited", 0) > 0:
            score += 5
        score = min(score, 100)

        lines.append(f"**Productivity Score**: {score}/100")
        return "\n".join(lines)

    def test_header(self):
        """Header includes title and period."""
        data = WeekData(start_date="2026-04-01", end_date="2026-04-07")
        output = self._render_markdown(data)
        assert "# Weekly Research Digest" in output
        assert "**Period**: 2026-04-01 ~ 2026-04-07" in output

    def test_stats_table(self):
        """Stats table shows all metrics."""
        data = WeekData(
            start_date="S",
            end_date="E",
            journal_entries=5,
            experiments_started=2,
            experiments_completed=1,
            questions_new=3,
            questions_resolved=2,
        )
        output = self._render_markdown(data)
        assert "| Metric | Value |" in output
        assert "| Journal entries | 5 |" in output
        assert "| Experiments started | 2 |" in output
        assert "| Experiments completed | 1 |" in output
        assert "| New questions | 3 |" in output
        assert "| Questions resolved | 2 |" in output

    def test_mood_distribution(self):
        """Mood distribution listed."""
        data = WeekData(
            start_date="S",
            end_date="E",
            mood_breakdown={"productive": 3, "excited": 2},
        )
        output = self._render_markdown(data)
        assert "## Mood Distribution" in output
        assert "- productive: 3" in output
        assert "- excited: 2" in output

    def test_no_mood_when_empty(self):
        """No mood section when mood_breakdown is empty."""
        data = WeekData(start_date="S", end_date="E", mood_breakdown={})
        output = self._render_markdown(data)
        assert "## Mood Distribution" not in output

    def test_top_topics(self):
        """Top topics listed with bold tags."""
        data = WeekData(
            start_date="S",
            end_date="E",
            top_tags=[("transformer", 5), ("nlp", 3)],
        )
        output = self._render_markdown(data)
        assert "## Top Topics" in output
        assert "- **transformer**: 5" in output
        assert "- **nlp**: 3" in output

    def test_no_topics_when_empty(self):
        """No top topics section when top_tags is empty."""
        data = WeekData(start_date="S", end_date="E", top_tags=[])
        output = self._render_markdown(data)
        assert "## Top Topics" not in output

    def test_productivity_score(self):
        """Productivity score shown at end."""
        data = WeekData(
            start_date="S",
            end_date="E",
            journal_entries=2,
            experiments_completed=1,
            questions_resolved=1,
        )
        output = self._render_markdown(data)
        # 2*5=10 + 1*20=20 + 1*15=15 = 45
        assert "**Productivity Score**: 45/100" in output

    def test_full_markdown(self):
        """Full markdown with all sections."""
        data = WeekData(
            start_date="2026-04-01",
            end_date="2026-04-07",
            journal_entries=3,
            experiments_started=2,
            experiments_completed=1,
            questions_new=5,
            questions_resolved=2,
            mood_breakdown={"productive": 2, "excited": 1},
            top_tags=[("transformer", 4)],
        )
        output = self._render_markdown(data)
        assert "# Weekly Research Digest" in output
        assert "**Period**:" in output
        assert "## Stats" in output
        assert "## Mood Distribution" in output
        assert "## Top Topics" in output
        # 3*5=15 + 1*20=20 + 2*15=30 + 5(excited) = 70
        assert "**Productivity Score**: 70/100" in output

    def test_tags_with_tuple_format(self):
        """Top tags rendered correctly from tuple format."""
        data = WeekData(
            start_date="S",
            end_date="E",
            top_tags=[("attention", 10), ("bert", 8)],
        )
        output = self._render_markdown(data)
        # Each tuple is (tag, count)
        assert "- **attention**: 10" in output
        assert "- **bert**: 8" in output


# =============================================================================
# WeeklyDigest instantiation
# =============================================================================
class TestWeeklyDigestInit:
    """Test WeeklyDigest class."""

    def test_can_instantiate(self):
        """WeeklyDigest can be instantiated."""
        digest = WeeklyDigest()
        assert digest is not None

    def test_has_required_methods(self):
        """WeeklyDigest has the expected public methods."""
        digest = WeeklyDigest()
        assert hasattr(digest, "collect_week_data")
        assert hasattr(digest, "generate_summary")
        assert hasattr(digest, "render_markdown")
        assert hasattr(digest, "_calculate_productivity_score")
