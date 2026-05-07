"""Watch daemon — continuous event monitoring + auto-processing."""

from __future__ import annotations

import json
import logging
import time
import threading
from datetime import datetime
from pathlib import Path
from typing import Any, Dict, List, Optional

from llm.events import process_event
from llm.scout import scout
from llm.mcp_jin10 import Jin10Client
from llm.insight.tracker import EvolutionTracker

logger = logging.getLogger(__name__)

# State file for resume/status
_STATE_DIR = Path.home() / ".ai_research_os" / "watch"
_STATE_FILE = _STATE_DIR / "watch_state.json"

# Watch topics derived from Gene Pool + manual additions
DEFAULT_TOPICS = [
    "伊朗", "霍尔木兹", "石油", "导弹", "无人机",
    "美联储", "通胀", "利率",
]


def _load_state() -> Dict:
    _STATE_DIR.mkdir(parents=True, exist_ok=True)
    if _STATE_FILE.exists():
        try:
            return json.loads(_STATE_FILE.read_text(encoding="utf-8"))
        except Exception:
            pass
    return {"running": False, "interval": 300, "events": [], "last_check": ""}


def _save_state(state: Dict) -> None:
    _STATE_FILE.write_text(json.dumps(state, indent=2, ensure_ascii=False), encoding="utf-8")


class WatchDaemon:
    """Continuous event monitoring daemon (singleton pattern - shared state)."""

    _instance = None
    _thread: Optional[threading.Thread] = None
    _stop_event = threading.Event()
    _interval: int = 300

    def __new__(cls, *args, **kwargs):
        if cls._instance is None:
            cls._instance = super().__new__(cls)
        return cls._instance

    def __init__(self, interval: int = 300):
        if not hasattr(self, '_initialized'):
            self._interval = interval
            self._client = Jin10Client()
            self._tracker = EvolutionTracker()
            self._event_count = 0
            self._processed_ids: set = set()
            self._initialized = True

    def start(self) -> None:
        """Start the watch daemon in a background thread."""
        if WatchDaemon._thread and WatchDaemon._thread.is_alive():
            logger.warning("Watch daemon already running")
            return

        WatchDaemon._stop_event.clear()
        WatchDaemon._thread = threading.Thread(target=self._loop, daemon=True, name="watch-daemon")
        WatchDaemon._thread.start()

        state = _load_state()
        state["running"] = True
        state["interval"] = self._interval
        _save_state(state)

        logger.info(f"Watch daemon started (interval={self._interval}s)")

    def stop(self) -> None:
        """Stop the watch daemon immediately."""
        WatchDaemon._stop_event.set()
        if WatchDaemon._thread and WatchDaemon._thread.is_alive():
            WatchDaemon._thread.join(timeout=3)
        state = _load_state()
        state["running"] = False
        _save_state(state)
        logger.info("Watch daemon stopped")

    @property
    def running(self) -> bool:
        return WatchDaemon._thread is not None and WatchDaemon._thread.is_alive()

    def _loop(self) -> None:
        """Main monitoring loop."""
        self._client.ensure_init()
        state = _load_state()
        _processed = set(state.get("processed_ids", []))

        while not WatchDaemon._stop_event.is_set():
            try:
                self._cycle()
            except Exception as e:
                logger.error(f"Watch cycle error: {e}")
            WatchDaemon._stop_event.wait(self._interval)

    def _cycle(self) -> None:
        """Run one monitoring cycle."""
        events = []

        # 1. Poll Jin10 for each watch topic
        capsules = self._tracker._load_capsules()
        active_topics = set(DEFAULT_TOPICS)

        # Also extract topics from Gene Pool
        for c in capsules:
            if hasattr(c, "source_arxiv_category") and c.source_arxiv_category == "cs.GL":
                words = c.trigger_topic.split()
                for w in words:
                    if len(w) > 1 and w not in active_topics:
                        active_topics.add(w)

        # 2. Check each topic for new events
        for topic in list(active_topics)[:8]:  # max 8 topics per cycle
            try:
                news = self._client.search_flash(topic)
                data = news.get("data", news) if isinstance(news, dict) else {"items": []}
                items = data.get("items", []) if isinstance(data, dict) else data
                for item in items[:5]:
                    if isinstance(item, dict):
                        item_id = item.get("time", "") + str(hash(str(item.get("content", ""))))
                    else:
                        item_id = str(hash(str(item)))

                    if item_id not in self._processed_ids:
                        self._processed_ids.add(item_id)
                        # Check against Gene Pool
                        text = item.get("content", "") if isinstance(item, dict) else str(item)
                        for c in capsules:
                            kw_match = sum(1 for kw in c.trigger_keywords if kw.lower() in text.lower())
                            if kw_match >= 2 or any(kw in topic.lower() for kw in c.trigger_keywords):
                                # High match → auto-process event
                                result = process_event(keyword=topic, max_news=3, max_papers=2)
                                if "capsule_id" in result:
                                    events.append(result["capsule_id"])
                                    logger.info(f"Event processed: {result.get('capsule_title', '')[:60]}")
                                break
            except Exception as e:
                logger.debug(f"Topic '{topic}' check failed: {e}")

        # 3. Regenerate report after each cycle
        try:
            from llm.report import save as _save_report
            _save_report()
        except Exception:
            pass

        # 4. Update state
        if events:
            state = _load_state()
            state["events"] = state.get("events", []) + events
            state["events"] = state["events"][-100:]  # keep last 100
            state["last_check"] = datetime.now().isoformat()
            _save_state(state)

    def get_status(self) -> Dict:
        """Get daemon status with real Gene Pool data."""
        state = _load_state()
        caps = self._tracker._load_capsules()
        pool_stats = self._tracker.get_gene_pool_stats()
        return {
            "running": self.running,
            "interval": self._interval,
            "last_check": state.get("last_check", ""),
            "total_events": len(state.get("events", [])),
            "gene_pool_size": len(caps),
            "gene_pool": {
                "total_capsules": len(caps),
                "avg_score": pool_stats.get("avg_score", 0),
                "by_gap_type": pool_stats.get("by_gap_type", {}),
            },
        }
