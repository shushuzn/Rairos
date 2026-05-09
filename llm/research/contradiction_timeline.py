"""Contradiction Timeline Tracker — track contradictions over time to detect paradigm shifts.

Architecture
────────────
Each detected contradiction (from detect_contradictions) is recorded as an event
with a timestamp. Over time, we build a timeline that shows:

  - When a contradiction first appeared (emergence)
  - How it evolved (polarity shifts)
  - When it was resolved or ceded
  - Paradigm shift signals (contradictions clustering in a short time window)

DB Schema
─────────
gap_contradiction_timeline:
  contradiction_key TEXT PRIMARY KEY
  gap_type TEXT
  field TEXT
  paper_a TEXT
  paper_b TEXT
  polarity_a TEXT
  polarity_b TEXT
  first_detected_at TEXT
  last_updated_at TEXT
  status TEXT
  event_count INTEGER
  resolution_type TEXT
  resolution_paper TEXT

Paradigm Shift Detection
────────────────────────
A paradigm shift signal fires when:
  - 3+ new contradictions in the same gap_type within 7 days
  - OR a contradiction shifts from 'confirmed' to 'rejected' (reversal)

Usage
─────
    from llm.research.contradiction_timeline import (
        record_contradictions, detect_paradigm_shifts,
        get_contradiction_timeline, summarize_timeline
    )
    new_count = record_contradictions(conn, contradictions=[...])
    alerts = detect_paradigm_shifts(conn)
    timeline = get_contradiction_timeline(conn, gap_type="method_limitation")
"""

import hashlib
import logging
import uuid
from collections import defaultdict
from datetime import datetime as DT, timedelta
from typing import Any, Dict, List, Optional

logger = logging.getLogger(__name__)


# ─── Dataclasses ──────────────────────────────────────────────────────────────


class ContradictionEvent:
    """A single contradiction event."""

    __slots__ = (
        "contradiction_key", "gap_type", "field", "paper_a", "paper_b",
        "polarity_a", "polarity_b", "detected_at", "source",
    )

    def __init__(
        self,
        contradiction_key: str,
        gap_type: str,
        field: str = "",
        paper_a: str = "",
        paper_b: str = "",
        polarity_a: str = "open",
        polarity_b: str = "open",
        detected_at: Optional[DT] = None,
        source: str = "capsule",
    ):
        self.contradiction_key = contradiction_key
        self.gap_type = gap_type
        self.field = field
        self.paper_a = paper_a
        self.paper_b = paper_b
        self.polarity_a = polarity_a
        self.polarity_b = polarity_b
        self.detected_at = detected_at or DT.now()
        self.source = source

    def to_dict(self) -> Dict[str, Any]:
        return {
            "contradiction_key": self.contradiction_key,
            "gap_type": self.gap_type,
            "field": self.field,
            "paper_a": self.paper_a,
            "paper_b": self.paper_b,
            "polarity_a": self.polarity_a,
            "polarity_b": self.polarity_b,
            "detected_at": self.detected_at.isoformat() if self.detected_at else "",
            "source": self.source,
        }


class ContradictionRecord:
    """Aggregated contradiction record from DB."""

    __slots__ = (
        "contradiction_key", "gap_type", "field", "paper_a", "paper_b",
        "polarity_a", "polarity_b", "first_detected_at", "last_updated_at",
        "status", "event_count", "resolution_type", "resolution_paper",
    )

    def __init__(
        self,
        contradiction_key: str,
        gap_type: str,
        field: str,
        paper_a: str,
        paper_b: str,
        polarity_a: str,
        polarity_b: str,
        first_detected_at: str,
        last_updated_at: str,
        status: str,
        event_count: int,
        resolution_type: str = "",
        resolution_paper: str = "",
    ):
        self.contradiction_key = contradiction_key
        self.gap_type = gap_type
        self.field = field
        self.paper_a = paper_a
        self.paper_b = paper_b
        self.polarity_a = polarity_a
        self.polarity_b = polarity_b
        self.first_detected_at = first_detected_at
        self.last_updated_at = last_updated_at
        self.status = status
        self.event_count = event_count
        self.resolution_type = resolution_type
        self.resolution_paper = resolution_paper


class ParadigmShiftAlert:
    """Alert when a paradigm shift signal is detected."""

    __slots__ = (
        "alert_type", "gap_type", "message", "contradictions",
        "severity", "detected_at",
    )

    def __init__(
        self,
        alert_type: str,
        gap_type: str,
        message: str,
        contradictions: List[Dict[str, Any]],
        severity: str,
        detected_at: Optional[DT] = None,
    ):
        self.alert_type = alert_type
        self.gap_type = gap_type
        self.message = message
        self.contradictions = contradictions
        self.severity = severity
        self.detected_at = detected_at or DT.now()


# ─── DB Schema ────────────────────────────────────────────────────────────────


def _ensure_schema(conn) -> None:
    """Create the gap_contradiction_timeline table if it doesn't exist."""
    conn.execute("""
        CREATE TABLE IF NOT EXISTS gap_contradiction_timeline (
            contradiction_key TEXT PRIMARY KEY,
            gap_type TEXT NOT NULL,
            field TEXT DEFAULT '',
            paper_a TEXT DEFAULT '',
            paper_b TEXT DEFAULT '',
            polarity_a TEXT DEFAULT 'open',
            polarity_b TEXT DEFAULT 'open',
            first_detected_at TEXT NOT NULL,
            last_updated_at TEXT NOT NULL,
            status TEXT DEFAULT 'active',
            event_count INTEGER DEFAULT 1,
            resolution_type TEXT DEFAULT '',
            resolution_paper TEXT DEFAULT ''
        )
    """)
    conn.execute("""
        CREATE INDEX IF NOT EXISTS idx_gct_gap_type
        ON gap_contradiction_timeline(gap_type)
    """)
    conn.execute("""
        CREATE INDEX IF NOT EXISTS idx_gct_status
        ON gap_contradiction_timeline(status)
    """)
    conn.commit()


# ─── Key computation ──────────────────────────────────────────────────────────


def _make_contradiction_key(
    gap_type: str, field: str, paper_a: str, paper_b: str
) -> str:
    """Create a stable primary key for a contradiction.

    The key is order-independent for paper_a/paper_b.
    """
    pair = sorted([paper_a or "", paper_b or ""], key=lambda x: x)
    raw = f"{gap_type}|{field}|{pair[0]}|{pair[1]}"
    return hashlib.sha256(raw.encode()).hexdigest()[:24]


# ─── Record operations ───────────────────────────────────────────────────────


def record_contradictions(
    conn,
    contradictions: List[Dict[str, Any]],
    detected_at: Optional[DT] = None,
) -> int:
    """Record contradiction events into the timeline DB.

    Returns the number of new contradictions recorded (not already in DB).
    """
    if not contradictions:
        return 0

    _ensure_schema(conn)
    now = detected_at or DT.now()
    now_str = now.isoformat()

    new_count = 0
    for c in contradictions:
        gap_type = c.get("gap_type", "") or ""
        field = c.get("field", "") or ""
        paper_a = c.get("paper_a", "") or c.get("source_paper_id", "") or ""
        paper_b = c.get("paper_b", "") or ""

        polarity_a = c.get("polarity_a", "open")
        polarity_b = c.get("polarity_b", "open")

        key = _make_contradiction_key(gap_type, field, paper_a, paper_b)

        existing = conn.execute(
            "SELECT polarity_a, polarity_b, status, event_count FROM gap_contradiction_timeline WHERE contradiction_key = ?",
            (key,),
        ).fetchone()

        if existing:
            old_pol_a, old_pol_b, old_status, old_count = existing
            new_count_event = old_count + 1

            new_status = old_status
            if old_pol_a != polarity_a or old_pol_b != polarity_b:
                if old_status == "active":
                    new_status = "escalated"

            conn.execute(
                """
                UPDATE gap_contradiction_timeline
                SET polarity_a=?, polarity_b=?, last_updated_at=?, status=?, event_count=?
                WHERE contradiction_key=?
                """,
                (polarity_a, polarity_b, now_str, new_status, new_count_event, key),
            )
        else:
            conn.execute(
                """
                INSERT INTO gap_contradiction_timeline
                (contradiction_key, gap_type, field, paper_a, paper_b,
                 polarity_a, polarity_b, first_detected_at, last_updated_at,
                 status, event_count)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 'active', 1)
                """,
                (key, gap_type, field, paper_a, paper_b, polarity_a, polarity_b, now_str, now_str),
            )
            new_count += 1

    conn.commit()
    return new_count


def mark_contradiction_resolved(
    conn,
    contradiction_key: str,
    resolution_type: str,
    resolution_paper: str = "",
) -> bool:
    """Mark a contradiction as resolved."""
    _ensure_schema(conn)
    now_str = DT.now().isoformat()
    cursor = conn.execute(
        """
        UPDATE gap_contradiction_timeline
        SET status=?, resolution_type=?, resolution_paper=?, last_updated_at=?
        WHERE contradiction_key=?
        """,
        (resolution_type, resolution_type, resolution_paper, now_str, contradiction_key),
    )
    conn.commit()
    return cursor.rowcount > 0


def get_contradiction_timeline(
    conn,
    gap_type: Optional[str] = None,
    status: Optional[str] = None,
    limit: int = 50,
) -> List[ContradictionRecord]:
    """Get contradiction timeline records."""
    _ensure_schema(conn)
    query = "SELECT * FROM gap_contradiction_timeline"
    params: List[Any] = []
    where_parts = []
    if gap_type:
        where_parts.append("gap_type = ?")
        params.append(gap_type)
    if status:
        where_parts.append("status = ?")
        params.append(status)
    if where_parts:
        query += " WHERE " + " AND ".join(where_parts)
    query += " ORDER BY last_updated_at DESC LIMIT ?"
    params.append(limit)

    rows = conn.execute(query, params).fetchall()
    return [
        ContradictionRecord(
            contradiction_key=r[0],
            gap_type=r[1],
            field=r[2] or "",
            paper_a=r[3] or "",
            paper_b=r[4] or "",
            polarity_a=r[5] or "open",
            polarity_b=r[6] or "open",
            first_detected_at=r[7] or "",
            last_updated_at=r[8] or "",
            status=r[9] or "active",
            event_count=r[10] or 1,
            resolution_type=r[11] or "",
            resolution_paper=r[12] or "",
        )
        for r in rows
    ]


# ─── Paradigm Shift Detection ────────────────────────────────────────────────


def detect_paradigm_shifts(
    conn,
    window_days: int = 7,
    min_contradictions: int = 3,
) -> List[ParadigmShiftAlert]:
    """Detect paradigm shift signals from contradiction clustering.

    Fires when:
    1. Contradiction cluster: 3+ new contradictions in same gap_type within window_days
    2. Polarity reversal: a contradiction shifts polarity
    """
    _ensure_schema(conn)
    alerts: List[ParadigmShiftAlert] = []
    now = DT.now()
    cutoff = (now - timedelta(days=window_days)).isoformat()

    # Signal 1: Contradiction cluster
    recent = conn.execute(
        """
        SELECT gap_type, COUNT(*) as cnt
        FROM gap_contradiction_timeline
        WHERE first_detected_at >= ?
        GROUP BY gap_type
        HAVING cnt >= ?
        """,
        (cutoff, min_contradictions),
    ).fetchall()

    for (gt, cnt) in recent:
        key_rows = conn.execute(
            "SELECT contradiction_key FROM gap_contradiction_timeline "
            "WHERE gap_type = ? AND first_detected_at >= ?",
            (gt, cutoff),
        ).fetchall()
        keys = [r[0] for r in key_rows]

        records = conn.execute(
            "SELECT * FROM gap_contradiction_timeline WHERE contradiction_key IN ("
            + ",".join("?" * len(keys)) + ")",
            keys,
        ).fetchall() if keys else []

        contradictions = [
            {
                "contradiction_key": str(r[0]),
                "gap_type": str(r[1]),
                "field": str(r[2] or ""),
                "paper_a": str(r[3] or ""),
                "paper_b": str(r[4] or ""),
                "polarity_a": str(r[5] or "open"),
                "polarity_b": str(r[6] or "open"),
                "first_detected_at": str(r[7] or ""),
                "status": str(r[9] or "active"),
            }
            for r in records
        ]

        alerts.append(ParadigmShiftAlert(
            alert_type="contradiction_cluster",
            gap_type=str(gt),
            message=(
                f"Paradigm tension: {cnt} new contradictions in '{gt}' "
                f"in the last {window_days} days — field may be contested"
            ),
            contradictions=contradictions,
            severity="high" if cnt >= 5 else "medium",
        ))

    # Signal 2: Polarity reversal (escalated contradictions)
    escalated = conn.execute(
        "SELECT * FROM gap_contradiction_timeline WHERE status = 'escalated' LIMIT 20"
    ).fetchall()

    for r in escalated:
        alerts.append(ParadigmShiftAlert(
            alert_type="polarity_reversal",
            gap_type=str(r[1] or ""),
            message=(
                f"Polarity reversal in '{r[1]}': "
                f"{r[5]}/{r[6]} — existing consensus may be breaking"
            ),
            contradictions=[{
                "contradiction_key": str(r[0]),
                "gap_type": str(r[1] or ""),
                "paper_a": str(r[3] or ""),
                "paper_b": str(r[4] or ""),
                "polarity_a": str(r[5] or "open"),
                "polarity_b": str(r[6] or "open"),
                "last_updated_at": str(r[8] or ""),
            }],
            severity="medium",
        ))

    return alerts


# ─── Timeline Analysis ────────────────────────────────────────────────────────


def summarize_timeline(conn) -> Dict[str, Any]:
    """Get a summary of the contradiction landscape."""
    _ensure_schema(conn)

    total = conn.execute("SELECT COUNT(*) FROM gap_contradiction_timeline").fetchone()[0]
    active = conn.execute(
        "SELECT COUNT(*) FROM gap_contradiction_timeline WHERE status = 'active'"
    ).fetchone()[0]
    escalated = conn.execute(
        "SELECT COUNT(*) FROM gap_contradiction_timeline WHERE status = 'escalated'"
    ).fetchone()[0]
    resolved = conn.execute(
        "SELECT COUNT(*) FROM gap_contradiction_timeline WHERE status = 'resolved'"
    ).fetchone()[0]

    by_type = conn.execute(
        """
        SELECT gap_type, COUNT(*) as cnt,
               SUM(CASE WHEN status='active' THEN 1 ELSE 0 END) as active_cnt
        FROM gap_contradiction_timeline
        GROUP BY gap_type
        ORDER BY cnt DESC
        """
    ).fetchall()

    return {
        "total_contradictions": total,
        "active": active,
        "escalated": escalated,
        "resolved": resolved,
        "by_gap_type": [
            {"gap_type": str(r[0]), "total": r[1], "active": r[2]}
            for r in by_type
        ],
    }
