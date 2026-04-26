"""
Research Question Tracker: Persistent management of research questions.

Track research questions from:
- Manual entry
- Gap detection from QuestionValidator
- Hypothesis generation results
"""

from __future__ import annotations

import json
import uuid
from dataclasses import dataclass, asdict, field
from datetime import datetime
from enum import Enum
from pathlib import Path
from typing import List, Optional


class QuestionStatus(Enum):
    """Research question status."""
    OPEN = "open"
    IN_PROGRESS = "in_progress"
    RESOLVED = "resolved"
    WONTFIX = "wontfix"


class QuestionSource(Enum):
    """Source of the research question."""
    MANUAL = "manual"
    GAP_DETECTION = "gap_detection"
    HYPOTHESIS = "hypothesis"
    LITERATURE_REVIEW = "literature_review"


@dataclass
class ResearchQuestion:
    """A research question tracked in the system."""
    id: str
    question: str
    source: str  # QuestionSource value
    status: str  # QuestionStatus value
    related_papers: List[str] = field(default_factory=list)
    created_at: str = ""
    updated_at: str = ""
    notes: str = ""
    priority: int = 5  # 1-10, higher = more important
    topic: str = ""  # Research topic

    def __post_init__(self):
        now = datetime.now().isoformat()
        if not self.created_at:
            self.created_at = now
        if not self.updated_at:
            self.updated_at = now

    def to_dict(self) -> dict:
        return asdict(self)

    @classmethod
    def from_dict(cls, data: dict) -> "ResearchQuestion":
        return cls(**data)


class QuestionTracker:
    """Track and manage research questions persistently."""

    def __init__(self, data_dir: Optional[Path] = None):
        if data_dir is None:
            data_dir = Path.home() / ".ai_research_os" / "questions"
        self.data_dir = Path(data_dir)
        self.data_dir.mkdir(parents=True, exist_ok=True)
        self.questions_file = self.data_dir / "questions.json"

    def _load(self) -> List[ResearchQuestion]:
        """Load all questions from disk."""
        if not self.questions_file.exists():
            return []
        try:
            with open(self.questions_file, 'r', encoding='utf-8') as f:
                data = json.load(f)
                return [ResearchQuestion.from_dict(q) for q in data]
        except (json.JSONDecodeError, KeyError):
            return []

    def _save(self, questions: List[ResearchQuestion]) -> None:
        """Save all questions to disk."""
        with open(self.questions_file, 'w', encoding='utf-8') as f:
            json.dump([q.to_dict() for q in questions], f, ensure_ascii=False, indent=2)

    def add(
        self,
        question: str,
        source: str = QuestionSource.MANUAL.value,
        topic: str = "",
        priority: int = 5,
        notes: str = "",
    ) -> ResearchQuestion:
        """Add a new research question."""
        questions = self._load()

        q = ResearchQuestion(
            id=str(uuid.uuid4())[:8],
            question=question,
            source=source,
            status=QuestionStatus.OPEN.value,
            topic=topic,
            priority=priority,
            notes=notes,
        )

        questions.append(q)
        self._save(questions)
        return q

    def list_questions(
        self,
        status: Optional[str] = None,
        topic: Optional[str] = None,
        source: Optional[str] = None,
    ) -> List[ResearchQuestion]:
        """List questions with optional filters."""
        questions = self._load()

        if status:
            questions = [q for q in questions if q.status == status]
        if topic:
            questions = [q for q in questions if topic.lower() in q.topic.lower()]
        if source:
            questions = [q for q in questions if q.source == source]

        return sorted(questions, key=lambda x: -x.priority)

    def get(self, question_id: str) -> Optional[ResearchQuestion]:
        """Get a question by ID."""
        questions = self._load()
        for q in questions:
            if q.id == question_id:
                return q
        return None

    def update(
        self,
        question_id: str,
        status: Optional[str] = None,
        notes: Optional[str] = None,
        priority: Optional[int] = None,
    ) -> Optional[ResearchQuestion]:
        """Update a question's fields."""
        questions = self._load()

        for q in questions:
            if q.id == question_id:
                if status:
                    q.status = status
                if notes is not None:
                    q.notes = notes
                if priority is not None:
                    q.priority = max(1, min(10, priority))
                q.updated_at = datetime.now().isoformat()
                self._save(questions)
                return q

        return None

    def link_paper(self, question_id: str, paper_id: str) -> Optional[ResearchQuestion]:
        """Link a paper to a question."""
        questions = self._load()

        for q in questions:
            if q.id == question_id:
                if paper_id not in q.related_papers:
                    q.related_papers.append(paper_id)
                    q.updated_at = datetime.now().isoformat()
                    self._save(questions)
                return q

        return None

    def unlink_paper(self, question_id: str, paper_id: str) -> Optional[ResearchQuestion]:
        """Unlink a paper from a question."""
        questions = self._load()

        for q in questions:
            if q.id == question_id:
                if paper_id in q.related_papers:
                    q.related_papers.remove(paper_id)
                    q.updated_at = datetime.now().isoformat()
                    self._save(questions)
                return q

        return None

    def delete(self, question_id: str) -> bool:
        """Delete a question."""
        questions = self._load()
        original_len = len(questions)
        questions = [q for q in questions if q.id != question_id]

        if len(questions) < original_len:
            self._save(questions)
            return True
        return False

    def sync_from_gaps(
        self,
        gaps: List[str],
        topic: str = "",
        priority: int = 7,
    ) -> List[ResearchQuestion]:
        """Sync questions from gap detection results."""
        questions = self._load()
        new_questions = []

        for gap in gaps:
            # Check if similar question already exists
            exists = any(
                gap.lower() in q.question.lower() or q.question.lower() in gap.lower()
                for q in questions
            )
            if not exists:
                q = ResearchQuestion(
                    id=str(uuid.uuid4())[:8],
                    question=f"如何解决: {gap}?",
                    source=QuestionSource.GAP_DETECTION.value,
                    status=QuestionStatus.OPEN.value,
                    topic=topic,
                    priority=priority,
                )
                questions.append(q)
                new_questions.append(q)

        if new_questions:
            self._save(questions)

        return new_questions

    def get_stats(self) -> dict:
        """Get statistics about tracked questions."""
        questions = self._load()

        status_counts = {}
        source_counts = {}
        topic_counts = {}

        for q in questions:
            status_counts[q.status] = status_counts.get(q.status, 0) + 1
            source_counts[q.source] = source_counts.get(q.source, 0) + 1
            if q.topic:
                topic_counts[q.topic] = topic_counts.get(q.topic, 0) + 1

        return {
            "total": len(questions),
            "by_status": status_counts,
            "by_source": source_counts,
            "by_topic": topic_counts,
        }

    def render_list(self, questions: List[ResearchQuestion], verbose: bool = False) -> str:
        """Render questions as formatted text."""
        if not questions:
            return "没有找到研究问题。"

        lines = []
        for i, q in enumerate(questions, 1):
            status_icon = {
                "open": "○",
                "in_progress": "◐",
                "resolved": "●",
                "wontfix": "✗",
            }.get(q.status, "?")

            lines.append(f"{i}. [{status_icon}] {q.question}")
            lines.append(f"   ID: {q.id} | 来源: {q.source} | 优先级: {q.priority}/10")

            if q.topic:
                lines.append(f"   主题: {q.topic}")

            if q.related_papers:
                lines.append(f"   关联论文: {len(q.related_papers)} 篇")

            if verbose and q.notes:
                lines.append(f"   备注: {q.notes[:100]}")

            lines.append("")

        return '\n'.join(lines)
