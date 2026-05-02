"""Base Agent + Message Bus for multi-agent coordination."""

from __future__ import annotations

import json
import time
import uuid
from dataclasses import asdict, dataclass, field
from enum import Enum
from pathlib import Path
from typing import Any, Dict, List, Optional, Callable
import threading
import logging

logger = logging.getLogger(__name__)


class AgentStatus(Enum):
    IDLE = "idle"
    WORKING = "working"
    DONE = "done"
    ERROR = "error"


# ─── Message Bus ────────────────────────────────────────────────────────────────


class MessageBus:
    """In-process pub/sub bus for inter-agent communication.

    Agents publish messages to named topics. Other agents subscribe to topics
    to receive messages. Messages are stored in a simple in-memory log.
    """

    _instance: Optional[MessageBus] = None
    _lock = threading.Lock()

    def __new__(cls) -> MessageBus:
        if cls._instance is None:
            with cls._lock:
                if cls._instance is None:
                    cls._instance = super().__new__(cls)
                    cls._instance._init()
        return cls._instance

    def _init(self):
        self._subscribers: Dict[str, List[str]] = {}  # topic -> [agent_name, ...]
        self._inbox: Dict[str, List[AgentMessage]] = {}  # agent_name -> [msg, ...]
        self._log: List[AgentMessage] = []
        self._lock = threading.Lock()
        self._state_file = Path.home() / ".ai_research_os" / "autonomous" / "agent_log.jsonl"
        self._state_file.parent.mkdir(parents=True, exist_ok=True)

    # ── Subscribe ──────────────────────────────────────────────────────────────

    def subscribe(self, topic: str, agent_name: str) -> None:
        with self._lock:
            if topic not in self._subscribers:
                self._subscribers[topic] = []
            if agent_name not in self._subscribers[topic]:
                self._subscribers[topic].append(agent_name)

    def unsubscribe(self, topic: str, agent_name: str) -> None:
        with self._lock:
            if topic in self._subscribers:
                self._subscribers[topic] = [a for a in self._subscribers[topic] if a != agent_name]

    # ── Publish ────────────────────────────────────────────────────────────────

    def publish(self, topic: str, sender: str, payload: Dict[str, Any]) -> AgentMessage:
        msg = AgentMessage(
            id=str(uuid.uuid4())[:8],
            topic=topic,
            sender=sender,
            payload=payload,
            timestamp=time.time(),
        )
        with self._lock:
            self._log.append(msg)
            # Fan out to subscribers
            for agent_name in self._subscribers.get(topic, []):
                if agent_name not in self._inbox:
                    self._inbox[agent_name] = []
                self._inbox[agent_name].append(msg)
        # Persist to disk
        self._persist(msg)
        return msg

    # ── Receive ────────────────────────────────────────────────────────────────

    def receive(self, agent_name: str, timeout: float = 0) -> Optional[AgentMessage]:
        """Receive one message for agent (non-blocking by default)."""
        with self._lock:
            if agent_name in self._inbox and self._inbox[agent_name]:
                return self._inbox[agent_name].pop(0)
        return None

    def receive_all(self, agent_name: str) -> List[AgentMessage]:
        """Receive all pending messages for agent."""
        with self._lock:
            msgs = self._inbox.get(agent_name, [])
            self._inbox[agent_name] = []
            return msgs

    def peek(self, agent_name: str) -> List[AgentMessage]:
        """Peek at pending messages without consuming them."""
        with self._lock:
            return list(self._inbox.get(agent_name, []))

    # ── Log ────────────────────────────────────────────────────────────────────

    def get_log(self, limit: int = 100) -> List[Dict[str, Any]]:
        with self._lock:
            return [asdict(m) for m in self._log[-limit:]]

    def clear_log(self) -> None:
        with self._lock:
            self._log.clear()
            self._inbox.clear()

    def _persist(self, msg: AgentMessage) -> None:
        try:
            with open(self._state_file, "a", encoding="utf-8") as f:
                f.write(json.dumps(asdict(msg), ensure_ascii=False) + "\n")
        except Exception as e:
            logger.warning(f"Failed to persist agent message: {e}")


# ─── Agent Message ─────────────────────────────────────────────────────────────


@dataclass
class AgentMessage:
    id: str
    topic: str
    sender: str
    payload: Dict[str, Any]
    timestamp: float
    reply_to: Optional[str] = None  # message id this is a response to

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


# ─── Base Agent ────────────────────────────────────────────────────────────────


class BaseAgent:
    """Base class for all research squad agents.

    Each agent:
    - Has a unique name and a set of topics it subscribes to
    - Runs in its own thread
    - Has an inbox (message queue) and an outbox (for replies)
    - Implements `think()` to handle incoming messages and produce outputs
    - Publishes results back to the message bus
    """

    STATUS = AgentStatus

    def __init__(
        self,
        name: str,
        topics: Optional[List[str]] = None,
        bus: Optional[MessageBus] = None,
    ):
        self.name = name
        self.topics = topics or []
        self.bus = bus or MessageBus()
        self.status = AgentStatus.IDLE
        self._thread: Optional[threading.Thread] = None
        self._stop_event = threading.Event()
        self._activity_log: List[Dict[str, Any]] = []
        self._lock = threading.Lock()

        # Subscribe to topics
        for topic in self.topics:
            self.bus.subscribe(topic, self.name)

    # ── Lifecycle ──────────────────────────────────────────────────────────────

    def start(self) -> None:
        if self._thread and self._thread.is_alive():
            return
        self._stop_event.clear()
        self._thread = threading.Thread(target=self._run, name=f"agent-{self.name}", daemon=True)
        self._thread.start()
        self._log("started")

    def stop(self) -> None:
        self._stop_event.set()
        if self._thread:
            self._thread.join(timeout=5)
        self._log("stopped")

    def _run(self) -> None:
        while not self._stop_event.is_set():
            msgs = self.bus.receive_all(self.name)
            for msg in msgs:
                self._log("received", topic=msg.topic, payload=msg.payload)
                try:
                    responses = self.think(msg)
                    if responses:
                        for resp in responses:
                            resp.reply_to = msg.id
                            self.bus.publish(resp.topic, self.name, resp.payload)
                            self._log("published", topic=resp.topic, payload=resp.payload)
                except Exception as e:
                    self.status = AgentStatus.ERROR
                    self._log("error", error=str(e))
                    logger.error(f"[{self.name}] think() error: {e}")

            # Brief sleep to avoid busy-waiting
            time.sleep(0.05)

    # ── Subclass hook ──────────────────────────────────────────────────────────

    def think(self, msg: AgentMessage) -> List[AgentMessage]:
        """Process an incoming message and return zero or more response messages.

        Override this in subclasses.
        """
        return []

    # ── Helpers ────────────────────────────────────────────────────────────────

    def publish(self, topic: str, payload: Dict[str, Any]) -> AgentMessage:
        return self.bus.publish(topic, self.name, payload)

    def _log(self, event: str, **kwargs) -> None:
        entry = {"ts": time.time(), "event": event, "agent": self.name, **kwargs}
        with self._lock:
            self._activity_log.append(entry)

    def get_activity(self, limit: int = 50) -> List[Dict[str, Any]]:
        with self._lock:
            return list(self._activity_log[-limit:])
