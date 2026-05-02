"""Watch papers.json for changes and trigger incremental KG rebuilds.

Uses a simple file-hash polling loop — no external dependencies, cross-platform.
"""
from __future__ import annotations

import hashlib
import signal
import sys
import time
from pathlib import Path
from typing import Optional

# ── Signal handling ────────────────────────────────────────────────────────────


class Watcher:
    """Watch a single file for content changes, calling a callback on each change."""

    def __init__(
        self,
        path: str | Path,
        interval: float = 5.0,
        on_change: Optional[callable] = None,
    ):
        self.path = Path(path)
        self.interval = interval
        self.on_change = on_change
        self._running = False
        self._last_hash: Optional[str] = None

    def _hash(self) -> Optional[str]:
        if not self.path.exists():
            return None
        return hashlib.md5(self.path.read_bytes()).hexdigest()

    def _detect_change(self) -> bool:
        current = self._hash()
        if current is None:
            return False
        if self._last_hash is None:
            self._last_hash = current
            return False  # first poll — don't fire, just record
        if current != self._last_hash:
            self._last_hash = current
            return True
        return False

    def start(self):
        """Start the watch loop. Blocks until stop() is called."""
        self._running = True
        self._last_hash = self._hash()  # initialise hash immediately
        print(f"[watch] Monitoring {self.path} (poll every {self.interval}s)")

        while self._running:
            time.sleep(self.interval)
            if not self._running:
                break
            if self._detect_change():
                print(f"[watch] Change detected in {self.path}")
                if self.on_change:
                    try:
                        self.on_change(str(self.path))
                    except Exception as exc:  # noqa: BLE001
                        print(f"[watch] Callback error: {exc}")

    def stop(self):
        """Stop the watch loop."""
        self._running = False


# ── CLI entry point ─────────────────────────────────────────────────────────


def watch_and_rebuild(
    papers_json: Optional[str] = None,
    interval: float = 5.0,
    incremental: bool = True,
):
    """Watch papers.json and run incremental KG rebuild on each change."""
    from pathlib import Path

    if papers_json is None:
        candidates = [Path("papers.json"), Path("data/papers.json")]
        for c in candidates:
            if c.exists():
                papers_json = str(c)
                break
        if papers_json is None:
            print("papers.json not found. Use --papers-json to specify.")
            sys.exit(1)

    from kg.manager import KGManager
    from kg.integration import KGIntegration

    kg = KGManager()
    integ = KGIntegration(kg)

    def on_change(path: str):
        print(f"[watch] Rebuilding KG from {path} ...")
        integ.rebuild_from_papers_json(path, incremental=incremental)
        stats = kg.stats()
        print(f"[watch] Done: {stats['total_nodes']} nodes, {stats['total_edges']} edges.")

    watcher = Watcher(papers_json, interval=interval, on_change=on_change)

    # Handle Ctrl+C gracefully
    def signal_handler(sig, frame):
        print("\n[watch] Stopping...")
        watcher.stop()
        sys.exit(0)

    signal.signal(signal.SIGINT, signal_handler)
    signal.signal(signal.SIGTERM, signal_handler)

    watcher.start()
