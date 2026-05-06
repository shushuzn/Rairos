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

    arxiv_id: str = ""
    title: str = ""
    abstract: str = ""
    url: str = ""
    extracted_text: str = ""
    summary: str = ""
    gaps_found: List[str] = field(default_factory=list)
    notes: str = ""
    keywords: List[str] = field(default_factory=list)

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)

    @classmethod
    def from_dict(cls, d: Dict[str, Any]) -> PaperSnapshot:
        # Filter to only known field names to avoid TypeError on extra keys
        field_names = {f.name for f in cls.__dataclass_fields__.values()}
        filtered = {k: v for k, v in d.items() if k in field_names}
        return cls(**filtered)


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
        field_names = {f.name for f in cls.__dataclass_fields__.values()}
        filtered = {k: v for k, v in d.items() if k in field_names}
        return cls(**filtered)


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
        field_names = {f.name for f in cls.__dataclass_fields__.values()}
        filtered = {k: v for k, v in d.items() if k in field_names}
        return cls(**filtered)

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

    # ─── Checkpoint & Rollback ────────────────────────────────────────────────

    def create_checkpoint(self, session: ResearchSession) -> str:
        """Save a named checkpoint of the current session state.

        Returns a checkpoint_id that can be used with rollback_to().
        Checkpoints are stored alongside the session file.
        """
        checkpoint_id = str(uuid.uuid4())[:8]
        ck_dir = self.base_dir / f"{session.session_id}_checkpoints"
        ck_dir.mkdir(parents=True, exist_ok=True)
        ck_path = ck_dir / f"{checkpoint_id}.json"
        data = session.to_dict()
        ck_path.write_text(json.dumps(data, indent=2, ensure_ascii=False), encoding="utf-8")
        return checkpoint_id

    def rollback_to(self, session_id: str, checkpoint_id: str) -> Optional[ResearchSession]:
        """Restore session to a previous checkpoint.

        Discards all state after the checkpoint and overwrites the current
        session file with the checkpointed state.
        Returns the restored ResearchSession, or None if not found.
        """
        ck_dir = self.base_dir / f"{session_id}_checkpoints"
        ck_path = ck_dir / f"{checkpoint_id}.json"
        if not ck_path.exists():
            return None
        try:
            data = json.loads(ck_path.read_text(encoding="utf-8"))
            restored = ResearchSession.from_dict(data)
            # Overwrite current session with checkpointed state
            self.save(restored)
            return restored
        except (json.JSONDecodeError, TypeError, KeyError):
            return None

    def list_checkpoints(self, session_id: str) -> List[Dict[str, Any]]:
        """List all checkpoints for a session."""
        ck_dir = self.base_dir / f"{session_id}_checkpoints"
        if not ck_dir.exists():
            return []
        checkpoints = []
        for ck_path in sorted(ck_dir.glob("*.json"), key=lambda p: -p.stat().st_mtime):
            try:
                data = json.loads(ck_path.read_text(encoding="utf-8"))
                checkpoints.append(
                    {
                        "checkpoint_id": ck_path.stem,
                        "created_at": ck_path.stat().st_mtime,
                        "iteration": data.get("iteration", 0),
                        "papers": len(data.get("papers", [])),
                        "gaps": len(data.get("gaps", [])),
                    }
                )
            except Exception:
                checkpoints.append({"checkpoint_id": ck_path.stem, "corrupt": True})
        return checkpoints

    def delete_checkpoint(self, session_id: str, checkpoint_id: str) -> bool:
        """Delete a specific checkpoint."""
        ck_path = self.base_dir / f"{session_id}_checkpoints" / f"{checkpoint_id}.json"
        if ck_path.exists():
            ck_path.unlink()
            return True
        return False

    def fork_session(
        self, session_id: str, checkpoint_id: str, new_query: Optional[str] = None
    ) -> Optional[ResearchSession]:
        """Fork a session at a checkpoint into a new session with a new session_id.

        Copies the checkpointed state, assigns a new session_id, resets status to 'running'.
        If new_query is provided, overrides the original query.
        Returns the new forked ResearchSession, or None if checkpoint not found.
        """
        ck_dir = self.base_dir / f"{session_id}_checkpoints"
        ck_path = ck_dir / f"{checkpoint_id}.json"
        if not ck_path.exists():
            return None
        try:
            data = json.loads(ck_path.read_text(encoding="utf-8"))
            forked = ResearchSession.from_dict(data)
            forked.session_id = str(uuid.uuid4())[:8]
            forked.status = "running"
            forked.created_at = time.time()
            forked.updated_at = time.time()
            if new_query is not None:
                forked.query = new_query
            self.save(forked)
            return forked
        except (json.JSONDecodeError, TypeError, KeyError):
            return None

    def compare_sessions(self, session_id_a: str, session_id_b: str) -> Dict[str, Any]:
        """Return a diff summary between two sessions."""
        sess_a = self.load(session_id_a)
        sess_b = self.load(session_id_b)

        if sess_a is None or sess_b is None:
            return {"error": "One or both sessions not found"}

        papers_a = {p.arxiv_id for p in sess_a.papers if p.arxiv_id}
        papers_b = {p.arxiv_id for p in sess_b.papers if p.arxiv_id}
        gaps_a = {g.title for g in sess_a.gaps}
        gaps_b = {g.title for g in sess_b.gaps}

        return {
            "session_id_a": session_id_a,
            "session_id_b": session_id_b,
            "status_a": sess_a.status,
            "status_b": sess_b.status,
            "iteration_a": sess_a.iteration,
            "iteration_b": sess_b.iteration,
            "papers_a_count": len(papers_a),
            "papers_b_count": len(papers_b),
            "papers_diff": len(papers_a) - len(papers_b),
            "gaps_a_count": len(gaps_a),
            "gaps_b_count": len(gaps_b),
            "gaps_diff": len(gaps_a) - len(gaps_b),
            "shared_papers": list(papers_a & papers_b),
            "shared_papers_count": len(papers_a & papers_b),
            "unique_to_a": list(papers_a - papers_b),
            "unique_to_b": list(papers_b - papers_a),
            "iteration_diff": sess_a.iteration - sess_b.iteration,
        }
