"""
Research Friction Tracker: Detect and record research efficiency bottlenecks.

Friction = any event that slows down or interrupts the user's research flow.
Tracking friction is the first step toward a self-improving research experience.
"""
from __future__ import annotations

import json
import uuid
from dataclasses import dataclass, asdict, field
from datetime import datetime
from enum import Enum
from pathlib import Path
from typing import Optional, List, Dict, Any


class FrictionType(Enum):
    """Categories of research friction."""
    COMMAND = "command"          # 命令失败/重试
    WORKFLOW = "workflow"       # 多步骤流程中途放弃
    RETRIEVAL = "retrieval"    # 搜不到/找不到
    COGNITIVE = "cognitive"     # 结果质量低于预期
    NAVIGATION = "navigation"  # 不知道该用什么命令


class FrictionSeverity(Enum):
    """How severe is the friction."""
    LOW = "low"
    MEDIUM = "medium"
    HIGH = "high"
    CRITICAL = "critical"


class Resolution(Enum):
    """How the user (or system) resolved the friction."""
    RETRIED = "retried"
    ABANDONED = "abandoned"
    WORKED_AROUND = "worked_around"
    SELF_RESOLVED = "self_resolved"
    SYSTEM_HELPED = "system_helped"


@dataclass
class FrictionEvent:
    """A single friction event."""
    id: str
    timestamp: str
    friction_type: str
    severity: str

    # Context
    command: str = ""
    query: str = ""
    step: str = ""
    error: str = ""

    # Outcome
    resolution: str = ""
    duration_seconds: int = 0
    retry_count: int = 0
    abandoned: bool = False
    notes: str = ""

    def __post_init__(self):
        if not self.id:
            self.id = f"fr_{uuid.uuid4().hex[:8]}"
        if not self.timestamp:
            self.timestamp = datetime.now().isoformat()

    def to_dict(self) -> dict:
        return asdict(self)

    @classmethod
    def from_dict(cls, data: dict) -> "FrictionEvent":
        return cls(**data)


class FrictionTracker:
    """Track and analyze research friction events."""

    def __init__(self, data_dir: Optional[str] = None):
        if data_dir is None:
            data_dir = Path.home() / ".ai_research_os" / "friction"
        self.data_dir = Path(data_dir)
        self.data_dir.mkdir(parents=True, exist_ok=True)
        self.events_file = self.data_dir / "friction_events.jsonl"
        if not self.events_file.exists():
            self.events_file.write_text("", encoding="utf-8")

    def record(
        self,
        friction_type: FrictionType,
        severity: FrictionSeverity,
        command: str = "",
        query: str = "",
        step: str = "",
        error: str = "",
        resolution: Resolution = None,
        duration_seconds: int = 0,
        retry_count: int = 0,
        abandoned: bool = False,
        notes: str = "",
    ) -> FrictionEvent:
        """Record a new friction event."""
        event = FrictionEvent(
            id=f"fr_{uuid.uuid4().hex[:8]}",
            timestamp=datetime.now().isoformat(),
            friction_type=friction_type.value,
            severity=severity.value,
            command=command,
            query=query,
            step=step,
            error=error,
            resolution=resolution.value if resolution else "",
            duration_seconds=duration_seconds,
            retry_count=retry_count,
            abandoned=abandoned,
            notes=notes,
        )
        with open(self.events_file, "a", encoding="utf-8") as f:
            f.write(json.dumps(event.to_dict(), ensure_ascii=False) + "\n")
        return event

    def record_command_failure(
        self,
        command: str,
        query: str = "",
        error: str = "",
        retry_count: int = 0,
    ) -> FrictionEvent:
        """Convenience: record a command failure."""
        severity = FrictionSeverity.HIGH if retry_count >= 3 else FrictionSeverity.MEDIUM
        return self.record(
            friction_type=FrictionType.COMMAND,
            severity=severity,
            command=command,
            query=query,
            error=error,
            retry_count=retry_count,
            resolution=Resolution.RETRIED if retry_count > 0 else Resolution.SELF_RESOLVED,
        )

    def record_workflow_abandon(
        self,
        command: str,
        step: str,
        query: str = "",
        duration_seconds: int = 0,
    ) -> FrictionEvent:
        """Convenience: record a workflow abandonment."""
        return self.record(
            friction_type=FrictionType.WORKFLOW,
            severity=FrictionSeverity.MEDIUM,
            command=command,
            query=query,
            step=step,
            duration_seconds=duration_seconds,
            abandoned=True,
            resolution=Resolution.ABANDONED,
        )

    def record_retrieval_failure(
        self,
        command: str,
        query: str,
        notes: str = "",
    ) -> FrictionEvent:
        """Convenience: record a retrieval/search failure."""
        return self.record(
            friction_type=FrictionType.RETRIEVAL,
            severity=FrictionSeverity.MEDIUM,
            command=command,
            query=query,
            notes=notes,
            resolution=Resolution.WORKED_AROUND,
        )

    def get_events(
        self,
        friction_type: FrictionType = None,
        since_days: int = 30,
        limit: int = 100,
    ) -> List[FrictionEvent]:
        """Load recent friction events."""
        events = []
        cutoff = datetime.now().timestamp() - (since_days * 86400)
        try:
            for line in reversed(open(self.events_file, encoding="utf-8").readlines()):
                if limit and len(events) >= limit:
                    break
                try:
                    event = FrictionEvent.from_dict(json.loads(line.strip()))
                    if event.timestamp:
                        ts = datetime.fromisoformat(event.timestamp).timestamp()
                        if ts < cutoff:
                            break
                    if friction_type and event.friction_type != friction_type.value:
                        continue
                    events.append(event)
                except (json.JSONDecodeError, TypeError):
                    continue
        except FileNotFoundError:
            pass
        return events

    def get_summary(self, since_days: int = 30) -> Dict[str, Any]:
        """Get a friction summary report."""
        events = self.get_events(since_days=since_days, limit=1000)
        if not events:
            return {
                "total_events": 0,
                "by_type": {},
                "by_severity": {},
                "top_commands": [],
                "abandon_rate": 0.0,
            }

        by_type: Dict[str, int] = {}
        by_severity: Dict[str, int] = {}
        command_counts: Dict[str, int] = {}
        abandoned = 0

        for e in events:
            by_type[e.friction_type] = by_type.get(e.friction_type, 0) + 1
            by_severity[e.severity] = by_severity.get(e.severity, 0) + 1
            if e.command:
                command_counts[e.command] = command_counts.get(e.command, 0) + 1
            if e.abandoned:
                abandoned += 1

        top_commands = sorted(command_counts.items(), key=lambda x: -x[1])[:5]

        return {
            "total_events": len(events),
            "by_type": by_type,
            "by_severity": by_severity,
            "top_commands": top_commands,
            "abandon_rate": abandoned / len(events) if events else 0.0,
        }
