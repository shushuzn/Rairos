"""Patch script: add paper_code_trace methods to database.py"""
import re

with open('db/database.py', 'r', encoding='utf-8') as f:
    content = f.read()

# Check if already patched
if 'def upsert_paper_code_trace' in content:
    print('Already has upsert_paper_code_trace')
    exit(0)

methods = '''
    def upsert_paper_code_trace(
        self,
        paper_id: str,
        code_path: str,
        module_name: str,
        framework: str = "pytorch",
        total_code_lines: int = 0,
        tagged_lines: int = 0,
        untagged_ranges: list = None,
        unreferenced_sources: list = None,
        paper_section_refs: list = None,
        gap_ids: list = None,
        benchmark_pass_rate: float = 0.0,
    ) -> int:
        """Upsert a paper-code traceability record. Returns trace id."""
        try:
            cur = self.conn.cursor()
            cur.execute("SELECT id FROM paper_code_trace WHERE paper_id = ? AND code_path = ?",
                        (paper_id, code_path))
            row = cur.fetchone()
            now = datetime.now(timezone.utc).isoformat()
            if row:
                cur.execute("""
                    UPDATE paper_code_trace SET
                        module_name = ?, framework = ?, total_code_lines = ?,
                        tagged_lines = ?, untagged_ranges = ?,
                        unreferenced_sources = ?, paper_section_refs = ?,
                        gap_ids = ?, benchmark_pass_rate = ?
                    WHERE id = ?
                """, (
                    module_name, framework, total_code_lines, tagged_lines,
                    orjson.dumps(untagged_ranges or []).decode(),
                    orjson.dumps(unreferenced_sources or []).decode(),
                    orjson.dumps(paper_section_refs or []).decode(),
                    orjson.dumps(gap_ids or []).decode(),
                    benchmark_pass_rate, row["id"]
                ))
                trace_id = row["id"]
            else:
                cur.execute("""
                    INSERT INTO paper_code_trace
                        (paper_id, code_path, module_name, framework,
                         total_code_lines, tagged_lines, untagged_ranges,
                         unreferenced_sources, paper_section_refs,
                         gap_ids, benchmark_pass_rate, created_at)
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                """, (
                    paper_id, code_path, module_name, framework,
                    total_code_lines, tagged_lines,
                    orjson.dumps(untagged_ranges or []).decode(),
                    orjson.dumps(unreferenced_sources or []).decode(),
                    orjson.dumps(paper_section_refs or []).decode(),
                    orjson.dumps(gap_ids or []).decode(),
                    benchmark_pass_rate, now
                ))
                trace_id = cur.lastrowid
            return trace_id
        except sqlite3.Error as exc:
            raise DatabaseError(f"upsert_paper_code_trace failed: {exc}") from exc

    def get_paper_code_trace(self, paper_id: str) -> list:
        """Return all code traces for a paper_id, newest first."""
        try:
            cur = self.conn.cursor()
            cur.execute(
                "SELECT * FROM paper_code_trace WHERE paper_id = ? ORDER BY created_at DESC",
                (paper_id,)
            )
            rows = cur.fetchall()
            result = []
            for row in rows:
                d = dict(row)
                for col in ("untagged_ranges", "unreferenced_sources",
                             "paper_section_refs", "gap_ids"):
                    raw = d.get(col)
                    if raw and isinstance(raw, str):
                        try:
                            d[col] = orjson.loads(raw)
                        except Exception:
                            d[col] = []
                result.append(d)
            return result
        except sqlite3.Error as exc:
            raise DatabaseError(f"get_paper_code_trace failed: {exc}") from exc

    def list_paper_code_traces(self, limit: int = 100) -> list:
        """Return all code traces, newest first."""
        try:
            cur = self.conn.cursor()
            cur.execute(
                "SELECT t.*, p.title as paper_title FROM paper_code_trace t "
                "LEFT JOIN papers p ON t.paper_id = p.id "
                "ORDER BY t.created_at DESC LIMIT ?",
                (limit,)
            )
            rows = cur.fetchall()
            result = []
            for row in rows:
                d = dict(row)
                for col in ("untagged_ranges", "unreferenced_sources",
                             "paper_section_refs", "gap_ids"):
                    raw = d.get(col)
                    if raw and isinstance(raw, str):
                        try:
                            d[col] = orjson.loads(raw)
                        except Exception:
                            d[col] = []
                result.append(d)
            return result
        except sqlite3.Error as exc:
            raise DatabaseError(f"list_paper_code_traces failed: {exc}") from exc
'''

old_marker = '        except sqlite3.Error as e:\n            raise DatabaseError(f"get_papers_bulk failed: {e}") from e\n\n    # list_papers is defined'

if old_marker not in content:
    print('WARNING: marker not found')
    idx = content.find('get_papers_bulk failed')
    print(repr(content[idx:idx+250]))
else:
    content = content.replace(old_marker,
        '        except sqlite3.Error as e:\n'
        '            raise DatabaseError(f"get_papers_bulk failed: {e}") from e\n'
        + methods + '\n\n    # list_papers is defined')
    print('Patched OK')

with open('db/database.py', 'w', encoding='utf-8', newline='\n') as f:
    f.write(content)
print(f'File size: {len(content)}')
