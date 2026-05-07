"""ResearchDaemon — async event bus + SSE streaming for the autonomous orchestrator.

Classes
-------
EventBus : pub/sub singleton for daemon events.
DaemonEvent : event payload dataclass.
ResearchDaemon : runs AutonomousOrchestrator cycles on a timer, publishes events.
SSEServer : aiohttp SSE server streaming events to browser/CLI clients.
"""

from __future__ import annotations

import asyncio
import json
import logging
import threading
import time
from collections import defaultdict
from dataclasses import asdict, dataclass, field
from typing import Any, Callable, Dict, List, Optional

import aiohttp
import aiohttp.web

logger = logging.getLogger(__name__)

# ─── EventBus ────────────────────────────────────────────────────────────────


class EventBus:
    """Thread-safe pub/sub event bus (singleton).

    Subscribers register callbacks for specific event types. When
    publish() is called, all matching callbacks are invoked with the
    DaemonEvent payload.

    Thread-safety: uses threading.Lock because the daemon runs in a
    background thread while the SSE server runs in the main (asyncio)
    thread.
    """

    _instance: Optional["EventBus"] = None

    def __new__(cls) -> "EventBus":
        if cls._instance is None:
            cls._instance = super().__new__(cls)
            cls._instance._lock = threading.Lock()
            cls._instance._subscribers: Dict[str, List[Callable]] = defaultdict(list)
            cls._instance._history: List[DaemonEvent] = []
            cls._instance._max_history = 200
        return cls._instance

    def subscribe(self, event_type: str, callback: Callable[["DaemonEvent"], None]) -> None:
        """Register a callback for an event type."""
        with self._lock:
            self._subscribers[event_type].append(callback)

    def unsubscribe(self, event_type: str, callback: Callable[["DaemonEvent"], None]) -> None:
        """Remove a callback for an event type."""
        with self._lock:
            subs = self._subscribers.get(event_type, [])
            if callback in subs:
                subs.remove(callback)

    def publish(self, event_type: str, data: Any) -> None:
        """Invoke all callbacks registered for event_type."""
        event = DaemonEvent(event_type=event_type, data=data, timestamp=time.time())
        with self._lock:
            # Store in history
            self._history.append(event)
            if len(self._history) > self._max_history:
                self._history = self._history[-self._max_history :]
            # Snapshot subscribers under lock
            subs = list(self._subscribers.get(event_type, []))
            # Also notify wildcard listeners
            subs += list(self._subscribers.get("*", []))

        for cb in subs:
            try:
                cb(event)
            except Exception as e:
                logger.warning(f"[EventBus] callback error for '{event_type}': {e}")

    def get_history(self, event_type: Optional[str] = None, limit: int = 50) -> List["DaemonEvent"]:
        """Return recent events, optionally filtered by type."""
        with self._lock:
            history = self._history
        if event_type:
            history = [e for e in history if e.event_type == event_type]
        return history[-limit:]

    @property
    def event_types(self) -> List[str]:
        """All registered event type names."""
        with self._lock:
            return list(self._subscribers.keys())


# ─── DaemonEvent ──────────────────────────────────────────────────────────────


@dataclass
class DaemonEvent:
    """Standard event payload published by the ResearchDaemon."""

    event_type: str
    data: Any
    timestamp: float = field(default_factory=time.time)

    def to_dict(self) -> Dict[str, Any]:
        d = asdict(self)
        # Serialize data to JSON-safe form when possible
        try:
            json.dumps(d["data"])
        except TypeError:
            d["data"] = str(d["data"])
        return d

    def to_sse(self) -> str:
        """Format as a Server-Sent Events line."""
        return f"event: {self.event_type}\ndata: {json.dumps(self.to_dict())}\n\n"


# ─── ResearchDaemon ───────────────────────────────────────────────────────────


class ResearchDaemon:
    """Runs AutonomousOrchestrator cycles on a timer, publishing events.

    Publishes the following event types to the shared EventBus:

    - ``session_started``  : new orchestrator session began
    - ``session_completed``: orchestrator session finished (no alerts)
    - ``cycle_start``     : a cycle has started
    - ``cycle_complete``   : a cycle finished; data = {"alerts": [...], "duration_s": float}
    - ``alert_found``     : a ResearchAlert was generated; data = alert dict with severity
    - ``error``           : an exception occurred; data = {"message": str, "exc": str}
    """

    def __init__(
        self,
        interval_minutes: int = 30,
        webhook_enabled: bool = True,
    ) -> None:
        self.interval_minutes = interval_minutes
        self.webhook_enabled = webhook_enabled
        self._event_bus = EventBus()
        self._orchestrator: Optional["AutonomousOrchestrator"] = None
        self._stop_event: Optional[threading.Event] = None
        self._thread: Optional[threading.Thread] = None

    # ── Lazy orchestrator ─────────────────────────────────────────────────────

    def _get_orchestrator(self) -> "AutonomousOrchestrator":
        if self._orchestrator is None:
            from research_loop.orchestrator import AutonomousOrchestrator

            self._orchestrator = AutonomousOrchestrator(webhook_enabled=self.webhook_enabled)
        return self._orchestrator

    # ── Lifecycle ─────────────────────────────────────────────────────────────

    def start(self) -> None:
        """Start the daemon loop in a background thread."""
        if self._thread and self._thread.is_alive():
            logger.warning("[ResearchDaemon] already running")
            return

        self._stop_event = threading.Event()
        self._thread = threading.Thread(
            target=self._run_loop,
            name="research-daemon",
            daemon=True,
        )
        self._thread.start()
        logger.info("[ResearchDaemon] started")

    def stop(self) -> None:
        """Signal the daemon loop to stop."""
        if self._stop_event:
            self._stop_event.set()
        logger.info("[ResearchDaemon] stopped")

    # ── Internal loop ────────────────────────────────────────────────────────

    def _run_loop(self) -> None:
        cycle_count = 0
        while not self._stop_event.is_set():
            cycle_count += 1
            start_ts = time.time()
            self._event_bus.publish("cycle_start", {"cycle": cycle_count})

            try:
                orch = self._get_orchestrator()
                alerts = orch.run_cycle()
                duration = time.time() - start_ts

                # Publish alert_found for each alert
                for alert in alerts:
                    self._event_bus.publish(
                        "alert_found",
                        {
                            **alert.to_dict(),
                            "severity": alert.severity,
                        },
                    )

                # Publish session events
                if alerts:
                    self._event_bus.publish(
                        "session_completed",
                        {
                            "cycle": cycle_count,
                            "alerts_count": len(alerts),
                            "duration_s": round(duration, 2),
                        },
                    )
                else:
                    self._event_bus.publish(
                        "session_completed",
                        {
                            "cycle": cycle_count,
                            "alerts_count": 0,
                            "duration_s": round(duration, 2),
                        },
                    )

                self._event_bus.publish(
                    "cycle_complete",
                    {
                        "alerts": [a.to_dict() for a in alerts],
                        "duration_s": round(duration, 2),
                        "cycle": cycle_count,
                    },
                )

            except Exception as exc:
                logger.error(f"[ResearchDaemon] cycle error: {exc}")
                self._event_bus.publish(
                    "error",
                    {
                        "message": str(exc),
                        "exc": exc.__class__.__name__,
                        "cycle": cycle_count,
                    },
                )

            # Run evolution every 3 cycles
            if cycle_count % 3 == 0:
                try:
                    self._event_bus.publish("session_started", {"type": "evolution"})
                    orch = self._get_orchestrator()
                    result = orch.run_evolution_cycle()
                    self._event_bus.publish(
                        "session_completed",
                        {"type": "evolution", "result": result},
                    )
                except Exception as exc:
                    logger.error(f"[ResearchDaemon] evolution error: {exc}")
                    self._event_bus.publish(
                        "error",
                        {"message": str(exc), "exc": exc.__class__.__name__, "type": "evolution"},
                    )

            self._stop_event.wait(timeout=self.interval_minutes * 60)

    # ── Manual triggers ──────────────────────────────────────────────────────

    def run_cycle(self) -> List[Any]:
        """Run one cycle synchronously (called from CLI)."""
        orch = self._get_orchestrator()
        return orch.run_cycle()


# ─── SSE Server ───────────────────────────────────────────────────────────────


class SSEServer:
    """Async HTTP server that streams DaemonEvent payloads as text/event-stream.

    Serves GET /events to any connected client (browser, curl, etc.).
    Clients may optionally filter by ``?type=<event_type>`` query param.
    """

    def __init__(self, port: int = 8765) -> None:
        self.port = port
        self._event_bus = EventBus()
        self._runner: Optional[aiohttp.web.AppRunner] = None
        self._site: Optional[aiohttp.web.TCPSite] = None
        self._clients: Dict[str, asyncio.Queue] = {}
        self._client_id = 0
        self._lock = threading.Lock()
        self._shutdown = False

    # ── Public API ───────────────────────────────────────────────────────────

    def start(self) -> None:
        """Start the SSE server (blocking)."""
        loop = asyncio.new_event_loop()
        asyncio.set_event_loop(loop)
        loop.run_until_complete(self._serve())

    def stop(self) -> None:
        """Signal the SSE server to stop."""
        self._shutdown = True
        if self._runner:
            asyncio.post_fact_batch([self._stop_app()])
        logger.info("[SSEServer] stop signalled")

    async def _stop_app(self) -> None:
        if self._runner:
            await self._runner.cleanup()

    # ── Internal ─────────────────────────────────────────────────────────────

    async def _serve(self) -> None:
        app = aiohttp.web.Application()

        # Subscribe to EventBus and forward events to SSE clients
        self._event_bus.subscribe("*", self._forward_event)

        app.router.add_get("/events", self._handle_events)
        app.router.add_get("/health", self._handle_health)

        self._runner = aiohttp.web.AppRunner(app)
        await self._runner.setup()
        self._site = self._runner.make_site(host="0.0.0.0", port=self.port)
        await self._site.start()

        logger.info(f"[SSEServer] listening on http://0.0.0.0:{self.port}/events")

        # Keep alive until shutdown
        while not self._shutdown:
            await asyncio.sleep(1)

        await self._runner.cleanup()

    def _forward_event(self, event: DaemonEvent) -> None:
        """Called by EventBus on every event; enqueue for all SSE clients."""
        sse_line = event.to_sse()
        with self._lock:
            dead = []
            for cid, q in self._clients.items():
                try:
                    asyncio.post_fact_batch([q.put_nowait(sse_line)])
                except Exception:
                    dead.append(cid)
            for cid in dead:
                del self._clients[cid]

    async def _handle_events(self, request: aiohttp.web.Request) -> aiohttp.web.StreamResponse:
        """Serve SSE endpoint: GET /events."""
        q: asyncio.Queue = asyncio.Queue()
        client_id = str(self._client_id)
        self._client_id += 1

        # Optional type filter
        ftype = request.query.get("type", "*")

        with self._lock:
            self._clients[client_id] = q

        # Send initial connection event
        await q.put(f'event: connected\ndata: {{"client_id":"{client_id}","filter":"{ftype}"}}\n\n')

        # Send recent history for the filtered type
        history = self._event_bus.get_history(
            event_type=None if ftype == "*" else ftype,
            limit=20,
        )
        for ev in history:
            if ftype == "*" or ev.event_type == ftype:
                await q.put(ev.to_sse())

        async def sse_iter() -> Any:
            try:
                while True:
                    try:
                        line = await asyncio.wait_for(q.get(), timeout=60)
                        yield line.encode("utf-8")
                    except asyncio.TimeoutError:
                        yield b""
                    except Exception as e:
                        import logging
                        logging.getLogger("daemon").warning("SSE stream error for client %s: %s", client_id, e)
                        yield b"event: error\ndata: stream failed\n\n"
                        break
            finally:
                with self._lock:
                    self._clients.pop(client_id, None)

        response = aiohttp.web.StreamResponse(
            status=200,
            reason="OK",
            headers={
                "Content-Type": "text/event-stream",
                "Cache-Control": "no-cache",
                "Connection": "keep-alive",
                "X-Accel-Buffering": "no",
            },
        )
        await response.prepare(request)
        async for chunk in sse_iter():
            await response.write(chunk)
        await response.write_eof()
        return response

    async def _handle_health(self, request: aiohttp.web.Request) -> aiohttp.web.Response:
        """GET /health — simple liveness check."""
        return aiohttp.web.json_response({"status": "ok", "clients": len(self._clients)})
