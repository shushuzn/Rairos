"""Patch: add gap_history table + methods to database.py"""
import hashlib

with open('db/database.py', 'r', encoding='utf-8') as f:
    content = f.read()

changes = 0

# 1. Add gap_history table before Search Data Classes
if 'CREATE TABLE IF NOT EXISTS gap_history' not in content:
    old_marker = '# ─── Search Data Classes ────────────────────────────────────────────────────────'
    gap_history_table = '''# ─── Gap History for Incremental Reporting ──────────────────────────────────

CREATE TABLE IF NOT EXISTS gap_history (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    topic           TEXT    NOT NULL,
    session_id      TEXT    NOT NULL,
    gap_type        TEXT    NOT NULL,
    gap_title_hash  TEXT    NOT NULL,      -- sha256(gap.title) for dedup
    gap_title       TEXT    NOT NULL,
    gap_hash        TEXT    NOT NULL,       -- sha256(topic+gap_type+title) for dedup
    novelty_score  REAL    DEFAULT 0.0,
    priority        INTEGER DEFAULT 0,
    created_at      TEXT    NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_gap_history_topic ON gap_history(topic);
CREATE INDEX IF NOT EXISTS idx_gap_history_hash  ON gap_history(gap_hash);
CREATE INDEX IF NOT EXISTS idx_gap_history_session ON gap_history(session_id);

'''

    content = content.replace(old_marker, gap_history_table + old_marker, 1)
    changes += 1
    print('Added gap_history table')
else:
    print('gap_history table already exists')

# 2. Add methods after list_paper_code_traces
if 'def get_session_gap_count' not in content:
    old_marker = '    def list_paper_code_traces('
    new_methods = '''    # ── Gap History ──────────────────────────────────────────────────────────────

    def record_gap_history(
        self,
        topic: str,
        session_id: str,
        gaps: list,
    ) -> int:
        """Record accepted gaps from a research session.

        Returns number of gaps recorded.
        """
        import hashlib
        from datetime import datetime

        count = 0
        now = datetime.utcnow().isoformat()
        conn = self.conn
        for gap in gaps:
            if not getattr(gap, 'accepted', False):
                continue
            title = getattr(gap, 'title', '') or str(gap)
            gap_type = str(getattr(gap, 'gap_type', '') or '')
            title_hash = hashlib.sha256(title.encode()).hexdigest()[:16]
            gap_hash = hashlib.sha256(f"{topic}{gap_type}{title}".encode()).hexdigest()[:16]
            novelty = getattr(gap, 'novelty_score', 0.0) or 0.0
            priority = getattr(gap, 'priority', 0) or 0
            try:
                conn.execute(
                    """INSERT INTO gap_history
                       (topic, session_id, gap_type, gap_title_hash, gap_title, gap_hash, novelty_score, priority, created_at)
                       VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)""",
                    (topic, session_id, gap_type, title_hash, title, gap_hash,
                     novelty, priority, now),
                )
                count += 1
            except Exception:
                pass  # skip duplicates
        conn.commit()
        return count

    def filter_new_gaps(
        self,
        topic: str,
        gaps: list,
        similarity_threshold: float = 0.7,
    ) -> tuple[list, dict]:
        """Filter gaps against gap_history, returning only new/significant ones.

        A gap is "new" if:
        - Its gap_hash is not in gap_history for this topic, OR
        - Its novelty_score is in the top 20% of all seen gaps for this topic

        Returns (new_gaps, stats_dict).
        """
        import hashlib

        if not gaps:
            return [], {"total": 0, "new": 0, "seen": 0, "suppressed": 0}

        # Load known hashes for this topic
        conn = self.conn
        rows = conn.execute(
            "SELECT gap_hash, novelty_score FROM gap_history WHERE topic = ? ORDER BY novelty_score DESC",
            (topic,),
        ).fetchall()
        known_hashes = {r[0]: r[1] for r in rows}
        seen_count = len(known_hashes)

        if seen_count > 0:
            # Top 20% novelty threshold from history
            sorted_novelties = sorted(known_hashes.values(), reverse=True)
            top_pct_idx = max(0, int(len(sorted_novelties) * 0.2))
            novelty_threshold = sorted_novelties[top_pct_idx] if sorted_novelties else 0.0
        else:
            novelty_threshold = 0.0

        new_gaps = []
        suppressed = 0
        for gap in gaps:
            if not getattr(gap, 'accepted', False):
                continue
            title = getattr(gap, 'title', '') or str(gap)
            gap_type = str(getattr(gap, 'gap_type', '') or '')
            gap_hash = hashlib.sha256(f"{topic}{gap_type}{title}".encode()).hexdigest()[:16]
            novelty = getattr(gap, 'novelty_score', 0.0) or 0.0

            is_new = gap_hash not in known_hashes
            is_top_novelty = novelty >= novelty_threshold and novelty > 0.0

            if is_new or is_top_novelty:
                new_gaps.append(gap)
            else:
                suppressed += 1

        stats = {
            "total": len(gaps),
            "new": len(new_gaps),
            "seen": seen_count,
            "suppressed": suppressed,
        }
        return new_gaps, stats

    def get_session_gap_count(self, topic: str, session_id: str) -> int:
        """Return count of recorded gaps for a session."""
        row = self.conn.execute(
            "SELECT COUNT(*) FROM gap_history WHERE topic = ? AND session_id = ?",
            (topic, session_id),
        ).fetchone()
        return row[0] if row else 0

    def get_latest_session_id(self, topic: str) -> str | None:
        """Return the most recent session_id for a topic."""
        row = self.conn.execute(
            "SELECT session_id FROM gap_history WHERE topic = ? ORDER BY created_at DESC LIMIT 1",
            (topic,),
        ).fetchone()
        return row[0] if row else None

    def list_paper_code_traces('''

    content = content.replace(old_marker, new_methods, 1)
    changes += 1
    print('Added gap history methods')
else:
    print('gap history methods already exist')

with open('db/database.py', 'w', encoding='utf-8', newline='\n') as f:
    f.write(content)
print(f'Changes: {changes}/2, File size: {len(content)}')
