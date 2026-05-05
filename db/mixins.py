"""Database mixins for Rairos — embeddings, chat, subscriptions, reviews.

Each mixin expects the host class to have:
    self._conn: sqlite3.Connection
    self._dict_factory(sql, rows) -> List[dict]
"""

from __future__ import annotations

import sqlite3
from typing import Any, Dict, List, Optional, Tuple


class EmbeddingMixin:
    """Vector embedding operations for semantic paper search."""

    def set_embedding(self, paper_id: str, vector: List[float]) -> bool:
        try:
            # Check if paper exists first
            row = self._conn.execute(
                "SELECT 1 FROM papers WHERE id = ?", (paper_id,)
            ).fetchone()
            if not row:
                return False
            import struct

            blob = struct.pack(f"{len(vector)}f", *vector)
            self._conn.execute(
                "INSERT OR REPLACE INTO embeddings (paper_id, vector, updated_at) VALUES (?, ?, datetime('now'))",
                (paper_id, blob),
            )
            self._conn.commit()
            return True
        except Exception:
            return False

    def get_embedding(self, paper_id: str) -> Optional[List[float]]:
        import struct

        row = self._conn.execute(
            "SELECT vector FROM embeddings WHERE paper_id = ?", (paper_id,)
        ).fetchone()
        if row:
            count = len(row[0]) // 4
            return list(struct.unpack(f"{count}f", row[0]))
        return None

    def get_embeddings_bulk(self, paper_ids: List[str]) -> dict[str, Optional[List[float]]]:
        import struct

        rows = self._conn.execute(
            "SELECT paper_id, vector FROM embeddings WHERE paper_id IN ({})".format(
                ",".join("?" * len(paper_ids))
            ),
            paper_ids,
        ).fetchall()
        result = {}
        for r in rows:
            if r[1]:
                count = len(r[1]) // 4
                result[r[0]] = list(struct.unpack(f"{count}f", r[1]))
        return result

    def get_papers_without_embeddings(self, limit: int = 1000) -> List["PaperRecord"]:
        from db.database import PaperRecord

        rows = self._conn.execute(
            "SELECT p.* FROM papers p LEFT JOIN embeddings e ON p.id = e.paper_id "
            "WHERE e.paper_id IS NULL AND p.title IS NOT NULL AND p.title != '' LIMIT ?",
            (limit,),
        ).fetchall()
        return [PaperRecord.from_row(r) for r in rows]

    def find_similar(
        self, paper_id: str, top_k: int = 10, threshold: float = 0.0, limit: int = 0
    ) -> List[Tuple[str, float]]:
        import struct

        row = self._conn.execute(
            "SELECT vector FROM embeddings WHERE paper_id = ?", (paper_id,)
        ).fetchone()
        if not row:
            return []
        import numpy as np

        count = len(row[0]) // 4
        target = np.array(struct.unpack(f"{count}f", row[0]), dtype=np.float32)
        rows = self._conn.execute(
            "SELECT paper_id, vector FROM embeddings WHERE paper_id != ?",
            (paper_id,),
        ).fetchall()
        scored = []
        for pid, blob in rows:
            cnt = len(blob) // 4
            vec = np.array(struct.unpack(f"{cnt}f", blob), dtype=np.float32)
            sim = float(np.dot(target, vec) / (np.linalg.norm(target) * np.linalg.norm(vec) + 1e-10))
            if sim >= threshold:
                scored.append((pid, sim))
        scored.sort(key=lambda x: x[1], reverse=True)
        # limit=0 means no limit (return all above threshold)
        return scored[:limit] if limit else scored[:top_k]

    def get_similarity(self, paper_id1: str, paper_id2: str) -> Optional[float]:
        e1 = self.get_embedding(paper_id1)
        e2 = self.get_embedding(paper_id2)
        if e1 and e2:
            import numpy as np

            v1, v2 = np.array(e1), np.array(e2)
            return float(np.dot(v1, v2) / (np.linalg.norm(v1) * np.linalg.norm(v2) + 1e-10))
        return None

    def get_embedding_stats(self) -> dict:
        with_embedding = self._conn.execute(
            "SELECT COUNT(*) FROM embeddings"
        ).fetchone()[0]
        total_with_text = self._conn.execute(
            "SELECT COUNT(*) FROM papers WHERE title IS NOT NULL AND title != ''"
        ).fetchone()[0]
        return {
            "with_embedding": with_embedding,
            "total_with_text": total_with_text,
        }


class ChatMixin:
    """Chat session persistence."""

    def create_chat_session(self, session_id: str, title: str = "") -> None:
        self._conn.execute(
            "INSERT OR IGNORE INTO chat_sessions (session_id, title, created_at) VALUES (?, ?, datetime('now'))",
            (session_id, title),
        )
        self._conn.commit()

    def add_chat_message(
        self, session_id: str, role: str, content: str, sources: Optional[str] = None
    ) -> None:
        self._conn.execute(
            "INSERT INTO chat_messages (session_id, role, content, sources, created_at) VALUES (?, ?, ?, ?, datetime('now'))",
            (session_id, role, content, sources),
        )
        self._conn.commit()

    def get_chat_sessions(self, limit: int = 20) -> List[dict]:
        rows = self._conn.execute(
            "SELECT session_id, title, created_at, "
            "(SELECT content FROM chat_messages WHERE session_id = cs.session_id ORDER BY created_at DESC LIMIT 1) AS last_message "
            "FROM chat_sessions cs ORDER BY created_at DESC LIMIT ?",
            (limit,),
        ).fetchall()
        return self._dict_factory(
            ["session_id", "title", "created_at", "last_message"], rows
        )

    def get_chat_messages(self, session_id: str) -> List[dict]:
        rows = self._conn.execute(
            "SELECT id, role, content, sources, created_at FROM chat_messages "
            "WHERE session_id = ? ORDER BY created_at ASC",
            (session_id,),
        ).fetchall()
        return self._dict_factory(
            ["id", "role", "content", "sources", "created_at"], rows
        )

    def delete_chat_session(self, session_id: str) -> None:
        self._conn.execute("DELETE FROM chat_messages WHERE session_id = ?", (session_id,))
        self._conn.execute("DELETE FROM chat_sessions WHERE session_id = ?", (session_id,))
        self._conn.commit()

    def search_chat_sessions(self, query: str, limit: int = 20) -> List[dict]:
        rows = self._conn.execute(
            "SELECT DISTINCT cs.session_id, cs.title, cs.created_at, "
            "(SELECT content FROM chat_messages WHERE session_id = cs.session_id ORDER BY created_at DESC LIMIT 1) AS last_message "
            "FROM chat_sessions cs JOIN chat_messages cm ON cs.session_id = cm.session_id "
            "WHERE cm.content LIKE ? ORDER BY cs.created_at DESC LIMIT ?",
            (f"%{query}%", limit),
        ).fetchall()
        return self._dict_factory(
            ["session_id", "title", "created_at", "last_message"], rows
        )

    def update_chat_session_title(self, session_id: str, title: str) -> None:
        self._conn.execute(
            "UPDATE chat_sessions SET title = ? WHERE session_id = ?",
            (title, session_id),
        )
        self._conn.commit()


class SubscriptionMixin:
    """arXiv subscription management."""

    def add_arxiv_subscription(
        self, topic: str, categories: str = "", keywords: str = ""
    ) -> int:
        cursor = self._conn.execute(
            "INSERT INTO arxiv_subscriptions (topic, categories, keywords, enabled, created_at) "
            "VALUES (?, ?, ?, 1, datetime('now'))",
            (topic, categories, keywords),
        )
        self._conn.commit()
        return cursor.lastrowid

    def list_arxiv_subscriptions(self) -> List[dict]:
        rows = self._conn.execute(
            "SELECT id, topic, categories, keywords, enabled, last_check_id, created_at "
            "FROM arxiv_subscriptions ORDER BY created_at DESC"
        ).fetchall()
        return self._dict_factory(
            ["id", "topic", "categories", "keywords", "enabled", "last_check_id", "created_at"],
            rows,
        )

    def get_arxiv_subscription(self, sub_id: str) -> Optional[dict]:
        row = self._conn.execute(
            "SELECT id, topic, categories, keywords, enabled, last_check_id, created_at "
            "FROM arxiv_subscriptions WHERE id = ?",
            (sub_id,),
        ).fetchone()
        if row:
            return self._dict_factory(
                ["id", "topic", "categories", "keywords", "enabled", "last_check_id", "created_at"],
                [row],
            )[0]
        return None

    def delete_arxiv_subscription(self, sub_id: str) -> bool:
        self._conn.execute("DELETE FROM arxiv_subscriptions WHERE id = ?", (sub_id,))
        self._conn.commit()
        return True

    def update_subscription_last_check(self, sub_id: str, last_check_id: str) -> None:
        self._conn.execute(
            "UPDATE arxiv_subscriptions SET last_check_id = ? WHERE id = ?",
            (last_check_id, sub_id),
        )
        self._conn.commit()

    def record_subscription_paper(
        self, sub_id: int, paper_id: str, title: str, published: str
    ) -> None:
        self._conn.execute(
            "INSERT OR IGNORE INTO subscription_papers (sub_id, paper_id, title, published, discovered_at) "
            "VALUES (?, ?, ?, ?, datetime('now'))",
            (sub_id, paper_id, title, published),
        )
        self._conn.commit()

    def get_subscription_papers(
        self, sub_id: int, limit: int = 50
    ) -> List[dict]:
        rows = self._conn.execute(
            "SELECT paper_id, title, published, discovered_at FROM subscription_papers "
            "WHERE sub_id = ? ORDER BY discovered_at DESC LIMIT ?",
            (sub_id, limit),
        ).fetchall()
        return self._dict_factory(
            ["paper_id", "title", "published", "discovered_at"], rows
        )

    def get_recent_subscription_papers_grouped(
        self, limit_per: int = 5
    ) -> Dict[str, List[dict]]:
        rows = self._conn.execute(
            "SELECT sp.sub_id, s.topic, sp.paper_id, sp.title, sp.published, sp.discovered_at "
            "FROM subscription_papers sp "
            "JOIN arxiv_subscriptions s ON sp.sub_id = s.id "
            "ORDER BY sp.discovered_at DESC"
        ).fetchall()
        result: Dict[str, List[dict]] = {}
        for r in rows:
            topic = r[1]
            if topic not in result:
                result[topic] = []
            if len(result[topic]) < limit_per:
                result[topic].append(
                    {"paper_id": r[2], "title": r[3], "published": r[4], "discovered_at": r[5]}
                )
        return result


class LiteratureMixin:
    """Literature review storage."""

    def add_literature_review(
        self, review_id: str, topic: str, content: str, paper_ids: Optional[str] = None
    ) -> bool:
        try:
            self._conn.execute(
                "INSERT INTO literature_reviews (review_id, topic, content, paper_ids, created_at) "
                "VALUES (?, ?, ?, ?, datetime('now'))",
                (review_id, topic, content, paper_ids),
            )
            self._conn.commit()
            return True
        except Exception:
            return False

    def list_literature_reviews(self) -> List[dict]:
        rows = self._conn.execute(
            "SELECT review_id, topic, created_at, "
            "LENGTH(content) - LENGTH(REPLACE(content, ' ', '')) + 1 AS word_count "
            "FROM literature_reviews ORDER BY created_at DESC"
        ).fetchall()
        return self._dict_factory(
            ["review_id", "topic", "created_at", "word_count"], rows
        )

    def get_literature_review(self, review_id: str) -> Optional[dict]:
        row = self._conn.execute(
            "SELECT review_id, topic, content, paper_ids, created_at FROM literature_reviews WHERE review_id = ?",
            (review_id,),
        ).fetchone()
        if row:
            return self._dict_factory(
                ["review_id", "topic", "content", "paper_ids", "created_at"], [row]
            )[0]
        return None

    def update_literature_review(
        self, review_id: str, content: Optional[str] = None, paper_ids: Optional[str] = None
    ) -> bool:
        updates = []
        params = []
        if content is not None:
            updates.append("content = ?")
            params.append(content)
        if paper_ids is not None:
            updates.append("paper_ids = ?")
            params.append(paper_ids)
        if not updates:
            return False
        params.append(review_id)
        self._conn.execute(
            f"UPDATE literature_reviews SET {', '.join(updates)} WHERE review_id = ?",
            params,
        )
        self._conn.commit()
        return True

    def delete_literature_review(self, review_id: str) -> bool:
        self._conn.execute("DELETE FROM literature_reviews WHERE review_id = ?", (review_id,))
        self._conn.commit()
        return True
