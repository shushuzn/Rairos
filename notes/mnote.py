"""M-Note (comparison note) management."""
import re
from pathlib import Path
from typing import List, Optional, Tuple

from core.basics import read_text, write_text
from renderers.mnote import render_mnote


def pick_top3_pnotes_for_tag(tag: str, tag_map: dict) -> Optional[List[Path]]:
    items = tag_map.get(tag, [])
    if len(items) < 3:
        return None
    return [items[0][1], items[1][1], items[2][1]]


def mnote_filename(tag: str, a: Path, b: Path, c: Path) -> str:
    def short(stem: str, n: int = 19) -> str:
        s = re.sub(r"^P\s*-\s*\d{4}\s*-\s*", "", stem).strip()
        if len(s) <= n:
            return s
        # Truncate to n-5 to make room for 5-char hash suffix, preventing
        # collision when two titles differ only after position n.
        truncated = s[: n - 5].rstrip("-_ ")
        suffix = format(hash(s) % 100000, "05d")
        return f"{truncated}~{suffix}"

    A = short(a.stem)
    B = short(b.stem)
    C = short(c.stem)
    return f"M - {tag} - {A} vs {B} vs {C}.md"


def parse_current_abc(md: str) -> Tuple[Optional[str], Optional[str], Optional[str]]:
    def find(label: str) -> Optional[str]:
        m = re.search(rf"^\-\s*{label}:\s*(.+)\s*$", md, flags=re.M)
        return m.group(1).strip() if m else None

    return find("A"), find("B"), find("C")


def append_view_evolution_log(md: str, old_abc: Tuple[str, str, str], new_abc: Tuple[str, str, str]) -> str:
    from core import today_iso
    today = today_iso()
    block = f"""
* {today}

  * 旧观点：A/B/C = {old_abc[0]} / {old_abc[1]} / {old_abc[2]}
  * 新证据：新增/更新同主题论文，A/B/C 刷新为 {new_abc[0]} / {new_abc[1]} / {new_abc[2]}
  * 更新结论：

"""
    m = re.search(r"^##\s+View Evolution Log\s*$", md, flags=re.M)
    if not m:
        return md.rstrip() + "\n\n## View Evolution Log\n" + block

    insert_pos = m.end()
    return md[:insert_pos] + "\n" + block + md[insert_pos:]


def ensure_or_update_mnote(mnote_dir: Path, tag: str, top3: List[Path]) -> Optional[Path]:
    mnote_dir.mkdir(parents=True, exist_ok=True)
    if len(top3) < 3:
        return None

    existing = sorted([p for p in mnote_dir.glob(f"M - {tag} - *.md") if p.is_file()])
    a, b, c = top3
    newA, newB, newC = a.stem, b.stem, c.stem

    if not existing:
        fname = mnote_filename(tag, a, b, c)
        path = mnote_dir / fname
        title = f"{tag}: {newA} vs {newB} vs {newC}"
        write_text(path, render_mnote(title, newA, newB, newC))
        return path

    path = existing[0]
    md = read_text(path)
    curA, curB, curC = parse_current_abc(md)

    if not (curA and curB and curC):
        md2 = md.rstrip() + f"\n\n---\n\n## 当前 A/B/C（自动补齐）\n\n- A: {newA}\n- B: {newB}\n- C: {newC}\n"
        write_text(path, md2)
        return path

    if (curA, curB, curC) != (newA, newB, newC):
        md2 = re.sub(r"^\-\s*A:\s*.*$", f"- A: {newA}", md, flags=re.M)
        md2 = re.sub(r"^\-\s*B:\s*.*$", f"- B: {newB}", md2, flags=re.M)
        md2 = re.sub(r"^\-\s*C:\s*.*$", f"- C: {newC}", md2, flags=re.M)
        md2 = append_view_evolution_log(md2, (curA, curB, curC), (newA, newB, newC))
        write_text(path, md2)

    return path
