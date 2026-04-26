"""Tests for research roadmap generator."""
import pytest

from llm.roadmap_generator import (
    RoadmapGenerator,
    ResearchRoadmap,
    Phase,
    Milestone,
)


class TestRoadmapGenerator:
    """Test RoadmapGenerator."""

    def test_milestone_creation(self):
        """Test Milestone dataclass."""
        m = Milestone(
            id="m1",
            name="文献调研",
            description="阅读10篇论文",
            duration_weeks=1,
        )

        assert m.id == "m1"
        assert m.duration_weeks == 1
        assert m.tasks == []
        assert m.dependencies == []

    def test_phase_creation(self):
        """Test Phase dataclass."""
        phase = Phase(
            id="phase1",
            name="问题分析",
            description="分析问题",
            duration_weeks=2,
        )

        assert phase.id == "phase1"
        assert phase.order == 0
        assert phase.milestones == []

    def test_roadmap_creation(self):
        """Test ResearchRoadmap dataclass."""
        roadmap = ResearchRoadmap(
            question="如何提升RAG?",
            question_id="q123",
        )

        assert roadmap.question == "如何提升RAG?"
        assert roadmap.question_id == "q123"
        assert roadmap.total_weeks == 0  # No phases yet

    def test_roadmap_total_weeks_calculation(self):
        """Test total weeks is calculated from phases."""
        roadmap = ResearchRoadmap(
            question="Test",
            question_id="",
            phases=[
                Phase(id="p1", name="P1", description="", duration_weeks=2),
                Phase(id="p2", name="P2", description="", duration_weeks=3),
            ],
        )

        assert roadmap.total_weeks == 5

    def test_generate_default_phases(self):
        """Test generating roadmap with default phases."""
        generator = RoadmapGenerator()
        roadmap = generator.generate(
            question="如何改进检索?",
            question_id="q456",
        )

        assert len(roadmap.phases) == 4  # 4 default phases
        assert roadmap.total_weeks == 13  # 2+4+4+3
        assert roadmap.question == "如何改进检索?"
        assert roadmap.question_id == "q456"

    def test_generate_custom_phases(self):
        """Test generating roadmap with custom phases."""
        generator = RoadmapGenerator()
        custom = [
            {
                "name": "快速验证",
                "description": "快速验证想法",
                "duration_weeks": 1,
                "milestones": [
                    {"name": "MVP", "description": "最小可行产品", "duration_weeks": 1},
                ],
            },
        ]

        roadmap = generator.generate(
            question="Test question",
            custom_phases=custom,
        )

        assert len(roadmap.phases) == 1
        assert roadmap.phases[0].name == "快速验证"
        assert len(roadmap.phases[0].milestones) == 1

    def test_generate_milestone_ids(self):
        """Test milestone IDs are unique and sequential."""
        generator = RoadmapGenerator()
        roadmap = generator.generate(question="Test")

        milestone_ids = []
        for phase in roadmap.phases:
            for m in phase.milestones:
                milestone_ids.append(m.id)

        assert len(milestone_ids) == len(set(milestone_ids))  # All unique
        assert "m1" in milestone_ids

    def test_render_text(self):
        """Test text rendering."""
        generator = RoadmapGenerator()
        roadmap = generator.generate(question="测试问题")

        output = generator.render_text(roadmap)

        assert "# 研究路线图" in output
        assert "测试问题" in output
        assert "问题分析" in output
        assert "原型开发" in output

    def test_render_text_with_question_id(self):
        """Test text rendering includes question ID."""
        generator = RoadmapGenerator()
        roadmap = generator.generate(
            question="测试",
            question_id="q123",
        )

        output = generator.render_text(roadmap)
        assert "q123" in output

    def test_render_markdown(self):
        """Test Markdown rendering."""
        generator = RoadmapGenerator()
        roadmap = generator.generate(question="测试")

        output = generator.render_markdown(roadmap)

        assert "# 研究路线图" in output
        assert "1. 问题分析" in output
        assert "| 阶段 |" in output  # Gantt table

    def test_render_json(self):
        """Test JSON rendering."""
        import json

        generator = RoadmapGenerator()
        roadmap = generator.generate(question="测试")

        output = generator.render_json(roadmap)
        data = json.loads(output)

        assert data["question"] == "测试"
        assert len(data["phases"]) == 4
        assert "total_weeks" in data

    def test_render_json_structure(self):
        """Test JSON structure."""
        import json

        generator = RoadmapGenerator()
        roadmap = generator.generate(question="测试")

        output = generator.render_json(roadmap)
        data = json.loads(output)

        # Check phase structure
        phase = data["phases"][0]
        assert "id" in phase
        assert "name" in phase
        assert "milestones" in phase

        # Check milestone structure
        milestone = phase["milestones"][0]
        assert "id" in milestone
        assert "name" in milestone
        assert "duration_weeks" in milestone

    def test_default_phases_have_expected_names(self):
        """Test default phases have expected names."""
        generator = RoadmapGenerator()
        roadmap = generator.generate(question="Test")

        phase_names = [p.name for p in roadmap.phases]
        assert "问题分析" in phase_names
        assert "原型开发" in phase_names
        assert "实验验证" in phase_names
        assert "论文撰写" in phase_names

    def test_phase_order(self):
        """Test phase order is set correctly."""
        generator = RoadmapGenerator()
        roadmap = generator.generate(question="Test")

        for i, phase in enumerate(roadmap.phases):
            assert phase.order == i

    def test_milestone_in_phase(self):
        """Test milestones are associated with phases."""
        generator = RoadmapGenerator()
        roadmap = generator.generate(question="Test")

        for phase in roadmap.phases:
            assert phase.milestones is not None
            for m in phase.milestones:
                assert m.id is not None
                assert m.name is not None
