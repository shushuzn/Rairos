"""Research Game Mode — badges and progression system.

Badges:
  - Contradiction Hunter: 3+ contradiction pairs detected
  - Gap Extractor: 10+ capsules in Gene Pool
  - Evolution Master: 1+ capsule that has been evolved
  - Bold Explorer: 5+ bold hypothesis capsules
  - Rigor Rater: 10+ papers with rigor scores
  - Paradigm Sentinel: paradigm concentration alert triggered
"""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
from typing import List, Optional

CAPSULES_PATH = Path.home() / ".ai_research_os" / "gene_pool" / "capsules.json"
BADGES_PATH = Path.home() / ".ai_research_os" / "badges.json"


@dataclass
class Badge:
    id: str
    name: str
    description: str
    icon: str
    earned: bool = False
    earned_at: Optional[str] = None


def _load_capsules() -> List[dict]:
    if not CAPSULES_PATH.exists():
        return []
    return json.loads(CAPSULES_PATH.read_text(encoding="utf-8")).get("capsules", [])  # type: ignore[no-any-return]


def _load_badges() -> dict:
    if not BADGES_PATH.exists():
        return {}
    return json.loads(BADGES_PATH.read_text(encoding="utf-8"))  # type: ignore[no-any-return]


def _save_badges(badges: dict) -> None:
    BADGES_PATH.parent.mkdir(parents=True, exist_ok=True)
    BADGES_PATH.write_text(json.dumps(badges, indent=2, ensure_ascii=False), encoding="utf-8")


def _check_contradiction_hunter() -> bool:
    try:
        from llm.contradiction_heatmap import compute_paper_contradictions

        result = compute_paper_contradictions()  # type: ignore[no-any-return]
        total = sum(v.get("count", 0) for v in result.values())  # type: ignore[no-any-return]
        return total >= 3  # type: ignore[no-any-return]
    except Exception:
        return False


def _check_gap_extractor() -> bool:
    capsules = _load_capsules()
    active = [c for c in capsules if c.get("status") in ("active", "")]
    return len(active) >= 10


def _check_evolution_master() -> bool:
    capsules = _load_capsules()
    return any(c.get("evolved_from") or c.get("source_cap_id") for c in capsules)


def _check_bold_explorer() -> bool:
    capsules = _load_capsules()
    bold_types = {"theoretical_gap"}
    bold_polarity = {"negative"}
    count = 0
    for c in capsules:
        gap_type = c.get("action_gap_type", "") or c.get("trigger_gap_type", "")
        polarity = c.get("polarity", "positive")
        if gap_type in bold_types or polarity in bold_polarity:
            count += 1
    return count >= 5


def _check_rigor_rater() -> bool:
    # Track scored papers via a flag file
    flag = Path.home() / ".ai_research_os" / ".rigor_rated"
    if not flag.exists():
        return False
    count = int(flag.read_text().strip()) if flag.read_text().strip().isdigit() else 0
    return count >= 10


def _check_paradigm_sentinel() -> bool:
    from llm.paradigm_monitor import check_paradigm_concentration

    try:
        result = check_paradigm_concentration()  # type: ignore[no-any-return]
        return result.get("alert_triggered", False)  # type: ignore[no-any-return]
    except Exception:
        return False


def compute_badges() -> List[Badge]:
    checks = {
        "contradiction_hunter": (
            "Contradiction Hunter",
            "Detect 3+ contradiction pairs",
            "🎯",
            _check_contradiction_hunter,
        ),
        "gap_extractor": (
            "Gap Extractor",
            "Build Gene Pool to 10+ capsules",
            "🧬",
            _check_gap_extractor,
        ),
        "evolution_master": (
            "Evolution Master",
            "Have 1 capsule evolved",
            "🔄",
            _check_evolution_master,
        ),
        "bold_explorer": (
            "Bold Explorer",
            "Collect 5 bold hypothesis capsules",
            "🔴",
            _check_bold_explorer,
        ),
        "rigor_rater": (
            "Rigor Rater",
            "Score 10+ papers for research rigor",
            "🏆",
            _check_rigor_rater,
        ),
        "paradigm_sentinel": (
            "Paradigm Sentinel",
            "Trigger a paradigm concentration alert",
            "⚠️",
            _check_paradigm_sentinel,
        ),
    }

    saved = _load_badges()
    badges: List[Badge] = []

    for bid, (name, desc, icon, check_fn) in checks.items():
        earned = check_fn()
        earned_at = saved.get(bid, {}).get("earned_at") if earned else None
        if earned and not earned_at:
            from datetime import datetime

            earned_at = datetime.now().isoformat()
            saved[bid] = {"earned_at": earned_at}
        if earned:
            saved[bid] = {"earned_at": earned_at or saved.get(bid, {}).get("earned_at")}
        badges.append(
            Badge(
                id=bid, name=name, description=desc, icon=icon, earned=earned, earned_at=earned_at
            )
        )

    _save_badges(saved)
    return badges


def render_game_mode_html(badges: Optional[List[Badge]] = None) -> str:
    if badges is None:
        badges = compute_badges()

    earned = [b for b in badges if b.earned]
    locked = [b for b in badges if not b.earned]

    lines = ['<div class="game-mode">']
    lines.append("<h3>🎮 Research Game Mode</h3>")
    lines.append(
        f"<p style='font-size:13px;color:#A89E8C;margin-bottom:20px'>{len(earned)}/{len(badges)} badges earned</p>"
    )

    if earned:
        lines.append("<div class='badge-grid'>")
        for b in earned:
            lines.append(
                f"<div class='badge-card earned'>"
                f"<div class='badge-icon'>{b.icon}</div>"
                f"<div class='badge-name'>{b.name}</div>"
                f"<div class='badge-desc'>{b.description}</div>"
                f"</div>"
            )
        lines.append("</div>")

    if locked:
        lines.append(
            "<div style='margin-top:16px;font-size:12px;color:#A89E8C;text-transform:uppercase;letter-spacing:0.5px;margin-bottom:8px'>Locked</div>"
        )
        lines.append("<div class='badge-grid'>")
        for b in locked:
            lines.append(
                f"<div class='badge-card locked'>"
                f"<div class='badge-icon' style='opacity:0.3'>{b.icon}</div>"
                f"<div class='badge-name' style='color:#A89E8C'>{b.name}</div>"
                f"<div class='badge-desc' style='color:#C0B8AE'>{b.description}</div>"
                f"</div>"
            )
        lines.append("</div>")

    lines.append("<style>")
    lines.append(
        ".badge-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(160px, 1fr)); gap: 10px; }"
    )
    lines.append(".badge-card { border-radius: 8px; padding: 14px; text-align: center; }")
    lines.append(
        ".badge-card.earned { border: 2px solid #6B8FB5; background: rgba(107,143,181,0.08); }"
    )
    lines.append(
        ".badge-card.locked { border: 1px dashed #A89E8C; background: rgba(168,158,140,0.04); }"
    )
    lines.append(".badge-icon { font-size: 28px; margin-bottom: 6px; }")
    lines.append(
        ".badge-name { font-weight: 700; font-size: 13px; margin-bottom: 4px; color: #2a2a2a; }"
    )
    lines.append(".badge-desc { font-size: 11px; color: #7a7570; line-height: 1.4; }")
    lines.append("</style>")
    lines.append("</div>")
    return "\n".join(lines)
