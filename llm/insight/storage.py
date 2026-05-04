"""Capsule storage mixin for EvolutionTracker — gene_pool.db (SQLite)."""

from __future__ import annotations

import json
import sqlite3
import threading
import uuid
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

from llm.insight.gene import CapsuleGene

_GENEPOOL_DB = "gene_pool.db"

# Thread-local connections for thread safety
_local = threading.local()


def _get_conn(db_path: Path) -> sqlite3.Connection:
    """Get a thread-local SQLite connection."""
    if not hasattr(_local, "conn") or _local.conn is None:
        _local.conn = sqlite3.connect(str(db_path))
        _local.conn.row_factory = sqlite3.Row
        _local.conn.execute("PRAGMA journal_mode=WAL")
        _local.conn.execute("PRAGMA synchronous=NORMAL")
    return _local.conn


_SCHEMA_SQL = """
CREATE TABLE IF NOT EXISTS capsules (
    capsule_id TEXT PRIMARY KEY,
    created_at TEXT NOT NULL DEFAULT '',
    trigger_topic TEXT NOT NULL DEFAULT '',
    trigger_gap_type TEXT NOT NULL DEFAULT '',
    trigger_keywords TEXT NOT NULL DEFAULT '[]',
    action_gap_type TEXT NOT NULL DEFAULT '',
    action_gap_title TEXT NOT NULL DEFAULT '',
    outcome_success_score REAL NOT NULL DEFAULT 0.5,
    feedback_count INTEGER NOT NULL DEFAULT 0,
    evolved_generation INTEGER NOT NULL DEFAULT 0,
    archetype TEXT NOT NULL DEFAULT '{}',
    status TEXT NOT NULL DEFAULT 'active',
    low_score_streak INTEGER NOT NULL DEFAULT 0,
    credibility_score REAL NOT NULL DEFAULT 0.5,
    trendslop INTEGER NOT NULL DEFAULT 0,
    trendslop_reason TEXT NOT NULL DEFAULT '',
    credibility_badge TEXT NOT NULL DEFAULT 'medium',
    source_arxiv_category TEXT NOT NULL DEFAULT ''
);

CREATE INDEX IF NOT EXISTS idx_capsules_status ON capsules(status);
CREATE INDEX IF NOT EXISTS idx_capsules_gap_type ON capsules(trigger_gap_type);
CREATE INDEX IF NOT EXISTS idx_capsules_topic ON capsules(trigger_topic);
CREATE INDEX IF NOT EXISTS idx_capsules_created ON capsules(created_at);
"""


def _init_db(db_path: Path) -> None:
    """Initialize the SQLite database schema."""
    conn = _get_conn(db_path)
    conn.executescript(_SCHEMA_SQL)
    conn.commit()


def _capsule_from_row(row: sqlite3.Row) -> CapsuleGene:
    """Convert a SQLite row to a CapsuleGene."""
    return CapsuleGene(
        capsule_id=row["capsule_id"],
        created_at=row["created_at"],
        trigger_topic=row["trigger_topic"],
        trigger_gap_type=row["trigger_gap_type"],
        trigger_keywords=json.loads(row["trigger_keywords"]),
        action_gap_type=row["action_gap_type"],
        action_gap_title=row["action_gap_title"],
        outcome_success_score=row["outcome_success_score"],
        feedback_count=row["feedback_count"],
        evolved_generation=row["evolved_generation"],
        archetype=json.loads(row["archetype"]),
        status=row["status"],
        low_score_streak=row["low_score_streak"],
        credibility_score=row["credibility_score"],
        trendslop=bool(row["trendslop"]),
        trendslop_reason=row["trendslop_reason"],
        credibility_badge=row["credibility_badge"],
        source_arxiv_category=row["source_arxiv_category"],
    )


def _capsule_to_row(c: CapsuleGene) -> Dict[str, Any]:
    """Convert a CapsuleGene to a SQLite row dict."""
    return {
        "capsule_id": c.capsule_id,
        "created_at": c.created_at,
        "trigger_topic": c.trigger_topic,
        "trigger_gap_type": c.trigger_gap_type,
        "trigger_keywords": json.dumps(c.trigger_keywords, ensure_ascii=False),
        "action_gap_type": c.action_gap_type,
        "action_gap_title": c.action_gap_title,
        "outcome_success_score": c.outcome_success_score,
        "feedback_count": c.feedback_count,
        "evolved_generation": c.evolved_generation,
        "archetype": json.dumps(c.archetype, ensure_ascii=False),
        "status": c.status,
        "low_score_streak": c.low_score_streak,
        "credibility_score": c.credibility_score,
        "trendslop": 1 if c.trendslop else 0,
        "trendslop_reason": c.trendslop_reason,
        "credibility_badge": c.credibility_badge,
        "source_arxiv_category": c.source_arxiv_category,
    }


def _migrate_jsonl_to_sqlite(jsonl_path: Path, db_path: Path) -> bool:
    """Migrate from gene_pool.jsonl to gene_pool.db if needed.

    Returns True if migration happened, False if SQLite already has data.
    """
    conn = _get_conn(db_path)
    existing = conn.execute("SELECT COUNT(*) FROM capsules").fetchone()[0]
    if existing > 0:
        return False  # Already migrated

    if not jsonl_path.exists():
        return False  # No source data

    migrated = 0
    with open(jsonl_path, encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            try:
                data = json.loads(line)
                capsule = CapsuleGene.from_dict(data)
                _insert_capsule(conn, capsule)
                migrated += 1
            except Exception:
                continue

    conn.commit()
    # Rename old file so we don't re-migrate
    if migrated > 0:
        jsonl_path.rename(jsonl_path.with_suffix(".jsonl.migrated"))
    return migrated > 0


def _insert_capsule(conn: sqlite3.Connection, c: CapsuleGene) -> None:
    """Insert a single capsule into the database."""
    row = _capsule_to_row(c)
    conn.execute(
        """INSERT OR REPLACE INTO capsules (
            capsule_id, created_at, trigger_topic, trigger_gap_type,
            trigger_keywords, action_gap_type, action_gap_title,
            outcome_success_score, feedback_count, evolved_generation,
            archetype, status, low_score_streak,
            credibility_score, trendslop, trendslop_reason,
            credibility_badge, source_arxiv_category
        ) VALUES (
            :capsule_id, :created_at, :trigger_topic, :trigger_gap_type,
            :trigger_keywords, :action_gap_type, :action_gap_title,
            :outcome_success_score, :feedback_count, :evolved_generation,
            :archetype, :status, :low_score_streak,
            :credibility_score, :trendslop, :trendslop_reason,
            :credibility_badge, :source_arxiv_category
        )""",
        row,
    )


# ---------------------------------------------------------------------------
# CapsuleStorageMixin
# ---------------------------------------------------------------------------


class CapsuleStorageMixin:
    """Mixin that provides Gene Pool capsule storage via SQLite.

    Expects the host class to provide:
        self.data_dir: Path
        self.get_archetype() -> dict
        self._extract_keywords(text) -> list[str]
        self._get_timestamp() -> str
        self.record_capsule_lifecycle_event(...)
    """

    @property
    def _gene_pool_db(self) -> Path:
        return self.data_dir / _GENEPOOL_DB

    @property
    def _gene_pool_file(self) -> Path:
        """Legacy JSONL path — used for migration detection."""
        return self.data_dir / "gene_pool.jsonl"

    def _ensure_db(self) -> sqlite3.Connection:
        """Ensure SQLite DB is initialized, migrate from JSONL if needed."""
        db_path = self._gene_pool_db
        _init_db(db_path)

        # Auto-migrate from JSONL on first use
        jsonl_path = self._gene_pool_file
        if jsonl_path.exists() and not db_path.exists():
            _migrate_jsonl_to_sqlite(jsonl_path, db_path)

        return _get_conn(db_path)

    def encode_capsule(
        self,
        topic: str,
        gap_type: str,
        gap_title: str,
        gap_description: str = "",
        success_score: float = 0.8,
        status: str = "active",
        source_paper_id: str = "",
        source_arxiv_category: str = "",
    ) -> CapsuleGene:
        archetype = self.get_archetype()
        if source_paper_id:
            archetype["source_paper_id"] = source_paper_id
        if source_arxiv_category:
            archetype["source_arxiv_category"] = source_arxiv_category
        capsule = CapsuleGene(
            capsule_id=uuid.uuid4().hex[:12],
            created_at=self._get_timestamp(),
            trigger_topic=topic,
            trigger_gap_type=gap_type,
            trigger_keywords=self._extract_keywords(gap_title),
            action_gap_type=gap_type,
            action_gap_title=gap_title,
            outcome_success_score=success_score,
            feedback_count=1,
            evolved_generation=0,
            archetype=archetype,
            status=status,
            source_arxiv_category=source_arxiv_category,
        )

        # Compute credibility for new capsule
        try:
            self._update_credibility(capsule)
        except Exception:
            pass

        conn = self._ensure_db()
        _insert_capsule(conn, capsule)
        conn.commit()

        self.record_capsule_lifecycle_event(
            capsule_id=capsule.capsule_id,
            action="created",
            gap_title=capsule.action_gap_title,
            gap_type=capsule.action_gap_type,
        )

        return capsule

    def _update_credibility(self, capsule: CapsuleGene) -> None:
        """Set credibility fields on a capsule by comparing against pool."""
        from llm.insight.credibility import CredibilityScorer

        all_capsules = self._load_capsules()
        scorer = CredibilityScorer()
        is_trendslop, overlap, reason = scorer.is_trendslop(capsule, all_capsules)
        capsule.trendslop = is_trendslop
        capsule.trendslop_reason = reason

        n = max(capsule.feedback_count, 1)
        evidence = capsule.outcome_success_score * (1.0 - 1.0 / (n + 1))
        novelty = max(0.0, 1.0 - overlap)
        base = 0.4 * evidence + 0.4 * novelty + 0.2 * 0.5
        capsule.credibility_score = min(1.0, max(0.0, base))

        if capsule.credibility_score >= 0.70:
            capsule.credibility_badge = "high"
        elif capsule.credibility_score < 0.35:
            capsule.credibility_badge = "low"
        else:
            capsule.credibility_badge = "medium"

    def find_capsule(
        self,
        topic: str,
        gap_type: str,
        keywords: Optional[List[str]] = None,
        min_score: float = 0.2,
    ) -> List[CapsuleGene]:
        keywords = keywords or []
        capsules = self._load_capsules()  # still need full scan for trigger_match
        scored: List[Tuple[CapsuleGene, float]] = []

        for capsule in capsules:
            if capsule.status == "archived":
                continue
            match_score = capsule.trigger_match(topic, gap_type, keywords)
            if match_score >= min_score:
                scored.append((capsule, match_score))

        scored.sort(key=lambda x: x[1], reverse=True)
        return [c for c, _ in scored]

    def archive_capsule(self, capsule_id: str) -> bool:
        conn = self._ensure_db()
        cursor = conn.execute("SELECT action_gap_title, action_gap_type FROM capsules WHERE capsule_id = ?", (capsule_id,))
        row = cursor.fetchone()
        if not row:
            return False

        conn.execute("UPDATE capsules SET status = 'archived' WHERE capsule_id = ?", (capsule_id,))
        conn.commit()

        self.record_capsule_lifecycle_event(
            capsule_id=capsule_id,
            action="archived",
            gap_title=row["action_gap_title"],
            gap_type=row["action_gap_type"],
        )
        return True

    def _load_capsules(self) -> List[CapsuleGene]:
        conn = self._ensure_db()
        rows = conn.execute("SELECT * FROM capsules").fetchall()
        return [_capsule_from_row(r) for r in rows]

    def _save_capsules(self, capsules: List[CapsuleGene]) -> None:
        conn = self._ensure_db()
        conn.execute("DELETE FROM capsules")
        for c in capsules:
            _insert_capsule(conn, c)
        conn.commit()

    def get_gene_pool_stats(self) -> Dict[str, Any]:
        conn = self._ensure_db()
        total = conn.execute("SELECT COUNT(*) FROM capsules").fetchone()[0]
        if total == 0:
            return {"total": 0, "avg_score": 0.0, "by_gap_type": {}}

        avg = conn.execute("SELECT AVG(outcome_success_score) FROM capsules").fetchone()[0] or 0.0

        by_type: Dict[str, int] = {}
        for row in conn.execute("SELECT action_gap_type, COUNT(*) as cnt FROM capsules GROUP BY action_gap_type"):
            by_type[row["action_gap_type"]] = row["cnt"]

        gens = conn.execute("SELECT DISTINCT evolved_generation FROM capsules ORDER BY evolved_generation").fetchall()
        generations = [r["evolved_generation"] for r in gens]

        return {
            "total": total,
            "avg_score": round(avg, 3),
            "by_gap_type": by_type,
            "generations": generations,
        }

    def get_capsule_by_id(self, capsule_id: str) -> Optional[CapsuleGene]:
        """Fast single-capsule lookup by ID."""
        conn = self._ensure_db()
        row = conn.execute("SELECT * FROM capsules WHERE capsule_id = ?", (capsule_id,)).fetchone()
        if row:
            return _capsule_from_row(row)
        return None

    def update_capsule(self, capsule: CapsuleGene) -> None:
        """Update an existing capsule in the database."""
        conn = self._ensure_db()
        _insert_capsule(conn, capsule)
        conn.commit()
