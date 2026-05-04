"""Research Session Snapstate — pause/resume for deep research agent workflows."""

from __future__ import annotations

import json
import time
import uuid
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Any, Dict, List, Optional

import shutil


@dataclass
class PaperSnapshot:
    """A paper captured during research."""

    arxiv_id: str
    title: str
    abstract: str
    url: str
    extracted_text: str = ""
    summary: str = ""
    gaps_found: List[str] = field(default_factory=list)
    notes: str = ""

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)

    @classmethod
    def from_dict(cls, d: Dict[str, Any]) -> PaperSnapshot:
        return cls(**d)


@dataclass
class GapSnapshot:
    """A research gap captured during agent iteration."""

    gap_type: str
    title: str
    description: str
    matched_papers: List[str] = field(default_factory=list)  # arxiv_ids
    archetype_match: float = 0.0
    accepted: bool = False

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)

    @classmethod
    def from_dict(cls, d: Dict[str, Any]) -> GapSnapshot:
        return cls(**d)


@dataclass
class ResearchSession:
    """Complete state of a deep research agent run."""

    session_id: str
    query: str
    created_at: float = field(default_factory=time.time)
    updated_at: float = field(default_factory=time.time)
    iteration: int = 0
    max_iterations: int = 3

    # Search state
    papers: List[PaperSnapshot] = field(default_factory=list)
    gaps: List[GapSnapshot] = field(default_factory=list)
    search_history: List[str] = field(default_factory=list)  # queries tried

    # Agent memory
    hypotheses: List[str] = field(default_factory=list)
    findings: List[str] = field(default_factory=list)
    reflections: List[str] = field(default_factory=list)

    # Archetype context
    archetype: Dict[str, float] = field(default_factory=dict)

    # Status
    status: str = "running"  # running | paused | completed | failed
    error: str = ""

    def to_dict(self) -> Dict[str, Any]:
        d = asdict(self)
        d["updated_at"] = time.time()
        return d

    @classmethod
    def from_dict(cls, d: Dict[str, Any]) -> ResearchSession:
        d.pop("updated_at", None)
        return cls(**d)

    def duration(self) -> float:
        return time.time() - self.created_at


class Snapstate:
    """Snapstate manager — save, load, list research sessions."""

    def __init__(self, base_dir: Optional[Path] = None):
        if base_dir is None:
            base_dir = Path.home() / ".ai_research_os" / "sessions"
        self.base_dir = Path(base_dir)
        self.base_dir.mkdir(parents=True, exist_ok=True)

    def _session_path(self, session_id: str) -> Path:
        return self.base_dir / f"{session_id}.json"

    def save(self, session: ResearchSession) -> Path:
        """Save session to disk. Returns path."""
        session.updated_at = time.time()
        path = self._session_path(session.session_id)
        data = session.to_dict()
        # Write atomically
        tmp = path.with_suffix(".tmp")
        tmp.write_text(json.dumps(data, indent=2, ensure_ascii=False), encoding="utf-8")
        tmp.replace(path)
        return path

    def load(self, session_id: str) -> Optional[ResearchSession]:
        """Load session by ID. Returns None if not found."""
        path = self._session_path(session_id)
        if not path.exists():
            return None
        try:
            data = json.loads(path.read_text(encoding="utf-8"))
            return ResearchSession.from_dict(data)
        except (json.JSONDecodeError, TypeError, KeyError):
            return None

    def load_latest(self) -> Optional[ResearchSession]:
        """Load the most recently updated session."""
        sessions = sorted(self.base_dir.glob("*.json"), key=lambda p: p.stat().st_mtime)
        if not sessions:
            return None
        return self.load(sessions[-1].stem)

    def list_sessions(self) -> List[Dict[str, Any]]:
        """List all saved sessions (summary info only)."""
        sessions = []
        for path in sorted(self.base_dir.glob("*.json"), key=lambda p: -p.stat().st_mtime):
            try:
                data = json.loads(path.read_text(encoding="utf-8"))
                sessions.append(
                    {
                        "session_id": data.get("session_id", path.stem),
                        "query": data.get("query", ""),
                        "status": data.get("status", "?"),
                        "iteration": data.get("iteration", 0),
                        "duration": round(time.time() - data.get("created_at", time.time()), 1),
                        "papers": len(data.get("papers", [])),
                        "gaps": len(data.get("gaps", [])),
                    }
                )
            except Exception:
                sessions.append({"session_id": path.stem, "status": "corrupt"})
        return sessions

    def delete(self, session_id: str) -> bool:
        """Delete a session. Returns True if deleted."""
        path = self._session_path(session_id)
        if path.exists():
            path.unlink()
            return True
        return False

    def new_session(
        self, query: str, max_iterations: int = 3, archetype: Optional[Dict[str, float]] = None
    ) -> ResearchSession:
        """Create a new research session."""
        from llm.insight_evolution import get_evolution_tracker

        tracker = get_evolution_tracker()
        profile = tracker.get_archetype()

        return ResearchSession(
            session_id=str(uuid.uuid4())[:8],
            query=query,
            max_iterations=max_iterations,
            archetype=archetype or {k: v[1] for k, v in profile.get("dimensions", {}).items()},
        )
