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
    # Check if db file still exists (handles temp dir deletion)
    elif hasattr(_local, "conn") and _local.conn is not None:
        try:
            _local.conn.execute("SELECT 1")
        except (sqlite3.DatabaseError, OSError):
            _local.conn = None
            return _get_conn(db_path)
    return _local.conn


def _close_conn() -> None:
    """Close the thread-local connection."""
    if hasattr(_local, "conn") and _local.conn is not None:
        try:
            _local.conn.close()
        except Exception:
            pass
        _local.conn = None


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
    source_arxiv_category TEXT NOT NULL DEFAULT '',
    title_embedding BLOB
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
    import base64
    embedding_blob = None
    archetype = c.archetype or {}
    emb_list = archetype.get("title_embedding")
    if emb_list:
        try:
            import numpy as np
            vec = np.array(emb_list, dtype=np.float32)
            embedding_blob = vec.tobytes()
        except Exception:
            pass
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
        "title_embedding": embedding_blob,
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


# Valid gap types from GapType enum (llm/gap_detector.py)
_VALID_GAP_TYPES = frozenset([
    "unexplored_application",
    "method_limitation",
    "contradiction",
    "evaluation_gap",
    "scalability_issue",
    "theoretical_gap",
    "dataset_gap",
    "generalization_gap",
    "method_gap",
    "exploration_gap",
    "implementation",
    "theory_gap",
])

# Mapping from legacy/unknown types to nearest valid type
_GAP_TYPE_FALLBACK = {
    "capability": "method_limitation",
    "application_gap": "unexplored_application",
    "theory_gap": "theoretical_gap",
    "method_gap": "method_limitation",
    "exploration_gap": "unexplored_application",
    "general_gap": "method_limitation",
}


def _normalize_gap_type(gap_type: str) -> str:
    """Normalize a gap_type to a valid value, or return 'method_limitation' as last resort."""
    if not gap_type or not isinstance(gap_type, str):
        return "method_limitation"
    normalized = gap_type.strip().lower()
    if normalized in _VALID_GAP_TYPES:
        return normalized
    return _GAP_TYPE_FALLBACK.get(normalized, "method_limitation")


def _compute_title_embedding(text: str) -> Optional[List[float]]:
    """Compute embedding for a text using all-MiniLM-L6-v2.

    Returns None if embedding fails (model not installed).
    """
    if not text or not text.strip():
        return None
    try:
        from sentence_transformers import SentenceTransformer
        import numpy as np
        model = SentenceTransformer("all-MiniLM-L6-v2")
        vec = model.encode(text, normalize_embeddings=True)
        return vec.tolist()
    except Exception:
        return None


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
        capsule_archetype: Optional[Dict[str, Any]] = None,
        capsule_id: Optional[str] = None,
    ) -> CapsuleGene:
        archetype = capsule_archetype if capsule_archetype else self.get_archetype()
        if source_paper_id:
            archetype["source_paper_id"] = source_paper_id
        if source_arxiv_category:
            archetype["source_arxiv_category"] = source_arxiv_category
        # Compute semantic embedding for action_gap_title (lazy, on new capsules only)
        if "title_embedding" not in archetype and gap_title:
            embedding = _compute_title_embedding(gap_title)
            if embedding:
                archetype["title_embedding"] = embedding
        normalized_gap_type = _normalize_gap_type(gap_type)
        capsule = CapsuleGene(
            capsule_id=capsule_id if capsule_id else uuid.uuid4().hex[:12],
            created_at=self._get_timestamp(),
            trigger_topic=topic,
            trigger_gap_type=normalized_gap_type,
            trigger_keywords=self._extract_keywords(gap_title),
            action_gap_type=normalized_gap_type,
            action_gap_title=gap_title,
            outcome_success_score=success_score,
            feedback_count=1,
            evolved_generation=0,
            archetype=archetype,
            status=status,
            source_arxiv_category=source_arxiv_category,
        )

        # Guard: skip test/synthetic capsule titles
        if capsule.action_gap_title.lower().startswith("test "):
            return capsule

        # Compute credibility for new capsule
        try:
            self._update_credibility(capsule)
        except Exception:
            pass

        conn = self._ensure_db()
        _insert_capsule(conn, capsule)
        conn.commit()

        # Also append to JSONL so both stores stay in sync going forward
        jsonl_path = self._gene_pool_file
        if jsonl_path:
            try:
                self.data_dir.mkdir(parents=True, exist_ok=True)
                with open(jsonl_path, "a", encoding="utf-8") as f:
                    f.write(json.dumps(capsule.to_dict(), ensure_ascii=False) + "\n")
            except Exception:
                pass

        self.record_capsule_lifecycle_event(
            capsule_id=capsule.capsule_id,
            action="created",
            gap_title=capsule.action_gap_title,
            gap_type=capsule.action_gap_type,
        )

        return capsule

    def _update_credibility(self, capsule: CapsuleGene) -> None:
        """Set credibility fields on a capsule using CredibilityScorer."""
        from llm.insight.credibility import CredibilityScorer

        all_capsules = self._load_capsules()
        scorer = CredibilityScorer()
        scores = scorer.compute_novelty_scores(all_capsules)
        score = scores.get(capsule.capsule_id)
        if score:
            capsule.credibility_score = score.overall
            capsule.trendslop = score.trendslop
            capsule.trendslop_reason = score.trendslop_reason
            capsule.credibility_badge = score.badge
        else:
            # New capsule not yet in pool — quick is_trendslop check + evidence-only score
            is_trendslop, overlap, reason = scorer.is_trendslop(capsule, all_capsules)
            capsule.trendslop = is_trendslop
            capsule.trendslop_reason = reason
            import math
            n = max(capsule.feedback_count, 1)
            evidence = capsule.outcome_success_score * math.log(n + 1) / math.log(12)
            novelty = max(0.0, 1.0 - overlap)
            source = 0.5
            consistency = 0.7
            capsule.credibility_score = 0.35 * evidence + 0.30 * novelty + 0.20 * source + 0.15 * consistency
            from llm.insight.credibility import CREDIBILITY_HIGH_THRESHOLD, CREDIBILITY_LOW_THRESHOLD
            if capsule.credibility_score >= CREDIBILITY_HIGH_THRESHOLD:
                capsule.credibility_badge = "high"
            elif capsule.credibility_score < CREDIBILITY_LOW_THRESHOLD:
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

    def find_capsule_kg_aware(
        self,
        topic: str,
        gap_type: str,
        keywords: Optional[List[str]] = None,
        min_score: float = 0.2,
        kg_boost: float = 0.15,
    ) -> List[CapsuleGene]:
        """KG-aware capsule retrieval — boosts capsules whose source paper is topically related via KG.

        KG-boost signal: if the capsule's source_paper shares a Tag or Citation edge with a paper
        whose title/abstract matches the query topic, the capsule gets a +kg_boost bonus.
        This captures cross-paper research lineage that trigger_match alone cannot see.
        """
        keywords = keywords or []
        capsules = self._load_capsules()
        scored: List[Tuple[CapsuleGene, float]] = []

        # Build KG capsule→source_paper map (lazy init to avoid import overhead)
        kg_capsule_boost: Dict[str, float] = {}
        try:
            from kg.manager import KGManager
            kg = KGManager()
            gp_nodes = {
                n["entity_id"]: n for n in kg.get_all_nodes("GenePool-Capsule")
            }
            for cap in capsules:
                if cap.status == "archived":
                    continue
                archetype = cap.archetype or {}
                source_id = archetype.get("source_paper_id", "")
                if not source_id:
                    continue
                source_node = kg.get_node_by_entity("Paper", source_id)
                if not source_node:
                    continue
                # Check if source paper shares tags with query topic
                source_edges = kg.get_edges_by_node(source_node["id"], direction="out", rel_type="same_tag")
                topic_lower = topic.lower()
                for edge in source_edges:
                    neighbor = kg.get_node(edge["target_id"])
                    if neighbor and topic_lower in neighbor.get("label", "").lower():
                        kg_capsule_boost[cap.capsule_id] = kg_boost
                        break
                # Also boost if source paper was cited by / cites a paper matching topic
                if cap.capsule_id not in kg_capsule_boost:
                    source_in_edges = kg.get_edges_by_node(source_node["id"], direction="in", rel_type="cite")
                    for edge in source_in_edges:
                        citing = kg.get_node(edge["source_id"])
                        if citing and topic_lower in citing.get("label", "").lower():
                            kg_capsule_boost[cap.capsule_id] = kg_boost
                            break
        except Exception:
            kg_capsule_boost = {}

        for capsule in capsules:
            if capsule.status == "archived":
                continue
            match_score = capsule.trigger_match(topic, gap_type, keywords)
            kg_boost_val = kg_capsule_boost.get(capsule.capsule_id, 0.0)
            total_score = match_score + kg_boost_val
            if total_score >= min_score:
                scored.append((capsule, total_score))

        scored.sort(key=lambda x: x[1], reverse=True)
        return [c for c, _ in scored]

    def find_capsule_semantic(
        self,
        topic: str,
        gap_type: str,
        min_score: float = 0.1,
        top_k: int = 20,
    ) -> List[CapsuleGene]:
        """Semantic retrieval using action_gap_title embeddings.

        Computes query embedding on-the-fly, then cosine-similarity
        ranks all capsules with stored embeddings.
        Falls back to keyword match if no embeddings available.
        """
        query_emb = _compute_title_embedding(topic)
        if not query_emb:
            return self.find_capsule(topic, gap_type, keywords=[], min_score=min_score)

        capsules = self._load_capsules()
        scored: List[Tuple[CapsuleGene, float]] = []

        for capsule in capsules:
            if capsule.status == "archived":
                continue
            archetype = capsule.archetype or {}
            emb_list = archetype.get("title_embedding")
            if not emb_list:
                continue
            try:
                import numpy as np
                cap_vec = np.array(emb_list, dtype=np.float32)
                q_vec = np.array(query_emb, dtype=np.float32)
                # Both normalized → dot product = cosine similarity
                sim = float(np.dot(cap_vec, q_vec))
                if sim >= min_score:
                    scored.append((capsule, sim))
            except Exception:
                continue

        scored.sort(key=lambda x: x[1], reverse=True)
        return [c for c, _ in scored[:top_k]]

    def find_capsule_hybrid(
        self,
        topic: str,
        gap_type: str,
        keywords: Optional[List[str]] = None,
        min_lex_score: float = 0.1,
        min_total_score: float = 0.3,
        semantic_weight: float = 0.5,
        top_k: int = 20,
    ) -> List[CapsuleGene]:
        """Hybrid retrieval: lexical trigger_match + semantic cosine similarity.

        Lexical score (0-1) and semantic score (0-1) are blended with semantic_weight.
        Returns capsules where combined score >= min_total_score.
        """
        keywords = keywords or []
        query_emb = _compute_title_embedding(topic)

        capsules = self._load_capsules()
        scored: List[Tuple[CapsuleGene, float]] = []

        for capsule in capsules:
            if capsule.status == "archived":
                continue

            lex_score = capsule.trigger_match(topic, gap_type, keywords)

            sem_score = 0.0
            if query_emb:
                archetype = capsule.archetype or {}
                emb_list = archetype.get("title_embedding")
                if emb_list:
                    try:
                        import numpy as np
                        cap_vec = np.array(emb_list, dtype=np.float32)
                        q_vec = np.array(query_emb, dtype=np.float32)
                        sem_score = float(np.dot(cap_vec, q_vec))
                    except Exception:
                        sem_score = 0.0

            total = (1 - semantic_weight) * lex_score + semantic_weight * sem_score
            if total >= min_total_score:
                scored.append((capsule, total))

        scored.sort(key=lambda x: x[1], reverse=True)
        return [c for c, _ in scored[:top_k]]

    def recompute_embeddings_all(self) -> dict:
        """Batch-compute title embeddings for all active capsules.

        Stores embedding in archetype['title_embedding'] and persists to SQLite.
        Returns dict with counts of updated and skipped capsules.
        """
        capsules = self._load_capsules()
        updated = 0
        skipped = 0
        errors = 0

        for cap in capsules:
            if cap.status == "archived":
                skipped += 1
                continue
            archetype = cap.archetype or {}
            if archetype.get("title_embedding"):
                skipped += 1
                continue
            emb = _compute_title_embedding(cap.action_gap_title)
            if not emb:
                errors += 1
                continue
            archetype["title_embedding"] = emb
            cap.archetype = archetype
            updated += 1

        if updated > 0:
            self._save_capsules(capsules)
        return {"updated": updated, "skipped": skipped, "errors": errors}

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
        """Load capsules from SQLite + JSONL, deduplicating by (action_gap_title, trigger_topic).

        After JSONL→SQLite migration the two stores diverge. The same capsule may appear
        in both stores with the same (title, topic) but different feedback counts.
        We keep the entry with the highest feedback_count. Preserving entries with the
        same title but different trigger_topic is intentional — it allows the retrieval
        scorer to pick the best topic match at query time.
        """
        conn = self._ensure_db()
        rows = conn.execute("SELECT * FROM capsules").fetchall()
        # Key by (title_lower, topic_lower) to keep entries with different topics separate
        capsules_by_key: Dict[tuple, CapsuleGene] = {}

        for row in rows:
            capsule = _capsule_from_row(row)
            key = (capsule.action_gap_title.lower(), capsule.trigger_topic.lower())
            existing = capsules_by_key.get(key)
            if existing is None or capsule.feedback_count > existing.feedback_count:
                capsules_by_key[key] = capsule

        jsonl_path = self._gene_pool_file
        if jsonl_path.exists():
            try:
                with open(jsonl_path, encoding="utf-8") as f:
                    for line in f:
                        line = line.strip()
                        if not line:
                            continue
                        try:
                            data = json.loads(line)
                            capsule = CapsuleGene.from_dict(data)
                            key = (capsule.action_gap_title.lower(), capsule.trigger_topic.lower())
                            existing = capsules_by_key.get(key)
                            if existing is None or capsule.feedback_count > existing.feedback_count:
                                capsules_by_key[key] = capsule
                        except Exception:
                            continue
            except Exception:
                pass

        return list(capsules_by_key.values())

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

    def get_capsule_by_title(self, gap_title: str, topic: str = "") -> Optional[CapsuleGene]:
        """Find a capsule by its action_gap_title (case-insensitive)."""
        conn = self._ensure_db()
        query = "SELECT * FROM capsules WHERE LOWER(action_gap_title) = LOWER(?) AND status = 'active'"
        params: List[Any] = [gap_title]
        if topic:
            query += " AND LOWER(trigger_topic) = LOWER(?)"
            params.append(topic)
        row = conn.execute(query, params).fetchone()
        if row:
            return _capsule_from_row(row)
        return None

    def update_capsule(self, capsule: CapsuleGene) -> None:
        """Update an existing capsule in the database."""
        conn = self._ensure_db()
        _insert_capsule(conn, capsule)
        conn.commit()

    def eval_retrieval(self, limit: int = 50) -> Dict[str, Any]:
        """Evaluate Gene Pool retrieval quality using accepted gap events as ground truth.

        Loads ACCEPT events from events.jsonl as ground-truth (topic, gap_type, gap_title).
        Then calls find_capsule for each and checks whether the accepted gap_title appears
        in the top-K results.

        Returns:
            dict with recall@3, recall@5, MRR, total_evaluated
        """
        events_file = self.data_dir / "events.jsonl"
        if not events_file.exists():
            return {"error": "No events file found", "recall@3": 0.0, "recall@5": 0.0, "mrr": 0.0, "total": 0}

        # Load ground-truth: ACCEPT events with known gap_title
        accepted: List[Dict[str, str]] = []
        try:
            with open(events_file, encoding="utf-8") as f:
                for line in f:
                    line = line.strip()
                    if not line:
                        continue
                    try:
                        ev = json.loads(line)
                        if ev.get("action") == "accepted" and ev.get("gap_title"):
                            accepted.append({
                                "topic": ev.get("topic", ""),
                                "gap_type": ev.get("gap_type", ""),
                                "gap_title": ev.get("gap_title", ""),
                            })
                    except Exception:
                        continue
        except Exception:
            return {"error": "Failed to read events", "recall@3": 0.0, "recall@5": 0.0, "mrr": 0.0, "total": 0}

        if not accepted:
            return {"error": "No accepted events found", "recall@3": 0.0, "recall@5": 0.0, "mrr": 0.0, "total": 0}

        # Filter out test/synthetic events that have no meaningful topic or gap_title
        def is_test_event(ev: Dict[str, str]) -> bool:
            topic = ev.get("topic", "").lower()
            title = ev.get("gap_title", "").lower()
            if topic in ("test", "rl"):
                return True
            if "test" in topic.split():
                return True
            if title == "test gap" or "limitation test" in title:
                return True
            return False

        # Deduplicate by (topic, gap_title) — keep first occurrence
        seen: set = set()
        unique: List[Dict[str, str]] = []
        for ev in accepted:
            if is_test_event(ev):
                continue
            key = (ev["topic"], ev["gap_type"], ev["gap_title"])
            if key not in seen:
                seen.add(key)
                unique.append(ev)
        accepted = unique[:limit]

        recall_at_3 = 0
        recall_at_5 = 0
        mrr_sum = 0.0

        for ev in accepted:
            topic = ev["topic"]
            gap_type = ev["gap_type"]
            gap_title = ev["gap_title"]

            candidates = self.find_capsule(topic, gap_type, keywords=[], min_score=0.0)
            if not candidates:
                continue

            titles_lower = {c.action_gap_title.lower(): i for i, c in enumerate(candidates)}
            title_lower = gap_title.lower()

            if title_lower in titles_lower:
                rank = titles_lower[title_lower] + 1
                mrr_sum += 1.0 / rank
                if rank <= 3:
                    recall_at_3 += 1
                if rank <= 5:
                    recall_at_5 += 1

        total = len(accepted)
        return {
            "recall@3": round(recall_at_3 / total, 3) if total > 0 else 0.0,
            "recall@5": round(recall_at_5 / total, 3) if total > 0 else 0.0,
            "mrr": round(mrr_sum / total, 3) if total > 0 else 0.0,
            "total": total,
        }

    def get_gene_pool_quality_report(self) -> Dict[str, Any]:
        """Comprehensive GenePool quality report.

        Returns credibility distribution, trendslop analysis, quality capsule breakdown,
        and retrieval signal metrics — everything needed for a bi-weekly health check.
        """
        capsules = self._load_capsules()

        total = len(capsules)
        if total == 0:
            return {"error": "No capsules in gene pool", "total": 0}

        # Credibility distribution
        credibility_buckets = {"high": 0, "medium": 0, "low": 0, "unknown": 0}
        for c in capsules:
            if c.credibility_badge in credibility_buckets:
                credibility_buckets[c.credibility_badge] += 1
            else:
                credibility_buckets["unknown"] += 1

        # Trendslop analysis
        trendslop_capsules = [c for c in capsules if c.trendslop]
        trendslop_reasons: Dict[str, int] = {}
        for c in trendslop_capsules:
            reason_key = c.trendslop_reason[:50] if c.trendslop_reason else "unknown"
            trendslop_reasons[reason_key] = trendslop_reasons.get(reason_key, 0) + 1

        # Score distribution
        scores = [c.outcome_success_score for c in capsules]
        avg_score = sum(scores) / total
        high_score = sum(1 for s in scores if s >= 0.7)
        mid_score = sum(1 for s in scores if 0.4 <= s < 0.7)
        low_score = sum(1 for s in scores if s < 0.4)

        # Feedback count distribution
        feedback_counts = [c.feedback_count for c in capsules]
        high_use = sum(1 for f in feedback_counts if f >= 3)
        low_use = sum(1 for f in feedback_counts if f == 0)

        # Generation distribution
        generation_counts: Dict[int, int] = {}
        for c in capsules:
            generation_counts[c.evolved_generation] = generation_counts.get(c.evolved_generation, 0) + 1

        # Status breakdown
        status_counts: Dict[str, int] = {}
        for c in capsules:
            status_counts[c.status] = status_counts.get(c.status, 0) + 1

        # Source arxiv category breakdown
        arxiv_categories: Dict[str, int] = {}
        for c in capsules:
            cat = c.source_arxiv_category or "none"
            arxiv_categories[cat] = arxiv_categories.get(cat, 0) + 1

        # Low score streak analysis (at-risk capsules, active only)
        at_risk = [c for c in capsules if c.low_score_streak >= 2 and c.status == "active"]

        # Top gap types
        gap_type_counts: Dict[str, int] = {}
        for c in capsules:
            gap_type_counts[c.action_gap_type] = gap_type_counts.get(c.action_gap_type, 0) + 1
        top_gap_types = sorted(gap_type_counts.items(), key=lambda x: x[1], reverse=True)[:5]

        # Compute alerts
        alerts = []
        trendslop_pct = len(trendslop_capsules) / total
        low_score_pct = low_score / total
        zero_feedback_pct = low_use / total
        low_cred_pct = credibility_buckets.get("low", 0) / total
        archived_pct = status_counts.get("archived", 0) / total
        evolved_count = generation_counts.get(1, 0)

        if trendslop_pct > 0.15:
            alerts.append({
                "level": "warning",
                "code": "TRENDSLOP_HIGH",
                "message": f"Trendslop {trendslop_pct:.1%} of pool (>{.15:.1%})",
                "detail": {"count": len(trendslop_capsules), "pct": round(trendslop_pct, 3)},
            })
        if low_score_pct > 0.10:
            alerts.append({
                "level": "warning",
                "code": "LOW_QUALITY_HIGH",
                "message": f"Low-score capsules {low_score_pct:.1%} of pool (>{.10:.1%})",
                "detail": {"count": low_score, "pct": round(low_score_pct, 3)},
            })
        if zero_feedback_pct > 0.30:
            alerts.append({
                "level": "info",
                "code": "ZERO_FEEDBACK_HIGH",
                "message": f"{zero_feedback_pct:.1%} capsules have zero feedback",
                "detail": {"count": low_use, "pct": round(zero_feedback_pct, 3)},
            })
        if len(at_risk) > 0:
            alerts.append({
                "level": "critical",
                "code": "AT_RISK",
                "message": f"{len(at_risk)} capsule(s) at risk (streak≥2)",
                "detail": {"count": len(at_risk)},
            })
        if low_cred_pct > 0.20:
            alerts.append({
                "level": "warning",
                "code": "CREDIBILITY_LOW_HIGH",
                "message": f"Low-credibility capsules {low_cred_pct:.1%} of pool (>{.20:.1%})",
                "detail": {"count": credibility_buckets.get("low", 0), "pct": round(low_cred_pct, 3)},
            })
        if archived_pct > 0.60:
            alerts.append({
                "level": "warning",
                "code": "ARCHIVED_HIGH",
                "message": f"Archived {archived_pct:.1%} of pool (>60%)",
                "detail": {"count": status_counts.get("archived", 0), "pct": round(archived_pct, 3)},
            })
        if total >= 200 and evolved_count == 0:
            alerts.append({
                "level": "warning",
                "code": "EVOLUTION_STALLED",
                "message": "No evolved (V2) capsules after 200+ total",
                "detail": {"total": total, "evolved": evolved_count},
            })

        return {
            "total": total,
            "avg_score": round(avg_score, 3),
            "score_distribution": {
                "high (≥0.7)": high_score,
                "mid (0.4-0.7)": mid_score,
                "low (<0.4)": low_score,
            },
            "credibility_distribution": credibility_buckets,
            "trendslop": {
                "count": len(trendslop_capsules),
                "pct": round(trendslop_pct, 3),
                "top_reasons": dict(sorted(trendslop_reasons.items(), key=lambda x: x[1], reverse=True)[:3]),
            },
            "feedback_distribution": {
                "high_use (≥3)": high_use,
                "low_use (0)": low_use,
            },
            "generation_distribution": dict(sorted(generation_counts.items())),
            "status_breakdown": status_counts,
            "top_gap_types": dict(top_gap_types),
            "arxiv_categories": dict(sorted(arxiv_categories.items(), key=lambda x: x[1], reverse=True)[:5]),
            "at_risk_capsules": len(at_risk),
            "at_risk_detail": [
                {"capsule_id": c.capsule_id, "title": c.action_gap_title[:40], "streak": c.low_score_streak}
                for c in at_risk[:5]
            ],
            "alerts": alerts,
        }

    def recompute_credibility_all(self) -> Dict[str, int]:
        """Recompute credibility for all capsules using CredibilityScorer.

        One-time migration to unify the simplified _update_credibility formula with
        the full 4-dimension CredibilityScorer.compute_novelty_scores formula.

        Returns {"updated": N, "errors": M}.
        """
        from llm.insight.credibility import CredibilityScorer

        all_capsules = self._load_capsules()
        scorer = CredibilityScorer()
        scores = scorer.compute_novelty_scores(all_capsules)

        updated = 0
        errors = 0
        for capsule in all_capsules:
            score = scores.get(capsule.capsule_id)
            if score:
                capsule.credibility_score = score.overall
                capsule.trendslop = score.trendslop
                capsule.trendslop_reason = score.trendslop_reason
                capsule.credibility_badge = score.badge
                self.update_capsule(capsule)
                updated += 1
            else:
                errors += 1

        return {"updated": updated, "errors": errors}

    def close(self) -> None:
        """Close the database connection."""
        _close_conn()
