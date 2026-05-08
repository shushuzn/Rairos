"""Workspace snapshot — side-git file snapshots for workspace rollback.

Inspired by DeepSeek-TUI's side-git feature: before each agent step,
snapshot all generated files to a staging area. If the step produces bad
output (e.g. broken tests), rollback to the previous snapshot.

Usage:
    snap = WorkspaceSnapshot(base_dir=Path("data/workspace_snapshots"))
    snap.capture("session-abc", step=1, paths=[generated_code.py])
    # ... later, if broken:
    snap.rollback("session-abc", step=1)

Unlike snapstate.py (session state), this snapshots generated code files.
"""

from __future__ import annotations

import hashlib
import shutil
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Dict, List, Optional


@dataclass
class FileSnapshot:
    """A single file snapshot."""

    rel_path: str
    content_hash: str
    size_bytes: int
    snapshot_path: Path  # absolute path in snapshot staging area


@dataclass
class WorkspaceSnapshot:
    """Side-git workspace snapshots — capture and rollback generated files.

    Stores snapshots in: {base_dir}/{session_id}/step_{N}/
    Each step dir contains mirrored copy of all tracked files.
    """

    base_dir: Optional[Path] = None
    max_snapshots_per_session: int = 10  # keep last N snapshots per session

    def __post_init__(self):
        if self.base_dir is None:
            self.base_dir = Path.home() / ".ai_research_os" / "workspace_snapshots"
        self.base_dir = Path(self.base_dir)  # type: ignore[assignment]
        self.base_dir.mkdir(parents=True, exist_ok=True)

    def _session_dir(self, session_id: str) -> Path:
        return self.base_dir / session_id  # type: ignore[operator]

    def _step_dir(self, session_id: str, step: int) -> Path:
        return self._session_dir(session_id) / f"step_{step:03d}"

    def _file_hash(self, path: Path) -> str:
        """SHA-256 hash of file content."""
        return hashlib.sha256(path.read_bytes()).hexdigest()[:16]

    def capture(
        self,
        session_id: str,
        step: int,
        paths: List[Path],
        metadata: Optional[Dict[str, Any]] = None,
    ) -> Path:
        """Capture current state of files for a given step.

        Copies each file into the snapshot staging area while computing
        content hashes for rollback verification.

        Returns path to the snapshot directory.
        """
        step_dir = self._step_dir(session_id, step)
        step_dir.mkdir(parents=True, exist_ok=True)

        snapshot_manifest: Dict[str, Any] = {
            "session_id": session_id,
            "step": step,
            "captured_at": time.time(),
            "files": [],
            "metadata": metadata or {},
        }

        for file_path in paths:
            if not file_path.exists():
                continue
            rel = file_path.name
            target = step_dir / rel
            shutil.copy2(file_path, target)
            snapshot_manifest["files"].append(
                {
                    "name": rel,
                    "hash": self._file_hash(file_path),
                    "size": file_path.stat().st_size,
                }
            )

        # Write manifest
        import json

        manifest_path = step_dir / "_snapshot_manifest.json"
        manifest_path.write_text(json.dumps(snapshot_manifest, indent=2), encoding="utf-8")

        self._prune_old_snapshots(session_id)
        return step_dir

    def rollback(self, session_id: str, step: int, target_dir: Path) -> List[Path]:
        """Rollback files from a given step back to target_dir.

        Returns list of files that were restored.
        """
        step_dir = self._step_dir(session_id, step)
        if not step_dir.exists():
            return []

        restored: List[Path] = []
        import json

        manifest_path = step_dir / "_snapshot_manifest.json"
        if manifest_path.exists():
            manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
            for entry in manifest.get("files", []):
                src = step_dir / entry["name"]
                dst = target_dir / entry["name"]
                if src.exists():
                    dst.parent.mkdir(parents=True, exist_ok=True)
                    shutil.copy2(src, dst)
                    restored.append(dst)

        return restored

    def list_snapshots(self, session_id: str) -> List[Dict[str, Any]]:
        """List all snapshots for a session."""
        session_dir = self._session_dir(session_id)
        if not session_dir.exists():
            return []

        import json

        snapshots: List[Dict[str, Any]] = []
        for step_dir in sorted(session_dir.glob("step_*")):
            if not step_dir.is_dir():
                continue
            manifest_path = step_dir / "_snapshot_manifest.json"
            if manifest_path.exists():
                try:
                    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
                except Exception:
                    manifest = {}
            else:
                manifest = {}
            snapshots.append(
                {
                    "step": step_dir.name,
                    "path": str(step_dir),
                    "files": len(manifest.get("files", [])),
                    "captured_at": manifest.get("captured_at", step_dir.stat().st_mtime),
                    "metadata": manifest.get("metadata", {}),
                }
            )
        return snapshots

    def latest_snapshot(self, session_id: str) -> Optional[int]:
        """Return step number of the most recent snapshot, or None."""
        snapshots = self.list_snapshots(session_id)
        if not snapshots:
            return None
        result = max(
            s["step"] for s in snapshots if isinstance(s["step"], int) or s["step"].isdigit()
        )
        return result  # type: ignore[return-value,no-any-return]

    def _prune_old_snapshots(self, session_id: str) -> None:
        """Remove oldest snapshots beyond max_snapshots_per_session."""
        snapshots = self.list_snapshots(session_id)
        if len(snapshots) <= self.max_snapshots_per_session:
            return
        # Sort by step, drop oldest
        to_delete = snapshots[: -self.max_snapshots_per_session]
        for snap in to_delete:
            snap_dir = Path(snap["path"])
            if snap_dir.exists():
                shutil.rmtree(snap_dir)
