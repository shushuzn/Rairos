"""
Research Journal: Track research activities and thoughts.
"""
import json
import uuid
from dataclasses import dataclass, asdict, field
from datetime import datetime, timedelta
from pathlib import Path
from typing import List, Optional

from llm.tracker_base import JsonFileStore


@dataclass
class JournalEntry:
    """A journal entry."""
    id: str
    content: str
    created_at: str = ""
    updated_at: str = ""
    tags: List[str] = field(default_factory=list)
    question_id: str = ""
    experiment_id: str = ""
    paper_id: str = ""
    mood: str = ""  # productive, stuck, excited, neutral
    highlights: List[str] = field(default_factory=list)

    def __post_init__(self):
        if not self.created_at:
            self.created_at = datetime.now().isoformat()

    def to_dict(self):
        return asdict(self)

    @classmethod
    def from_dict(cls, data):
        return cls(**data)


class Journal(JsonFileStore):
    """Research journal for tracking activities."""

    def __init__(self, data_dir=None):
        p = Path(data_dir or Path.home() / ".ai_research_os" / "journal")
        p.mkdir(parents=True, exist_ok=True)
        self.data_file = p / "journal.json"

    def _post_load(self, raw: List[dict]) -> List[JournalEntry]:
        return [JournalEntry.from_dict(e) for e in raw]

    def _pre_save(self, entries: List[JournalEntry]) -> List[dict]:
        return [e.to_dict() for e in entries]

    def add(
        self,
        content: str,
        tags: Optional[List[str]] = None,
        question_id: str = "",
        experiment_id: str = "",
        paper_id: str = "",
        mood: str = "",
    ) -> JournalEntry:
        """Add a journal entry."""
        entry = JournalEntry(
            id=str(uuid.uuid4())[:8],
            content=content,
            tags=tags or [],
            question_id=question_id,
            experiment_id=experiment_id,
            paper_id=paper_id,
            mood=mood,
        )
        entries = self._load()
        entries.append(entry)
        self._save(entries)
        return entry

    def get(self, entry_id: str) -> Optional[JournalEntry]:
        """Get entry by ID."""
        for e in self._load():
            if e.id == entry_id:
                return e
        return None

    def update(self, entry_id: str, content: str = "", tags: Optional[List[str]] = None) -> Optional[JournalEntry]:
        """Update an entry."""
        entries = self._load()
        for e in entries:
            if e.id == entry_id:
                if content:
                    e.content = content
                if tags is not None:
                    e.tags = tags
                e.updated_at = datetime.now().isoformat()
                self._save(entries)
                return e
        return None

    def delete(self, entry_id: str) -> bool:
        """Delete an entry."""
        entries = self._load()
        n = len(entries)
        entries = [e for e in entries if e.id != entry_id]
        if len(entries) < n:
            self._save(entries)
            return True
        return False

    def list_entries(
        self,
        limit: int = 50,
        tag: str = "",
        question_id: str = "",
        experiment_id: str = "",
        today: bool = False,
        days: int = 0,
    ) -> List[JournalEntry]:
        """List journal entries."""
        entries = self._load()

        if today:
            today_str = datetime.now().strftime("%Y-%m-%d")
            entries = [e for e in entries if e.created_at.startswith(today_str)]
        elif days > 0:
            cutoff = (datetime.now() - timedelta(days=days)).isoformat()
            entries = [e for e in entries if e.created_at >= cutoff]

        if tag:
            entries = [e for e in entries if tag in e.tags]
        if question_id:
            entries = [e for e in entries if e.question_id == question_id]
        if experiment_id:
            entries = [e for e in entries if e.experiment_id == experiment_id]

        # Sort by date descending
        entries.sort(key=lambda x: x.created_at, reverse=True)
        return entries[:limit]

    def search(self, query: str, limit: int = 20) -> List[JournalEntry]:
        """Search entries by content."""
        q = query.lower()
        entries = [e for e in self._load() if q in e.content.lower()]
        entries.sort(key=lambda x: x.created_at, reverse=True)
        return entries[:limit]

    def stats(self) -> dict:
        """Get journal statistics."""
        entries = self._load()
        if not entries:
            return {"total": 0, "this_week": 0, "this_month": 0}

        now = datetime.now()
        week_ago = (now - timedelta(days=7)).isoformat()
        month_ago = (now - timedelta(days=30)).isoformat()

        tags_count = {}
        mood_count = {}
        for e in entries:
            for t in e.tags:
                tags_count[t] = tags_count.get(t, 0) + 1
            if e.mood:
                mood_count[e.mood] = mood_count.get(e.mood, 0) + 1

        return {
            "total": len(entries),
            "this_week": len([e for e in entries if e.created_at >= week_ago]),
            "this_month": len([e for e in entries if e.created_at >= month_ago]),
            "top_tags": sorted(tags_count.items(), key=lambda x: -x[1])[:10],
            "mood_distribution": mood_count,
        }

    def render_list(self, entries: List[JournalEntry], verbose: bool = False) -> str:
        """Render entries as text."""
        if not entries:
            return "No journal entries."

        mood_icons = {"productive": "⚡", "stuck": "😓", "excited": "🎉", "neutral": "📝"}
        lines = []
        for e in entries:
            icon = mood_icons.get(e.mood, "📝")
            date = e.created_at[:10]
            lines.append(f"{icon} [{date}] {e.content[:80]}")
            if verbose:
                if e.tags:
                    lines.append(f"   Tags: {', '.join(e.tags)}")
                if e.question_id:
                    lines.append(f"   Question: {e.question_id}")
                if e.experiment_id:
                    lines.append(f"   Experiment: {e.experiment_id}")
        return '\n'.join(lines)
