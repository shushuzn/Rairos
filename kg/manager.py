"""SQLite-based Knowledge Graph Manager with adjacency list."""

import json
import sqlite3
import uuid
from datetime import datetime
from pathlib import Path
from typing import Any, Optional


class KGManager:
    """Manages a SQLite-backed knowledge graph with nodes and edges.

    Node types: Paper, P-Note, C-Note, M-Note, Tag, Author
    Edge types: cite, derive, same_tag, in_comparison, has_note, about_tag
    """

    def __init__(self, db_path: str | None = None):
        if db_path is None:
            base = Path(__file__).parent.parent
            db_path = base / "data" / "kg.db"
        self.db_path = str(db_path)
        self._ensure_data_dir()
        self._init_db()

    def _ensure_data_dir(self):
        Path(self.db_path).parent.mkdir(parents=True, exist_ok=True)

    def _init_db(self):
        conn = self._conn()
        conn.execute("""
            CREATE TABLE IF NOT EXISTS kg_nodes (
                id TEXT PRIMARY KEY,
                type TEXT NOT NULL,
                entity_id TEXT NOT NULL,
                label TEXT NOT NULL,
                properties_json TEXT NOT NULL DEFAULT '{}',
                created_at TEXT NOT NULL,
                UNIQUE(type, entity_id)
            )
        """)
        conn.execute("""
            CREATE TABLE IF NOT EXISTS kg_edges (
                id TEXT PRIMARY KEY,
                source_id TEXT NOT NULL,
                target_id TEXT NOT NULL,
                relation_type TEXT NOT NULL,
                weight REAL NOT NULL DEFAULT 1.0,
                properties_json TEXT NOT NULL DEFAULT '{}',
                created_at TEXT NOT NULL,
                FOREIGN KEY (source_id) REFERENCES kg_nodes(id),
                FOREIGN KEY (target_id) REFERENCES kg_nodes(id)
            )
        """)
        conn.execute("CREATE INDEX IF NOT EXISTS idx_nodes_type ON kg_nodes(type)")
        conn.execute("CREATE INDEX IF NOT EXISTS idx_edges_rel_type ON kg_edges(relation_type)")
        conn.execute("CREATE INDEX IF NOT EXISTS idx_edges_source ON kg_edges(source_id)")
        conn.execute("CREATE INDEX IF NOT EXISTS idx_edges_target ON kg_edges(target_id)")
        conn.execute("CREATE INDEX IF NOT EXISTS idx_nodes_entity ON kg_nodes(type, entity_id)")
        conn.commit()

    def _conn(self) -> sqlite3.Connection:
        return sqlite3.connect(self.db_path, check_same_thread=False)

    def _now(self) -> str:
        return datetime.utcnow().isoformat()

    # ─── Node Operations ──────────────────────────────────────────────

    def add_node(
        self,
        node_type: str,
        entity_id: str,
        label: str,
        **properties,
    ) -> str:
        """Add a node. Returns existing node_id if (type, entity_id) already exists."""
        conn = self._conn()
        existing = conn.execute(
            "SELECT id FROM kg_nodes WHERE type=? AND entity_id=?",
            (node_type, entity_id),
        ).fetchone()
        if existing:
            return existing[0]

        node_id = str(uuid.uuid4())
        props = json.dumps(properties, ensure_ascii=False)
        now = self._now()
        conn.execute(
            "INSERT INTO kg_nodes (id, type, entity_id, label, properties_json, created_at) VALUES (?, ?, ?, ?, ?, ?)",
            (node_id, node_type, entity_id, label, props, now),
        )
        conn.commit()
        return node_id

    def upsert_node(
        self,
        node_type: str,
        entity_id: str,
        label: str,
        **properties,
    ) -> str:
        """Update or insert node properties."""
        conn = self._conn()
        existing = conn.execute(
            "SELECT id, properties_json FROM kg_nodes WHERE type=? AND entity_id=?",
            (node_type, entity_id),
        ).fetchone()
        now = self._now()
        if existing:
            old_props = json.loads(existing[1]) if existing[1] else {}
            old_props.update(properties)
            props = json.dumps(old_props, ensure_ascii=False)
            conn.execute(
                "UPDATE kg_nodes SET label=?, properties_json=?, created_at=? WHERE id=?",
                (label, props, now, existing[0]),
            )
            conn.commit()
            return existing[0]
        return self.add_node(node_type, entity_id, label, **properties)

    def get_node(self, node_id: str) -> Optional[dict]:
        conn = self._conn()
        row = conn.execute("SELECT * FROM kg_nodes WHERE id=?", (node_id,)).fetchone()
        if not row:
            return None
        return self._row_to_node(row)

    def get_node_by_entity(self, node_type: str, entity_id: str) -> Optional[dict]:
        conn = self._conn()
        row = conn.execute(
            "SELECT * FROM kg_nodes WHERE type=? AND entity_id=?",
            (node_type, entity_id),
        ).fetchone()
        if not row:
            return None
        return self._row_to_node(row)

    def _row_to_node(self, row: tuple) -> dict:
        return {
            "id": row[0],
            "type": row[1],
            "entity_id": row[2],
            "label": row[3],
            "properties": json.loads(row[4]) if row[4] else {},
            "created_at": row[5],
        }

    def get_all_nodes(self, node_type: Optional[str] = None) -> list[dict]:
        conn = self._conn()
        if node_type:
            rows = conn.execute(
                "SELECT * FROM kg_nodes WHERE type=? ORDER BY created_at DESC",
                (node_type,),
            ).fetchall()
        else:
            rows = conn.execute("SELECT * FROM kg_nodes ORDER BY created_at DESC").fetchall()
        return [self._row_to_node(r) for r in rows]

    # ─── Edge Operations ──────────────────────────────────────────────

    def add_edge(
        self,
        source_id: str,
        target_id: str,
        relation_type: str,
        weight: float = 1.0,
        **properties,
    ) -> str:
        """Add an edge. Returns existing edge_id if exact duplicate exists."""
        conn = self._conn()
        existing = conn.execute(
            "SELECT id FROM kg_edges WHERE source_id=? AND target_id=? AND relation_type=?",
            (source_id, target_id, relation_type),
        ).fetchone()
        if existing:
            return existing[0]

        edge_id = str(uuid.uuid4())
        props = json.dumps(properties, ensure_ascii=False)
        now = self._now()
        conn.execute(
            "INSERT INTO kg_edges (id, source_id, target_id, relation_type, weight, properties_json, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
            (edge_id, source_id, target_id, relation_type, weight, props, now),
        )
        conn.commit()
        return edge_id

    def get_edge(self, edge_id: str) -> Optional[dict]:
        conn = self._conn()
        row = conn.execute("SELECT * FROM kg_edges WHERE id=?", (edge_id,)).fetchone()
        if not row:
            return None
        return self._row_to_edge(row)

    def _row_to_edge(self, row: tuple) -> dict:
        return {
            "id": row[0],
            "source_id": row[1],
            "target_id": row[2],
            "relation_type": row[3],
            "weight": row[4],
            "properties": json.loads(row[5]) if row[5] else {},
            "created_at": row[6],
        }

    def get_edges_by_node(
        self,
        node_id: str,
        direction: str = "both",
        rel_type: Optional[str] = None,
    ) -> list[dict]:
        """direction: 'out' (source), 'in' (target), 'both'"""
        conn = self._conn()
        if direction == "out":
            clause = "source_id=? AND (? IS NULL OR relation_type=?)"
            params = (node_id, rel_type, rel_type)
        elif direction == "in":
            clause = "target_id=? AND (? IS NULL OR relation_type=?)"
            params = (node_id, rel_type, rel_type)
        else:
            clause = "(source_id=? OR target_id=?) AND (? IS NULL OR relation_type=?)"
            params = (node_id, node_id, rel_type, rel_type)

        rows = conn.execute(
            f"SELECT * FROM kg_edges WHERE {clause}", params,
        ).fetchall()
        return [self._row_to_edge(r) for r in rows]

    # ─── Graph Queries ────────────────────────────────────────────────

    def find_neighbors(
        self,
        node_id: str,
        depth: int = 1,
        relation_type: Optional[str] = None,
    ) -> list[tuple[dict, dict, int]]:
        """BFS: return [(neighbor_node, edge, depth), ...]"""
        result = []
        visited = {node_id}
        queue = [(node_id, 0)]

        while queue:
            current_id, d = queue.pop(0)
            if d >= depth:
                continue

            edges = self.get_edges_by_node(current_id, direction="both", rel_type=relation_type)
            for edge in edges:
                neighbor_id = edge["target_id"] if edge["source_id"] == current_id else edge["source_id"]
                if neighbor_id in visited:
                    continue
                visited.add(neighbor_id)
                neighbor = self.get_node(neighbor_id)
                if neighbor:
                    result.append((neighbor, edge, d + 1))
                    queue.append((neighbor_id, d + 1))

        return result

    def find_shortest_path(self, idA: str, idB: str) -> Optional[list[str]]:
        """BFS shortest path between two nodes. Returns list of node_ids or None."""
        if idA == idB:
            return [idA]

        visited = {idA}
        queue = [(idA, [idA])]

        while queue:
            current_id, path = queue.pop(0)
            edges = self.get_edges_by_node(current_id, direction="both")
            for edge in edges:
                neighbor_id = edge["target_id"] if edge["source_id"] == current_id else edge["source_id"]
                if neighbor_id in visited:
                    continue
                new_path = path + [neighbor_id]
                if neighbor_id == idB:
                    return new_path
                visited.add(neighbor_id)
                queue.append((neighbor_id, new_path))

        return None

    def find_papers_by_tag(self, tag: str) -> list[dict]:
        """Find all Paper nodes with a given tag via same_tag edges."""
        conn = self._conn()
        rows = conn.execute(
            """SELECT DISTINCT n.* FROM kg_nodes n
               JOIN kg_edges e ON (e.target_id = n.id OR e.source_id = n.id)
               JOIN kg_nodes tag ON (tag.id = e.target_id OR tag.id = e.source_id)
               WHERE n.type = 'Paper'
                 AND tag.type = 'Tag'
                 AND tag.label = ?
                 AND e.relation_type = 'same_tag'""",
            (tag,),
        ).fetchall()
        return [self._row_to_node(r) for r in rows]

    def find_comparison_group(self, mnote_id: str) -> list[dict]:
        """Find all Paper nodes that are in the same M-Note comparison group."""
        conn = self._conn()
        rows = conn.execute(
            """SELECT DISTINCT n.* FROM kg_nodes n
               JOIN kg_edges e ON (e.target_id = n.id OR e.source_id = n.id)
               WHERE n.type = 'Paper'
                 AND e.relation_type = 'in_comparison'
                 AND (e.target_id = ? OR e.source_id = ?)""",
            (mnote_id, mnote_id),
        ).fetchall()
        return [self._row_to_node(r) for r in rows]

    def find_mnotes_by_tag(self, tag: str) -> list[dict]:
        """Find all M-Note nodes mentioning a tag."""
        conn = self._conn()
        rows = conn.execute(
            """SELECT * FROM kg_nodes
               WHERE type = 'M-Note'
                 AND label LIKE '%' || ? || '%'""",
            (tag,),
        ).fetchall()
        return [self._row_to_node(r) for r in rows]

    # ─── Stats ────────────────────────────────────────────────────────

    def stats(self) -> dict:
        conn = self._conn()
        node_counts = dict(conn.execute(
            "SELECT type, COUNT(*) FROM kg_nodes GROUP BY type"
        ).fetchall())
        edge_counts = dict(conn.execute(
            "SELECT relation_type, COUNT(*) FROM kg_edges GROUP BY relation_type"
        ).fetchall())
        total_nodes = sum(node_counts.values())
        total_edges = sum(edge_counts.values())
        return {
            "total_nodes": total_nodes,
            "total_edges": total_edges,
            "nodes_by_type": node_counts,
            "edges_by_type": edge_counts,
        }
