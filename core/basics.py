"""Basic utilities."""
import os

import orjson
import re
from functools import lru_cache
from pathlib import Path

# Canonical research tree directory names (in display order)
from core._constants import DEFAULT_RESEARCH_DIRS


def _get_config_path() -> Path:
    """Path to user config file (~/.ai_research_os/categories.json)."""
    return Path(os.path.expanduser("~/.ai_research_os/categories.json"))


@lru_cache(maxsize=8)
def get_research_dirs() -> list:
    """
    Return the list of research tree directory names.
    Loads from ~/.ai_research_os/categories.json if it exists and is valid.
    Falls back to DEFAULT_RESEARCH_DIRS.
    """
    cfg = _get_config_path()
    if cfg.exists():
        try:
            data = orjson.loads(cfg.read_bytes())
            if isinstance(data, list) and data:
                return data
        except Exception:
            pass
    return list(DEFAULT_RESEARCH_DIRS)


def get_default_concept_dir() -> str:
    """
    Return the default directory for C-Notes (concept notes).
    Conventionally the second entry (after Radar). Falls back to '01-Foundations'.
    """
    dirs = get_research_dirs()
    # Convention: index 0 is Radar, index 1 is the concept/concept note directory
    if len(dirs) > 1:
        return dirs[1]  # type: ignore[no-any-return]
    return "01-Foundations"


def get_default_radar_dir() -> str:
    """
    Return the default Radar directory (index 0).
    Falls back to '00-Radar'.
    """
    dirs = get_research_dirs()
    return dirs[0] if dirs else "00-Radar"


_RE_SPACES = re.compile(r" {2,}")
_RE_NONWORD = re.compile(r"[^\w\s\-]")
_RE_DASHES = re.compile(r"-{2,}")


def slugify_title(title: str, max_len: int = 80) -> str:
    if not title:
        return "Paper"
    t = title.strip()
    t = _RE_SPACES.sub(" ", t)
    t = _RE_NONWORD.sub("", t)
    t = t.replace(" ", "-")
    t = _RE_DASHES.sub("-", t).strip("-_")
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
    root.mkdir(parents=True, exist_ok=True)
    for d in get_research_dirs():
        (root / d).mkdir(parents=True, exist_ok=True)
