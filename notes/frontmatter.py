"""Frontmatter parsing for markdown notes."""
import re
from typing import Any, Dict, List


def parse_frontmatter(md: str) -> Dict[str, Any]:
    out: Dict[str, Any] = {}
    lines = md.splitlines()
    i = 0
    while i < len(lines):
        line = lines[i]
        if line.strip() == "------------------":
            break
        m = re.match(r"^\s*([A-Za-z0-9_\-]+)\s*:\s*(.*)\s*$", line)
        if m:
            key = m.group(1).strip()
            val = m.group(2).strip()
            # Check if next lines are YAML list items
            if val == "" and i + 1 < len(lines) and re.match(r"^\s+-\s+", lines[i + 1]):
                items = []
                j = i + 1
                while j < len(lines):
                    item_line = lines[j]
                    item_m = re.match(r"^\s+-\s+(.*)\s*$", item_line)
                    if not item_m:
                        break
                    items.append(item_m.group(1).strip())
                    j += 1
                out[key] = items
                i = j - 1
            else:
                out[key] = val
        i += 1
    return out


def parse_tags_from_frontmatter(fm: Dict[str, Any]) -> List[str]:
    raw = fm.get("tags", "")
    # Handle Python list directly (e.g., ["LLM", "RAG"])
    if isinstance(raw, list):
        return [str(t).strip() for t in raw if str(t).strip()]
    raw = str(raw).strip()
    if not raw:
        return []
    # Handle comma-separated string (e.g. "LLM,Agent,RAG")
    if "," in raw and not raw.startswith("["):
        return [t.strip() for t in raw.split(",") if t.strip()]
    m = re.match(r"^\[(.*)\]$", raw)
    if not m:
        return []
    inner = m.group(1).strip()
    if not inner:
        return []
    return [t.strip() for t in inner.split(",") if t.strip()]


def parse_date_from_frontmatter(fm: Dict[str, str]) -> str:
    import warnings
    d = fm.get("date", "").strip()
    if not d:
        return ""
    if re.fullmatch(r"\d{4}-\d{2}-\d{2}", d):
        return d
    warnings.warn(f"Unrecognized date format in frontmatter: {d!r}")
    return d
