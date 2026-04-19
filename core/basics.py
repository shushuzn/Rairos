"""Basic utilities."""
import re
from pathlib import Path


def slugify_title(title: str, max_len: int = 80) -> str:
    if not title:
        return "Paper"
    t = title.strip()
    t = re.sub(r"\s+", " ", t)
    t = re.sub(r"[^\w\s\-]", "", t, flags=re.UNICODE)
    t = t.replace(" ", "-")
    t = re.sub(r"-{2,}", "-", t).strip("-_")
    if len(t) > max_len:
        t = t[:max_len].rstrip("-_")
    return t or "Paper"


def safe_uid(s: str) -> str:
    return re.sub(r"[^\w\.-]+", "_", s.strip())


def read_text(p: Path) -> str:
    return p.read_text(encoding="utf-8") if p.exists() else ""


def write_text(p: Path, content: str) -> None:
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(content, encoding="utf-8")


def ensure_research_tree(root: Path) -> None:
    dirs = [
        "00-Radar",
        "01-Foundations",
        "02-Models",
        "03-Training",
        "04-Scaling",
        "05-Alignment",
        "06-Agents",
        "07-Infrastructure",
        "08-Optimization",
        "09-Evaluation",
        "10-Applications",
        "11-Future-Directions",
    ]
    root.mkdir(parents=True, exist_ok=True)
    for d in dirs:
        (root / d).mkdir(parents=True, exist_ok=True)
