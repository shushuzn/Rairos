"""
Research Roadmap Generator: Generate research roadmaps from questions and hypotheses.

Generates structured research plans with:
- Phase breakdown
- Milestones
- Resource estimates
- Timeline visualization
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import List, Optional, Dict
from datetime import datetime, timedelta


@dataclass
class Milestone:
    """A milestone in the research roadmap."""
    id: str
    name: str
    description: str
    duration_weeks: int  # Expected duration in weeks
    tasks: List[str] = field(default_factory=list)
    dependencies: List[str] = field(default_factory=list)  # IDs of dependent milestones


@dataclass
class Phase:
    """A phase of the research roadmap."""
    id: str
    name: str
    description: str
    duration_weeks: int
    milestones: List[Milestone] = field(default_factory=list)
    order: int = 0


@dataclass
class ResearchRoadmap:
    """Generated research roadmap."""
    question: str
    question_id: str
    phases: List[Phase] = field(default_factory=list)
    total_weeks: int = 0
    created_at: str = ""
    notes: str = ""

    def __post_init__(self):
        if not self.created_at:
            self.created_at = datetime.now().isoformat()
        self.total_weeks = sum(p.duration_weeks for p in self.phases)


# Default phase templates
DEFAULT_PHASES: list[dict] = [
    {
        "name": "问题分析",
        "description": "深入理解问题，阅读相关工作，确定技术路线",
        "duration_weeks": 2,
        "milestones": [
            {"name": "文献调研", "description": "阅读10-20篇相关论文", "duration_weeks": 1},
            {"name": "技术方案确定", "description": "确定初步技术路线", "duration_weeks": 1},
        ],
    },
    {
        "name": "原型开发",
        "description": "搭建baseline，实现核心算法",
        "duration_weeks": 4,
        "milestones": [
            {"name": "Baseline搭建", "description": "实现简单baseline", "duration_weeks": 1},
            {"name": "核心算法实现", "description": "实现核心改进方法", "duration_weeks": 2},
            {"name": "初步验证", "description": "在小规模数据上验证", "duration_weeks": 1},
        ],
    },
    {
        "name": "实验验证",
        "description": "大规模实验，对比分析",
        "duration_weeks": 4,
        "milestones": [
            {"name": "实验设计", "description": "设计实验方案", "duration_weeks": 0.5},
            {"name": "对比实验", "description": "与现有方法对比", "duration_weeks": 2},
            {"name": "消融实验", "description": "验证各组件贡献", "duration_weeks": 1},
            {"name": "结果分析", "description": "分析实验结果", "duration_weeks": 0.5},
        ],
    },
    {
        "name": "论文撰写",
        "description": "撰写论文，准备投稿",
        "duration_weeks": 3,
        "milestones": [
            {"name": "初稿撰写", "description": "完成论文初稿", "duration_weeks": 2},
            {"name": "修改润色", "description": "修改完善论文", "duration_weeks": 0.5},
            {"name": "投稿准备", "description": "准备投稿材料", "duration_weeks": 0.5},
        ],
    },
]


class RoadmapGenerator:
    """Generate structured research roadmaps."""

    def generate(
        self,
        question: str,
        question_id: str = "",
        custom_phases: List[Dict] = None,
    ) -> ResearchRoadmap:
        """
        Generate a research roadmap.

        Args:
            question: Research question
            question_id: Optional question ID
            custom_phases: Optional custom phase definitions

        Returns:
            ResearchRoadmap with phases and milestones
        """
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

    def render_text(self, roadmap: ResearchRoadmap) -> str:
        """Render roadmap as formatted text."""
        lines = [
            f"# 研究路线图: {roadmap.question}",
            "",
            f"📅 总预计时长: {roadmap.total_weeks} 周",
            f"📅 创建时间: {roadmap.created_at[:10]}",
            "",
        ]

        if roadmap.question_id:
            lines.append(f"🔗 问题ID: {roadmap.question_id}")
            lines.append("")

        lines.append("=" * 60)
        lines.append("")

        for phase in roadmap.phases:
            lines.append(f"## 📦 阶段 {phase.order + 1}: {phase.name}")
            lines.append(f"⏱️  预计: {phase.duration_weeks} 周")
            lines.append(f"📝 {phase.description}")
            lines.append("")

            for milestone in phase.milestones:
                deps = ""
                if milestone.dependencies:
                    deps = f" ← [{', '.join(milestone.dependencies)}]"
                lines.append(f"  └── 🎯 [{milestone.id}] {milestone.name} ({milestone.duration_weeks}周){deps}")
                lines.append(f"      {milestone.description}")
                if milestone.tasks:
                    for task in milestone.tasks:
                        lines.append(f"         - {task}")
                lines.append("")

        # Timeline summary
        lines.append("=" * 60)
        lines.append("")
        lines.append("## 📊 时间线概览")

        current_week = 1
        for phase in roadmap.phases:
            end_week = current_week + phase.duration_weeks - 1
            lines.append(f"Week {current_week}-{end_week}: {phase.name} ({phase.duration_weeks}周)")
            current_week = end_week + 1

        if roadmap.notes:
            lines.append("")
            lines.append("## 📋 备注")
            lines.append(roadmap.notes)

        return '\n'.join(lines)

    def render_markdown(self, roadmap: ResearchRoadmap) -> str:
        """Render roadmap as Markdown."""
        lines = [
            f"# 研究路线图: {roadmap.question}",
            "",
            f"**总预计时长**: {roadmap.total_weeks} 周",
            "",
            "---",
            "",
        ]

        for phase in roadmap.phases:
            lines.append(f"## {phase.order + 1}. {phase.name}")
            lines.append(f"**时长**: {phase.duration_weeks} 周 | *{phase.description}*")
            lines.append("")

            for milestone in phase.milestones:
                deps = ""
                if milestone.dependencies:
                    deps = f" ← [{', '.join(milestone.dependencies)}]"
                lines.append(f"- [ ] **[{milestone.id}]** {milestone.name} ({milestone.duration_weeks}周){deps}")
                lines.append(f"  - {milestone.description}")
                if milestone.tasks:
                    for task in milestone.tasks:
                        lines.append(f"    - [ ] {task}")
            lines.append("")

        # Gantt-style timeline
        lines.append("---")
        lines.append("")
        lines.append("## 📊 时间线")
        lines.append("")
        lines.append("| 阶段 | 周数 | 内容 |")
        lines.append("|------|------|------|")

        current_week = 1
        for phase in roadmap.phases:
            end_week = current_week + phase.duration_weeks - 1
            milestone_names = ", ".join([m.name for m in phase.milestones])
            lines.append(f"| {phase.name} | Week {current_week}-{end_week} | {milestone_names} |")
            current_week = end_week + 1

        return '\n'.join(lines)

    def render_json(self, roadmap: ResearchRoadmap) -> str:
        """Render roadmap as JSON."""
        import json
        return json.dumps({
            "question": roadmap.question,
            "question_id": roadmap.question_id,
            "total_weeks": roadmap.total_weeks,
            "created_at": roadmap.created_at,
            "phases": [
                {
                    "id": p.id,
                    "name": p.name,
                    "description": p.description,
                    "duration_weeks": p.duration_weeks,
                    "order": p.order,
                    "milestones": [
                        {
                            "id": m.id,
                            "name": m.name,
                            "description": m.description,
                            "duration_weeks": m.duration_weeks,
                            "tasks": m.tasks,
                            "dependencies": m.dependencies,
                        }
                        for m in p.milestones
                    ],
                }
                for p in roadmap.phases
            ],
        }, ensure_ascii=False, indent=2)
