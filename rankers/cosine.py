"""Cosine similarity ranking strategy."""
from __future__ import annotations

import struct
from typing import List

from rankers.base import RankedResult, Ranker


class CosineSimilarityRanker(Ranker):
    """
    Rank papers by cosine similarity of their embedding vectors.

    Inherits db reference and embedding fetch from the database layer.
    """

    name = "cosine"

    def __init__(self, db: "Database") -> None:  # noqa: F821
        self._db = db

    def rank(
        self,
        paper_id: str,
        threshold: float = 0.85,
        limit: int = 20,
    ) -> List[RankedResult]:
        """
        Find papers similar to paper_id by cosine similarity of embeddings.

        Args:
            paper_id: Query paper ID
            threshold: Minimum cosine similarity to include (default 0.85)
            limit: Maximum number of results (default 20)

        Returns:
            List of (PaperRecord, cosine_similarity) sorted by score descending
        """
        query_emb = self._db.get_embedding(paper_id)
        if query_emb is None:
            return []

        q_vec = query_emb
        q_norm = sum(x * x for x in q_vec) ** 0.5
        if q_norm == 0:
            return []

        cur = self._db.conn.cursor()
        cur.execute(
            "SELECT id, embed_vector FROM papers WHERE id != ? AND embed_vector IS NOT NULL",
            (paper_id,),
        )

        results: List[RankedResult] = []
        for row in cur.fetchall():
            pid, blob = row["id"], row["embed_vector"]
            if blob is None:
                continue
            c_count = len(blob) // 4
            c_vec = list(struct.unpack(f"{c_count}f", blob))
            c_norm = sum(x * x for x in c_vec) ** 0.5
            if c_norm == 0:
                continue
            dot = sum(a * b for a, b in zip(q_vec, c_vec))
            sim = dot / (q_norm * c_norm)
            if sim >= threshold:
                cur2 = self._db.conn.cursor()
                cur2.execute("SELECT * FROM papers WHERE id = ?", (pid,))
                prow = cur2.fetchone()
                if prow:
                    from db.database import PaperRecord

                    results.append((PaperRecord.from_row(prow), sim))

        results.sort(key=lambda x: x[1], reverse=True)
        return results[:limit]
