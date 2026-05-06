"""Skill discovery — scan directories for SKILL.md files and parse frontmatter.

Implements the DeepSeek-TUI / Claude Code skill discovery pattern:
- Scan .claude/skills/ (project) and ~/.claude/skills/ (user) for SKILL.md
- Parse YAML frontmatter to extract name + description
- Return flat list of discovered skills for agent tool registration

Usage:
    from research_loop.skill_discovery import discover_skills, get_skill_by_name
    skills = discover_skills()
    for s in skills:
        print(f"{s['name']}: {s['description'][:60]}")
"""

from __future__ import annotations

import re
import shutil
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Dict, List, Optional


SKILL_FILENAME = "SKILL.md"
SKILL_MARKER = "---"

# Cached mtime of skill dirs for hot-reload detection
_skill_dir_mtimes: Dict[Path, float] = {}


def reload_skills(
    project_skills_dir: Optional[Path] = None,
    user_skills_dir: Optional[Path] = None,
) -> List[Skill]:
    """Re-scan skill directories, returning fresh list of skills.

    Compares directory mtimes against cached values to detect changes.
    """
    if project_skills_dir is None:
        project_skills_dir = Path(__file__).parent.parent / ".claude" / "skills"
    if user_skills_dir is None:
        user_skills_dir = Path.home() / ".claude" / "skills"
    else:
        user_skills_dir = Path(user_skills_dir).expanduser()

    dirs = [p for p in [project_skills_dir, user_skills_dir] if p.exists()]
    changed = False
    for d in dirs:
        mtime = d.stat().st_mtime
        if _skill_dir_mtimes.get(d) != mtime:
            _skill_dir_mtimes[d] = mtime
            changed = True

    if changed:
        return discover_skills(project_skills_dir, user_skills_dir)
    return []


@dataclass
class Skill:
    """A discovered skill with metadata."""
    name: str
    description: str
    path: Path
    dir: Path  # directory containing SKILL.md

    def to_dict(self) -> Dict[str, Any]:
        return {
            "name": self.name,
            "description": self.description,
            "path": str(self.path),
            "dir": str(self.dir),
        }


def _parse_frontmatter(content: str) -> Dict[str, str]:
    """Parse YAML frontmatter from SKILL.md content.

    Returns dict with 'name' and 'description' keys.
    Returns empty dict if frontmatter is missing or malformed.
    """
    if not content.startswith(SKILL_MARKER):
        return {}
    # Find closing ---
    end_match = re.search(r"^---$", content[3:], re.MULTILINE)
    if not end_match:
        return {}
    yaml_text = content[3:end_match.start() + 3]
    result: Dict[str, str] = {}
    for line in yaml_text.splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        if ":" not in line:
            continue
        key, _, value = line.partition(":")
        key = key.strip()
        value = value.strip().strip('"').strip("'")
        if key in ("name", "description"):
            result[key] = value
    return result


def _is_skill_dir(path: Path) -> bool:
    """Return True if path is a directory containing a SKILL.md file."""
    return path.is_dir() and (path / SKILL_FILENAME).exists()


def _discover_in_dir(base: Path) -> List[Skill]:
    """Scan base directory for skill directories and parse their SKILL.md."""
    if not base.exists():
        return []
    skills: List[Skill] = []
    for item in base.iterdir():
        if not item.is_dir():
            continue
        skill_md = item / SKILL_FILENAME
        if not skill_md.exists():
            continue
        try:
            text = skill_md.read_text(encoding="utf-8", errors="replace")
        except Exception:
            continue
        fm = _parse_frontmatter(text)
        name = fm.get("name", item.name)
        desc = fm.get("description", "")
        skills.append(Skill(name=name, description=desc, path=skill_md, dir=item))
    return skills


def discover_skills(
    project_skills_dir: Optional[Path] = None,
    user_skills_dir: Optional[Path] = None,
) -> List[Skill]:
    """Discover all skills from project and user skill directories.

    Searches in order:
    1. project_skills_dir / .claude/skills/  (project-scoped)
    2. user_skills_dir  / .claude/skills/     (user-wide, expanded ~)
    3. Also scans project_skills_dir directly if it's a .claude/skills dir itself

    Args:
        project_skills_dir: Project root (default: repo root via __file__../../)
        user_skills_dir: User home (default: ~/.claude/skills/)

    Returns:
        List of Skill objects, sorted by name
    """
    discovered: Dict[str, Skill] = {}

    # 1. Project skills
    if project_skills_dir is None:
        project_skills_dir = Path(__file__).parent.parent / ".claude" / "skills"
    for skill in _discover_in_dir(project_skills_dir):
        discovered[skill.name] = skill

    # 2. User skills (~ expansion)
    if user_skills_dir is None:
        user_skills_dir = Path.home() / ".claude" / "skills"
    else:
        user_skills_dir = Path(user_skills_dir).expanduser()
    for skill in _discover_in_dir(user_skills_dir):
        if skill.name not in discovered:
            discovered[skill.name] = skill

    return sorted(discovered.values(), key=lambda s: s.name)


def get_skill_by_name(name: str, skills: Optional[List[Skill]] = None) -> Optional[Skill]:
    """Find a skill by exact name. Uses provided list or re-discovers if None."""
    if skills is None:
        skills = discover_skills()
    for s in skills:
        if s.name == name:
            return s
    return None


def match_skills(query: str, skills: Optional[List[Skill]] = None) -> List[Skill]:
    """Match skills by query string (simple keyword match in name + description).

    Returns skills sorted by relevance (name match > description match).
    """
    if skills is None:
        skills = discover_skills()
    q = query.lower()
    scored: List[tuple[int, int, Skill]] = []
    for s in skills:
        name_lower = s.name.lower()
        desc_lower = s.description.lower()
        if q in name_lower:
            score = 2 if q in desc_lower else 3  # name match wins
        elif q in desc_lower:
            score = 1
        else:
            continue
        scored.append((score, name_lower.find(q), s))
    scored.sort(key=lambda x: (-x[0], x[1]))
    return [s for _, _, s in scored]


def list_skill_names() -> List[str]:
    """Quick helper — return just skill names as strings."""
    return [s.name for s in discover_skills()]


# ─── CLI helper ────────────────────────────────────────────────────────────────


def main():
    """List all discovered skills (for debugging / CLI)."""
    skills = discover_skills()
    if not skills:
        print("No skills discovered.")
        print("Create skills as .claude/skills/<name>/SKILL.md")
        return
    print(f"Discovered {len(skills)} skill(s):\n")
    for s in skills:
        print(f"  {s.name}")
        print(f"    {s.description[:80]}")
        print(f"    → {s.path}")
        print()


if __name__ == "__main__":
    main()
