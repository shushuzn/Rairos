"""Cosine similarity ranking strategy — numpy-accelerated."""
from __future__ import annotations

from typing import TYPE_CHECKING, List

if TYPE_CHECKING:
    from db.database import Database

import numpy as np

from rankers.base import RankedResult, Ranker


class CosineSimilarityRanker(Ranker):
    """
    Rank papers by cosine similarity of their embedding vectors.

    Uses numpy for batch vector operations — O(n) scan but all similarity
    computation happens in a single vectorized pass instead of per-row Python loops.
    """

    name = "cosine"

    def __init__(self, db: "Database") -> None:
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

        q_vec = np.array(query_emb, dtype=np.float32)
        q_norm = np.linalg.norm(q_vec)
        if q_norm == 0:
            return []

        cur = self._db.conn.cursor()
        # Fetch embeddings + all paper columns in a single query — no per-result round-trip
        cur.execute(
            """SELECT id, embed_vector, *
               FROM papers
               WHERE id != ? AND embed_vector IS NOT NULL""",
            (paper_id,),
        )
        rows = cur.fetchall()
        if not rows:
            return []

        col_names = [d[0] for d in cur.description]
        idx_emb = col_names.index("embed_vector")

        # Batch unpack all embeddings into a 2D numpy array
        embeddings: List[np.ndarray] = []
        orig_indices: List[int] = []   # maps array index → original row index
        for i, row in enumerate(rows):
            blob = row[idx_emb]
            if blob is None:
                continue
            vec = np.frombuffer(blob, dtype=np.float32, count=len(blob) // 4)
            embeddings.append(vec)
            orig_indices.append(i)

        if not embeddings:
            return []

        emb_matrix = np.stack(embeddings)                     # (N, D)
        norms = np.linalg.norm(emb_matrix, axis=1)             # (N,)
        nonzero = norms > 0

        # Compute cosine similarities only for non-zero-norm rows
        nz_matrix = emb_matrix[nonzero]
        nz_norms = norms[nonzero]

        # Prevent div-by-zero in similarity: q_norm is already checked above
        nz_sims = (nz_matrix @ q_vec) / (nz_norms * q_norm)   # (M,)

        # Build full (N,) sims array with -inf for zero-norm rows
        sims = np.full(len(nonzero), -np.inf, dtype=np.float32)
        sims[nonzero] = nz_sims

        # Collect all above threshold
        threshold_sims: List[tuple] = []  # (sim, orig_idx)
        for j, sim in enumerate(sims):
            if sim >= threshold:
                threshold_sims.append((float(sim), orig_indices[j]))

        threshold_sims.sort(key=lambda x: x[0], reverse=True)
        top = threshold_sims[:limit]

        from db.database import PaperRecord
        results: List[RankedResult] = []
        for sim, orig_idx in top:
            row = rows[orig_idx]
            results.append((PaperRecord.from_row(row), sim))

        return results
