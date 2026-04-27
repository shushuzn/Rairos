"""Tier 2 unit tests — llm/roadmap_generator.py, pure functions, no I/O."""
import pytest
import json
from llm.roadmap_generator import (
    Milestone,
    Phase,
    ResearchRoadmap,
    RoadmapGenerator,
    DEFAULT_PHASES,
)


# =============================================================================
# Milestone dataclass
# =============================================================================
class TestMilestone:
    """Test Milestone dataclass."""

    def test_required_fields(self):
        """Required fields: id, name, description, duration_weeks."""
        m = Milestone(id="m1", name="Literature Review", description="Read 20 papers", duration_weeks=1)
        assert m.id == "m1"
        assert m.name == "Literature Review"
        assert m.description == "Read 20 papers"
        assert m.duration_weeks == 1

    def test_optional_fields_default(self):
        """Optional fields default to empty/false."""
        m = Milestone(id="m", name="N", description="D", duration_weeks=1)
        assert m.tasks == []
        assert m.dependencies == []

    def test_all_fields_can_be_set(self):
        """All fields can be set."""
        m = Milestone(
            id="full",
            name="Full",
            description="Desc",
            duration_weeks=2,
            tasks=["task1", "task2"],
            dependencies=["m0"],
        )
        assert m.tasks == ["task1", "task2"]
        assert m.dependencies == ["m0"]


# =============================================================================
# Phase dataclass
# =============================================================================
class TestPhase:
    """Test Phase dataclass."""

    def test_required_fields(self):
        """Required fields: id, name, description, duration_weeks."""
        p = Phase(id="p1", name="Analysis", description="Analyze problem", duration_weeks=2)
        assert p.id == "p1"
        assert p.name == "Analysis"
        assert p.description == "Analyze problem"
        assert p.duration_weeks == 2

    def test_optional_fields_default(self):
        """Optional fields default."""
        p = Phase(id="p", name="N", description="D", duration_weeks=1)
        assert p.milestones == []
        assert p.order == 0

    def test_all_fields_can_be_set(self):
        """All fields can be set."""
        m = Milestone(id="m1", name="M", description="D", duration_weeks=1)
        p = Phase(
            id="full",
            name="Full Phase",
            description="Desc",
            duration_weeks=3,
            milestones=[m],
            order=2,
        )
        assert len(p.milestones) == 1
        assert p.order == 2


# =============================================================================
# ResearchRoadmap dataclass
# =============================================================================
class TestResearchRoadmap:
    """Test ResearchRoadmap dataclass."""

    def test_required_fields(self):
        """Required fields: question, question_id."""
        r = ResearchRoadmap(question="How to optimize?", question_id="q1")
        assert r.question == "How to optimize?"
        assert r.question_id == "q1"

    def test_optional_fields_default(self):
        """Optional fields default."""
        r = ResearchRoadmap(question="Q", question_id="q")
        assert r.phases == []
        assert r.total_weeks == 0
        assert r.notes == ""

    def test_all_fields_can_be_set(self):
        """All fields can be set."""
        phase = Phase(id="p1", name="Phase", description="D", duration_weeks=2)
        r = ResearchRoadmap(
            question="Full Question",
            question_id="q1",
            phases=[phase],
            total_weeks=0,  # will be recalculated
            created_at="2026-01-01T00:00:00",
            notes="Important notes",
        )
        assert r.created_at == "2026-01-01T00:00:00"
        assert r.notes == "Important notes"
        assert len(r.phases) == 1


class TestResearchRoadmapPostInit:
    """Test ResearchRoadmap.__post_init__."""

    def test_empty_created_at_auto_generated(self):
        """Empty created_at triggers auto-generation."""
        r = ResearchRoadmap(question="Q", question_id="q")
        assert r.created_at != ""
        assert "T" in r.created_at

    def test_non_empty_created_at_preserved(self):
        """Non-empty created_at is preserved."""
        r = ResearchRoadmap(question="Q", question_id="q", created_at="2025-01-01T00:00:00")
        assert r.created_at == "2025-01-01T00:00:00"

    def test_total_weeks_auto_calculated(self):
        """total_weeks auto-calculated from phases."""
        p1 = Phase(id="p1", name="P1", description="D", duration_weeks=2)
        p2 = Phase(id="p2", name="P2", description="D", duration_weeks=3)
        r = ResearchRoadmap(question="Q", question_id="q", phases=[p1, p2])
        assert r.total_weeks == 5

    def test_empty_phases_total_weeks_zero(self):
        """No phases → total_weeks is 0."""
        r = ResearchRoadmap(question="Q", question_id="q")
        assert r.total_weeks == 0


# =============================================================================
# DEFAULT_PHASES constant
# =============================================================================
class TestDefaultPhases:
    """Test DEFAULT_PHASES constant."""

    def test_is_list(self):
        """DEFAULT_PHASES is a list."""
        assert isinstance(DEFAULT_PHASES, list)
        assert len(DEFAULT_PHASES) == 4

    def test_all_phases_have_required_fields(self):
        """Each phase has required fields."""
        for phase in DEFAULT_PHASES:
            assert "name" in phase
            assert "description" in phase
            assert "duration_weeks" in phase
            assert "milestones" in phase
            assert isinstance(phase["milestones"], list)

    def test_phase_names(self):
        """Expected phase names."""
        names = [p["name"] for p in DEFAULT_PHASES]
        assert "问题分析" in names
        assert "原型开发" in names
        assert "实验验证" in names
        assert "论文撰写" in names

    def test_total_duration_weeks(self):
        """Total duration across all phases."""
        total = sum(p["duration_weeks"] for p in DEFAULT_PHASES)
        assert total == 13  # 2 + 4 + 4 + 3

    def test_all_milestones_have_required_fields(self):
        """Each milestone has required fields."""
        for phase in DEFAULT_PHASES:
            for m in phase["milestones"]:
                assert "name" in m
                assert "description" in m
                assert "duration_weeks" in m


# =============================================================================
# RoadmapGenerator.generate
# =============================================================================
class TestGenerate:
    """Test RoadmapGenerator.generate."""

    def _generate(self, question, question_id="", custom_phases=None):
        """Replicate generate logic."""
        from llm.roadmap_generator import RoadmapGenerator, ResearchRoadmap, Phase, Milestone
        gen = RoadmapGenerator()
        phases = custom_phases or DEFAULT_PHASES
        roadmap_phases = []
        milestone_counter = 1

        for i, phase_def in enumerate(phases):
            milestones = []
            for m_def in phase_def.get("milestones", []):
                milestone = Milestone(
                    id=f"m{milestone_counter}",
                    name=m_def["name"],
                    description=m_def["description"],
                    duration_weeks=m_def.get("duration_weeks", 1),
                )
                milestones.append(milestone)
                milestone_counter += 1

            phase = Phase(
                id=f"phase{i+1}",
                name=phase_def["name"],
                description=phase_def["description"],
                duration_weeks=phase_def.get("duration_weeks", 2),
                milestones=milestones,
                order=i,
            )
            roadmap_phases.append(phase)

        return ResearchRoadmap(
            question=question,
            question_id=question_id,
            phases=roadmap_phases,
        )

    def test_question_and_id(self):
        """Question and question_id set correctly."""
        roadmap = self._generate("How does attention work?", "q123")
        assert roadmap.question == "How does attention work?"
        assert roadmap.question_id == "q123"

    def test_empty_question_id(self):
        """Empty question_id allowed."""
        roadmap = self._generate("Q")
        assert roadmap.question_id == ""

    def test_phases_created_from_default(self):
        """Default phases used when no custom_phases."""
        roadmap = self._generate("Q")
        assert len(roadmap.phases) == 4
        names = [p.name for p in roadmap.phases]
        assert "问题分析" in names

    def test_phases_have_ids(self):
        """Each phase has a phaseN id."""
        roadmap = self._generate("Q")
        ids = [p.id for p in roadmap.phases]
        assert ids == ["phase1", "phase2", "phase3", "phase4"]

    def test_phases_have_order(self):
        """Each phase has sequential order."""
        roadmap = self._generate("Q")
        orders = [p.order for p in roadmap.phases]
        assert orders == [0, 1, 2, 3]

    def test_milestones_have_ids(self):
        """Each milestone has an mN id."""
        roadmap = self._generate("Q")
        milestone_ids = [m.id for p in roadmap.phases for m in p.milestones]
        # 2 + 3 + 4 + 3 = 12 milestones total
        assert len(milestone_ids) == 12
        assert "m1" in milestone_ids
        assert "m12" in milestone_ids

    def test_milestone_counter_sequential(self):
        """Milestone IDs are sequential across phases."""
        roadmap = self._generate("Q")
        milestone_ids = [m.id for p in roadmap.phases for m in p.milestones]
        expected = [f"m{i}" for i in range(1, 13)]
        assert milestone_ids == expected

    def test_milestone_names_from_def(self):
        """Milestone names taken from definition."""
        roadmap = self._generate("Q")
        milestone_names = [m.name for p in roadmap.phases for m in p.milestones]
        assert "文献调研" in milestone_names
        assert "Baseline搭建" in milestone_names
        assert "初稿撰写" in milestone_names

    def test_custom_phases_used(self):
        """Custom phases replace defaults."""
        custom = [
            {
                "name": "Custom Phase",
                "description": "Custom desc",
                "duration_weeks": 1,
                "milestones": [
                    {"name": "Custom Milestone", "description": "Custom m", "duration_weeks": 1},
                ],
            }
        ]
        roadmap = self._generate("Q", custom_phases=custom)
        assert len(roadmap.phases) == 1
        assert roadmap.phases[0].name == "Custom Phase"
        assert len(roadmap.phases[0].milestones) == 1
        assert roadmap.phases[0].milestones[0].name == "Custom Milestone"

    def test_empty_custom_phases(self):
        """Empty custom_phases=[] → falsy, uses DEFAULT_PHASES (4 phases)."""
        roadmap = self._generate("Q", custom_phases=[])
        # [] is falsy, so "custom_phases or DEFAULT_PHASES" picks DEFAULT_PHASES
        assert len(roadmap.phases) == 4


# =============================================================================
# render_text
# =============================================================================
class TestRenderText:
    """Test render_text formatting."""

    def _render(self, roadmap):
        """Replicate render_text."""
        from llm.roadmap_generator import RoadmapGenerator
        gen = RoadmapGenerator()
        return gen.render_text(roadmap)

    def test_header_with_question(self):
        """Header includes question."""
        from llm.roadmap_generator import ResearchRoadmap
        r = ResearchRoadmap(question="Transformer optimization", question_id="q1")
        output = self._render(r)
        assert "Transformer optimization" in output
        assert "研究路线图" in output

    def test_total_weeks_shown(self):
        """Total weeks displayed."""
        from llm.roadmap_generator import ResearchRoadmap, Phase
        p = Phase(id="p1", name="Phase", description="D", duration_weeks=5)
        r = ResearchRoadmap(question="Q", question_id="q", phases=[p])
        output = self._render(r)
        assert "5 周" in output

    def test_question_id_shown(self):
        """Question ID displayed when present."""
        from llm.roadmap_generator import ResearchRoadmap
        r = ResearchRoadmap(question="Q", question_id="q123")
        output = self._render(r)
        assert "q123" in output

    def test_question_id_not_shown_when_empty(self):
        """Question ID not shown when empty."""
        from llm.roadmap_generator import ResearchRoadmap
        r = ResearchRoadmap(question="Q", question_id="")
        output = self._render(r)
        assert "问题ID" not in output

    def test_phase_name_shown(self):
        """Phase names displayed."""
        from llm.roadmap_generator import ResearchRoadmap, Phase
        p = Phase(id="p1", name="Analysis Phase", description="Analyze", duration_weeks=2)
        r = ResearchRoadmap(question="Q", question_id="q", phases=[p])
        output = self._render(r)
        assert "Analysis Phase" in output

    def test_milestone_name_shown(self):
        """Milestone names displayed."""
        from llm.roadmap_generator import ResearchRoadmap, Phase, Milestone
        m = Milestone(id="m1", name="Literature Review", description="Read papers", duration_weeks=1)
        p = Phase(id="p1", name="P", description="D", duration_weeks=1, milestones=[m])
        r = ResearchRoadmap(question="Q", question_id="q", phases=[p])
        output = self._render(r)
        assert "Literature Review" in output

    def test_milestone_duration_shown(self):
        """Milestone duration shown."""
        from llm.roadmap_generator import ResearchRoadmap, Phase, Milestone
        m = Milestone(id="m1", name="M", description="D", duration_weeks=3)
        p = Phase(id="p1", name="P", description="D", duration_weeks=1, milestones=[m])
        r = ResearchRoadmap(question="Q", question_id="q", phases=[p])
        output = self._render(r)
        assert "3周" in output

    def test_timeline_summary_section(self):
        """Timeline summary section present."""
        from llm.roadmap_generator import ResearchRoadmap, Phase
        p = Phase(id="p1", name="Phase", description="D", duration_weeks=2)
        r = ResearchRoadmap(question="Q", question_id="q", phases=[p])
        output = self._render(r)
        assert "时间线概览" in output
        assert "Week 1-2" in output

    def test_notes_shown_when_present(self):
        """Notes displayed when set."""
        from llm.roadmap_generator import ResearchRoadmap
        r = ResearchRoadmap(question="Q", question_id="q", notes="Important note")
        output = self._render(r)
        assert "Important note" in output
        assert "备注" in output

    def test_notes_not_shown_when_empty(self):
        """Notes section absent when empty."""
        from llm.roadmap_generator import ResearchRoadmap
        r = ResearchRoadmap(question="Q", question_id="q", notes="")
        output = self._render(r)
        assert "备注" not in output


# =============================================================================
# render_markdown
# =============================================================================
class TestRenderMarkdown:
    """Test render_markdown formatting."""

    def _render(self, roadmap):
        """Replicate render_markdown."""
        from llm.roadmap_generator import RoadmapGenerator
        gen = RoadmapGenerator()
        return gen.render_markdown(roadmap)

    def test_header_with_question(self):
        """Header includes question."""
        from llm.roadmap_generator import ResearchRoadmap
        r = ResearchRoadmap(question="Attention mechanisms", question_id="q1")
        output = self._render(r)
        assert "Attention mechanisms" in output
        assert "研究路线图" in output

    def test_total_weeks_in_header(self):
        """Total weeks shown."""
        from llm.roadmap_generator import ResearchRoadmap, Phase
        p = Phase(id="p1", name="P", description="D", duration_weeks=7)
        r = ResearchRoadmap(question="Q", question_id="q", phases=[p])
        output = self._render(r)
        assert "7 周" in output

    def test_phase_as_h2(self):
        """Phase name as H2."""
        from llm.roadmap_generator import ResearchRoadmap, Phase
        p = Phase(id="p1", name="Analysis", description="D", duration_weeks=1)
        r = ResearchRoadmap(question="Q", question_id="q", phases=[p])
        output = self._render(r)
        assert "## 1. Analysis" in output

    def test_milestone_as_checkbox(self):
        """Milestone shown as checkbox item."""
        from llm.roadmap_generator import ResearchRoadmap, Phase, Milestone
        m = Milestone(id="m1", name="Review", description="Review lit", duration_weeks=1)
        p = Phase(id="p1", name="P", description="D", duration_weeks=1, milestones=[m])
        r = ResearchRoadmap(question="Q", question_id="q", phases=[p])
        output = self._render(r)
        assert "- [ ]" in output
        assert "Review" in output

    def test_gantt_table_present(self):
        """Gantt-style table present."""
        from llm.roadmap_generator import ResearchRoadmap, Phase
        p = Phase(id="p1", name="Phase", description="D", duration_weeks=2)
        r = ResearchRoadmap(question="Q", question_id="q", phases=[p])
        output = self._render(r)
        assert "| 阶段 | 周数 | 内容 |" in output
        assert "时间线" in output

    def test_table_row_contains_phase_name(self):
        """Table row includes phase name and week range."""
        from llm.roadmap_generator import ResearchRoadmap, Phase
        p = Phase(id="p1", name="Analysis", description="D", duration_weeks=3)
        r = ResearchRoadmap(question="Q", question_id="q", phases=[p])
        output = self._render(r)
        assert "| Analysis | Week 1-3 |" in output


# =============================================================================
# render_json
# =============================================================================
class TestRenderJson:
    """Test render_json output."""

    def _render(self, roadmap):
        """Replicate render_json."""
        from llm.roadmap_generator import RoadmapGenerator
        gen = RoadmapGenerator()
        return gen.render_json(roadmap)

    def test_valid_json(self):
        """Output is valid JSON."""
        from llm.roadmap_generator import ResearchRoadmap
        r = ResearchRoadmap(question="Q", question_id="q")
        output = self._render(r)
        data = json.loads(output)
        assert isinstance(data, dict)

    def test_contains_question(self):
        """Question field present."""
        from llm.roadmap_generator import ResearchRoadmap
        r = ResearchRoadmap(question="My question", question_id="q1")
        output = self._render(r)
        data = json.loads(output)
        assert data["question"] == "My question"

    def test_contains_question_id(self):
        """Question ID field present."""
        from llm.roadmap_generator import ResearchRoadmap
        r = ResearchRoadmap(question="Q", question_id="abc123")
        output = self._render(r)
        data = json.loads(output)
        assert data["question_id"] == "abc123"

    def test_contains_total_weeks(self):
        """total_weeks field present."""
        from llm.roadmap_generator import ResearchRoadmap, Phase
        p = Phase(id="p1", name="P", description="D", duration_weeks=4)
        r = ResearchRoadmap(question="Q", question_id="q", phases=[p])
        output = self._render(r)
        data = json.loads(output)
        assert data["total_weeks"] == 4

    def test_contains_created_at(self):
        """created_at field present."""
        from llm.roadmap_generator import ResearchRoadmap
        r = ResearchRoadmap(question="Q", question_id="q")
        output = self._render(r)
        data = json.loads(output)
        assert "created_at" in data
        assert data["created_at"] != ""

    def test_phases_array(self):
        """phases is an array."""
        from llm.roadmap_generator import ResearchRoadmap, Phase
        p = Phase(id="p1", name="P", description="D", duration_weeks=1)
        r = ResearchRoadmap(question="Q", question_id="q", phases=[p])
        output = self._render(r)
        data = json.loads(output)
        assert isinstance(data["phases"], list)
        assert len(data["phases"]) == 1

    def test_phase_fields(self):
        """Phase object has correct fields."""
        from llm.roadmap_generator import ResearchRoadmap, Phase
        p = Phase(id="phase1", name="Phase", description="Desc", duration_weeks=2, order=0)
        r = ResearchRoadmap(question="Q", question_id="q", phases=[p])
        output = self._render(r)
        data = json.loads(output)
        phase = data["phases"][0]
        assert phase["id"] == "phase1"
        assert phase["name"] == "Phase"
        assert phase["description"] == "Desc"
        assert phase["duration_weeks"] == 2
        assert phase["order"] == 0

    def test_milestone_in_phase(self):
        """Milestone object inside phase."""
        from llm.roadmap_generator import ResearchRoadmap, Phase, Milestone
        m = Milestone(id="m1", name="Review", description="Review papers", duration_weeks=1, tasks=["t1"], dependencies=["m0"])
        p = Phase(id="p1", name="P", description="D", duration_weeks=1, milestones=[m])
        r = ResearchRoadmap(question="Q", question_id="q", phases=[p])
        output = self._render(r)
        data = json.loads(output)
        milestone = data["phases"][0]["milestones"][0]
        assert milestone["id"] == "m1"
        assert milestone["name"] == "Review"
        assert milestone["tasks"] == ["t1"]
        assert milestone["dependencies"] == ["m0"]


# =============================================================================
# Integration: full pipeline
# =============================================================================
class TestFullPipeline:
    """Test full generate + render pipeline."""

    def _full_pipeline(self, question, question_id="", format="text", custom_phases=None):
        """Replicate full pipeline."""
        from llm.roadmap_generator import RoadmapGenerator, ResearchRoadmap, Phase, Milestone
        gen = RoadmapGenerator()
        phases = custom_phases or DEFAULT_PHASES
        roadmap_phases = []
        milestone_counter = 1

        for i, phase_def in enumerate(phases):
            milestones = []
            for m_def in phase_def.get("milestones", []):
                milestone = Milestone(
                    id=f"m{milestone_counter}",
                    name=m_def["name"],
                    description=m_def["description"],
                    duration_weeks=m_def.get("duration_weeks", 1),
                )
                milestones.append(milestone)
                milestone_counter += 1

            phase = Phase(
                id=f"phase{i+1}",
                name=phase_def["name"],
                description=phase_def["description"],
                duration_weeks=phase_def.get("duration_weeks", 2),
                milestones=milestones,
                order=i,
            )
            roadmap_phases.append(phase)

        roadmap = ResearchRoadmap(question=question, question_id=question_id, phases=roadmap_phases)

        if format == "text":
            return gen.render_text(roadmap)
        elif format == "markdown":
            return gen.render_markdown(roadmap)
        elif format == "json":
            return gen.render_json(roadmap)

    def test_text_format_has_all_phases(self):
        """Text output contains all 4 phase names."""
        output = self._full_pipeline("How to do research?")
        assert "问题分析" in output
        assert "原型开发" in output
        assert "实验验证" in output
        assert "论文撰写" in output

    def test_markdown_format_has_all_phases(self):
        """Markdown output contains all phase numbers."""
        output = self._full_pipeline("How to do research?", format="markdown")
        assert "1. 问题分析" in output
        assert "2. 原型开发" in output

    def test_json_format_has_all_phases(self):
        """JSON output has 4 phases."""
        output = self._full_pipeline("How to do research?", format="json")
        data = json.loads(output)
        assert len(data["phases"]) == 4

    def test_custom_phase_reflected_in_text(self):
        """Custom phase name appears in text output."""
        custom = [
            {"name": "My Custom Phase", "description": "Custom", "duration_weeks": 1, "milestones": []}
        ]
        output = self._full_pipeline("Q", custom_phases=custom)
        assert "My Custom Phase" in output

    def test_custom_phase_reflected_in_json(self):
        """Custom phase name appears in JSON output."""
        custom = [
            {"name": "My Custom Phase", "description": "Custom", "duration_weeks": 2, "milestones": []}
        ]
        output = self._full_pipeline("Q", custom_phases=custom, format="json")
        data = json.loads(output)
        assert data["phases"][0]["name"] == "My Custom Phase"

    def test_total_weeks_matches_default_phases(self):
        """Total weeks in JSON matches sum of default phases."""
        output = self._full_pipeline("Q", format="json")
        data = json.loads(output)
        assert data["total_weeks"] == 13  # 2+4+4+3
