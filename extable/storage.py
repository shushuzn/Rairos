"""Experiment Table SQLite storage."""

import sqlite3

import orjson
import uuid
from typing import Optional
from datetime import datetime
from pathlib import Path




class ExperimentDB:
    """SQLite-backed experiment table storage."""

    def __init__(self, db_path: Optional[str] = None):
        if db_path is None:
            db_path = "data/extable.db"
        self.db_path = db_path
        Path(self.db_path).parent.mkdir(parents=True, exist_ok=True)
        self._conn_cache: Optional[sqlite3.Connection] = None
        self._init_db()

    def _init_db(self):
        conn = self._conn()
        conn.execute("""
            CREATE TABLE IF NOT EXISTS extable_papers (
                paper_uid TEXT PRIMARY KEY,
                title TEXT,
                added_at TEXT
            )
        """)
        conn.execute("""
            CREATE TABLE IF NOT EXISTS extable_tables (
                id TEXT PRIMARY KEY,
                paper_uid TEXT NOT NULL,
                caption TEXT,
                metrics_json TEXT NOT NULL,
                datasets_json TEXT NOT NULL,
                models_json TEXT NOT NULL,
                baselines_json TEXT,
                ours_best_json TEXT,
                raw_table_json TEXT NOT NULL,
                added_at TEXT NOT NULL,
                FOREIGN KEY (paper_uid) REFERENCES extable_papers(paper_uid)
            )
        """)
        conn.execute("CREATE INDEX IF NOT EXISTS idx_tables_paper ON extable_tables(paper_uid)")
        conn.commit()

    def _conn(self) -> sqlite3.Connection:
        if self._conn_cache is None:
            self._conn_cache = sqlite3.connect(self.db_path, check_same_thread=False)
        return self._conn_cache

    def close(self):
        if self._conn_cache:
            self._conn_cache.close()
            self._conn_cache = None

    def _now(self) -> str:
        return datetime.utcnow().isoformat()

    def add_paper(self, paper_uid: str, title: str):
        conn = self._conn()
        conn.execute(
            "INSERT OR IGNORE INTO extable_papers (paper_uid, title, added_at) VALUES (?, ?, ?)",
            (paper_uid, title, self._now()),
        )
        conn.commit()

    def add_table(self, paper_uid: str, table_struct: dict,
                  raw_table: list[list[str]]) -> str:
        table_id = str(uuid.uuid4())
        conn = self._conn()
        conn.execute(
            "INSERT INTO extable_tables (id, paper_uid, caption, metrics_json, datasets_json, models_json, baselines_json, ours_best_json, raw_table_json, added_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            (
                table_id, paper_uid,
                table_struct.get("caption", ""),
                orjson.dumps(table_struct.get("metrics", [])).decode("utf-8"),
                orjson.dumps(table_struct.get("datasets", [])).decode("utf-8"),
                orjson.dumps(table_struct.get("models", [])).decode("utf-8"),
                orjson.dumps(table_struct.get("baselines", {})).decode("utf-8"),
                orjson.dumps(table_struct.get("ours_best", {})).decode("utf-8"),
                orjson.dumps(raw_table).decode("utf-8"),
                self._now(),
            ),
        )
        conn.commit()
        return table_id

    def get_paper_tables(self, paper_uid: str) -> list[dict]:
        conn = self._conn()
        rows = conn.execute(
            "SELECT * FROM extable_tables WHERE paper_uid=?", (paper_uid,),
        ).fetchall()
        return [self._row_to_table(r) for r in rows]

    def _row_to_table(self, row: tuple) -> dict:
        return {
            "id": row[0],
            "paper_uid": row[1],
            "caption": row[2],
            "metrics": orjson.loads(row[3]) if row[3] else [],
            "datasets": orjson.loads(row[4]) if row[4] else [],
            "models": orjson.loads(row[5]) if row[5] else [],
            "baselines": orjson.loads(row[6]) if row[6] else {},
            "ours_best": orjson.loads(row[7]) if row[7] else {},
            "raw_table": orjson.loads(row[8]) if row[8] else [],
            "added_at": row[9],
        }

    def search_tables(
        self,
        paper_uid: Optional[str] = None,
        metric: Optional[str] = None,
        dataset: Optional[str] = None,
        model: Optional[str] = None,
        min_value: Optional[float] = None,
    ) -> list[dict]:
        conn = self._conn()
        conditions = []
        params: list = []

        if paper_uid:
            conditions.append("paper_uid = ?")
            params.append(paper_uid)
        # Push metric/dataset/model filters into SQL via JSON LIKE
        # (case-insensitive using LOWER on both sides)
        if metric:
            conditions.append("LOWER(metrics_json) LIKE LOWER('%' || ? || '%')")
            params.append(metric)
        if dataset:
            conditions.append("LOWER(datasets_json) LIKE LOWER('%' || ? || '%')")
            params.append(dataset)
        if model:
            conditions.append("LOWER(models_json) LIKE LOWER('%' || ? || '%')")
            params.append(model)

        if conditions:
            where = "WHERE " + " AND ".join(conditions)
            rows = conn.execute(f"SELECT * FROM extable_tables {where}", params).fetchall()
        else:
            rows = conn.execute("SELECT * FROM extable_tables").fetchall()

        results = []
        for row in rows:
            t = self._row_to_table(row)
            # min_value requires parsing JSON floats — keep in Python
            if min_value is not None:
                vals = [m["value"] for m in t["metrics"]]
                if not any(v >= min_value for v in vals):
                    continue
            results.append(t)
        return results

    def export_to_csv(self, paper_uid: Optional[str] = None) -> str:
        conn = self._conn()
        if paper_uid:
            rows = conn.execute("SELECT * FROM extable_tables WHERE paper_uid=?", (paper_uid,)).fetchall()
        else:
            rows = conn.execute("SELECT * FROM extable_tables").fetchall()

        lines = ["paper_uid,table_id,caption,datasets,models,ours_best_value,ours_best_dataset"]
        for row in rows:
            t = self._row_to_table(row)
            ours = t.get("ours_best", {})
            lines.append(
                f"{t['paper_uid']},{t['id']},{repr(t.get('caption',''))},"
                f"{repr(','.join(t.get('datasets',[])))},"
                f"{repr(','.join(t.get('models',[])))},"
                f"{ours.get('value','')},{ours.get('dataset','')}"
            )
        return "\n".join(lines)

    def stats(self) -> dict:
        conn = self._conn()
        row = conn.execute(
            """
            SELECT
                (SELECT COUNT(*) FROM extable_papers) AS papers,
                (SELECT COUNT(*) FROM extable_tables) AS tables
            """
        ).fetchone()
        return {"papers": row[0], "tables": row[1]}
