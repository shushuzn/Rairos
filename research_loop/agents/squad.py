"""Squad Coordinator — orchestrates the multi-agent research squad."""

from __future__ import annotations

import json
import time
import threading
import uuid
from pathlib import Path
from typing import Any, Dict, List, Optional

from research_loop.agents.base import BaseAgent, AgentMessage, AgentStatus, MessageBus
from research_loop.agents.scout import ScoutAgent, TOPIC_PAPER_DISCOVERED
from research_loop.agents.analyzer import AnalyzerAgent, TOPIC_GAP_FOUND
from research_loop.agents.curator import CuratorAgent, TOPIC_ALERT_READY
from research_loop.agents.citation_hunter import CitationHunterAgent, TOPIC_CITATION_FOUND

TOPIC_SUBSCRIPTION_CHECK = "subscription.check"


def _state_path() -> Path:
    p = Path.home() / ".ai_research_os" / "autonomous"
    p.mkdir(parents=True, exist_ok=True)
    return p / "squad_state.json"


def _load_state() -> dict:
    try:
        return json.loads(_state_path().read_text(encoding="utf-8"))
    except Exception:
        return {"running": False, "interval_minutes": 30, "last_cycle": ""}


def _save_state(running: bool, interval: int) -> None:
    state = _load_state()
    state["running"] = running
    state["interval_minutes"] = interval
    state["last_cycle"] = time.strftime("%Y-%m-%dT%H:%M:%S")
    _state_path().write_text(json.dumps(state, indent=2, ensure_ascii=False), encoding="utf-8")


class SquadCoordinator:
    """Manages the research squad of specialized agents.

    Coordinates:
        ScoutAgent    → discovers papers from arXiv subscriptions
        AnalyzerAgent → extracts research gaps from papers
        CitationHunterAgent → traces citation chains
        CuratorAgent  → scores gaps against Gene Pool, generates alerts

    Pipeline:
        subscription.check → [ScoutAgent] → paper.discovered
                                              ↓
                            [ScoutAgent] → paper.discovered → [AnalyzerAgent] → gap.found
                                              ↓                                        ↓
                            [ScoutAgent] → paper.discovered → [CitationHunterAgent] → citation.found
                                                                                     ↓
                                                                              [CuratorAgent] → alert.ready
    """

    def __init__(self):
        self.bus = MessageBus()
        self.agents: Dict[str, BaseAgent] = {}
        self._watch_thread: Optional[threading.Thread] = None
        self._stop_event = threading.Event()
        self._config = {"interval_minutes": 30}
        self._alert_count = 0

        # Register agents
        self._register_agents()

    def _register_agents(self) -> None:
        self.agents["scout"] = ScoutAgent(bus=self.bus)
        self.agents["analyzer"] = AnalyzerAgent(bus=self.bus)
        self.agents["citation_hunter"] = CitationHunterAgent(bus=self.bus)
        self.agents["curator"] = CuratorAgent(bus=self.bus)

    # ── Lifecycle ──────────────────────────────────────────────────────────────

    def start_watch(self, interval_minutes: int = 30) -> None:
        if self._watch_thread and self._watch_thread.is_alive():
            return
        self._config["interval_minutes"] = interval_minutes
        self._stop_event.clear()

        # Start all agents
        for agent in self.agents.values():
            agent.start()

        self._watch_thread = threading.Thread(
            target=self._watch_loop,
            args=(interval_minutes, self._stop_event),
            name="squad-coordinator",
            daemon=True,
        )
        self._watch_thread.start()
        _save_state(True, interval_minutes)

    def stop_watch(self) -> None:
        self._stop_event.set()
        for agent in self.agents.values():
            agent.stop()
        _save_state(False, self._config["interval_minutes"])

    def _watch_loop(self, interval_minutes: int, stop_event: threading.Event) -> None:
        while not stop_event.is_set():
            try:
                self.run_cycle()
            except Exception as e:
                import logging
                logging.getLogger(__name__).error(f"[SquadCoordinator] cycle error: {e}")
            stop_event.wait(timeout=interval_minutes * 60)

    def run_cycle(self) -> Dict[str, Any]:
        """Run one full squad cycle: trigger scan and collect all results."""
        self.bus.clear_log()
        self._alert_count = 0

        # Scout scans subscriptions → publishes paper.discovered
        self.bus.publish(
            TOPIC_SUBSCRIPTION_CHECK,
            sender="coordinator",
            payload={"triggered_at": time.time()},
        )

        # Wait briefly for agents to process
        time.sleep(3)

        # Collect alert count from log
        log = self.bus.get_log(limit=200)
        for entry in log:
            if entry.get("topic") == TOPIC_ALERT_READY:
                self._alert_count += 1

        state = _load_state()
        state["last_cycle"] = time.strftime("%Y-%m-%dT%H:%M:%S")
        _save_state(True, self._config["interval_minutes"])

        return {
            "agents": {name: ag.status.value for name, ag in self.agents.items()},
            "log": log,
            "alerts_generated": self._alert_count,
            "last_cycle": state["last_cycle"],
        }

    # ── Status ────────────────────────────────────────────────────────────────

    def get_status(self) -> Dict[str, Any]:
        state = _load_state()
        return {
            "running": state.get("running", False),
            "interval_minutes": state.get("interval_minutes", 30),
            "last_cycle": state.get("last_cycle", ""),
            "agents": {name: _format_agent(ag) for name, ag in self.agents.items()},
            "alert_count": self._alert_count,
        }

    def get_activity(self, limit: int = 100) -> List[Dict[str, Any]]:
        return self.bus.get_log(limit=limit)

    def get_alerts(self, limit: int = 20) -> List[Dict[str, Any]]:
        """Extract alert.ready messages from the bus log."""
        alerts = []
        for entry in self.bus.get_log(limit=200):
            if entry.get("topic") == TOPIC_ALERT_READY:
                alerts.append(entry["payload"])
                if len(alerts) >= limit:
                    break
        return alerts


def _format_agent(ag: BaseAgent) -> Dict[str, Any]:
    return {
        "name": ag.name,
        "status": ag.status.value,
        "topics": ag.topics,
        "activity": ag.get_activity(limit=10),
    }
